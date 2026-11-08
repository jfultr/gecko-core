//! Get and Observe tests: atomic attach, resume, gaps, slow observers, retention.

use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;

use operations::report::{Progress, State};
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::testing::{PermitAllForTests, TestExecutor};
use operations::{
    Acceptor, Applied, Authorizer, CallerContext, Command, Decision, End, EventKind,
    ExecutionToken, ExecutorEvent, Follower, Gap, GetRequest, GetResponse, Guard, Handoff,
    MemoryDecisionLog, MemoryStorage, Notified, Observation, ObserveRequest, ObserveResponse,
    OperationId, Outcome, Poll, Report, Storage, Unavailable,
};

type TestAcceptor = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

fn acceptor() -> TestAcceptor {
    Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        MemoryStorage::new(),
    )
}

/// Starts an execution for alice and hands it off as acknowledged (`running`, sequence 2).
fn running(core: &TestAcceptor) -> (OperationId, ExecutionToken) {
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let dispatch = outcome.dispatch.expect("created");
    let id = dispatch.operation_id();
    let token = core.storage().operation(&id).unwrap().unwrap().token;
    core.dispatch(dispatch, &mut TestExecutor::new()).unwrap();
    (id, token)
}

fn progress(token: &ExecutionToken, event_seq: u64) -> ExecutorEvent {
    ExecutorEvent {
        token: token.clone(),
        event_seq,
        kind: EventKind::Progress {
            progress: Progress {
                stage: format!("step-{event_seq}"),
                ..Progress::default()
            },
            result: None,
        },
    }
}

fn finish(token: &ExecutionToken, event_seq: u64, outcome: Outcome) -> ExecutorEvent {
    ExecutorEvent {
        token: token.clone(),
        event_seq,
        kind: EventKind::Terminal {
            outcome,
            result: None,
            explanation: "done".to_string(),
        },
    }
}

fn get(core: &TestAcceptor, who: &str, id: OperationId) -> GetResponse {
    core.get(&caller(who), &GetRequest { operation_id: id })
}

fn observe(
    core: &TestAcceptor,
    who: &str,
    id: OperationId,
    after: Option<u64>,
) -> (ObserveResponse, Option<Observation>) {
    let outcome = core.observe(
        &caller(who),
        &ObserveRequest {
            operation_id: id,
            after_sequence: after,
        },
    );
    (outcome.response, outcome.stream)
}

fn attached(response: ObserveResponse) -> (Option<Report>, u64, Option<Gap>) {
    match response {
        ObserveResponse::Attached {
            current,
            sequence,
            gap,
        } => (current.map(|c| *c), sequence, gap),
        other => panic!("expected attach, got {other:?}"),
    }
}

fn next(stream: &Observation) -> Report {
    match stream.next_timeout(Duration::from_secs(5)) {
        Poll::Snapshot(report) => *report,
        other => panic!("expected a snapshot, got {other:?}"),
    }
}

fn state_of(core: &TestAcceptor, id: &OperationId) -> (State, u64) {
    let snap = core.storage().operation(id).unwrap().unwrap().snapshot;
    (snap.state, snap.sequence)
}

#[test]
fn get_returns_the_latest_full_snapshot_at_each_stage() {
    let core = acceptor();
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let dispatch = outcome.dispatch.expect("created");
    let id = dispatch.operation_id();
    let token = core.storage().operation(&id).unwrap().unwrap().token;

    let GetResponse::Report(pending) = get(&core, "alice", id) else {
        panic!("expected a report");
    };
    assert_eq!((pending.state, pending.sequence), (State::Pending, 1));

    core.dispatch(dispatch, &mut TestExecutor::new()).unwrap();
    core.notify(&progress(&token, 1)).unwrap();
    let GetResponse::Report(mid) = get(&core, "alice", id) else {
        panic!("expected a report");
    };
    assert_eq!((mid.state, mid.sequence), (State::Running, 3));
    assert_eq!(
        mid.context, pending.context,
        "a full snapshot keeps context"
    );

    core.notify(&finish(&token, 2, Outcome::Succeeded)).unwrap();
    let GetResponse::Report(done) = get(&core, "alice", id) else {
        panic!("expected a report");
    };
    assert_eq!((done.state, done.sequence), (State::Succeeded, 4));
}

