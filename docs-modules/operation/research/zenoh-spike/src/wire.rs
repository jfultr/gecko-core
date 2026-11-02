//! Candidate wire types for the 0.1.0 spike. JSON via serde_json; not a proposal of final schema names.
//! See REPORT.md "Mapping to the model" for how these relate to model §4–§5.

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

impl State {
    pub fn is_terminal(&self) -> bool {
        matches!(self, State::Succeeded | State::Failed | State::Cancelled)
    }
}

/// Model §4 `operation` object.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationRef {
    pub id: Uuid,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
}

/// Model §5 `progress` object.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Progress {
    pub stage: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

impl Progress {
    pub fn stage(s: &str) -> Self {
        Self { stage: s.into(), message: s.into(), ..Default::default() }
    }
}

/// Subset of the model §5 report. Snapshot identity on the wire is (`operation.id`, `sequence`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Snapshot {
    pub operation: OperationRef,
    pub sequence: u64,
    /// u64, not u128: serde_json cannot read u128 inside internally tagged enums (observed).
    pub reported_at_unix_ms: u64,
    pub state: State,
    pub progress: Progress,
    /// Model §5 `context`, reduced to the applicable target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum GetResponse {
    Report { snapshot: Snapshot },
    /// Operation is known but its report is no longer retained.
    Unavailable { operation_id: Uuid },
    /// Unknown or not visible to this caller; deliberately indistinguishable (choice 5).
    NotFound,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// The operation is terminal; no later snapshot will be produced.
    Terminal,
    /// The observer fell behind at Core; resume with `after=<last sequence>`.
    Lagged,
    /// The server-side stream lease expired; resume with `after=<last sequence>`.
    Lease,
}

/// Streaming-query Observe frame.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ObserveFrame {
    Snapshot { snapshot: Snapshot, gap_before: bool },
    End { reason: EndReason },
    Unavailable { operation_id: Uuid },
    NotFound,
    Rejected { code: RejectCode, message: String },
}

// ------------------------------------------------------------------ executor notifications (P5)

/// What the executor reports. `Ack` confirms handoff of the execution identified by the token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EventKind {
    Ack,
    Progress { progress: Progress },
    Terminal { state: State, progress: Progress },
}

/// Identified executor notification: (`token`, `event_seq`) is the event identity. `event_seq` is
/// assigned by the executor, starts at 1 per token and is contiguous. It is distinct from the
/// Core-assigned snapshot `sequence`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExecutorEvent {
    pub token: Uuid,
    pub event_seq: u64,
    pub event: EventKind,
}

/// Core's answer to an executor notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotifyOutcome {
    /// A new snapshot with this `sequence` was recorded.
    Applied { sequence: u64 },
    /// Same identity and content as an already accepted event; no new snapshot.
    Duplicate { sequence: u64 },
    /// Accepted event that changes nothing (e.g. repeated ack); no new snapshot.
    NoChange { sequence: u64 },
    /// Protocol errors: no snapshot, state unchanged.
    UnknownToken,
    /// Same `event_seq` as an accepted event but different content.
    Stale,
    /// `event_seq` skips ahead of the next expected value.
    OutOfOrder { expected: u64 },
    /// A terminal notification disagrees with the recorded terminal outcome.
    ConflictingTerminal { recorded: State },
    /// Transition forbidden by model §7 (e.g. progress after terminal, progress before ack).
    InvalidTransition { from: State },
}

// ------------------------------------------------------------------ normalization (P4)

/// Canonical form used for retry comparison. Object keys are sorted explicitly here, so the result
/// does not depend on serde_json's `preserve_order` feature (which Cargo feature unification can
/// enable from any crate in a workspace). Arrays keep order. Numbers keep serde_json's textual
/// form, so `1` and `1.0` differ (choice 4). No insignificant whitespace.
pub fn normalized_content(r: &StartRequest) -> String {
    let mut out = String::new();
    canonical_json(
        &serde_json::json!({ "type": r.operation_type, "target": r.target, "params": r.params }),
        &mut out,
    );
    out
}

pub fn canonical_json(v: &Value, out: &mut String) {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                canonical_json(&m[k], out);
            }
            out.push('}');
        }
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                canonical_json(x, out);
            }
            out.push(']');
        }
        scalar => out.push_str(&scalar.to_string()),
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon(s: &str) -> String {
        let mut o = String::new();
        canonical_json(&serde_json::from_str(s).unwrap(), &mut o);
        o
    }

    #[test]
    fn key_order_is_insignificant_at_every_depth() {
        assert_eq!(canon(r#"{"b":1,"a":{"y":[{"q":1,"p":2}],"x":3}}"#), canon(r#"{"a":{"x":3,"y":[{"p":2,"q":1}]},"b":1}"#));
        assert_eq!(canon(r#"{"b":1,"a":2}"#), r#"{"a":2,"b":1}"#);
    }

    #[test]
    fn keys_are_sorted_even_if_map_preserves_insertion_order() {
        // Build an object whose iteration order is reverse-sorted regardless of the map backend.
        let mut m = serde_json::Map::new();
        m.insert("z".into(), Value::from(1));
        m.insert("a".into(), Value::from(2));
        let mut o = String::new();
        canonical_json(&Value::Object(m), &mut o);
        assert_eq!(o, r#"{"a":2,"z":1}"#);
    }

    #[test]
    fn array_order_and_number_spelling_are_significant() {
        assert_ne!(canon("[1,2]"), canon("[2,1]"));
        assert_ne!(canon(r#"{"a":1}"#), canon(r#"{"a":1.0}"#));
    }

    #[test]
    fn strings_are_escaped() {
        assert_eq!(canon(r#"{"k\"":"v\n"}"#), r#"{"k\"":"v\n"}"#);
    }

    #[test]
    fn trace_is_excluded() {
        let mut a = StartRequest {
            request_id: Uuid::nil(),
            operation_type: "t".into(),
            target: None,
            params: Value::Null,
            trace: Some("x".into()),
        };
        let n = normalized_content(&a);
        a.trace = Some("y".into());
        assert_eq!(n, normalized_content(&a));
    }
}
