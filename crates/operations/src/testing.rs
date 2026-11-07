//! Test support (enabled by the `test-support` feature): a scriptable executor
//! and deterministic fixtures. Not for production use.
//!
//! Tests queue the handoff outcomes they need, then inspect recorded calls.

use std::collections::VecDeque;

use crate::executor::{ExecutionToken, Executor, Handoff};

/// A scriptable executor that records every dispatched token.
///
/// Queued responses are consumed in call order; with an empty queue every
/// handoff is acknowledged.
#[derive(Debug, Default)]
pub struct TestExecutor {
    responses: VecDeque<Handoff>,
    dispatched: Vec<ExecutionToken>,
}

impl TestExecutor {
    /// Creates an executor with an empty response queue and no recorded calls.
    pub fn new() -> Self {
        Self::default()
    }

    /// Queues the outcome of the next handoff that is not yet answered.
    pub fn queue_response(&mut self, response: Handoff) -> &mut Self {
        self.responses.push_back(response);
        self
    }

    /// Returns every token handed to this executor, in call order.
    pub fn dispatched(&self) -> &[ExecutionToken] {
        &self.dispatched
    }
}

impl Executor for TestExecutor {
    fn hand_off(&mut self, token: &ExecutionToken) -> Handoff {
        self.dispatched.push(token.clone());
        self.responses.pop_front().unwrap_or(Handoff::Acknowledged)
    }
}

/// Deterministic fixtures for tests.
pub mod fixtures {
    use crate::executor::ExecutionToken;

    /// Returns the deterministic token for test case `n`.
    ///
    /// Equal inputs give equal tokens.
    pub fn execution_token(n: u32) -> ExecutionToken {
        ExecutionToken::new(format!("test-execution-{n}"))
    }
}

/// Permissive authorization for the restricted `0.1.0` test deployment only.
///
/// Permits every command. It exists only behind the `test-support` feature, so
/// a production build cannot construct it. It makes no authentication or audit
/// claim.
#[derive(Debug, Clone, Copy)]
pub struct PermitAllForTests(());

impl PermitAllForTests {
    /// Creates the permissive test policy.
    pub fn new() -> Self {
        Self(())
    }
}

impl Default for PermitAllForTests {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::auth::Authorizer for PermitAllForTests {
    fn authorize(
        &self,
        _caller: &crate::identity::CallerContext,
        _command: &crate::auth::Command,
    ) -> crate::auth::Decision {
        crate::auth::Decision::Permit
    }
}

/// Contract fixtures: a catalog with one test operation type and a trusted caller.
pub mod contract_fixtures {
    use crate::identity::{CallerContext, RequestId, Subject};
    use crate::request::{
        Catalog, OperationSpec, OperationType, ParameterSpec, StartRequest, TargetRule,
    };
    use crate::value::{Parameters, Value, ValueKind};

    /// Operation type of the test catalog.
    pub fn echo_type() -> OperationType {
        OperationType::new("test.echo").expect("valid type")
    }

    /// A catalog with `test.echo`: optional target, required text `message`, optional integer `repeat`.
    pub fn catalog() -> Catalog {
        let mut catalog = Catalog::new();
        catalog.register(OperationSpec {
            operation_type: echo_type(),
            target: TargetRule::Optional,
            parameters: vec![
                ParameterSpec {
                    name: "message".into(),
                    kind: ValueKind::Text,
                    required: true,
                },
                ParameterSpec {
                    name: "repeat".into(),
                    kind: ValueKind::Integer,
                    required: false,
                },
            ],
        });
        catalog
    }

    /// A trusted caller context for `name`, vouched for by a test adapter.
    pub fn caller(name: &str) -> CallerContext {
        CallerContext::from_adapter("test-adapter", Subject::new(name).expect("subject"))
    }

    /// A valid `test.echo` request with a fresh `request_id`.
    pub fn echo_request(message: &str) -> StartRequest {
        let mut parameters = Parameters::new();
        parameters.insert("message".into(), Value::Text(message.into()));
        StartRequest {
            request_id: RequestId::generate(),
            operation_type: echo_type(),
            target: None,
            parameters,
            trace: Default::default(),
        }
    }
}
