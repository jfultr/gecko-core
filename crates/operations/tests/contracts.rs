//! Contract tests: round-trip, validation, retry identity, trust hooks, report independence.

use operations::report::{Blocker, Operation, Progress, Report, State, Timestamp};
use operations::testing::PermitAllForTests;
use operations::testing::contract_fixtures::{caller, catalog, echo_request, echo_type};
use operations::testing::fixtures;
use operations::{
    Authorizer, CallerContext, Command, Decision, DecisionLog, DecisionRecord, DenyAll, GetRequest,
    GetResponse, Guard, Handoff, MemoryDecisionLog, ObservationAddress, ObserveRequest,
    ObserveResponse, OperationId, Refusal, RejectionCode, RequestId, RetryClass, RetryKey,
    StartRequest, StartResponse, Unavailable, Value, auth,
};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::json;

fn round_trip<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(value: &T) {
    let text = serde_json::to_string(value).expect("serialize");
    let back: T = serde_json::from_str(&text).expect("deserialize");
    assert_eq!(&back, value, "round trip through {text}");
}

fn sample_report(state: State) -> Report {
    let operation = Operation {
        id: OperationId::generate(),
        operation_type: echo_type(),
        parent_id: None,
    };
    Report::new(operation, 3, Timestamp(1_700_000_000_000), state)
}

// ---- round trip ----

#[test]
fn requests_and_responses_round_trip() {
    let mut request = echo_request("hello");
    request.target = Some(operations::Target::new("device-1").unwrap());
    request.trace.insert("trace_id".into(), "abc".into());
    round_trip(&request);
    round_trip(&GetRequest {
        operation_id: OperationId::generate(),
    });
    round_trip(&ObserveRequest {
        operation_id: OperationId::generate(),
        after_sequence: Some(7),
    });

    let report = sample_report(State::Running);
    round_trip(&report);
    round_trip(&StartResponse::Accepted {
        operation: report.operation.clone(),
        observation: ObservationAddress("observe/1".into()),
    });
    round_trip(&StartResponse::Rejected(operations::Rejection::new(
        RejectionCode::UnknownOperationType,
        "nope",
        vec![],
    )));
    round_trip(&GetResponse::Report(Box::new(report.clone())));
    round_trip(&GetResponse::Unavailable(Unavailable {}));
    round_trip(&ObserveResponse::Attached {
        current: Box::new(report),
    });
    round_trip(&ObserveResponse::Unavailable(Unavailable {}));
}

