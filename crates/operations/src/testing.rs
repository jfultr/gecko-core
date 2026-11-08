//! Test support (enabled by the `test-support` feature): a scriptable executor
//! and deterministic fixtures. Not for production use.
//!
//! Tests queue the handoff outcomes they need, then inspect recorded calls.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::acceptance::{Acceptance, AcceptedOperation, CommitError, MemoryStorage, Storage};
use crate::executor::{ExecutionToken, Executor, Handoff};
use crate::identity::OperationId;
use crate::request::{RequestFingerprint, RetryKey};

/// An in-memory [`Storage`] whose reads or updates fail on demand.
///
/// `fail_next_reads(n)` makes the next `n` calls to [`Storage::operation`] fail,
/// and `fail_next_updates(n)` does the same for [`Storage::update`]. Failed calls
/// change nothing. Every other call delegates to a [`MemoryStorage`].
#[derive(Debug, Default)]
pub struct FlakyStorage {
    inner: MemoryStorage,
    failing_reads: AtomicU32,
    failing_updates: AtomicU32,
}

impl FlakyStorage {
    /// Creates an empty storage with no failures queued.
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes the next `count` reads of an execution record fail.
    pub fn fail_next_reads(&self, count: u32) {
        self.failing_reads.store(count, Ordering::SeqCst);
    }

    /// Makes the next `count` updates of an execution record fail.
    pub fn fail_next_updates(&self, count: u32) {
        self.failing_updates.store(count, Ordering::SeqCst);
    }
}

/// Consumes one queued failure; `true` when the call must fail.
fn take_failure(counter: &AtomicU32) -> bool {
    counter
        .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
        .is_ok()
}

impl Storage for FlakyStorage {
    fn accept(
        &self,
        key: RetryKey,
        fingerprint: RequestFingerprint,
        candidate: AcceptedOperation,
    ) -> Result<Acceptance, CommitError> {
        self.inner.accept(key, fingerprint, candidate)
    }

    fn operation(&self, id: &OperationId) -> Result<Option<AcceptedOperation>, CommitError> {
        if take_failure(&self.failing_reads) {
            return Err(CommitError("injected read failure".to_string()));
        }
        self.inner.operation(id)
    }

    fn operation_for_token(
        &self,
        token: &ExecutionToken,
    ) -> Result<Option<OperationId>, CommitError> {
        self.inner.operation_for_token(token)
    }

    fn update(
        &self,
        id: &OperationId,
        change: &mut dyn FnMut(&mut AcceptedOperation),
    ) -> Result<bool, CommitError> {
        if take_failure(&self.failing_updates) {
            return Err(CommitError("injected update failure".to_string()));
        }
        self.inner.update(id, change)
    }
}

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
