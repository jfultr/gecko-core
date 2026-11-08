//! Lifecycle tests: executor handoff, event arbitration, and Core snapshots.

use std::sync::Arc;
use std::thread;

use operations::report::{Progress, State};
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::testing::{PermitAllForTests, TestExecutor, fixtures};
use operations::{
    Acceptor, EventKind, ExecutionToken, ExecutorEvent, Guard, Handoff, MemoryDecisionLog,
    MemoryStorage, Notified, NotifyRejection, OperationId, Outcome, Storage, Value,
};

type TestAcceptor = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

fn acceptor() -> TestAcceptor {
    Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        MemoryStorage::new(),
    )
}

/// Starts one execution and returns its id, token, and the unspent dispatch.
fn started(core: &TestAcceptor) -> (OperationId, ExecutionToken, operations::Dispatch) {
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let dispatch = outcome.dispatch.expect("created");
    let id = dispatch.operation_id();
    let token = core.storage().operation(&id).unwrap().unwrap().token;
    (id, token, dispatch)
}

/// Starts one execution and hands it to an executor that answers `handoff`.
fn running_with(core: &TestAcceptor, handoff: Handoff) -> (OperationId, ExecutionToken) {
    let (id, token, dispatch) = started(core);
    let mut executor = TestExecutor::new();
    executor.queue_response(handoff);
    core.dispatch(dispatch, &mut executor).expect("recorded");
    (id, token)
}

fn snapshot(core: &TestAcceptor, id: &OperationId) -> operations::Report {
    core.storage().operation(id).unwrap().unwrap().snapshot
}

fn event(token: &ExecutionToken, event_seq: u64, kind: EventKind) -> ExecutorEvent {
    ExecutorEvent {
        token: token.clone(),
        event_seq,
        kind,
    }
}

fn progress(stage: &str, result: Option<Value>) -> EventKind {
    EventKind::Progress {
        progress: Progress {
            stage: stage.to_string(),
            message: format!("{stage} message"),
            ..Progress::default()
        },
        result,
    }
}

fn terminal(outcome: Outcome) -> EventKind {
    EventKind::Terminal {
        outcome,
        result: Some(Value::Text(format!("{outcome:?}"))),
        explanation: format!("{outcome:?} explanation"),
    }
}

#[test]
fn acknowledged_handoff_makes_the_execution_running_with_the_next_sequence() {
    let core = acceptor();
    let (id, _) = running_with(&core, Handoff::Acknowledged);
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Running);
    assert_eq!(snap.sequence, 2);
}

#[test]
fn refused_handoff_is_a_proven_failure_and_stays_failed() {
    let core = acceptor();
    let (id, token) = running_with(
        &core,
        Handoff::Refused {
            reason: "queue closed".to_string(),
        },
    );
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Failed);
    assert_eq!(snap.sequence, 2);
    assert!(snap.explanation.contains("before any side effect"));
    assert!(snap.explanation.contains("queue closed"));

    // A failed execution is immutable: no progress, no other outcome. Number 1
    // is consumed by the rejected progress, so the outcome takes number 2.
    assert_eq!(
        core.notify(&event(&token, 1, progress("late", None))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Failed
        })
    );
    assert_eq!(
        core.notify(&event(&token, 2, terminal(Outcome::Succeeded))),
        Err(NotifyRejection::ConflictingTerminal {
            recorded: State::Failed
        })
    );
    assert_eq!(snapshot(&core, &id), snap);
}

#[test]
fn lost_acknowledgement_is_unknown_and_never_redispatched() {
    let core = acceptor();
    let request = echo_request("hi");
    let outcome = core.start(&caller("alice"), &request);
    let dispatch = outcome.dispatch.expect("created");
    let id = dispatch.operation_id();
    let token = core.storage().operation(&id).unwrap().unwrap().token;
    let mut executor = TestExecutor::new();
    executor.queue_response(Handoff::Ambiguous);
    let notified = core.dispatch(dispatch, &mut executor).expect("recorded");
    assert_eq!(notified, Notified::Applied { sequence: 2 });
    assert_eq!(snapshot(&core, &id).state, State::Unknown);
    // The only handoff used the issued token; the dispatch was consumed.
    assert_eq!(executor.dispatched(), std::slice::from_ref(&token));
    // A same-content retry of Start does not create a dispatch either.
    let retry = core.start(&caller("alice"), &request);
    assert!(retry.dispatch.is_none());
    assert_eq!(executor.dispatched().len(), 1);
}

#[test]
fn late_evidence_for_the_same_token_refines_unknown() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Ambiguous);
    let notified = core.notify(&event(&token, 1, EventKind::Ack)).unwrap();
    assert_eq!(notified, Notified::Applied { sequence: 3 });
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Running);
    assert_eq!(snap.operation.id, id);

    let (other, other_token) = running_with(&core, Handoff::Ambiguous);
    core.notify(&event(&other_token, 1, terminal(Outcome::Failed)))
        .unwrap();
    assert_eq!(snapshot(&core, &other).state, State::Failed);
    assert_eq!(snapshot(&core, &other).sequence, 3);
}

