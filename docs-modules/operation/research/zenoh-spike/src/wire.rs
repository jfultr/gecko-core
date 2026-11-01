//! Candidate wire types for the 0.1.0 spike. JSON via serde_json; not a proposal of final schema names.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StartRequest {
    pub request_id: Uuid,
    pub operation_type: String,
    pub target: Option<String>,
    pub params: Value,
    /// Tracing metadata: excluded from retry comparison.
    #[serde(default)]
    pub trace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StartResponse {
    Accepted { operation_id: Uuid, report_key: String, observe_key: String, retried: bool },
    Rejected { code: RejectCode, message: String },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RejectCode {
    InvalidInput,
    Unauthorized,
    UnknownOperationType,
    RequestIdConflict,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Pending,
    Running,
    Succeeded,
    Failed,
    Unknown,
}

impl State {
    pub fn is_terminal(&self) -> bool {
        matches!(self, State::Succeeded | State::Failed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub operation_id: Uuid,
    pub operation_type: String,
    pub sequence: u64,
    /// u64, not u128: serde_json cannot read u128 inside internally tagged enums (observed).
    pub reported_at_unix_ms: u64,
    pub state: State,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GetResponse {
    Report { snapshot: Snapshot },
    /// Operation is known but no report is retained.
    Unavailable { operation_id: Uuid },
    /// Unknown or not visible to this caller; deliberately indistinguishable.
    NotFound,
}

/// Streaming-query Observe frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ObserveFrame {
    Snapshot { snapshot: Snapshot, gap_before: bool },
    /// Server closes the stream (terminal reached or lease ended); client re-issues with its cursor.
    End { reason: String },
    NotFound,
}

/// Canonical form used for retry comparison: sorted object keys (serde_json default BTreeMap),
/// no insignificant whitespace. Number spelling (1 vs 1.0) is preserved and therefore significant.
pub fn normalized_content(r: &StartRequest) -> String {
    let v = serde_json::json!({
        "type": r.operation_type,
        "target": r.target,
        "params": r.params,
    });
    serde_json::to_string(&v).unwrap()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
}
