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
