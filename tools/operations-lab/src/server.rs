//! The lab Gecko: Core, the Zenoh server, a cuttable proxy, and a stub executor
//! whose behaviour is changed at run time. The terminal UI only calls this.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use operations::report::Progress;
use operations::testing::PermitAllForTests;
use operations::testing::contract_fixtures::{caller, catalog};
use operations::{
    Acceptor, CallerContext, EventKind, ExecutionToken, Executor, ExecutorEvent, GetRequest,
    GetResponse, Guard, Handoff, MemoryDecisionLog, MemoryStorage, Notified, NotifyRejection,
    OperationId, Outcome, Report, State, Storage, Value,
};
use operations_zenoh::proxy::Proxy;
use operations_zenoh::{EventSink, Keys, Server, ServerConfig};

use crate::logbook::LogBook;

/// The Core type the lab runs: permissive test policy, in-memory storage.
pub type Core = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

/// Settings fixed at start.
#[derive(Debug, Clone)]
pub struct LabServerConfig {
    /// TCP port the Gecko listens on (`127.0.0.1`).
    pub port: u16,
    /// TCP port of the cuttable proxy in front of the Gecko.
    pub proxy_port: u16,
    /// Subject of the trusted caller applied to every request.
    pub caller: String,
    /// Longest an Observe stream stays open before it ends with `lease`.
    pub observe_lease: Duration,
    /// Snapshots buffered per observer before it is ended as `lagged`.
    pub observer_capacity: usize,
}

impl Default for LabServerConfig {
    fn default() -> Self {
        Self {
            port: 7447,
            proxy_port: 7448,
            caller: "lab-caller".to_string(),
            observe_lease: Duration::from_secs(20),
            observer_capacity: 4,
        }
    }
}

/// How the stub executor answers a handoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckMode {
    /// Acknowledge at once: the execution becomes `running`.
    Acknowledge,
    /// Acknowledge after [`Script::slow_ack`]; the execution is `pending` meanwhile.
    Slow,
    /// Refuse before any side effect: the execution becomes `failed`.
    Refuse,
    /// Lose the acknowledgement: the execution becomes `unknown`.
    Lose,
}

impl AckMode {
    /// The next mode in the cycle the UI offers.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Acknowledge => Self::Slow,
            Self::Slow => Self::Refuse,
            Self::Refuse => Self::Lose,
            Self::Lose => Self::Acknowledge,
        }
    }

    /// Short label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Acknowledge => "acknowledge",
            Self::Slow => "slow acknowledge",
            Self::Refuse => "refuse",
            Self::Lose => "lose acknowledgement",
        }
    }
}

/// What the executor does after the last automatic progress step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// Report `succeeded` with a result.
    Succeed,
    /// Report `failed`, keeping the last partial result.
    Fail,
    /// Do nothing: finish by hand.
    Hold,
}

impl Finish {
    /// The next choice in the cycle the UI offers.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Succeed => Self::Fail,
            Self::Fail => Self::Hold,
            Self::Hold => Self::Succeed,
        }
    }

    /// Short label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Succeed => "succeed",
            Self::Fail => "fail",
            Self::Hold => "hold (finish by hand)",
        }
    }
}

/// The stub executor's behaviour for the next handoffs. Changing it does not
/// affect executions already running their script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Script {
    /// Answer to the handoff.
    pub ack: AckMode,
    /// Delay before a slow acknowledgement.
    pub slow_ack: Duration,
    /// Automatic progress events after the acknowledgement.
    pub steps: u32,
    /// Pause before each automatic event.
    pub interval: Duration,
    /// What happens after the last step.
    pub finish: Finish,
}

impl Default for Script {
    fn default() -> Self {
        Self {
            ack: AckMode::Acknowledge,
            slow_ack: Duration::from_secs(3),
            steps: 3,
            interval: Duration::from_secs(1),
            finish: Finish::Succeed,
        }
    }
}

/// Start reply delays the UI cycles through. The client default timeout is 2 s.
pub const REPLY_DELAYS_MS: [u64; 4] = [0, 1000, 3000, 6000];
/// Intervals between automatic events the UI cycles through.
pub const INTERVALS_MS: [u64; 4] = [200, 1000, 3000, 100];