#[test]
fn terminal_outcome_confirms_a_pending_execution_whose_ack_was_lost() {
    let core = acceptor();
    let (id, token, _dispatch) = started(&core);
    let notified = core
        .notify(&event(&token, 1, terminal(Outcome::Succeeded)))
        .unwrap();
    assert_eq!(notified, Notified::Applied { sequence: 2 });
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Succeeded);
    assert_eq!(snap.result, Value::Text("Succeeded".to_string()));
}

#[test]
fn progress_before_acknowledgement_is_an_invalid_transition() {
    let core = acceptor();
    let (id, token, _dispatch) = started(&core);
    assert_eq!(
        core.notify(&event(&token, 1, progress("early", None))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Pending
        })
    );
    assert_eq!(snapshot(&core, &id).sequence, 1);
    // The rejected event consumed number 1 (decision Z6, 2026-11-08): another
    // event with number 1 is stale, and the executor continues with 2.
    assert_eq!(
        core.notify(&event(&token, 1, EventKind::Ack)),
        Err(NotifyRejection::Stale)
    );
    assert_eq!(
        core.notify(&event(&token, 2, EventKind::Ack)),
        Ok(Notified::Applied { sequence: 2 })
    );
    assert_eq!(snapshot(&core, &id).state, State::Running);
}

#[test]
fn rejected_event_consumes_its_number_so_later_events_proceed() {
    let core = acceptor();
    let (id, token, _dispatch) = started(&core);
    assert_eq!(
        core.notify(&event(&token, 1, progress("early", None))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Pending
        })
    );
    assert_eq!(
        core.notify(&event(&token, 2, EventKind::Ack)),
        Ok(Notified::Applied { sequence: 2 })
    );
    assert_eq!(
        core.notify(&event(&token, 3, progress("work", None))),
        Ok(Notified::Applied { sequence: 3 })
    );
    assert_eq!(
        core.notify(&event(&token, 4, terminal(Outcome::Succeeded))),
        Ok(Notified::Applied { sequence: 4 })
    );
    assert_eq!(snapshot(&core, &id).state, State::Succeeded);
}

#[test]
fn early_progress_during_handoff_does_not_wedge_the_execution() {
    let core = acceptor();
    let (id, token, dispatch) = started(&core);
    // Progress arrives before the handoff records `running`.
    assert_eq!(
        core.notify(&event(&token, 1, progress("early", None))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Pending
        })
    );
    let mut executor = TestExecutor::new();
    core.dispatch(dispatch, &mut executor).expect("recorded");
    assert_eq!(snapshot(&core, &id).state, State::Running);
    // The handoff consumed no number, so the executor's next event is 2.
    assert_eq!(
        core.notify(&event(&token, 2, progress("work", None))),
        Ok(Notified::Applied { sequence: 3 })
    );
    assert_eq!(
        core.notify(&event(&token, 3, terminal(Outcome::Succeeded))),
        Ok(Notified::Applied { sequence: 4 })
    );
    assert_eq!(snapshot(&core, &id).state, State::Succeeded);
}

#[test]
fn resending_a_rejected_event_returns_the_same_rejection() {
    let core = acceptor();
    let (id, token, dispatch) = started(&core);
    let early = event(&token, 1, progress("early", None));
    assert_eq!(
        core.notify(&early),
        Err(NotifyRejection::InvalidTransition {
            from: State::Pending
        })
    );
    let mut executor = TestExecutor::new();
    core.dispatch(dispatch, &mut executor).expect("recorded");
    core.notify(&event(&token, 2, progress("work", None)))
        .unwrap();
    let before = snapshot(&core, &id);
    // The stored rejection is returned, although the state is now `running`.
    assert_eq!(
        core.notify(&early),
        Err(NotifyRejection::InvalidTransition {
            from: State::Pending
        })
    );
    assert_eq!(snapshot(&core, &id), before);
}

#[test]
fn conflicting_terminal_consumes_its_number() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    assert_eq!(
        core.notify(&event(&token, 1, terminal(Outcome::Succeeded))),
        Ok(Notified::Applied { sequence: 3 })
    );
    let failed = event(&token, 2, terminal(Outcome::Failed));
    let rejection = NotifyRejection::ConflictingTerminal {
        recorded: State::Succeeded,
    };
    assert_eq!(core.notify(&failed), Err(rejection.clone()));
    // The same event again gets the same rejection, not Stale.
    assert_eq!(core.notify(&failed), Err(rejection));
    // Number 3 is the next one, so it is accepted without OutOfOrder.
    assert_eq!(
        core.notify(&event(&token, 3, EventKind::Ack)),
        Ok(Notified::NoChange { sequence: 3 })
    );
    assert_eq!(snapshot(&core, &id).sequence, 3);
    assert_eq!(snapshot(&core, &id).state, State::Succeeded);
}

