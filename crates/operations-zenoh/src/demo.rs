//! The scripted local scenario of GKO-66: Start, Get, and Observe over Zenoh
//! with a trusted local caller, an in-process executor, a lost Start reply, and
//! an observation reconnect across a cut link. Run it with `make demo`.
//!
//! Topology (all on `127.0.0.1`): one Gecko session listening on a free port;
//! one client connected directly, one through a [`Proxy`] whose link is cut.
//! The executor runs in the same process as Core and is driven by the scenario.

use std::io::Write;
use std::net::TcpListener;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use operations::report::{Progress, State};
use operations::testing::PermitAllForTests;
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::{
    Acceptor, EventKind, ExecutionToken, Executor, ExecutorEvent, Follower, Gap, GetResponse,
    Guard, Handoff, MemoryDecisionLog, MemoryStorage, ObserveResponse, Operation, OperationId,
    Outcome, RejectionCode, StartResponse, Unavailable, Value,
};

use crate::proxy::Proxy;
use crate::{Client, ClientConfig, EndReason, Followed, Keys, ObserveItem, Server, ServerConfig};

/// Snapshots a follower saw: sequence, state, and any gap disclosed with it.
type Seen = Vec<(u64, State, Option<Gap>)>;

type Core = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

/// Hands every execution to the scenario's worker side by sending its token.
struct ChannelExecutor(Sender<ExecutionToken>);

impl Executor for ChannelExecutor {
    fn hand_off(&mut self, token: &ExecutionToken) -> Handoff {
        match self.0.send(token.clone()) {
            Ok(()) => Handoff::Acknowledged,
            Err(_) => Handoff::Ambiguous,
        }
    }
}

fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind")
        .local_addr()
        .expect("addr")
        .port()
}

struct Log<'a>(&'a mut dyn Write);

impl Log<'_> {
    fn say(&mut self, line: impl AsRef<str>) {
        let _ = writeln!(self.0, "{}", line.as_ref());
    }
}

fn check(condition: bool, what: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(format!("check failed: {what}"))
    }
}

fn accepted(response: &StartResponse) -> Result<(Operation, String), String> {
    match response {
        StartResponse::Accepted {
            operation,
            observation,
        } => Ok((operation.clone(), observation.0.clone())),
        other => Err(format!("expected acceptance, got {other:?}")),
    }
}

fn report(response: GetResponse) -> Result<operations::Report, String> {
    match response {
        GetResponse::Report(report) => Ok(*report),
        other => Err(format!("expected a report, got {other:?}")),
    }
}

fn event(token: &ExecutionToken, event_seq: u64, kind: EventKind) -> ExecutorEvent {
    ExecutorEvent {
        token: token.clone(),
        event_seq,
        kind,
    }
}

fn progress(stage: &str) -> EventKind {
    EventKind::Progress {
        progress: Progress {
            stage: stage.to_string(),
            message: format!("{stage} done"),
            ..Progress::default()
        },
        result: None,
    }
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < deadline {
        if ready() {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(20));
    }
    Err(format!("timed out waiting for {what}"))
}

