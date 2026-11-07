//! Full Operation Report types (model section 5).
//!
//! A report describes one execution. `state` is the execution state; blockers,
//! limitations, and partial results are independent fields. No constructor or
//! validation couples them: a `succeeded` report may carry limitations, and a
//! `failed` report may carry a partial `result`.

use serde::{Deserialize, Serialize};

use crate::identity::OperationId;
use crate::request::OperationType;
use crate::value::Value;

/// Core-side snapshot generation time, in milliseconds since the Unix epoch.
///
/// Used for diagnostics; `sequence` orders snapshots.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(pub u64);

/// Identity of an operation execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    /// Identifier of this execution.
    pub id: OperationId,
    /// Domain operation kind.
    #[serde(rename = "type")]
    pub operation_type: OperationType,
    /// Parent operation of a composite operation, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<OperationId>,
}

/// Execution state. It does not replace readiness or decision application results.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    /// Accepted and visible, not yet confirmed by the executor.
    Pending,
    /// Confirmed by the executor.
    Running,
    /// Authoritative successful outcome.
    Succeeded,
    /// Authoritative failed outcome.
    Failed,
    /// Authoritative cancelled outcome.
    Cancelled,
    /// Reliable knowledge of the execution was lost.
    Unknown,
}

impl State {
    /// Returns whether the state is terminal (`succeeded`, `failed`, `cancelled`).
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Succeeded | Self::Failed | Self::Cancelled)
    }
}

/// Progress assessment exposed by the executor. All numeric parts are optional.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    /// Machine-readable current stage name.
    pub stage: String,
    /// Work description for a person or agent.
    pub message: String,
    /// Completed amount in `unit`, if measurable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    /// Total amount in `unit`, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    /// Unit of `completed` and `total`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// A proposed action. It is neither an accepted decision nor a performed action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedAction {
    /// Machine-readable action name.
    pub code: String,
    /// What is proposed and why.
    pub message: String,
    /// Participant or resource the action concerns.
    pub subject: String,
    /// Detailed instructions and conditions for a person or agent.
    pub instructions: String,
    /// Prepared API request description, when the action is available through Gecko.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request: Option<Value>,
}

/// A blocking problem with possible remedies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Blocker {
    /// Machine-readable reason code.
    pub code: String,
    /// Device, participant, or dependency the problem concerns.
    pub subject: String,
    /// Obstacle explanation for a person or agent.
    pub message: String,
    /// Supporting data or related-report references.
    #[serde(default)]
    pub evidence: Vec<Value>,
    /// Possible remedies and their conditions.
    #[serde(default)]
    pub next_actions: Vec<ProposedAction>,
}

/// Link to a nested operation of a composite operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChildRef {
    /// Identifier of the nested execution.
    pub id: OperationId,
    /// Operation kind of the nested execution.
    #[serde(rename = "type")]
    pub operation_type: OperationType,
}

/// A full snapshot of one operation execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    /// Execution identity.
    pub operation: Operation,
    /// Increasing snapshot number within the operation.
    pub sequence: u64,
    /// Core-side snapshot generation time.
    pub reported_at: Timestamp,
    /// Participant and operation subject.
    pub subject: Value,
    /// Inputs, parameters, dependencies, and App association.
    pub context: Value,
    /// Execution state.
    pub state: State,
    /// Progress, if the executor exposes any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<Progress>,
    /// Data actually obtained and actions performed, including partial results.
    pub result: Value,
    /// What preparation acquired, created, or reserved.
    #[serde(default)]
    pub acquired_resources: Vec<Value>,
    /// Obstacles and possible remedies, independent of `state`.
    #[serde(default)]
    pub blockers: Vec<Blocker>,
    /// Known limitations of the result.
    #[serde(default)]
    pub limitations: Vec<String>,
    /// Explanation of work, outcome reasons, and context.
    pub explanation: String,
    /// Recommendations, separate from performed actions.
    #[serde(default)]
    pub next_actions: Vec<ProposedAction>,
    /// Supporting measurements, results, and artifacts.
    #[serde(default)]
    pub evidence: Vec<Value>,
    /// Nested operations of a composite operation.
    #[serde(default)]
    pub children: Vec<ChildRef>,
    /// Domain information specific to the operation or layer.
    pub details: Value,
}

impl Report {
    /// Creates a report with empty content for the given identity, snapshot number, and state.
    pub fn new(operation: Operation, sequence: u64, reported_at: Timestamp, state: State) -> Self {
        Self {
            operation,
            sequence,
            reported_at,
            subject: Value::Null,
            context: Value::Null,
            state,
            progress: None,
            result: Value::Null,
            acquired_resources: Vec::new(),
            blockers: Vec::new(),
            limitations: Vec::new(),
            explanation: String::new(),
            next_actions: Vec::new(),
            evidence: Vec::new(),
            children: Vec::new(),
            details: Value::Null,
        }
    }
}
