//! Acceptance tests: atomic commit, safe retry, concurrency, scope, failure injection.

use std::thread;

use operations::report::State;
use operations::testing::PermitAllForTests;
use operations::testing::contract_fixtures::{caller, catalog, echo_request};
use operations::{
    Acceptor, DenyAll, Guard, MemoryDecisionLog, MemoryStorage, RejectionCode, StartRequest,
    StartResponse, Storage, Value,
};

type TestAcceptor = Acceptor<PermitAllForTests, MemoryDecisionLog, MemoryStorage>;

fn acceptor() -> TestAcceptor {
    Acceptor::new(
        catalog(),
        Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new()),
        MemoryStorage::new(),
    )
}

fn accepted_id(response: &StartResponse) -> operations::OperationId {
    match response {
        StartResponse::Accepted { operation, .. } => operation.id,
        other => panic!("expected acceptance, got {other:?}"),
    }
}

fn rejection_code(response: &StartResponse) -> RejectionCode {
    match response {
        StartResponse::Rejected(r) => r.code,
        other => panic!("expected rejection, got {other:?}"),
    }
}

#[test]
fn acceptance_commits_mapping_record_and_pending_snapshot() {
    let core = acceptor();
    let request = echo_request("hi");
    let outcome = core.start(&caller("alice"), &request);
    let id = accepted_id(&outcome.response);
    assert_eq!(
        outcome.dispatch.as_ref().map(|d| d.operation_id()),
        Some(id)
    );

    let stored = core.storage().operation(&id).unwrap().expect("record");
    assert_eq!(stored.owner.as_str(), "alice");
    assert_eq!(stored.snapshot.state, State::Pending);
    assert_eq!(stored.snapshot.sequence, 1);
    assert_eq!(stored.snapshot.operation.id, id);
    assert!(stored.snapshot.blockers.is_empty());
    assert_eq!(core.storage().mapping_count(), 1);
    assert_eq!(core.storage().operation_count(), 1);
    assert_eq!(core.with_decision_log(|l| l.records().len()), Some(1));
    // The execution is linked to the decision that permitted it.
    let linked = core
        .with_decision_log(|l| l.records()[stored.decision_index as usize].clone())
        .unwrap();
    assert_eq!(linked.decision, operations::Decision::Permit);
    assert!(matches!(
        linked.command,
        operations::Command::Start { request_id, .. } if request_id == request.request_id
    ));
}

#[test]
fn lost_response_retry_returns_the_same_operation_without_a_second_dispatch() {
    let core = acceptor();
    let request = echo_request("hi");
    let first = core.start(&caller("alice"), &request);
    // The caller never saw `first.response` and retries.
    let retry = core.start(&caller("alice"), &request);
    assert_eq!(accepted_id(&first.response), accepted_id(&retry.response));
    assert_eq!(first.response, retry.response);
    assert!(first.dispatch.is_some());
    assert!(retry.dispatch.is_none(), "a retry must not dispatch");
    assert_eq!(core.storage().operation_count(), 1);
}

#[test]
fn retry_ignores_tracing_metadata() {
    let core = acceptor();
    let request = echo_request("hi");
    let first = core.start(&caller("alice"), &request);
    let mut retry = request.clone();
    retry.trace.insert("trace-id".into(), "other".into());
    let second = core.start(&caller("alice"), &retry);
    assert_eq!(accepted_id(&first.response), accepted_id(&second.response));
    assert_eq!(core.storage().operation_count(), 1);
}

#[test]
fn changed_content_conflicts_and_leaves_the_original_unchanged() {
    let core = acceptor();
    let request = echo_request("hi");
    let first = core.start(&caller("alice"), &request);
    let id = accepted_id(&first.response);
    let before = core.storage().operation(&id).unwrap().unwrap();

    let mut changed = request.clone();
    changed
        .parameters
        .insert("message".into(), Value::Text("different".into()));
    let outcome = core.start(&caller("alice"), &changed);
    assert_eq!(
        rejection_code(&outcome.response),
        RejectionCode::IdentifierConflict
    );
    assert!(outcome.dispatch.is_none());

    assert_eq!(core.storage().operation(&id).unwrap().unwrap(), before);
    assert_eq!(core.storage().operation_count(), 1);
    assert_eq!(core.storage().mapping_count(), 1);
    // The original content still replays to the original operation.
    let again = core.start(&caller("alice"), &request);
    assert_eq!(accepted_id(&again.response), id);
}