/// Runs the scenario, writing a narrative to `out`.
///
/// # Errors
///
/// A description of the first step whose observed result differed from the
/// expected one.
pub fn run(out: &mut dyn Write) -> Result<(), String> {
    let mut log = Log(out);
    let keys = Keys::default();
    let gecko_endpoint = format!("tcp/127.0.0.1:{}", free_port());

    let address_keys = keys.clone();
    let core: Arc<Core> = Arc::new(
        Acceptor::new(
            catalog(),
            Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
            MemoryStorage::new(),
        )
        .with_observation_address(move |id| address_keys.address(id)),
    );
    let (tokens_tx, tokens): (Sender<ExecutionToken>, Receiver<ExecutionToken>) = mpsc::channel();
    let mut server_config = ServerConfig::new(vec![gecko_endpoint.clone()], caller("demo-caller"));
    server_config.observe_lease = Duration::from_secs(1);
    let server = Server::open(
        Arc::clone(&core),
        Box::new(ChannelExecutor(tokens_tx)),
        server_config,
    )
    .map_err(|e| format!("server: {e}"))?;
    log.say(format!(
        "Gecko listening on {gecko_endpoint} (Zenoh 1.10.2, peer)"
    ));

    let proxy =
        Proxy::start(gecko_endpoint.trim_start_matches("tcp/")).map_err(|e| e.to_string())?;
    let mut direct_config = ClientConfig::new(vec![gecko_endpoint]);
    direct_config.timeout = Duration::from_millis(500);
    let direct = Client::open(direct_config).map_err(|e| format!("client: {e}"))?;
    let mut proxied_config = ClientConfig::new(vec![proxy.endpoint()]);
    proxied_config.timeout = Duration::from_millis(500);
    proxied_config.observe_timeout = Duration::from_secs(3);
    let proxied = Client::open(proxied_config).map_err(|e| format!("proxied client: {e}"))?;
    log.say(format!(
        "clients: direct, and through a cuttable proxy at {}",
        proxy.endpoint()
    ));

    // 1. Accepted Start, and Get through Zenoh equals Core read directly.
    log.say("\n[1] Start accepted");
    let request = echo_request("hello");
    let (operation, address) = accepted(&direct.start(&request).map_err(|e| e.to_string())?)?;
    log.say(format!(
        "    operation {} observed at {address}",
        operation.id
    ));
    check(
        address == keys.observe(operation.id),
        "observation address is the Observe key",
    )?;
    let token = tokens
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "the executor was not handed the execution".to_string())?;
    let over_zenoh = report(direct.get(operation.id).map_err(|e| e.to_string())?)?;
    log.say(format!(
        "    Get over Zenoh: {:?}, sequence {}",
        over_zenoh.state, over_zenoh.sequence
    ));
    check(
        (over_zenoh.state, over_zenoh.sequence) == (State::Running, 2),
        "the acknowledged handoff made the execution running (sequence 2)",
    )?;
    let in_process = report(core.get(
        &caller("demo-caller"),
        &operations::GetRequest {
            operation_id: operation.id,
        },
    ))?;
    check(
        in_process == over_zenoh,
        "Core answers the same without the transport",
    )?;
    log.say("    the same report read in-process, with no transport, is identical");

    // 2. Rejections and malformed input are typed replies, not transport errors.
    log.say("\n[2] Rejected and invalid requests");
    let mut unknown = echo_request("x");
    unknown.operation_type = operations::OperationType::new("no.such.type")?;
    let response = direct.start(&unknown).map_err(|e| e.to_string())?;
    check(
        matches!(&response, StartResponse::Rejected(r) if r.code == RejectionCode::UnknownOperationType),
        "unknown operation type is a typed rejection",
    )?;
    log.say("    unknown operation type -> rejected(unknown_operation_type)");
    let mut invalid = echo_request("x");
    invalid.parameters.clear();
    let response = direct.start(&invalid).map_err(|e| e.to_string())?;
    check(
        matches!(&response, StartResponse::Rejected(r) if r.code == RejectionCode::InvalidParameters),
        "missing parameter is a typed rejection",
    )?;
    log.say("    missing required parameter -> rejected(invalid_parameters)");
    let response = direct
        .start_raw(b"not json".to_vec())
        .map_err(|e| e.to_string())?;
    check(
        matches!(response, StartResponse::InvalidInput(_)),
        "malformed payload is typed invalid input",
    )?;
    log.say("    malformed payload -> invalid_input");
    check(
        direct
            .get(OperationId::generate())
            .map_err(|e| e.to_string())?
            == GetResponse::Unavailable(Unavailable {}),
        "an unknown operation is unavailable",
    )?;
    check(
        matches!(
            direct.get_key(&format!(
                "{}/not-a-uuid/report",
                keys.start().trim_end_matches("/start")
            )),
            Ok(GetResponse::InvalidInput(_))
        ),
        "a key without an operation id is invalid input",
    )?;
    log.say("    unknown operation id -> unavailable; bad key -> invalid_input");

    // 3. Lost Start reply, then a retry with the same request_id.
    log.say("\n[3] Lost Start reply and retry with the same request_id");
    let before = core.storage().operation_count();
    let lost = echo_request("lost reply");
    server.set_start_reply_delay(Duration::from_millis(1500));
    let first = direct.start(&lost);
    check(first.is_err(), "the delayed reply is not seen: no answer")?;
    log.say(format!(
        "    first attempt: {}",
        first.err().map_or_else(String::new, |e| e.to_string())
    ));
    server.set_start_reply_delay(Duration::ZERO);
    let (retried, _) = accepted(&direct.start_retrying(&lost, 3).map_err(|e| e.to_string())?)?;
    check(
        core.storage().operation_count() == before + 1,
        "the lost reply and the retry created exactly one execution",
    )?;
    log.say(format!(
        "    retry returned operation {}; executions created: 1",
        retried.id
    ));
    let handed: Vec<_> = tokens.try_iter().collect();
    check(handed.len() == 1, "the executor received the work once")?;
    let mut changed = lost.clone();
    changed
        .parameters
        .insert("message".into(), Value::Text("different".into()));
    let response = direct.start(&changed).map_err(|e| e.to_string())?;
    check(
        matches!(&response, StartResponse::Rejected(r) if r.code == RejectionCode::IdentifierConflict),
        "the same request_id with other content is a conflict",
    )?;
    log.say("    same request_id with changed content -> rejected(identifier_conflict)");

    // 4. Progress, result, and observation that survives a cut link.
    log.say("\n[4] Progress, result, and observation across a cut link");
    let seen: Arc<Mutex<Seen>> = Arc::default();
    let follower_thread = {
        let seen = Arc::clone(&seen);
        let id = operation.id;
        thread::spawn(move || {
            let mut follower = Follower::new();
            let result = proxied.follow(id, &mut follower, 200, |report, gap| {
                seen.lock()
                    .expect("seen")
                    .push((report.sequence, report.state, gap));
            });
            proxied.close();
            result
        })
    };
    let count = || seen.lock().expect("seen").len();
    wait_until("the follower's first snapshot", || count() >= 1)?;
    let mut event_seq = 0;
    let mut send = |kind: EventKind| -> Result<(), String> {
        event_seq += 1;
        core.notify(&event(&token, event_seq, kind))
            .map(|_| ())
            .map_err(|e| format!("executor event {event_seq}: {e:?}"))
    };
    send(progress("step 1"))?;
    send(progress("step 2"))?;
    wait_until("live progress", || count() >= 3)?;
    log.say("    follower received the running snapshots live");
    proxy.cut();
    log.say("    link cut; the executor keeps working");
    send(progress("step 3"))?;
    send(progress("step 4"))?;
    send(progress("step 5"))?;
    thread::sleep(Duration::from_millis(500));
    proxy.restore();
    log.say("    link restored");
    send(EventKind::Terminal {
        outcome: Outcome::Succeeded,
        result: Some(Value::Text("echo: hello".into())),
        explanation: "echoed the message".to_string(),
    })?;
    let followed = follower_thread
        .join()
        .map_err(|_| "the follower panicked".to_string())?;
    let Ok(Followed::Terminal(last)) = followed else {
        return Err(format!(
            "the follower did not reach the terminal state: {followed:?}"
        ));
    };
    let seen = seen.lock().expect("seen").clone();
    for (sequence, state, gap) in &seen {
        log.say(format!(
            "    saw sequence {sequence} {state:?}{}",
            gap.map_or_else(String::new, |g| {
                format!(
                    " (gap disclosed: {}..={} not delivered)",
                    g.first_missed, g.last_missed
                )
            })
        ));
    }
    check(
        last.state == State::Succeeded,
        "the follower ended on the terminal snapshot",
    )?;
    let sequences: Vec<u64> = seen.iter().map(|(s, _, _)| *s).collect();
    check(
        sequences.windows(2).all(|w| w[0] < w[1]),
        "sequences strictly increase: no duplicates, no reordering",
    )?;
    let mut covered: Vec<u64> = sequences.clone();
    for (_, _, gap) in &seen {
        if let Some(g) = gap {
            covered.extend(g.first_missed..=g.last_missed);
        }
    }
    covered.sort_unstable();
    let first = sequences[0];
    check(
        (first..=last.sequence).all(|n| covered.binary_search(&n).is_ok()),
        "every number from the first to the last is either seen or disclosed as a gap",
    )?;
    let final_core = report(direct.get(operation.id).map_err(|e| e.to_string())?)?;
    check(
        final_core == *last,
        "the follower's last snapshot is the one Core holds",
    )?;
    check(
        final_core.result == Value::Text("echo: hello".into()),
        "the result reached the caller",
    )?;
    log.say(format!(
        "    final: {:?}, result {:?}",
        final_core.state, final_core.result
    ));

    // 5. Resume semantics on the wire: the server, not only the client, discloses gaps
    //    and does not redeliver a snapshot the observer holds.
    log.say("\n[5] Resume by after_sequence on the wire");
    let last_sequence = final_core.sequence;
    let mut resumed = direct
        .observe(operation.id, Some(2))
        .map_err(|e| e.to_string())?;
    let Some(ObserveItem::Response(ObserveResponse::Attached {
        current,
        sequence,
        gap,
    })) = resumed.recv().map_err(|e| e.to_string())?
    else {
        return Err("expected the attach outcome first".to_string());
    };
    check(
        current.as_ref().map(|c| c.sequence) == Some(last_sequence) && sequence == last_sequence,
        "resuming from 2 delivers the latest snapshot",
    )?;
    check(
        gap == Some(Gap {
            first_missed: 3,
            last_missed: last_sequence - 1,
        }),
        "resuming from 2 discloses the skipped numbers",
    )?;
    check(
        resumed.recv().map_err(|e| e.to_string())? == Some(ObserveItem::End(EndReason::Terminal)),
        "a terminal execution ends the stream right after",
    )?;
    log.say(format!(
        "    after=2 -> latest {last_sequence} and gap 3..={} disclosed",
        last_sequence - 1
    ));
    let mut current_cursor = direct
        .observe(operation.id, Some(last_sequence))
        .map_err(|e| e.to_string())?;
    check(
        matches!(
            current_cursor.recv().map_err(|e| e.to_string())?,
            Some(ObserveItem::Response(ObserveResponse::Attached {
                current: None,
                gap: None,
                ..
            }))
        ),
        "a cursor at the latest snapshot redelivers nothing",
    )?;
    let mut future = direct
        .observe(operation.id, Some(last_sequence + 5))
        .map_err(|e| e.to_string())?;
    check(
        matches!(
            future.recv().map_err(|e| e.to_string())?,
            Some(ObserveItem::Response(ObserveResponse::InvalidInput(_)))
        ),
        "a cursor from the future is invalid input",
    )?;
    log.say("    after=latest -> nothing redelivered; after=future -> invalid_input");

    // 6. Late observer of completed work: final snapshot, no work restarted.
    log.say("\n[6] Late observer of completed work");
    let mut late = Follower::new();
    let result = direct.follow(operation.id, &mut late, 3, |_, _| {});
    check(
        matches!(&result, Ok(Followed::Terminal(r)) if r.sequence == final_core.sequence),
        "a late observer gets the final snapshot and the stream ends as terminal",
    )?;
    check(
        tokens.try_recv().is_err(),
        "observing handed nothing to the executor",
    )?;
    log.say("    final snapshot delivered, stream ended as terminal, nothing restarted");

    direct.close();
    server.close();
    log.say("\nscenario complete");
    Ok(())
}