#[test]
fn duplicates_and_repeated_acks_do_not_increment_the_sequence() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    // The handoff made it running (sequence 2). Event 1 is a repeated ack.
    let ack = event(&token, 1, EventKind::Ack);
    assert_eq!(core.notify(&ack), Ok(Notified::NoChange { sequence: 2 }));
    assert_eq!(core.notify(&ack), Ok(Notified::Duplicate { sequence: 2 }));

    let step = event(&token, 2, progress("half", None));
    assert_eq!(core.notify(&step), Ok(Notified::Applied { sequence: 3 }));
    assert_eq!(core.notify(&step), Ok(Notified::Duplicate { sequence: 3 }));
    // Later duplicates still report the number of their own first application.
    assert_eq!(core.notify(&ack), Ok(Notified::Duplicate { sequence: 2 }));
    assert_eq!(snapshot(&core, &id).sequence, 3);
}

#[test]
fn stale_out_of_order_and_unknown_events_are_rejected_without_change() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    core.notify(&event(&token, 1, EventKind::Ack)).unwrap();
    core.notify(&event(&token, 2, progress("a", None))).unwrap();
    let before = snapshot(&core, &id);

    assert_eq!(
        core.notify(&event(&token, 2, progress("other content", None))),
        Err(NotifyRejection::Stale)
    );
    assert_eq!(
        core.notify(&event(&token, 4, progress("b", None))),
        Err(NotifyRejection::OutOfOrder { expected: 3 })
    );
    assert_eq!(
        core.notify(&event(&token, 0, EventKind::Ack)),
        Err(NotifyRejection::Stale)
    );
    assert_eq!(
        core.notify(&event(&fixtures::execution_token(1), 1, EventKind::Ack)),
        Err(NotifyRejection::UnknownToken)
    );
    assert_eq!(snapshot(&core, &id), before);

    // The missing number then makes the skipped one acceptable.
    assert!(core.notify(&event(&token, 3, progress("b", None))).is_ok());
    assert!(core.notify(&event(&token, 4, progress("c", None))).is_ok());
}

#[test]
fn a_token_cannot_be_used_for_another_execution() {
    let core = acceptor();
    let (first, first_token) = running_with(&core, Handoff::Acknowledged);
    let (second, _) = running_with(&core, Handoff::Acknowledged);
    core.notify(&event(&first_token, 1, terminal(Outcome::Succeeded)))
        .unwrap();
    assert_eq!(snapshot(&core, &first).state, State::Succeeded);
    assert_eq!(snapshot(&core, &second).state, State::Running);
}

#[test]
fn terminal_outcome_is_immutable_and_arrival_order_does_not_decide() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    core.notify(&event(&token, 1, terminal(Outcome::Succeeded)))
        .unwrap();
    let done = snapshot(&core, &id);
    assert_eq!(done.state, State::Succeeded);

    // A different outcome arriving later, with a fresh event number, is a conflict.
    assert_eq!(
        core.notify(&event(&token, 2, terminal(Outcome::Failed))),
        Err(NotifyRejection::ConflictingTerminal {
            recorded: State::Succeeded
        })
    );
    // The same outcome again, even with other detail, changes nothing. Number 2
    // was consumed by the conflict, so this uses number 3.
    let again = EventKind::Terminal {
        outcome: Outcome::Succeeded,
        result: Some(Value::Text("rewritten".to_string())),
        explanation: "rewritten".to_string(),
    };
    assert_eq!(
        core.notify(&event(&token, 3, again)),
        Ok(Notified::NoChange { sequence: 3 })
    );
    assert_eq!(
        core.notify(&event(&token, 4, progress("late", None))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Succeeded
        })
    );
    assert_eq!(snapshot(&core, &id), done);
}

#[test]
fn cancelled_is_not_accepted_before_cancel_exists() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    assert_eq!(
        core.notify(&event(&token, 1, terminal(Outcome::Cancelled))),
        Err(NotifyRejection::InvalidTransition {
            from: State::Running
        })
    );
    assert_eq!(snapshot(&core, &id).state, State::Running);
}