#[test]
fn unowned_nonexistent_and_denied_reads_are_indistinguishable() {
    let core = acceptor();
    let (id, _) = running(&core);
    let unavailable = GetResponse::Unavailable(Unavailable {});
    assert_eq!(get(&core, "mallory", id), unavailable);
    assert_eq!(get(&core, "alice", OperationId::generate()), unavailable);
    assert_eq!(
        observe(&core, "mallory", id, None).0,
        ObserveResponse::Unavailable(Unavailable {})
    );
    assert_eq!(
        observe(&core, "alice", OperationId::generate(), None).0,
        ObserveResponse::Unavailable(Unavailable {})
    );
    // Even a bad cursor gives a non-owner nothing to distinguish.
    assert_eq!(
        observe(&core, "mallory", id, Some(999)).0,
        ObserveResponse::Unavailable(Unavailable {})
    );

    struct StartOnly;
    impl Authorizer for StartOnly {
        fn authorize(&self, _: &CallerContext, command: &Command) -> Decision {
            match command {
                Command::Start { .. } => Decision::Permit,
                _ => Decision::Deny {
                    reason: "read denied".to_string(),
                },
            }
        }
    }
    let denying = Acceptor::new(
        catalog(),
        Guard::new(StartOnly, MemoryDecisionLog::new()),
        MemoryStorage::new(),
    );
    let outcome = denying.start(&caller("alice"), &echo_request("hi"));
    let id = outcome.dispatch.expect("created").operation_id();
    assert_eq!(
        denying.get(&caller("alice"), &GetRequest { operation_id: id }),
        unavailable
    );
    let observed = denying.observe(
        &caller("alice"),
        &ObserveRequest {
            operation_id: id,
            after_sequence: None,
        },
    );
    assert_eq!(
        observed.response,
        ObserveResponse::Unavailable(Unavailable {})
    );
    assert!(observed.stream.is_none());
}

#[test]
fn reading_and_observing_never_start_resume_or_cancel_execution() {
    let core = acceptor();
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let id = outcome.dispatch.expect("created").operation_id();
    // Never dispatched: the execution stays pending however it is read.
    for _ in 0..3 {
        get(&core, "alice", id);
        let (_, stream) = observe(&core, "alice", id, None);
        drop(stream);
    }
    assert_eq!(state_of(&core, &id), (State::Pending, 1));

    // Disconnecting an observer of a running execution changes nothing either.
    let (running_id, token) = running(&core);
    let (_, stream) = observe(&core, "alice", running_id, None);
    drop(stream);
    assert_eq!(state_of(&core, &running_id), (State::Running, 2));
    assert_eq!(
        core.notify(&progress(&token, 1)),
        Ok(Notified::Applied { sequence: 3 })
    );
}

#[test]
fn late_observer_of_running_work_gets_current_then_later_snapshots() {
    let core = acceptor();
    let (id, token) = running(&core);
    core.notify(&progress(&token, 1)).unwrap();
    let (response, stream) = observe(&core, "alice", id, None);
    let (current, sequence, gap) = attached(response);
    let current = current.expect("current");
    assert_eq!(
        (current.state, current.sequence, sequence),
        (State::Running, 3, 3)
    );
    assert_eq!(gap, None);
    let stream = stream.expect("stream");
    assert_eq!(stream.try_next(), Poll::Empty);

    core.notify(&progress(&token, 2)).unwrap();
    core.notify(&finish(&token, 3, Outcome::Succeeded)).unwrap();
    assert_eq!(next(&stream).sequence, 4);
    let last = next(&stream);
    assert_eq!((last.state, last.sequence), (State::Succeeded, 5));
    assert_eq!(stream.try_next(), Poll::Ended(End::Terminal));
}

#[test]
fn late_observer_of_completed_work_gets_the_final_snapshot_and_no_stream() {
    let core = acceptor();
    let (id, token) = running(&core);
    core.notify(&finish(&token, 1, Outcome::Failed)).unwrap();
    let (response, stream) = observe(&core, "alice", id, None);
    let (current, sequence, _) = attached(response);
    assert_eq!(current.expect("current").state, State::Failed);
    assert_eq!(sequence, 3);
    assert!(stream.is_none());

    // Resuming with the final cursor redelivers nothing.
    let (response, stream) = observe(&core, "alice", id, Some(3));
    assert_eq!(attached(response), (None, 3, None));
    assert!(stream.is_none());
}

