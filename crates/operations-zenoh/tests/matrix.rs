//! P1–P6 checks through Zenoh (GKO-67, OP-A9).
//!
//! Each test names the package it gives evidence for. The same rules are checked
//! directly against Core in `crates/operations/tests`; the matrix that links the
//! two is `docs-modules/operation/validation-0.1.0.md`. Everything runs on one
//! host over loopback, with the executor in the Core process.

use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use operations::report::State;
use operations::testing::PermitAllForTests;
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::{
    Acceptor, EventKind, ExecutionToken, Executor, ExecutorEvent, Follower, Gap, GetResponse,
    Guard, Handoff, MemoryDecisionLog, MemoryStorage, NotifyRejection, ObserveResponse, Operation,
    OperationId, Outcome, RejectionCode, StartResponse, Value,
};
use operations_zenoh::{
    Client, ClientConfig, EndReason, Followed, ObserveItem, Server, ServerConfig,
};

type Core = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

/// Records every handed-off token; answers from a queue, else acknowledges.
#[derive(Clone, Default)]
struct Shared {
    tokens: Arc<Mutex<Vec<ExecutionToken>>>,
    answers: Arc<Mutex<Vec<Handoff>>>,
    calls: Arc<AtomicUsize>,
}

impl Shared {
    fn answer_next(&self, handoff: Handoff) {
        self.answers.lock().unwrap().push(handoff);
    }
    fn tokens(&self) -> Vec<ExecutionToken> {
        self.tokens.lock().unwrap().clone()
    }
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Executor for Shared {
    fn hand_off(&mut self, token: &ExecutionToken) -> Handoff {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.tokens.lock().unwrap().push(token.clone());
        let mut answers = self.answers.lock().unwrap();
        if answers.is_empty() {
            Handoff::Acknowledged
        } else {
            answers.remove(0)
        }
    }
}

struct Rig {
    core: Arc<Core>,
    server: Server<PermitAllForTests, MemoryDecisionLog, MemoryStorage>,
    client: Arc<Client>,
    executor: Shared,
}

fn free_endpoint() -> String {
    let port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    format!("tcp/127.0.0.1:{port}")
}

fn rig(capacity: Option<usize>, lease: Duration) -> Rig {
    let keys = operations_zenoh::Keys::default();
    let mut core = Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        MemoryStorage::new(),
    )
    .with_observation_address(move |id| keys.address(id));
    if let Some(capacity) = capacity {
        core = core.with_observer_capacity(capacity);
    }
    let core = Arc::new(core);
    let executor = Shared::default();
    let endpoint = free_endpoint();
    let mut config = ServerConfig::new(vec![endpoint.clone()], caller("matrix"));
    config.observe_lease = lease;
    let server = Server::open(Arc::clone(&core), Box::new(executor.clone()), config).unwrap();
    let mut client_config = ClientConfig::new(vec![endpoint]);
    client_config.timeout = Duration::from_secs(3);
    client_config.observe_timeout = lease + Duration::from_secs(5);
    let client = Client::open(client_config).unwrap();
    Rig {
        core,
        server,
        client: Arc::new(client),
        executor,
    }
}

impl Rig {
    fn start(&self, message: &str) -> Operation {
        match self.client.start(&echo_request(message)).unwrap() {
            StartResponse::Accepted { operation, .. } => operation,
            other => panic!("expected acceptance, got {other:?}"),
        }
    }

    fn report(&self, id: OperationId) -> operations::Report {
        match self.client.get(id).unwrap() {
            GetResponse::Report(report) => *report,
            other => panic!("expected a report, got {other:?}"),
        }
    }

    fn notify(&self, token: &ExecutionToken, event_seq: u64, kind: EventKind) {
        self.core
            .notify(&ExecutorEvent {
                token: token.clone(),
                event_seq,
                kind,
            })
            .unwrap_or_else(|e| panic!("event {event_seq}: {e:?}"));
    }

    /// The client, shareable with the threads of a concurrent caller.
    fn client_handle(&self) -> Arc<Client> {
        Arc::clone(&self.client)
    }

    fn close(self) {
        // Threads that held a handle have been joined, so this is the last one.
        if let Ok(client) = Arc::try_unwrap(self.client) {
            client.close();
        }
        self.server.close();
    }
}