#[test]
fn the_retry_key_is_scoped_to_the_caller() {
    let core = acceptor();
    let request = echo_request("hi");
    let alice = core.start(&caller("alice"), &request);
    let bob = core.start(&caller("bob"), &request);
    assert_ne!(accepted_id(&alice.response), accepted_id(&bob.response));
    assert!(alice.dispatch.is_some() && bob.dispatch.is_some());
    assert_eq!(core.storage().operation_count(), 2);

    // Another caller reusing Alice's id with other content is not a conflict with Alice.
    let mut other = request.clone();
    other
        .parameters
        .insert("message".into(), Value::Text("other".into()));
    let carol = core.start(&caller("carol"), &other);
    assert!(matches!(carol.response, StartResponse::Accepted { .. }));
    assert!(carol.dispatch.is_some());
    // Bob's own key is still bound to his content, not Alice's or Carol's.
    let bob_retry = core.start(&caller("bob"), &request);
    assert_eq!(accepted_id(&bob_retry.response), accepted_id(&bob.response));
    assert_eq!(
        rejection_code(&core.start(&caller("bob"), &other).response),
        RejectionCode::IdentifierConflict
    );
    assert_eq!(core.storage().operation_count(), 3);
}

#[test]
fn the_retry_key_is_scoped_to_the_receiving_gecko() {
    // One acceptor owns one storage, so two Geckos never share a retry index.
    let first = acceptor();
    let second = acceptor();
    let request = echo_request("hi");
    let a = first.start(&caller("alice"), &request);
    let b = second.start(&caller("alice"), &request);
    assert_ne!(accepted_id(&a.response), accepted_id(&b.response));
    assert!(a.dispatch.is_some() && b.dispatch.is_some());
    assert_eq!(first.storage().operation_count(), 1);
    assert_eq!(second.storage().operation_count(), 1);
}

#[test]
fn equality_is_normalized_for_key_order_and_spelling_sensitive_for_numbers() {
    let core = acceptor();
    let mut request = echo_request("hi");
    request
        .parameters
        .insert("repeat".into(), Value::Integer(1));
    let original = core.start(&caller("alice"), &request);

    // Same content assembled in another insertion order is a replay.
    let mut reordered = StartRequest {
        parameters: Default::default(),
        ..request.clone()
    };
    reordered
        .parameters
        .insert("repeat".into(), Value::Integer(1));
    reordered
        .parameters
        .insert("message".into(), Value::Text("hi".into()));
    let replay = core.start(&caller("alice"), &reordered);
    assert_eq!(
        accepted_id(&replay.response),
        accepted_id(&original.response)
    );
    assert!(replay.dispatch.is_none());

    // `1` and `1.0` are different content (Z4).
    let mut float = request.clone();
    float
        .parameters
        .insert("repeat".into(), Value::float(1.0).unwrap());
    assert_ne!(request.fingerprint(), float.fingerprint());
    assert_eq!(core.storage().operation_count(), 1);
}

