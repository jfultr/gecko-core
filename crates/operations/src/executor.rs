//! Executor contract skeleton.
//!
//! Handoff outcomes follow the accepted first-experiment rules for `pending`
//! and `unknown` (model section 3.1, packages P2 and P5).
//!
//! The exact event schema, error taxonomy, identifier format (UUIDv4, a later
//! task), and sync vs async shape are NOT decided yet. This module only fixes
//! the shape needed to drive the test harness and is provisional.

/// Opaque identity of one execution, assigned by Core on acceptance.
///
/// Core generates tokens as random UUIDv4 text; the executor treats it as opaque.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutionToken(String);

impl ExecutionToken {
    /// Creates a token from its textual value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Generates a new random token (UUIDv4 text).
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Returns the textual value of the token.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Outcome of handing an execution to a domain executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handoff {
    /// The executor confirmed this execution token; the execution becomes `running`.
    Acknowledged,
    /// The executor refused before any side effect (a proven failure before side effects).
    ///
    /// `reason` is a human-readable explanation of the refusal.
    Refused {
        /// Human-readable explanation of why the executor refused the handoff.
        reason: String,
    },
    /// The handoff may have reached the executor but no acknowledgement arrived.
    ///
    /// The execution becomes `unknown` and Core must not redispatch it automatically.
    Ambiguous,
}

/// Contract a domain executor implements so Core can hand work to it.
pub trait Executor {
    /// Hands the execution identified by `token` to the executor and reports the outcome.
    fn hand_off(&mut self, token: &ExecutionToken) -> Handoff;
}
