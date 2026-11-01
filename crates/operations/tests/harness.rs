//! Harness tests: exercise the scriptable `TestExecutor` and deterministic fixtures.

use operations::testing::{TestExecutor, fixtures};
use operations::{Executor, Handoff};

#[test]
fn default_executor_acknowledges_and_records_the_token() {
    let mut executor = TestExecutor::new();
    let token = fixtures::execution_token(1);

    assert_eq!(executor.hand_off(&token), Handoff::Acknowledged);
    assert_eq!(executor.dispatched(), std::slice::from_ref(&token));
}

#[test]
fn queued_responses_are_used_in_call_order() {
    let mut executor = TestExecutor::new();
    executor
        .queue_response(Handoff::Refused {
            reason: "queued refusal".to_string(),
        })
        .queue_response(Handoff::Ambiguous);

    let tokens = [
        fixtures::execution_token(1),
        fixtures::execution_token(2),
        fixtures::execution_token(3),
    ];

    assert_eq!(
        executor.hand_off(&tokens[0]),
        Handoff::Refused {
            reason: "queued refusal".to_string(),
        }
    );
    assert_eq!(executor.hand_off(&tokens[1]), Handoff::Ambiguous);
    assert_eq!(executor.hand_off(&tokens[2]), Handoff::Acknowledged);
    assert_eq!(executor.dispatched(), &tokens);
}

#[test]
fn fixtures_are_deterministic_and_distinct() {
    assert_eq!(fixtures::execution_token(7), fixtures::execution_token(7));
    assert_ne!(fixtures::execution_token(7), fixtures::execution_token(8));
    assert_eq!(fixtures::execution_token(7).as_str(), "test-execution-7");
}
