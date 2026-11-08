//! Wire frames of the streaming Observe query.

use operations::{End, ObserveResponse, Report};
use serde::{Deserialize, Serialize};

/// Why a stream ended, as sent on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndReason {
    /// The execution is terminal; no later snapshot will be produced.
    Terminal,
    /// The observer fell behind at Core; resume with the last `sequence`.
    Lagged,
    /// The server-side stream lease expired; resume with the last `sequence`.
    Lease,
    /// The report is no longer retained.
    Unavailable,
    /// Core ended the stream without a stated reason; resume.
    Closed,
}

impl From<End> for EndReason {
    fn from(end: End) -> Self {
        match end {
            End::Terminal => Self::Terminal,
            End::Lagged => Self::Lagged,
            End::Unavailable => Self::Unavailable,
            End::Closed => Self::Closed,
        }
    }
}

/// One reply of an Observe query. The first is always `response`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "frame", rename_all = "snake_case")]
pub enum ObserveFrame {
    /// The typed attach outcome. Only `attached` is followed by more frames.
    Response {
        /// The outcome of attaching.
        response: ObserveResponse,
    },
    /// A snapshot newer than the one in the attach outcome.
    Snapshot {
        /// The full snapshot.
        report: Box<Report>,
    },
    /// The stream is over.
    End {
        /// Why.
        reason: EndReason,
    },
}
