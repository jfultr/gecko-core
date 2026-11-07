//! Transport-neutral Start, Get, and Observe inputs and typed outcomes.
//!
//! Every Operation outcome, including invalid input, is a typed value (Z3). A
//! transport error, no reply, or a closed stream is "no answer", which is not
//! modelled here and is safe to retry. An operation the caller may not see and
//! a nonexistent operation yield the same [`Unavailable`] (Z5), so neither
//! reveals the other.

use serde::{Deserialize, Serialize};

use crate::identity::OperationId;
use crate::report::{Operation, Report};

/// Observation address returned on acceptance. Its format is set by the Observe mapping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ObservationAddress(pub String);

/// Input of Get: the latest full report of one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GetRequest {
    /// Execution to read.
    pub operation_id: OperationId,
}

/// Input of Observe: subscribe to snapshots of one execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObserveRequest {
    /// Execution to observe.
    pub operation_id: OperationId,
    /// Last `sequence` the observer already holds; used to resume without duplicates.
    #[serde(default)]
    pub after_sequence: Option<u64>,
}

/// One violated rule of a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Violation {
    /// Path of the offending field, such as `parameters.count`.
    pub field: String,
    /// Explanation for a person or agent.
    pub message: String,
}

impl Violation {
    /// Creates a violation.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Machine-readable reason a Start request was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionCode {
    /// The operation type is not supported.
    UnknownOperationType,
    /// The target or parameters are invalid for the operation type.
    InvalidParameters,
    /// The same `request_id` was used with different content.
    IdentifierConflict,
    /// Authorization denied the request, or its state was unknown (fail closed).
    Denied,
}

/// A Start rejection: no execution was created.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rejection {
    /// Machine-readable reason.
    pub code: RejectionCode,
    /// Explanation for a person or agent.
    pub message: String,
    /// Violated rules, when the reason is invalid input.
    #[serde(default)]
    pub violations: Vec<Violation>,
}

impl Rejection {
    /// Creates a rejection.
    pub fn new(
        code: RejectionCode,
        message: impl Into<String>,
        violations: Vec<Violation>,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            violations,
        }
    }
}

/// The report or observation is not available to this caller.
///
/// Deliberately carries no reason: absent, never created, and not permitted look
/// the same. Absence does not prove the operation never ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Unavailable {}

/// Malformed input that could not be interpreted as a request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputError {
    /// Explanation for a person or agent.
    pub message: String,
    /// Violated rules.
    #[serde(default)]
    pub violations: Vec<Violation>,
}

/// An internal failure of the receiving Gecko; no execution state is implied.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceError {
    /// Explanation for a person or agent.
    pub message: String,
}

/// Typed outcome of Start.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StartResponse {
    /// An execution exists. Acceptance confirms neither success nor executor readiness.
    Accepted {
        /// The created, or on retry the previously created, execution.
        operation: Operation,
        /// Where to observe the execution.
        observation: ObservationAddress,
    },
    /// No execution was created.
    Rejected(Rejection),
    /// The input could not be interpreted.
    InvalidInput(InputError),
    /// Internal failure; the caller may retry with the same `request_id`.
    Error(ServiceError),
}

/// Typed outcome of Get.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GetResponse {
    /// The latest available full snapshot.
    Report(Box<Report>),
    /// No report is available to this caller.
    Unavailable(Unavailable),
    /// The input could not be interpreted.
    InvalidInput(InputError),
    /// Internal failure.
    Error(ServiceError),
}

/// Typed outcome of attaching with Observe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ObserveResponse {
    /// Attached; `current` is the snapshot at attach time. Later snapshots follow.
    Attached {
        /// Snapshot at attach time.
        current: Box<Report>,
    },
    /// Nothing can be observed by this caller.
    Unavailable(Unavailable),
    /// The input could not be interpreted.
    InvalidInput(InputError),
    /// Internal failure.
    Error(ServiceError),
}
