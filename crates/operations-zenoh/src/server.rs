//! The Gecko side: serves Start, Get, and streaming Observe from one [`Acceptor`].

use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use operations::{
    Acceptor, Authorizer, CallerContext, DecisionLog, Executor, GetRequest, GetResponse,
    InputError, ObserveRequest, ObserveResponse, Poll, ServiceError, StartRequest, StartResponse,
    Storage, Violation,
};
use zenoh::query::Query;
use zenoh::{Session, Wait};

use crate::keys::Keys;
use crate::wire::{EndReason, ObserveFrame};

/// Server settings. The caller identity is the adapter's own configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Zenoh endpoints to listen on, such as `tcp/127.0.0.1:7447`.
    pub listen: Vec<String>,
    /// Key prefix.
    pub keys: Keys,
    /// Trusted caller context applied to every request (restricted deployment).
    pub caller: CallerContext,
    /// Longest an Observe stream stays open before ending with `lease`.
    pub observe_lease: Duration,
    /// Most Observe streams served at once; further ones get a typed error.
    pub max_observers: usize,
    /// Test hook: wait this long after committing a Start before replying, to
    /// make a caller time out on an accepted request.
    pub start_reply_delay: Duration,
    /// Where to send a one-line description of each served request, stream
    /// end, and dispatch failure. Without it, only dispatch failures are
    /// reported, on stderr.
    pub events: Option<EventSink>,
}

/// Receives the server's one-line event descriptions (see [`ServerConfig::events`]).
///
/// Called from the serving threads; it must not block for long.
#[derive(Clone)]
pub struct EventSink(Arc<dyn Fn(String) + Send + Sync>);

impl EventSink {
    /// A sink that passes each line to `f`.
    pub fn new(f: impl Fn(String) + Send + Sync + 'static) -> Self {
        Self(Arc::new(f))
    }
}

impl std::fmt::Debug for EventSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("EventSink")
    }
}

/// The serde name of a value, such as `identifier_conflict`.
fn name<T: serde::Serialize>(value: &T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        _ => "?".to_string(),
    }
}

fn describe_start(response: &StartResponse) -> String {
    match response {
        StartResponse::Accepted { operation, .. } => format!("accepted {}", operation.id),
        StartResponse::Rejected(rejection) => format!("rejected({})", name(&rejection.code)),
        StartResponse::InvalidInput(error) => format!("invalid_input: {}", error.message),
        StartResponse::Error(error) => format!("error: {}", error.message),
    }
}

fn describe_get(response: &GetResponse) -> String {
    match response {
        GetResponse::Report(report) => {
            format!("report {} seq {}", name(&report.state), report.sequence)
        }
        GetResponse::Unavailable(_) => "unavailable".to_string(),
        GetResponse::InvalidInput(error) => format!("invalid_input: {}", error.message),
        GetResponse::Error(error) => format!("error: {}", error.message),
    }
}

fn describe_observe(response: &ObserveResponse) -> String {
    match response {
        ObserveResponse::Attached {
            current,
            sequence,
            gap,
        } => {
            let mut text = format!("attached at seq {sequence}");
            if current.is_none() {
                text.push_str(", nothing redelivered");
            }
            if let Some(gap) = gap {
                text.push_str(&format!(", gap {}..={}", gap.first_missed, gap.last_missed));
            }
            text
        }
        ObserveResponse::Unavailable(_) => "unavailable".to_string(),
        ObserveResponse::InvalidInput(error) => format!("invalid_input: {}", error.message),
        ObserveResponse::Error(error) => format!("error: {}", error.message),
    }
}

impl ServerConfig {
    /// Settings for `caller` listening on `listen`, with the default prefix.
    pub fn new(listen: Vec<String>, caller: CallerContext) -> Self {
        Self {
            listen,
            keys: Keys::default(),
            caller,
            observe_lease: Duration::from_secs(20),
            max_observers: 64,
            start_reply_delay: Duration::ZERO,
            events: None,
        }
    }
}

/// How often an idle Observe stream checks its lease and the Core stream.
const POLL: Duration = Duration::from_millis(50);