#[test]
fn resume_suppresses_duplicates_and_discloses_gaps() {
    let core = acceptor();
    let (id, token) = running(&core);
    for n in 1..=4 {
        core.notify(&progress(&token, n)).unwrap();
    }
    // Latest is 6 (2 after the handoff, 4 progress events).
    let latest = state_of(&core, &id).1;
    assert_eq!(latest, 6);

    // Up to date: nothing redelivered, still a stream for what follows.
    let (response, stream) = observe(&core, "alice", id, Some(6));
    assert_eq!(attached(response), (None, 6, None));
    let stream = stream.expect("stream");
    core.notify(&progress(&token, 5)).unwrap();
    assert_eq!(next(&stream).sequence, 7);

    // One behind: nothing missed.
    let (response, _) = observe(&core, "alice", id, Some(6));
    let (current, _, gap) = attached(response);
    assert_eq!(current.expect("current").sequence, 7);
    assert_eq!(gap, None);

    // Several behind: the latest is delivered and the skipped numbers named.
    let (response, _) = observe(&core, "alice", id, Some(3));
    let (current, sequence, gap) = attached(response);
    assert_eq!(current.expect("a gap never hides the latest").sequence, 7);
    assert_eq!(sequence, 7);
    assert_eq!(
        gap,
        Some(Gap {
            first_missed: 4,
            last_missed: 6
        })
    );

    // Zero means "holds nothing", like no cursor, and claims no gap.
    let (response, _) = observe(&core, "alice", id, Some(0));
    assert_eq!(attached(response).2, None);

    // A cursor from the future is invalid input, not a silent attach.
    let (response, stream) = observe(&core, "alice", id, Some(8));
    assert!(
        matches!(response, ObserveResponse::InvalidInput(_)),
        "{response:?}"
    );
    assert!(stream.is_none());
}

#[test]
fn reconnect_resumes_without_duplicates_or_restarting_work() {
    let core = acceptor();
    let (id, token) = running(&core);
    let mut follower = Follower::new();
    let (response, stream) = observe(&core, "alice", id, follower.sequence());
    let (current, _, _) = attached(response);
    follower.apply(current.expect("current"));
    let stream = stream.expect("stream");
    core.notify(&progress(&token, 1)).unwrap();
    assert!(matches!(
        follower.apply(next(&stream)),
        Applied::Replaced { missed: None }
    ));
    drop(stream); // the connection drops

    core.notify(&progress(&token, 2)).unwrap();
    core.notify(&progress(&token, 3)).unwrap();

    let (response, stream) = observe(&core, "alice", id, follower.sequence());
    let (current, _, gap) = attached(response);
    let current = current.expect("latest");
    assert_eq!(current.sequence, 5);
    assert_eq!(
        gap,
        Some(Gap {
            first_missed: 4,
            last_missed: 4
        })
    );
    assert_eq!(
        follower.apply(current),
        Applied::Replaced {
            missed: Some(Gap {
                first_missed: 4,
                last_missed: 4
            })
        }
    );
    core.notify(&progress(&token, 4)).unwrap();
    assert_eq!(
        follower.apply(next(&stream.expect("stream"))),
        Applied::Replaced { missed: None }
    );
    assert_eq!(follower.sequence(), Some(6));
}

#[test]
fn slow_observer_is_ended_as_lagged_and_never_blocks_core() {
    let core = acceptor().with_observer_capacity(2);
    let (id, token) = running(&core);
    let (_, stream) = observe(&core, "alice", id, None);
    let stream = stream.expect("stream");
    // The observer reads nothing while five snapshots are published.
    for n in 1..=5 {
        assert!(core.notify(&progress(&token, n)).is_ok());
    }
    // It still gets what fit in its buffer, in order, then a lagged end.
    assert_eq!(next(&stream).sequence, 3);
    assert_eq!(next(&stream).sequence, 4);
    assert_eq!(stream.try_next(), Poll::Ended(End::Lagged));

    // Resuming from the last number held shows the gap and the latest state.
    let (response, stream) = observe(&core, "alice", id, Some(4));
    let (current, _, gap) = attached(response);
    assert_eq!(current.expect("latest").sequence, 7);
    assert_eq!(
        gap,
        Some(Gap {
            first_missed: 5,
            last_missed: 6
        })
    );
    assert!(stream.is_some());
}

#[test]
fn one_slow_observer_does_not_affect_others() {
    let core = acceptor().with_observer_capacity(1);
    let (id, token) = running(&core);
    let (_, slow) = observe(&core, "alice", id, None);
    let (_, fast) = observe(&core, "alice", id, None);
    let (slow, fast) = (slow.unwrap(), fast.unwrap());
    core.notify(&progress(&token, 1)).unwrap();
    assert_eq!(next(&fast).sequence, 3);
    core.notify(&progress(&token, 2)).unwrap();
    assert_eq!(next(&fast).sequence, 4);
    assert_eq!(next(&slow).sequence, 3);
    assert_eq!(slow.try_next(), Poll::Ended(End::Lagged));
}

#[test]
fn released_report_is_unavailable_but_accounting_continues() {
    let core = acceptor();
    let (id, token) = running(&core);
    let (_, stream) = observe(&core, "alice", id, None);
    let stream = stream.expect("stream");

    assert_eq!(core.release_report(&id), Ok(true));
    assert_eq!(core.release_report(&OperationId::generate()), Ok(false));
    assert_eq!(stream.try_next(), Poll::Ended(End::Unavailable));
    assert_eq!(
        get(&core, "alice", id),
        GetResponse::Unavailable(Unavailable {})
    );
    assert_eq!(
        observe(&core, "alice", id, None).0,
        ObserveResponse::Unavailable(Unavailable {})
    );

    // The execution and its counter are untouched: later events still number on.
    assert_eq!(
        core.notify(&progress(&token, 1)),
        Ok(Notified::Applied { sequence: 3 })
    );
    assert_eq!(state_of(&core, &id), (State::Running, 3));
    assert_eq!(
        get(&core, "alice", id),
        GetResponse::Unavailable(Unavailable {}),
        "a released report does not reappear"
    );
}