fn finish(outcome: Outcome) -> EventKind {
    EventKind::Terminal {
        outcome,
        result: None,
        explanation: "matrix".to_string(),
    }
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if ready() {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for {what}");
}

/// P4: concurrent retries of one Start, over Zenoh, create one execution and
/// return one identity; the executor receives the work once.
#[test]
fn p4_concurrent_same_content_retries_create_one_execution() {
    let rig = rig(None, Duration::from_secs(5));
    let request = echo_request("race");
    let barrier = Barrier::new(8);
    let ids: Vec<OperationId> = thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    match rig.client.start_retrying(&request, 3).unwrap() {
                        StartResponse::Accepted { operation, .. } => operation.id,
                        other => panic!("expected acceptance, got {other:?}"),
                    }
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(ids.iter().all(|id| *id == ids[0]), "one identity: {ids:?}");
    assert_eq!(rig.core.storage().operation_count(), 1);
    assert_eq!(rig.core.storage().mapping_count(), 1);
    assert_eq!(rig.executor.calls(), 1, "one handoff");
    rig.close();
}

/// P4: one request_id with different contents has exactly one winner; the
/// losers get `identifier_conflict`, and the winner's execution is unchanged.
#[test]
fn p4_concurrent_requests_with_changed_content_have_one_winner() {
    let rig = rig(None, Duration::from_secs(5));
    let base = echo_request("a");
    let barrier = Barrier::new(6);
    let responses: Vec<StartResponse> = thread::scope(|scope| {
        let handles: Vec<_> = (0..6)
            .map(|n| {
                let (rig, barrier, base) = (&rig, &barrier, &base);
                scope.spawn(move || {
                    let mut request = base.clone();
                    request
                        .parameters
                        .insert("message".into(), Value::Text(format!("content {n}")));
                    barrier.wait();
                    rig.client.start(&request).unwrap()
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let accepted = responses
        .iter()
        .filter(|r| matches!(r, StartResponse::Accepted { .. }))
        .count();
    let conflicts = responses
        .iter()
        .filter(|r| matches!(r, StartResponse::Rejected(x) if x.code == RejectionCode::IdentifierConflict))
        .count();
    assert_eq!((accepted, conflicts), (1, 5), "{responses:?}");
    assert_eq!(rig.core.storage().operation_count(), 1);
    assert_eq!(rig.executor.calls(), 1);
    rig.close();
}

/// P3/P4: invalid and rejected Starts create no operation and no mapping, and
/// the reply is a typed outcome rather than a transport error.
#[test]
fn p3_p4_rejected_starts_create_nothing_over_zenoh() {
    let rig = rig(None, Duration::from_secs(5));
    let mut unknown = echo_request("x");
    unknown.operation_type = operations::OperationType::new("no.such.type").unwrap();
    let mut missing = echo_request("x");
    missing.parameters.clear();
    assert!(matches!(
        rig.client.start(&unknown).unwrap(),
        StartResponse::Rejected(r) if r.code == RejectionCode::UnknownOperationType
    ));
    assert!(matches!(
        rig.client.start(&missing).unwrap(),
        StartResponse::Rejected(r) if r.code == RejectionCode::InvalidParameters
    ));
    assert!(matches!(
        rig.client.start_raw(b"{".to_vec()).unwrap(),
        StartResponse::InvalidInput(_)
    ));
    assert_eq!(rig.core.storage().operation_count(), 0);
    assert_eq!(rig.core.storage().mapping_count(), 0);
    assert_eq!(rig.executor.calls(), 0);
    rig.close();
}

/// P6: when the acceptance commit fails, Start answers with a typed error,
/// nothing is stored or handed off, and the retry with the same request_id
/// creates exactly one execution.
#[test]
fn p6_failed_commit_accepts_and_dispatches_nothing_then_retry_succeeds() {
    let rig = rig(None, Duration::from_secs(5));
    let request = echo_request("commit");
    rig.core.storage().inject_commit_failures(1);
    assert!(matches!(
        rig.client.start(&request).unwrap(),
        StartResponse::Error(_)
    ));
    assert_eq!(rig.core.storage().operation_count(), 0);
    assert_eq!(rig.core.storage().mapping_count(), 0);
    assert_eq!(rig.executor.calls(), 0);

    let accepted = rig.client.start_retrying(&request, 2).unwrap();
    assert!(matches!(accepted, StartResponse::Accepted { .. }));
    assert_eq!(rig.core.storage().operation_count(), 1);
    assert_eq!(rig.executor.calls(), 1);
    rig.close();
}

/// P2/P5: a lost executor acknowledgement is `unknown`, visible over Zenoh with
/// the same identity; a retried Start never redispatches; reliable evidence for
/// the same token then moves the same execution on.
#[test]
fn p2_p5_lost_ack_is_unknown_never_redispatched_and_refined_by_evidence() {
    let rig = rig(None, Duration::from_secs(5));
    rig.executor.answer_next(Handoff::Ambiguous);
    let request = echo_request("ack lost");
    let operation = match rig.client.start(&request).unwrap() {
        StartResponse::Accepted { operation, .. } => operation,
        other => panic!("{other:?}"),
    };
    let unknown = rig.report(operation.id);
    assert_eq!((unknown.state, unknown.sequence), (State::Unknown, 2));

    // Retrying Start, even repeatedly, neither restarts nor redispatches.
    for _ in 0..3 {
        match rig.client.start(&request).unwrap() {
            StartResponse::Accepted {
                operation: again, ..
            } => assert_eq!(again.id, operation.id),
            other => panic!("{other:?}"),
        }
    }
    assert_eq!(rig.executor.calls(), 1);
    assert_eq!(rig.report(operation.id).state, State::Unknown);

    // The executor later proves it is running, then finishes: same identity.
    let token = rig.executor.tokens().remove(0);
    rig.notify(&token, 1, EventKind::Ack);
    let running = rig.report(operation.id);
    assert_eq!((running.state, running.sequence), (State::Running, 3));
    rig.notify(&token, 2, finish(Outcome::Succeeded));
    let done = rig.report(operation.id);
    assert_eq!((done.state, done.sequence), (State::Succeeded, 4));
    assert_eq!(done.operation.id, operation.id);
    assert_eq!(rig.executor.calls(), 1, "never restarted");
    rig.close();
}

/// P2/P5: after a terminal outcome, a conflicting terminal and late progress are
/// rejected at Core and the terminal snapshot seen over Zenoh does not change,
/// also for an observer that reconnects afterwards.
#[test]
fn p2_p5_terminal_state_is_immutable_over_zenoh() {
    let rig = rig(None, Duration::from_secs(5));
    let operation = rig.start("terminal");
    let token = rig.executor.tokens().remove(0);
    rig.notify(&token, 1, finish(Outcome::Succeeded));
    let before = rig.report(operation.id);
    assert_eq!(before.state, State::Succeeded);

    let conflicting = rig.core.notify(&ExecutorEvent {
        token: token.clone(),
        event_seq: 2,
        kind: finish(Outcome::Failed),
    });
    assert_eq!(
        conflicting,
        Err(NotifyRejection::ConflictingTerminal {
            recorded: State::Succeeded
        })
    );
    let late = rig.core.notify(&ExecutorEvent {
        token,
        event_seq: 3,
        kind: EventKind::Progress {
            progress: operations::report::Progress::default(),
            result: None,
        },
    });
    assert_eq!(
        late,
        Err(NotifyRejection::InvalidTransition {
            from: State::Succeeded
        })
    );

    assert_eq!(rig.report(operation.id), before);
    let followed = rig
        .client
        .follow(operation.id, &mut Follower::new(), 1, |_, _| {})
        .unwrap();
    assert_eq!(followed, Followed::Terminal(Box::new(before)));
    rig.close();
}

/// P3: a slow observer is detectable through Zenoh. With the server's Observe
/// stream paused and Core's buffer at 2, five snapshots overflow it: the stream
/// ends `lagged`, never blocking the executor, and a resume with the last
/// sequence held gets the latest snapshot and a disclosed gap.
#[test]
fn p3_slow_observer_is_ended_as_lagged_and_resumes_with_a_gap() {
    let rig = rig(Some(2), Duration::from_secs(10));
    let operation = rig.start("lag");
    let token = rig.executor.tokens().remove(0);
    let mut stream = rig.client.observe(operation.id, None).unwrap();
    let Some(ObserveItem::Response(ObserveResponse::Attached { current, .. })) =
        stream.recv().unwrap()
    else {
        panic!("expected the attach outcome");
    };
    assert_eq!(current.unwrap().sequence, 2);
    wait_until("the observer to attach", || {
        rig.core.observer_count(&operation.id) == 1
    });

    rig.server.set_observe_paused(true);
    // Let the server see the pause before the burst; it checks every 50 ms.
    thread::sleep(Duration::from_millis(200));
    for n in 1..=5 {
        let started = Instant::now();
        rig.notify(
            &token,
            n,
            EventKind::Progress {
                progress: operations::report::Progress::default(),
                result: None,
            },
        );
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "Core must not wait for a slow observer"
        );
    }
    rig.server.set_observe_paused(false);

    let mut held = 2;
    let mut seen = Vec::new();
    let end = loop {
        match stream.recv().unwrap().expect("the stream ended early") {
            ObserveItem::Snapshot(report) => {
                held = report.sequence;
                seen.push(report.sequence);
            }
            ObserveItem::End(reason) => break reason,
            other => panic!("unexpected {other:?}"),
        }
    };
    assert_eq!(end, EndReason::Lagged);
    assert_eq!(seen, vec![3, 4], "what fit in the buffer, in order");

    // Resume: the latest snapshot (sequence 7) and the numbers skipped.
    let mut resumed = rig.client.observe(operation.id, Some(held)).unwrap();
    let Some(ObserveItem::Response(ObserveResponse::Attached { current, gap, .. })) =
        resumed.recv().unwrap()
    else {
        panic!("expected the attach outcome");
    };
    assert_eq!(current.unwrap().sequence, 7);
    assert_eq!(
        gap,
        Some(Gap {
            first_missed: 5,
            last_missed: 6
        })
    );
    drop(resumed);
    rig.close();
}

/// P3: `follow` copes with a lagged stream by itself: it resumes, discloses the
/// gap, and still ends on the terminal snapshot Core holds.
#[test]
fn p3_follow_survives_a_lagged_stream_and_ends_on_the_core_snapshot() {
    let rig = rig(Some(2), Duration::from_secs(10));
    let operation = rig.start("lag follow");
    let token = rig.executor.tokens().remove(0);
    let id = operation.id;

    let gaps = Arc::new(Mutex::new(Vec::<Gap>::new()));
    let follower = {
        let client = rig.client_handle();
        let gaps = Arc::clone(&gaps);
        thread::spawn(move || {
            client.follow(id, &mut Follower::new(), 10, |_, gap| {
                if let Some(gap) = gap {
                    gaps.lock().unwrap().push(gap);
                }
            })
        })
    };
    wait_until("the follower to attach", || {
        rig.core.observer_count(&id) == 1
    });
    rig.server.set_observe_paused(true);
    thread::sleep(Duration::from_millis(200));
    for n in 1..=4 {
        rig.notify(
            &token,
            n,
            EventKind::Progress {
                progress: operations::report::Progress::default(),
                result: None,
            },
        );
    }
    rig.notify(&token, 5, finish(Outcome::Succeeded));
    rig.server.set_observe_paused(false);

    let followed = follower.join().unwrap().unwrap();
    let held = rig.report(id);
    assert_eq!(held.state, State::Succeeded);
    assert_eq!(followed, Followed::Terminal(Box::new(held)));
    // Snapshots 3 and 4 fit the buffer; 5 overflows it, so the stream ended
    // lagged and the resume from 4 disclosed exactly 5..=6 before the terminal 7.
    assert_eq!(
        *gaps.lock().unwrap(),
        vec![Gap {
            first_missed: 5,
            last_missed: 6
        }],
        "the stream lagged and the skipped snapshots were disclosed, not hidden"
    );
    rig.close();
}

/// P3/P5: attaching while the final snapshot is published never loses it, over
/// Zenoh. Each round races a terminal event against a new observer; whichever
/// wins, the client ends on the terminal snapshot Core holds.
#[test]
fn p3_p5_attach_racing_the_final_publication_never_loses_it() {
    let rig = rig(None, Duration::from_secs(5));
    for round in 0..40 {
        let operation = rig.start(&format!("race {round}"));
        let id = operation.id;
        let token = rig.executor.tokens().pop().unwrap();
        let barrier = Arc::new(Barrier::new(2));
        let observer = {
            let client = rig.client_handle();
            let barrier = Arc::clone(&barrier);
            thread::spawn(move || {
                barrier.wait();
                client.follow(id, &mut Follower::new(), 3, |_, _| {})
            })
        };
        barrier.wait();
        rig.notify(&token, 1, finish(Outcome::Succeeded));
        let followed = observer.join().unwrap().unwrap();
        let held = rig.report(id);
        assert_eq!(held.state, State::Succeeded, "round {round}");
        assert_eq!(
            followed,
            Followed::Terminal(Box::new(held)),
            "round {round}: the final snapshot was lost"
        );
    }
    rig.close();
}