struct Shared<A, L, S> {
    core: Arc<Acceptor<A, L, S>>,
    executor: Mutex<Box<dyn Executor + Send>>,
    config: ServerConfig,
    stop: AtomicBool,
    observers: AtomicUsize,
    start_delay_ms: AtomicU64,
    observe_paused: AtomicBool,
    /// Observe streams that have seen the pause and stopped reading Core.
    paused_streams: AtomicUsize,
}

/// A running server. Dropping it stops serving and closes the session.
pub struct Server<A, L, S> {
    shared: Arc<Shared<A, L, S>>,
    session: Option<Session>,
    threads: Vec<JoinHandle<()>>,
}

fn json<T: serde::Serialize>(value: &T) -> Vec<u8> {
    serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec())
}

fn input_error(field: &str, message: String) -> InputError {
    InputError {
        message: message.clone(),
        violations: vec![Violation::new(field, message)],
    }
}

impl<A, L, S> Server<A, L, S>
where
    A: Authorizer + Send + 'static,
    L: DecisionLog + Send + 'static,
    S: Storage + 'static,
{
    /// Opens a peer session that listens on the configured endpoints and
    /// serves `core`. Created executions are handed to `executor`.
    ///
    /// The `core` must give out `config.keys.address(id)` as observation
    /// address for clients to find the stream.
    ///
    /// # Errors
    ///
    /// Any Zenoh error opening the session or declaring a queryable.
    pub fn open(
        core: Arc<Acceptor<A, L, S>>,
        executor: Box<dyn Executor + Send>,
        config: ServerConfig,
    ) -> Result<Self, zenoh::Error> {
        let mut zconfig = zenoh::Config::default();
        zconfig.insert_json5("mode", "\"peer\"")?;
        zconfig.insert_json5("scouting/multicast/enabled", "false")?;
        zconfig.insert_json5("scouting/gossip/enabled", "false")?;
        let endpoints = serde_json::to_string(&config.listen)?;
        zconfig.insert_json5("listen/endpoints", &endpoints)?;
        let session = zenoh::open(zconfig).wait()?;

        let config_delay = config.start_reply_delay;
        let shared = Arc::new(Shared {
            core,
            executor: Mutex::new(executor),
            config,
            stop: AtomicBool::new(false),
            observers: AtomicUsize::new(0),
            observe_paused: AtomicBool::new(false),
            paused_streams: AtomicUsize::new(0),
            start_delay_ms: AtomicU64::new(
                u64::try_from(config_delay.as_millis()).unwrap_or(u64::MAX),
            ),
        });
        let keys = shared.config.keys.clone();
        let mut threads = Vec::new();

        let start = session.declare_queryable(keys.start()).wait()?;
        let report = session.declare_queryable(keys.report_pattern()).wait()?;
        let observe = session.declare_queryable(keys.observe_pattern()).wait()?;

        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = start.recv() {
                let s = Arc::clone(&s);
                thread::spawn(move || s.serve_start(&query));
            }
        }));
        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = report.recv() {
                s.serve_get(&query);
            }
        }));
        let s = Arc::clone(&shared);
        threads.push(thread::spawn(move || {
            while let Ok(query) = observe.recv() {
                if s.observers.fetch_add(1, Ordering::SeqCst) >= s.config.max_observers {
                    s.observers.fetch_sub(1, Ordering::SeqCst);
                    s.event(|| "observe refused: too many observers".to_string());
                    let response = ObserveResponse::Error(ServiceError {
                        message: "too many observers".to_string(),
                    });
                    s.send(&query, &ObserveFrame::Response { response });
                    continue;
                }
                let s = Arc::clone(&s);
                thread::spawn(move || {
                    s.serve_observe(&query);
                    s.observers.fetch_sub(1, Ordering::SeqCst);
                });
            }
        }));
        // The queryables are owned by their threads; they end when the session closes.
        Ok(Self {
            shared,
            session: Some(session),
            threads,
        })
    }

    /// Changes the Start reply delay test hook while running.
    pub fn set_start_reply_delay(&self, delay: Duration) {
        let millis = u64::try_from(delay.as_millis()).unwrap_or(u64::MAX);
        self.shared.start_delay_ms.store(millis, Ordering::SeqCst);
    }

    /// Test hook: while paused, attached Observe streams stop reading Core's
    /// stream, so an observer falls behind deterministically. Streams still end
    /// at their lease. Unpausing resumes delivery of what was buffered.
    #[cfg(feature = "test-support")]
    pub fn set_observe_paused(&self, paused: bool) {
        self.shared.observe_paused.store(paused, Ordering::SeqCst);
    }

    /// Test hook: how many attached Observe streams have seen the pause and
    /// stopped reading Core. Wait for it instead of sleeping after pausing.
    #[cfg(feature = "test-support")]
    #[must_use]
    pub fn observe_paused_streams(&self) -> usize {
        self.shared.paused_streams.load(Ordering::SeqCst)
    }

    /// Stops serving and closes the Zenoh session.
    pub fn close(mut self) {
        self.shutdown();
    }

    fn shutdown(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(session) = self.session.take() {
            let _ = session.close().wait();
        }
        for thread in self.threads.drain(..) {
            let _ = thread.join();
        }
    }
}