/// One execution handed to the stub executor, as the UI shows it.
#[derive(Debug, Clone)]
pub struct Row {
    /// Execution identity.
    pub operation_id: OperationId,
    /// Times Core handed this execution to the executor (must stay 1).
    pub handoffs: u32,
    /// What the executor answered.
    pub handoff: &'static str,
    /// Executor events sent for this execution.
    pub events_sent: u64,
    /// Observers attached at Core right now.
    pub observers: usize,
    /// Latest snapshot at Core, if readable.
    pub report: Option<Report>,
}

/// Totals at Core and the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counters {
    /// Execution records committed.
    pub operations: usize,
    /// Retry mappings (`request_id` scope) committed.
    pub mappings: usize,
    /// Handoffs the executor received.
    pub handoffs: u32,
}

struct Execution {
    operation_id: OperationId,
    token: ExecutionToken,
    handoffs: u32,
    handoff: &'static str,
    next_seq: u64,
    last: Option<ExecutorEvent>,
}

struct Shared {
    core: Arc<Core>,
    caller: CallerContext,
    script: Mutex<Script>,
    /// Also serializes event sending, so numbers reach Core in order.
    executions: Mutex<Vec<Execution>>,
    log: Arc<LogBook>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn short(id: OperationId) -> String {
    id.to_string()[..8].to_string()
}

fn describe_kind(kind: &EventKind) -> String {
    match kind {
        EventKind::Ack => "ack".to_string(),
        EventKind::Progress { progress, .. } => format!("progress \"{}\"", progress.stage),
        EventKind::Terminal { outcome, .. } => format!("terminal {outcome:?}").to_lowercase(),
    }
}

fn describe_notify(result: &Result<Notified, NotifyRejection>) -> String {
    match result {
        Ok(Notified::Applied { sequence }) => format!("applied, snapshot {sequence}"),
        Ok(Notified::Duplicate { sequence }) => {
            format!("duplicate, no new snapshot (still {sequence})")
        }
        Ok(Notified::NoChange { sequence }) => format!("no change (still {sequence})"),
        Err(rejection) => format!("rejected: {rejection:?}"),
    }
}

impl Shared {
    fn report(&self, id: OperationId) -> Option<Report> {
        match self
            .core
            .get(&self.caller, &GetRequest { operation_id: id })
        {
            GetResponse::Report(report) => Some(*report),
            _ => None,
        }
    }

    fn state(&self, id: OperationId) -> Option<State> {
        self.report(id).map(|r| r.state)
    }

    /// Sends the next numbered event for `id`. `None` if the executor never
    /// received that execution.
    fn send(&self, id: OperationId, kind: EventKind) -> Option<Result<Notified, NotifyRejection>> {
        let mut executions = lock(&self.executions);
        let execution = executions.iter_mut().find(|e| e.operation_id == id)?;
        execution.next_seq += 1;
        let event = ExecutorEvent {
            token: execution.token.clone(),
            event_seq: execution.next_seq,
            kind,
        };
        let result = self.core.notify(&event);
        self.log.push(format!(
            "[exec] {} event #{} {} -> {}",
            short(id),
            event.event_seq,
            describe_kind(&event.kind),
            describe_notify(&result)
        ));
        execution.last = Some(event);
        Some(result)
    }

    /// Sends the last event of `id` again, unchanged.
    fn resend(&self, id: OperationId) -> Option<Result<Notified, NotifyRejection>> {
        let executions = lock(&self.executions);
        let event = executions
            .iter()
            .find(|e| e.operation_id == id)?
            .last
            .clone()?;
        let result = self.core.notify(&event);
        self.log.push(format!(
            "[exec] {} event #{} {} resent -> {}",
            short(id),
            event.event_seq,
            describe_kind(&event.kind),
            describe_notify(&result)
        ));
        Some(result)
    }