#[test]
fn values_keep_number_kind_through_serialization() {
    let value: Value = serde_json::from_str(r#"{"a":1,"b":1.0,"c":[null,true,"x"]}"#).unwrap();
    let Value::Map(map) = &value else { panic!() };
    assert!(matches!(map["a"], Value::Integer(1)));
    assert!(matches!(map["b"], Value::Float(_)));
    round_trip(&value);
}

#[test]
fn identifiers_must_be_uuid_v4() {
    let id = RequestId::generate();
    assert_eq!(RequestId::parse(&id.to_string()), Ok(id));
    assert!(RequestId::parse("not-a-uuid").is_err());
    // A version-1 UUID is well formed but not v4.
    assert!(RequestId::parse("c232ab00-9414-11ec-b3c8-9f6bdeced846").is_err());
    assert!(
        serde_json::from_value::<RequestId>(json!("c232ab00-9414-11ec-b3c8-9f6bdeced846")).is_err()
    );
    assert_ne!(RequestId::generate(), RequestId::generate());
}

// ---- validation: no fictitious operation ----

#[test]
fn valid_request_passes_validation() {
    assert!(catalog().validate(&echo_request("hi")).is_ok());
}

#[test]
fn unknown_type_is_rejected() {
    let mut request = echo_request("hi");
    request.operation_type = operations::OperationType::new("test.unknown").unwrap();
    let rejection = catalog().validate(&request).unwrap_err();
    assert_eq!(rejection.code, RejectionCode::UnknownOperationType);
}

#[test]
fn invalid_parameters_are_rejected_with_all_violations() {
    let mut request = echo_request("hi");
    request.parameters.remove("message");
    request
        .parameters
        .insert("repeat".into(), Value::Text("3".into()));
    request.parameters.insert("extra".into(), Value::Null);
    let rejection = catalog().validate(&request).unwrap_err();
    assert_eq!(rejection.code, RejectionCode::InvalidParameters);
    let fields: Vec<_> = rejection
        .violations
        .iter()
        .map(|v| v.field.as_str())
        .collect();
    assert!(fields.contains(&"parameters.message"), "{fields:?}");
    assert!(fields.contains(&"parameters.repeat"), "{fields:?}");
    assert!(fields.contains(&"parameters.extra"), "{fields:?}");
}

#[test]
fn malformed_input_never_becomes_a_request() {
    for bad in [
        json!({"request_id": "x", "operation_type": "test.echo"}),
        json!({"request_id": RequestId::generate(), "operation_type": "Bad Type"}),
        json!({"request_id": RequestId::generate(), "operation_type": "test.echo", "target": " "}),
    ] {
        assert!(serde_json::from_value::<StartRequest>(bad).is_err());
    }
}

// ---- retry identity ----

#[test]
fn retry_comparison_uses_type_target_and_parameters_only() {
    let a = echo_request("hi");
    let mut b = a.clone();
    b.trace.insert("trace_id".into(), "other".into());
    assert_eq!(
        RetryClass::classify(Some(&a.fingerprint()), &b.fingerprint()),
        RetryClass::Replay
    );
    assert_eq!(
        RetryClass::classify(None, &a.fingerprint()),
        RetryClass::New
    );

    let mut changed = a.clone();
    changed
        .parameters
        .insert("message".into(), Value::Text("other".into()));
    assert_eq!(
        RetryClass::classify(Some(&a.fingerprint()), &changed.fingerprint()),
        RetryClass::Conflict
    );

    let mut targeted = a.clone();
    targeted.target = Some(operations::Target::new("device-1").unwrap());
    assert_eq!(
        RetryClass::classify(Some(&a.fingerprint()), &targeted.fingerprint()),
        RetryClass::Conflict
    );
}

#[test]
fn retry_comparison_is_spelling_sensitive_for_numbers_but_not_key_order() {
    let mut a = echo_request("hi");
    a.parameters.insert("repeat".into(), Value::Integer(1));
    let mut b = a.clone();
    b.parameters
        .insert("repeat".into(), Value::float(1.0).unwrap());
    assert_eq!(
        RetryClass::classify(Some(&a.fingerprint()), &b.fingerprint()),
        RetryClass::Conflict
    );

    let x: StartRequest = serde_json::from_value(json!({
        "request_id": a.request_id,
        "operation_type": "test.echo",
        "parameters": {"message": "hi", "repeat": 1}
    }))
    .unwrap();
    let y: StartRequest = serde_json::from_value(json!({
        "request_id": a.request_id,
        "operation_type": "test.echo",
        "parameters": {"repeat": 1, "message": "hi"}
    }))
    .unwrap();
    assert_eq!(x.fingerprint(), y.fingerprint());
}

#[test]
fn retry_key_is_scoped_to_the_trusted_subject() {
    let id = RequestId::generate();
    let alice = RetryKey::new(&caller("alice"), id);
    assert_eq!(alice, RetryKey::new(&caller("alice"), id));
    assert_ne!(alice, RetryKey::new(&caller("bob"), id));
    assert_ne!(
        alice,
        RetryKey::new(&caller("alice"), RequestId::generate())
    );
}

// ---- payload identity spoofing ----

#[test]
fn payload_cannot_carry_a_caller_identity() {
    for field in ["subject", "caller", "caller_context", "user"] {
        let mut payload = json!({
            "request_id": RequestId::generate(),
            "operation_type": "test.echo",
        });
        payload[field] = json!("admin");
        assert!(
            serde_json::from_value::<StartRequest>(payload).is_err(),
            "field `{field}` must be rejected"
        );
    }
    let get = json!({"operation_id": OperationId::generate(), "subject": "admin"});
    assert!(serde_json::from_value::<GetRequest>(get).is_err());
    let observe = json!({"operation_id": OperationId::generate(), "subject": "admin"});
    assert!(serde_json::from_value::<ObserveRequest>(observe).is_err());
}

// ---- authorization and decision records ----

fn start_command(request: &StartRequest) -> Command {
    Command::Start {
        request_id: request.request_id,
        operation_type: request.operation_type.clone(),
        target: request.target.clone(),
    }
}

#[test]
fn default_policy_denies_and_records_the_denial() {
    let mut guard = Guard::new(DenyAll, MemoryDecisionLog::new());
    let request = echo_request("hi");
    let result = guard.check(&caller("alice"), &start_command(&request));
    assert!(matches!(result, Err(Refusal::Denied { .. })));
    let records = guard.log().records();
    assert_eq!(records.len(), 1);
    assert!(matches!(records[0].decision, Decision::Deny { .. }));
    assert_eq!(records[0].subject.as_str(), "alice");
}

struct SubjectPolicy;
impl Authorizer for SubjectPolicy {
    fn authorize(&self, caller: &CallerContext, command: &Command) -> Decision {
        match (caller.subject().as_str(), command) {
            ("alice", _) => Decision::Permit,
            (_, Command::Get { .. }) => Decision::Unknown {
                reason: "policy store offline".into(),
            },
            _ => Decision::Deny {
                reason: "not allowed".into(),
            },
        }
    }
}

#[test]
fn every_public_command_is_authorized_and_recorded() {
    let mut guard = Guard::new(SubjectPolicy, MemoryDecisionLog::new());
    let request = echo_request("hi");
    let id = OperationId::generate();
    let commands = [
        start_command(&request),
        Command::Get { operation_id: id },
        Command::Observe { operation_id: id },
    ];
    for command in &commands {
        assert!(guard.check(&caller("alice"), command).is_ok());
    }
    assert!(matches!(
        guard.check(&caller("bob"), &commands[0]),
        Err(Refusal::Denied { .. })
    ));
    assert!(matches!(
        guard.check(&caller("bob"), &commands[1]),
        Err(Refusal::Unknown { .. })
    ));
    let records = guard.log().records();
    assert_eq!(records.len(), 5);
    assert!(records.iter().enumerate().all(|(i, r)| r.index == i as u64));
}

struct FailingLog;
impl DecisionLog for FailingLog {
    fn append(&mut self, _record: DecisionRecord) -> Result<(), operations::auth::LogError> {
        Err(operations::auth::LogError("disk full".into()))
    }
}

#[test]
fn unrecorded_decision_yields_no_proof_and_no_handoff() {
    let mut guard = Guard::new(PermitAllForTests::new(), FailingLog);
    let request = echo_request("hi");
    let result = guard.check(&caller("alice"), &start_command(&request));
    assert!(matches!(result, Err(Refusal::NotRecorded(_))));
}

#[test]
fn handoff_requires_a_recorded_permit() {
    let mut guard = Guard::new(PermitAllForTests::new(), MemoryDecisionLog::new());
    let request = echo_request("hi");
    let proof = guard
        .check(&caller("alice"), &start_command(&request))
        .unwrap();
    assert_eq!(
        guard.log().records().len(),
        1,
        "record exists before handoff"
    );
    assert_eq!(proof.record_index(), 0);

    let mut executor = operations::testing::TestExecutor::new();
    let token = fixtures::execution_token(1);
    assert_eq!(
        auth::hand_off(&proof, &mut executor, &token),
        Handoff::Acknowledged
    );
    assert_eq!(executor.dispatched().len(), 1);

    // A denied command produces no proof, so nothing reaches the executor.
    let mut denying = Guard::new(DenyAll, MemoryDecisionLog::new());
    assert!(
        denying
            .check(&caller("alice"), &start_command(&request))
            .is_err()
    );
    assert_eq!(executor.dispatched().len(), 1);
}

// ---- report independence ----

#[test]
fn blockers_and_partial_results_are_independent_of_state() {
    for state in [
        State::Pending,
        State::Running,
        State::Succeeded,
        State::Failed,
        State::Cancelled,
        State::Unknown,
    ] {
        let mut report = sample_report(state);
        report.blockers.push(Blocker {
            code: "driver_missing".into(),
            subject: "gpu-0".into(),
            message: "driver is missing".into(),
            evidence: vec![],
            next_actions: vec![],
        });
        report.limitations.push("only first frame checked".into());
        report.result = Value::Map([("frames".to_string(), Value::Integer(1))].into());
        report.progress = Some(Progress {
            stage: "probe".into(),
            message: "partial".into(),
            ..Default::default()
        });
        round_trip(&report);
        assert_eq!(report.state, state);
        assert_eq!(report.blockers.len(), 1);
    }
    assert!(State::Failed.is_terminal() && !State::Unknown.is_terminal());
}