#[test]
fn partial_data_survives_unknown_and_failure() {
    let core = acceptor();
    let (id, token) = running_with(&core, Handoff::Acknowledged);
    let partial = Value::Text("3 of 10".to_string());
    core.notify(&event(&token, 1, progress("work", Some(partial.clone()))))
        .unwrap();
    assert_eq!(snapshot(&core, &id).result, partial);
    // A progress event without data keeps what was obtained.
    core.notify(&event(&token, 2, progress("more", None)))
        .unwrap();
    assert_eq!(snapshot(&core, &id).result, partial);
    // A failure without data keeps the partial result too.
    let failure = EventKind::Terminal {
        outcome: Outcome::Failed,
        result: None,
        explanation: "device lost".to_string(),
    };
    core.notify(&event(&token, 3, failure)).unwrap();
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Failed);
    assert_eq!(snap.result, partial);
    assert_eq!(snap.explanation, "device lost");
}

#[test]
fn snapshots_are_full_and_numbered_without_gaps() {
    let core = acceptor();
    let (id, token, dispatch) = started(&core);
    let accepted = core.storage().operation(&id).unwrap().unwrap();
    let initial = accepted.snapshot.clone();
    let mut executor = TestExecutor::new();
    core.dispatch(dispatch, &mut executor).unwrap();
    let mut seen = vec![initial.sequence, snapshot(&core, &id).sequence];
    core.notify(&event(&token, 1, progress("a", None))).unwrap();
    seen.push(snapshot(&core, &id).sequence);
    core.notify(&event(&token, 2, terminal(Outcome::Succeeded)))
        .unwrap();
    seen.push(snapshot(&core, &id).sequence);
    assert_eq!(seen, [1, 2, 3, 4]);

    // Identity and the accepted content stay in every later snapshot.
    let last = snapshot(&core, &id);
    assert_eq!(last.operation, initial.operation);
    assert_eq!(last.subject, initial.subject);
    assert_eq!(last.context, initial.context);
}

#[test]
fn a_dispatch_hands_off_once_with_the_issued_token() {
    let core = acceptor();
    let (_, token, dispatch) = started(&core);
    let mut executor = TestExecutor::new();
    core.dispatch(dispatch, &mut executor).unwrap();
    assert_eq!(executor.dispatched(), &[token]);
}

#[test]
fn recording_a_refusal_after_the_execution_ran_is_rejected() {
    let core = acceptor();
    let (id, _) = running_with(&core, Handoff::Acknowledged);
    assert_eq!(
        core.record_handoff(
            &id,
            &Handoff::Refused {
                reason: "contradiction".to_string()
            }
        ),
        Err(NotifyRejection::InvalidTransition {
            from: State::Running
        })
    );
    // An ambiguous result after progress is no information; it never downgrades.
    assert_eq!(
        core.record_handoff(&id, &Handoff::Ambiguous),
        Ok(Notified::NoChange { sequence: 2 })
    );
    assert_eq!(snapshot(&core, &id).state, State::Running);
    assert_eq!(
        core.record_handoff(&OperationId::generate(), &Handoff::Ambiguous),
        Err(NotifyRejection::UnknownToken)
    );
}

#[test]
fn concurrent_copies_of_one_event_apply_once() {
    let core = Arc::new(acceptor());
    let (id, token, _dispatch) = started(&core);
    let ack = event(&token, 1, EventKind::Ack);
    let results: Vec<_> = (0..32)
        .map(|_| {
            let core = Arc::clone(&core);
            let ack = ack.clone();
            thread::spawn(move || core.notify(&ack))
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|h| h.join().unwrap())
        .collect();
    let applied = results
        .iter()
        .filter(|r| matches!(r, Ok(Notified::Applied { .. })))
        .count();
    let duplicates = results
        .iter()
        .filter(|r| matches!(r, Ok(Notified::Duplicate { sequence: 2 })))
        .count();
    assert_eq!((applied, duplicates), (1, 31));
    assert_eq!(snapshot(&core, &id).sequence, 2);
}

#[test]
fn competing_outcomes_on_one_event_number_leave_exactly_one_terminal() {
    for _ in 0..50 {
        let core = Arc::new(acceptor());
        let (id, token) = running_with(&core, Handoff::Acknowledged);
        let kinds = [terminal(Outcome::Succeeded), terminal(Outcome::Failed)];
        let results: Vec<_> = kinds
            .into_iter()
            .map(|kind| {
                let core = Arc::clone(&core);
                let ev = event(&token, 1, kind);
                thread::spawn(move || core.notify(&ev))
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect();
        let applied = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(applied, 1, "{results:?}");
        assert!(results.contains(&Err(NotifyRejection::Stale)));
        let snap = snapshot(&core, &id);
        assert!(snap.state.is_terminal());
        assert_eq!(snap.sequence, 3);
        // The winner is fixed: the loser cannot rewrite it later.
        let loser = if snap.state == State::Succeeded {
            Outcome::Failed
        } else {
            Outcome::Succeeded
        };
        assert_eq!(
            core.notify(&event(&token, 2, terminal(loser))),
            Err(NotifyRejection::ConflictingTerminal {
                recorded: snap.state
            })
        );
    }
}