    fn run_script(&self, id: OperationId, script: Script) {
        // Core records the acknowledged handoff after `hand_off` returns.
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.state(id) == Some(State::Pending) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        for step in 1..=script.steps {
            thread::sleep(script.interval);
            if self.state(id).is_none_or(State::is_terminal) {
                return;
            }
            self.send(id, progress(step, script.steps));
        }
        thread::sleep(script.interval);
        if self.state(id).is_none_or(State::is_terminal) {
            return;
        }
        match script.finish {
            Finish::Succeed => {
                self.send(id, succeed(script.steps));
            }
            Finish::Fail => {
                self.send(id, fail());
            }
            Finish::Hold => self.log.push(format!(
                "[exec] {} script done; holding, finish it by hand",
                short(id)
            )),
        }
    }
}

/// A progress event for step `step` of `steps`, with a partial result.
#[must_use]
pub fn progress(step: u32, steps: u32) -> EventKind {
    EventKind::Progress {
        progress: Progress {
            stage: format!("step {step}/{steps}"),
            message: format!("lab step {step} of {steps}"),
            completed: Some(u64::from(step)),
            total: Some(u64::from(steps)),
            unit: Some("steps".to_string()),
        },
        result: Some(Value::Text(format!("partial {step}/{steps}"))),
    }
}

/// A progress event sent by hand; `n` is the event number it will use.
fn manual_progress(n: u64) -> EventKind {
    EventKind::Progress {
        progress: Progress {
            stage: format!("manual {n}"),
            message: format!("manual progress, event {n}"),
            ..Progress::default()
        },
        result: Some(Value::Text(format!("partial (manual {n})"))),
    }
}

fn succeed(steps: u32) -> EventKind {
    EventKind::Terminal {
        outcome: Outcome::Succeeded,
        result: Some(Value::Text(format!("done after {steps} steps"))),
        explanation: "the lab executor finished".to_string(),
    }
}

fn fail() -> EventKind {
    EventKind::Terminal {
        outcome: Outcome::Failed,
        result: None,
        explanation: "the lab executor was told to fail; the partial result is kept".to_string(),
    }
}

struct LabExecutor(Arc<Shared>);

impl Executor for LabExecutor {
    fn hand_off(&mut self, token: &ExecutionToken) -> Handoff {
        let shared = &self.0;
        let script = *lock(&shared.script);
        let Ok(Some(id)) = shared.core.storage().operation_for_token(token) else {
            shared
                .log
                .push("[exec] handoff of a token Core does not know; refused");
            return Handoff::Refused {
                reason: "unknown token".to_string(),
            };
        };
        if script.ack == AckMode::Slow {
            shared.log.push(format!(
                "[exec] {} handoff received; holding the acknowledgement for {} ms",
                short(id),
                script.slow_ack.as_millis()
            ));
            thread::sleep(script.slow_ack);
        }
        let (handoff, label) = match script.ack {
            AckMode::Acknowledge | AckMode::Slow => (Handoff::Acknowledged, "acknowledged"),
            AckMode::Refuse => (
                Handoff::Refused {
                    reason: "the lab executor was set to refuse".to_string(),
                },
                "refused",
            ),
            AckMode::Lose => (Handoff::Ambiguous, "ack lost"),
        };
        {
            let mut executions = lock(&shared.executions);
            match executions.iter_mut().find(|e| e.operation_id == id) {
                Some(execution) => {
                    execution.handoffs += 1;
                    execution.handoff = label;
                }
                None => executions.push(Execution {
                    operation_id: id,
                    token: token.clone(),
                    handoffs: 1,
                    handoff: label,
                    next_seq: 0,
                    last: None,
                }),
            }
        }
        shared
            .log
            .push(format!("[exec] {} handoff -> {label}", short(id)));
        if handoff == Handoff::Acknowledged {
            let shared = Arc::clone(shared);
            thread::spawn(move || shared.run_script(id, script));
        }
        handoff
    }
}

/// The running lab Gecko.
pub struct Lab {
    shared: Arc<Shared>,
    server: Server<PermitAllForTests, MemoryDecisionLog, MemoryStorage>,
    proxy: Proxy,
    config: LabServerConfig,
    reply_delay_ms: u64,
    observe_paused: bool,
}

impl Lab {
    /// Starts Core, the Zenoh server on `127.0.0.1:<port>` and the proxy on
    /// `127.0.0.1:<proxy_port>`.
    ///
    /// # Errors
    ///
    /// A description of what could not be started, such as a port in use.
    pub fn start(config: LabServerConfig, log: Arc<LogBook>) -> Result<Self, String> {
        let keys = Keys::default();
        let address_keys = keys.clone();
        let core: Arc<Core> = Arc::new(
            Acceptor::new(
                catalog(),
                Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
                MemoryStorage::new(),
            )
            .with_observation_address(move |id| address_keys.address(id))
            .with_observer_capacity(config.observer_capacity),
        );
        let caller = caller(&config.caller);
        let shared = Arc::new(Shared {
            core: Arc::clone(&core),
            caller: caller.clone(),
            script: Mutex::new(Script::default()),
            executions: Mutex::default(),
            log: Arc::clone(&log),
        });
        let endpoint = format!("tcp/127.0.0.1:{}", config.port);
        let mut server_config = ServerConfig::new(vec![endpoint.clone()], caller);
        server_config.keys = keys;
        server_config.observe_lease = config.observe_lease;
        let sink_log = Arc::clone(&log);
        server_config.events = Some(EventSink::new(move |line| {
            sink_log.push(format!("[zenoh] {line}"));
        }));
        let server = Server::open(
            core,
            Box::new(LabExecutor(Arc::clone(&shared))),
            server_config,
        )
        .map_err(|e| format!("cannot listen on {endpoint}: {e}"))?;
        let proxy = Proxy::start_at(config.proxy_port, &format!("127.0.0.1:{}", config.port))
            .map_err(|e| format!("cannot start the proxy on port {}: {e}", config.proxy_port))?;
        let lab = Self {
            shared,
            server,
            proxy,
            config,
            reply_delay_ms: 0,
            observe_paused: false,
        };
        log.push(format!(
            "[lab] Gecko on {}, proxy on {}; caller {}, observe lease {} s, observer capacity {}",
            lab.endpoint(),
            lab.proxy_endpoint(),
            lab.config.caller,
            lab.config.observe_lease.as_secs(),
            lab.config.observer_capacity
        ));
        Ok(lab)
    }