impl<A, L, S> Drop for Server<A, L, S> {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::SeqCst);
        if let Some(session) = self.session.take() {
            let _ = session.close().wait();
        }
    }
}

impl<A, L, S> Shared<A, L, S>
where
    A: Authorizer + Send + 'static,
    L: DecisionLog + Send + 'static,
    S: Storage + 'static,
{
    fn send<T: serde::Serialize>(&self, query: &Query, frame: &T) -> bool {
        query
            .reply(query.key_expr().clone(), json(frame))
            .wait()
            .is_ok()
    }

    /// Reports one event line to the configured sink, if any.
    fn event(&self, line: impl FnOnce() -> String) {
        if let Some(sink) = &self.config.events {
            (sink.0)(line());
        }
    }

    fn serve_start(&self, query: &Query) {
        let parsed = match query.payload() {
            None => Err("the request has no payload".to_string()),
            Some(payload) => serde_json::from_slice::<StartRequest>(&payload.to_bytes())
                .map_err(|e| format!("malformed Start request: {e}")),
        };
        let request_id = parsed
            .as_ref()
            .map_or_else(|_| "-".to_string(), |r| r.request_id.to_string());
        let response = match parsed {
            Err(message) => StartResponse::InvalidInput(input_error("payload", message)),
            Ok(request) => {
                let outcome = self.core.start(&self.config.caller, &request);
                if let Some(dispatch) = outcome.dispatch {
                    // The commit is done; hand the new execution to the executor once.
                    // If the executor is unusable, the execution never reaches it and fails.
                    let result = match self.executor.lock() {
                        Ok(mut executor) => self.core.dispatch(dispatch, executor.as_mut()),
                        Err(_) => self.core.abandon(dispatch, "the executor is unavailable"),
                    };
                    if let Err(error) = result {
                        // Only these errors can leave the execution `pending`.
                        let note = match error {
                            operations::DispatchError::Unrecorded { .. }
                            | operations::DispatchError::Unavailable => {
                                "; the execution may stay pending"
                            }
                            _ => "",
                        };
                        let line = format!("dispatch failed: {error:?}{note}");
                        match &self.config.events {
                            Some(sink) => (sink.0)(line),
                            None => eprintln!("operations-zenoh: {line}"),
                        }
                    }
                }
                outcome.response
            }
        };
        let delay = self.start_delay_ms.load(Ordering::SeqCst);
        if delay > 0 {
            self.event(|| format!("start {request_id}: holding the reply for {delay} ms"));
            thread::sleep(Duration::from_millis(delay));
        }
        let sent = self.send(query, &response);
        self.event(|| {
            format!(
                "start {request_id} -> {}{}",
                describe_start(&response),
                if sent { "" } else { " (reply not sent)" }
            )
        });
    }

    fn serve_get(&self, query: &Query) {
        let response = match self.config.keys.operation_id(query.key_expr().as_str()) {
            Some(operation_id) => self
                .core
                .get(&self.config.caller, &GetRequest { operation_id }),
            None => GetResponse::InvalidInput(input_error(
                "key",
                "the key does not contain an operation id".to_string(),
            )),
        };
        self.send(query, &response);
        self.event(|| {
            format!(
                "get {} -> {}",
                query.key_expr().as_str(),
                describe_get(&response)
            )
        });
    }

    fn serve_observe(&self, query: &Query) {
        let reject = |response: ObserveResponse| {
            self.event(|| {
                format!(
                    "observe {} -> {}",
                    query.selector(),
                    describe_observe(&response)
                )
            });
            self.send(query, &ObserveFrame::Response { response });
        };
        let Some(operation_id) = self.config.keys.operation_id(query.key_expr().as_str()) else {
            return reject(ObserveResponse::InvalidInput(input_error(
                "key",
                "the key does not contain an operation id".to_string(),
            )));
        };
        let after_sequence = match query.parameters().get("after").map(str::parse::<u64>) {
            None => None,
            Some(Ok(after)) => Some(after),
            Some(Err(e)) => {
                return reject(ObserveResponse::InvalidInput(input_error(
                    "after_sequence",
                    format!("not a sequence number: {e}"),
                )));
            }
        };
        let outcome = self.core.observe(
            &self.config.caller,
            &ObserveRequest {
                operation_id,
                after_sequence,
            },
        );
        let attached = matches!(outcome.response, ObserveResponse::Attached { .. });
        self.event(|| {
            format!(
                "observe {operation_id} after={} -> {}",
                after_sequence.map_or_else(|| "-".to_string(), |a| a.to_string()),
                describe_observe(&outcome.response)
            )
        });
        let end = |reason: EndReason| {
            self.send(query, &ObserveFrame::End { reason });
            self.event(|| format!("observe {operation_id}: stream ended {}", name(&reason)));
        };
        if !self.send(
            query,
            &ObserveFrame::Response {
                response: outcome.response,
            },
        ) || !attached
        {
            return;
        }
        let Some(stream) = outcome.stream else {
            // Attached to a terminal execution: nothing more will be published.
            end(EndReason::Terminal);
            return;
        };
        let lease_ends = Instant::now() + self.config.observe_lease;
        let mut pause = PauseMark {
            count: &self.paused_streams,
            paused: false,
        };
        loop {
            // A paused stream reads nothing from Core but keeps its stop and
            // lease checks, exactly as an idle one.
            let poll = if pause.set(self.observe_paused.load(Ordering::SeqCst)) {
                thread::sleep(POLL);
                Poll::Empty
            } else {
                stream.next_timeout(POLL)
            };
            let frame = match poll {
                Poll::Snapshot(report) => ObserveFrame::Snapshot { report },
                Poll::Ended(reason) => {
                    end(reason.into());
                    return;
                }
                Poll::Empty => {
                    if self.stop.load(Ordering::SeqCst) {
                        return;
                    }
                    if Instant::now() >= lease_ends {
                        end(EndReason::Lease);
                        return;
                    }
                    continue;
                }
            };
            if !self.send(query, &frame) {
                // The observer is gone; dropping the stream detaches it.
                self.event(|| format!("observe {operation_id}: observer gone"));
                return;
            }
        }
    }
}

/// Counts one Observe stream in `count` while it honours the pause test hook.
struct PauseMark<'a> {
    count: &'a AtomicUsize,
    paused: bool,
}

impl PauseMark<'_> {
    /// Records whether the stream is paused now and returns it.
    fn set(&mut self, paused: bool) -> bool {
        if paused != self.paused {
            if paused {
                self.count.fetch_add(1, Ordering::SeqCst);
            } else {
                self.count.fetch_sub(1, Ordering::SeqCst);
            }
            self.paused = paused;
        }
        paused
    }
}

impl Drop for PauseMark<'_> {
    fn drop(&mut self) {
        self.set(false);
    }
}