#[test]
fn terminal_snapshot_ends_every_stream_in_order_without_duplicates() {
    let core = acceptor();
    let (id, token) = running(&core);
    let streams: Vec<_> = (0..3)
        .map(|_| observe(&core, "alice", id, None).1.expect("stream"))
        .collect();
    core.notify(&progress(&token, 1)).unwrap();
    core.notify(&progress(&token, 1)).unwrap(); // duplicate: nothing published
    core.notify(&finish(&token, 2, Outcome::Succeeded)).unwrap();
    for stream in &streams {
        assert_eq!(next(stream).sequence, 3);
        let last = next(stream);
        assert_eq!((last.sequence, last.state), (4, State::Succeeded));
        assert_eq!(stream.try_next(), Poll::Ended(End::Terminal));
    }
}

#[test]
fn attaching_while_the_final_snapshot_is_published_never_loses_it() {
    for round in 0..300 {
        let core = Arc::new(acceptor());
        let (id, token) = running(&core);
        let barrier = Arc::new(Barrier::new(2));

        let publisher = {
            let (core, barrier, token) = (Arc::clone(&core), Arc::clone(&barrier), token.clone());
            thread::spawn(move || {
                barrier.wait();
                core.notify(&finish(&token, 1, Outcome::Succeeded)).unwrap();
            })
        };
        let observer = {
            let (core, barrier) = (Arc::clone(&core), Arc::clone(&barrier));
            thread::spawn(move || {
                barrier.wait();
                observe(&core, "alice", id, None)
            })
        };
        publisher.join().unwrap();
        let (response, stream) = observer.join().unwrap();

        let (current, _, _) = attached(response);
        let current = current.expect("current");
        let final_seen = match (current.state.is_terminal(), stream) {
            // Attached after the final publication: the final snapshot is `current`.
            (true, None) => true,
            // Attached before it: the stream must carry it.
            (false, Some(stream)) => {
                let last = next(&stream);
                last.state == State::Succeeded
                    && last.sequence == current.sequence + 1
                    && stream.try_next() == Poll::Ended(End::Terminal)
            }
            _ => false,
        };
        assert!(final_seen, "round {round}: the final snapshot was lost");
    }
}

#[test]
fn follower_never_lets_an_equal_or_lower_sequence_replace_newer_state() {
    let core = acceptor();
    let (id, token) = running(&core);
    let snapshot_at = |core: &TestAcceptor| match get(core, "alice", id) {
        GetResponse::Report(r) => *r,
        other => panic!("{other:?}"),
    };
    let two = snapshot_at(&core);
    core.notify(&progress(&token, 1)).unwrap();
    let three = snapshot_at(&core);

    let mut follower = Follower::new();
    assert_eq!(
        follower.apply(three.clone()),
        Applied::Replaced { missed: None }
    );
    assert_eq!(follower.apply(two), Applied::Stale);
    assert_eq!(follower.apply(three.clone()), Applied::Duplicate);
    let mut forged = three.clone();
    forged.explanation = "different content under the same number".to_string();
    assert_eq!(follower.apply(forged), Applied::Conflict);
    assert_eq!(follower.latest(), Some(&three));

    let (other_id, _) = running(&core);
    let other = match get(&core, "alice", other_id) {
        GetResponse::Report(r) => *r,
        other => panic!("{other:?}"),
    };
    assert_eq!(follower.apply(other), Applied::WrongOperation);

    let mut skipping = three.clone();
    skipping.sequence = 9;
    assert_eq!(
        follower.apply(skipping),
        Applied::Replaced {
            missed: Some(Gap {
                first_missed: 4,
                last_missed: 8
            })
        }
    );
}

#[test]
fn handoff_publications_reach_observers_too() {
    let core = acceptor();
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let dispatch = outcome.dispatch.expect("created");
    let id = dispatch.operation_id();
    let (_, stream) = observe(&core, "alice", id, None);
    let stream = stream.expect("stream");
    let mut executor = TestExecutor::new();
    executor.queue_response(Handoff::Ambiguous);
    core.dispatch(dispatch, &mut executor).unwrap();
    let unknown = next(&stream);
    assert_eq!((unknown.state, unknown.sequence), (State::Unknown, 2));
}