    /// Endpoint for clients connecting directly.
    #[must_use]
    pub fn endpoint(&self) -> String {
        format!("tcp/127.0.0.1:{}", self.config.port)
    }

    /// Endpoint for clients connecting through the cuttable proxy.
    #[must_use]
    pub fn proxy_endpoint(&self) -> String {
        self.proxy.endpoint()
    }

    /// The settings the lab started with.
    #[must_use]
    pub fn config(&self) -> &LabServerConfig {
        &self.config
    }

    /// The shared event log.
    #[must_use]
    pub fn log(&self) -> &LogBook {
        &self.shared.log
    }

    /// The executor behaviour for the next handoffs.
    #[must_use]
    pub fn script(&self) -> Script {
        *lock(&self.shared.script)
    }

    /// Changes the executor behaviour for the next handoffs.
    pub fn set_script(&self, script: Script) {
        let mut current = lock(&self.shared.script);
        if *current != script {
            *current = script;
            self.shared.log.push(format!(
                "[lab] executor: {}, {} steps every {} ms, then {}",
                script.ack.label(),
                script.steps,
                script.interval.as_millis(),
                script.finish.label()
            ));
        }
    }

    /// Current Start reply delay.
    #[must_use]
    pub fn reply_delay(&self) -> Duration {
        Duration::from_millis(self.reply_delay_ms)
    }

    /// Sets the delay between committing a Start and replying.
    pub fn set_reply_delay(&mut self, delay: Duration) {
        self.reply_delay_ms = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        self.server.set_start_reply_delay(delay);
        self.shared.log.push(format!(
            "[lab] Start reply delay {} ms",
            self.reply_delay_ms
        ));
    }

