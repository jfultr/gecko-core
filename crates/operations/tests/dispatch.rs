//! Dispatch failures: a handoff that cannot be recorded, and an execution that
//! cannot reach its executor (O8 and O9, November 8, 2026).

use operations::report::State;
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::testing::{FlakyStorage, PermitAllForTests, TestExecutor};
use operations::{
    Acceptor, DispatchError, EventKind, ExecutionToken, ExecutorEvent, Guard, Handoff,
    MemoryDecisionLog, Notified, NotifyRejection, OperationId, Storage,
};

type TestAcceptor = Acceptor<PermitAllForTests, MemoryDecisionLog, FlakyStorage>;

fn acceptor() -> TestAcceptor {
    Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        FlakyStorage::new(),
    )
}

/// Starts one execution and returns its id and its unspent dispatch.
fn started(core: &TestAcceptor) -> (OperationId, operations::Dispatch) {
    let outcome = core.start(&caller("alice"), &echo_request("hi"));
    let dispatch = outcome.dispatch.expect("created");
    (dispatch.operation_id(), dispatch)
}

fn token_of(core: &TestAcceptor, id: &OperationId) -> ExecutionToken {
    core.storage().operation(id).unwrap().unwrap().token
}

fn snapshot(core: &TestAcceptor, id: &OperationId) -> operations::Report {
    core.storage().operation(id).unwrap().unwrap().snapshot
}

#[test]
fn dispatch_read_failure_fails_the_execution_without_reaching_the_executor() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_reads(1);
    let mut executor = TestExecutor::new();

    assert_eq!(
        core.dispatch(dispatch, &mut executor),
        Err(DispatchError::Unavailable)
    );
    assert!(executor.dispatched().is_empty());
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Failed);
    assert!(snap.explanation.contains("nothing reached the executor"));
}

#[test]
fn dispatch_read_failure_with_a_failed_record_stays_pending() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_reads(1);
    core.storage().fail_next_updates(1);
    let mut executor = TestExecutor::new();

    assert_eq!(
        core.dispatch(dispatch, &mut executor),
        Err(DispatchError::Unavailable)
    );
    assert!(executor.dispatched().is_empty());
    assert_eq!(snapshot(&core, &id).state, State::Pending);
}

#[test]
fn dispatch_retries_recording_the_handoff_result() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_updates(2);
    let mut executor = TestExecutor::new();

    let result = core.dispatch(dispatch, &mut executor);

    assert_eq!(result, Ok(Notified::Applied { sequence: 2 }));
    assert_eq!(executor.dispatched().len(), 1);
    assert_eq!(snapshot(&core, &id).state, State::Running);
}

#[test]
fn dispatch_falls_back_to_ambiguous_when_the_result_cannot_be_recorded() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_updates(3);
    let mut executor = TestExecutor::new();

    let result = core.dispatch(dispatch, &mut executor);

    assert_eq!(result, Ok(Notified::Applied { sequence: 2 }));
    assert_eq!(executor.dispatched().len(), 1);
    assert_eq!(snapshot(&core, &id).state, State::Unknown);

    // The executor confirms later; the unknown execution becomes running.
    let token = token_of(&core, &id);
    let ack = ExecutorEvent {
        token,
        event_seq: 1,
        kind: EventKind::Ack,
    };
    assert_eq!(core.notify(&ack), Ok(Notified::Applied { sequence: 3 }));
    assert_eq!(snapshot(&core, &id).state, State::Running);
}

#[test]
fn dispatch_reports_an_unrecorded_handoff_when_storage_stays_down() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_updates(100);
    let mut executor = TestExecutor::new();

    let result = core.dispatch(dispatch, &mut executor);

    assert_eq!(
        result,
        Err(DispatchError::Unrecorded {
            handoff: Handoff::Acknowledged
        })
    );
    assert_eq!(executor.dispatched().len(), 1);
    assert_eq!(snapshot(&core, &id).state, State::Pending);
}

#[test]
fn abandon_fails_the_execution_without_a_handoff() {
    let core = acceptor();
    let (id, dispatch) = started(&core);

    let result = core.abandon(dispatch, "the executor is unavailable");

    assert_eq!(result, Ok(Notified::Applied { sequence: 2 }));
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Failed);
    assert_eq!(
        snap.explanation,
        "the execution could not be handed to its executor; nothing reached the executor: \
         the executor is unavailable"
    );
}

#[test]
fn abandon_reports_storage_failure_as_unavailable() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_updates(1);

    assert_eq!(
        core.abandon(dispatch, "the executor is unavailable"),
        Err(DispatchError::Unavailable)
    );
    assert_eq!(snapshot(&core, &id).state, State::Pending);
}

#[test]
fn abandon_after_the_execution_left_pending_is_rejected() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    // An executor Ack can arrive while the execution is still pending, which
    // moves it to running. Abandon must then refuse to fail it.
    let ack = ExecutorEvent {
        token: token_of(&core, &id),
        event_seq: 1,
        kind: EventKind::Ack,
    };
    assert_eq!(core.notify(&ack), Ok(Notified::Applied { sequence: 2 }));

    assert_eq!(
        core.abandon(dispatch, "the executor is unavailable"),
        Err(DispatchError::Rejected(
            NotifyRejection::InvalidTransition {
                from: State::Running
            }
        ))
    );
    assert_eq!(snapshot(&core, &id).state, State::Running);
}

#[test]
fn a_refusal_that_cannot_be_recorded_is_never_downgraded_to_unknown() {
    let core = acceptor();
    let (id, dispatch) = started(&core);
    core.storage().fail_next_updates(3);
    let mut executor = TestExecutor::new();
    executor.queue_response(Handoff::Refused {
        reason: "no capacity".to_string(),
    });

    assert!(core.dispatch(dispatch, &mut executor).is_ok());
    let snap = snapshot(&core, &id);
    assert_eq!(snap.state, State::Failed);
    assert!(snap.explanation.contains("refused"));
}