#[test]
fn concurrent_same_content_retries_create_one_execution() {
    let core = acceptor();
    let request = echo_request("hi");
    let outcomes: Vec<_> = thread::scope(|scope| {
        let handles: Vec<_> = (0..32)
            .map(|_| scope.spawn(|| core.start(&caller("alice"), &request)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let id = accepted_id(&outcomes[0].response);
    assert!(outcomes.iter().all(|o| accepted_id(&o.response) == id));
    assert_eq!(outcomes.iter().filter(|o| o.dispatch.is_some()).count(), 1);
    assert_eq!(core.storage().operation_count(), 1);
    assert_eq!(core.storage().mapping_count(), 1);
}

#[test]
fn concurrent_requests_with_different_content_have_one_winner() {
    let core = acceptor();
    let base = echo_request("base");
    let outcomes: Vec<_> = thread::scope(|scope| {
        let handles: Vec<_> = (0..16)
            .map(|i| {
                let mut request = base.clone();
                request
                    .parameters
                    .insert("message".into(), Value::Text(format!("variant {i}")));
                let core = &core;
                scope.spawn(move || core.start(&caller("alice"), &request))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let accepted = outcomes
        .iter()
        .filter(|o| matches!(o.response, StartResponse::Accepted { .. }))
        .count();
    let conflicts = outcomes
        .iter()
        .filter(|o| matches!(&o.response, StartResponse::Rejected(r) if r.code == RejectionCode::IdentifierConflict))
        .count();
    assert_eq!((accepted, conflicts), (1, 15));
    assert_eq!(outcomes.iter().filter(|o| o.dispatch.is_some()).count(), 1);
    assert_eq!(core.storage().operation_count(), 1);
}

#[test]
fn unrelated_keys_are_accepted_concurrently_and_independently() {
    let core = acceptor();
    let outcomes: Vec<_> = thread::scope(|scope| {
        let handles: Vec<_> = (0..32)
            .map(|_| {
                let core = &core;
                scope.spawn(move || core.start(&caller("alice"), &echo_request("hi")))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    assert!(outcomes.iter().all(|o| o.dispatch.is_some()));
    assert_eq!(core.storage().operation_count(), 32);
    assert_eq!(core.with_decision_log(|l| l.records().len()), Some(32));
}

#[test]
fn commit_failure_accepts_nothing_dispatches_nothing_and_stores_no_mapping() {
    let core = acceptor();
    let request = echo_request("hi");
    core.storage().inject_commit_failures(1);

    let failed = core.start(&caller("alice"), &request);
    assert!(matches!(failed.response, StartResponse::Error(_)));
    assert!(failed.dispatch.is_none());
    assert_eq!(core.storage().operation_count(), 0);
    assert_eq!(core.storage().mapping_count(), 0);

    // The same request_id is still free: the retry creates exactly one execution.
    let retry = core.start(&caller("alice"), &request);
    assert!(matches!(retry.response, StartResponse::Accepted { .. }));
    assert!(retry.dispatch.is_some());
    assert_eq!(core.storage().operation_count(), 1);
    assert_eq!(core.storage().mapping_count(), 1);
}

#[test]
fn concurrent_retries_after_a_commit_failure_create_exactly_one_execution() {
    let core = acceptor();
    let request = echo_request("hi");
    core.storage().inject_commit_failures(1);
    let outcomes: Vec<_> = thread::scope(|scope| {
        let handles: Vec<_> = (0..32)
            .map(|_| scope.spawn(|| core.start(&caller("alice"), &request)))
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let errors = outcomes
        .iter()
        .filter(|o| matches!(o.response, StartResponse::Error(_)))
        .count();
    assert_eq!(errors, 1, "only the injected commit fails");
    assert_eq!(outcomes.iter().filter(|o| o.dispatch.is_some()).count(), 1);
    assert_eq!(core.storage().operation_count(), 1);
    assert_eq!(core.storage().mapping_count(), 1);
    let ids: Vec<_> = outcomes
        .iter()
        .filter(|o| matches!(o.response, StartResponse::Accepted { .. }))
        .map(|o| accepted_id(&o.response))
        .collect();
    assert_eq!(ids.len(), 31);
    assert!(ids.iter().all(|id| *id == ids[0]));
}

#[test]
fn denied_and_invalid_requests_create_no_operation() {
    let denying: Acceptor<DenyAll, MemoryDecisionLog, MemoryStorage> = Acceptor::new(
        catalog(),
        Guard::new(DenyAll, MemoryDecisionLog::new()),
        MemoryStorage::new(),
    );
    let mut unknown = echo_request("hi");
    unknown.operation_type = operations::OperationType::new("no.such.type").unwrap();
    // A denied caller learns nothing about the catalog: unknown type looks like any denial.
    for request in [echo_request("hi"), unknown.clone()] {
        let outcome = denying.start(&caller("alice"), &request);
        assert_eq!(rejection_code(&outcome.response), RejectionCode::Denied);
        assert!(outcome.dispatch.is_none());
    }
    assert_eq!(denying.storage().operation_count(), 0);
    assert_eq!(denying.with_decision_log(|l| l.records().len()), Some(2));

    let core = acceptor();
    let outcome = core.start(&caller("alice"), &unknown);
    assert_eq!(
        rejection_code(&outcome.response),
        RejectionCode::UnknownOperationType
    );
    let mut invalid: StartRequest = echo_request("hi");
    invalid.parameters.clear();
    let outcome = core.start(&caller("alice"), &invalid);
    assert_eq!(
        rejection_code(&outcome.response),
        RejectionCode::InvalidParameters
    );
    assert_eq!(core.storage().operation_count(), 0);
    assert_eq!(core.storage().mapping_count(), 0);
}