    /// Steps to the next of [`REPLY_DELAYS_MS`].
    pub fn cycle_reply_delay(&mut self) {
        let next = REPLY_DELAYS_MS
            .iter()
            .position(|&d| d == self.reply_delay_ms)
            .map_or(0, |i| (i + 1) % REPLY_DELAYS_MS.len());
        self.set_reply_delay(Duration::from_millis(REPLY_DELAYS_MS[next]));
    }

    /// Makes the next commit of a new execution fail.
    pub fn inject_commit_failure(&self) {
        self.shared.core.storage().inject_commit_failures(1);
        self.shared
            .log
            .push("[lab] the next commit of a new execution will fail");
    }

    /// Whether the proxy link is up.
    #[must_use]
    pub fn link_up(&self) -> bool {
        self.proxy.is_up()
    }

    /// Cuts the proxy link if it is up, restores it otherwise.
    pub fn toggle_link(&self) {
        if self.proxy.is_up() {
            self.proxy.cut();
            self.shared
                .log
                .push("[lab] proxy link CUT: proxied clients lose the connection");
        } else {
            self.proxy.restore();
            self.shared.log.push("[lab] proxy link restored");
        }
    }

    /// Whether Observe streams are paused (stop reading Core).
    #[must_use]
    pub fn observe_paused(&self) -> bool {
        self.observe_paused
    }

    /// Pauses or resumes reading Core in every Observe stream.
    pub fn toggle_observe_pause(&mut self) {
        self.observe_paused = !self.observe_paused;
        self.server.set_observe_paused(self.observe_paused);
        self.shared.log.push(if self.observe_paused {
            "[lab] observe PAUSED: streams stop reading Core; observers fall behind"
        } else {
            "[lab] observe resumed"
        });
    }

    /// Executions the executor received, oldest first, with their latest snapshot.
    #[must_use]
    pub fn rows(&self) -> Vec<Row> {
        let executions: Vec<_> = lock(&self.shared.executions)
            .iter()
            .map(|e| {
                (
                    e.operation_id,
                    e.handoffs,
                    e.handoff,
                    e.last.as_ref().map_or(0, |l| l.event_seq),
                )
            })
            .collect();
        executions
            .into_iter()
            .map(|(id, handoffs, handoff, events_sent)| Row {
                operation_id: id,
                handoffs,
                handoff,
                events_sent,
                observers: self.shared.core.observer_count(&id),
                report: self.shared.report(id),
            })
            .collect()
    }

    /// Totals at Core and the executor.
    #[must_use]
    pub fn counters(&self) -> Counters {
        let storage = self.shared.core.storage();
        Counters {
            operations: storage.operation_count(),
            mappings: storage.mapping_count(),
            handoffs: lock(&self.shared.executions)
                .iter()
                .map(|e| e.handoffs)
                .sum(),
        }
    }

    /// Sends one progress event for `id` by hand.
    pub fn send_progress(&self, id: OperationId) {
        let step = lock(&self.shared.executions)
            .iter()
            .find(|e| e.operation_id == id)
            .map_or(0, |e| e.next_seq + 1);
        self.shared.send(id, manual_progress(step));
    }

    /// Sends a terminal outcome for `id` by hand.
    pub fn send_terminal(&self, id: OperationId, outcome: Outcome) {
        let kind = match outcome {
            Outcome::Failed => fail(),
            _ => EventKind::Terminal {
                outcome,
                result: Some(Value::Text("finished by hand".to_string())),
                explanation: "finished by hand in the lab".to_string(),
            },
        };
        self.shared.send(id, kind);
    }

    /// Sends an `ack` event for `id`: late evidence for an `unknown` execution.
    pub fn send_ack(&self, id: OperationId) {
        self.shared.send(id, EventKind::Ack);
    }

    /// Sends the last event for `id` again, unchanged (a duplicate).
    pub fn resend_last(&self, id: OperationId) {
        if self.shared.resend(id).is_none() {
            self.shared
                .log
                .push(format!("[exec] {} has no event to resend", short(id)));
        }
    }

    /// Stops serving and closes the session.
    pub fn close(self) {
        self.shared.log.push("[lab] stopping");
        self.server.close();
    }
}
