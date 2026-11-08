//! Executor events and Core arbitration (model sections 6 and 7, packages P2
//! and P5, decision Z6).
//!
//! The domain executor reports facts; Core validates them against the common
//! lifecycle and numbers the resulting full snapshots. Each event is identified
//! by (`token`, `event_seq`), where the executor assigns `event_seq`
//! contiguously from 1 per token. It is separate from the Core-assigned
//! snapshot `sequence`.
//!
//! Arbitration is a pure function over one [`AcceptedOperation`]; storage
//! applies it under its per-record lock, so concurrent notifications for one
//! execution are decided one at a time. Arrival order alone never decides an
//! outcome: a terminal state is immutable, and a different terminal outcome
//! after a recorded one is rejected however it arrives.
//!
//! Rules (no rejection creates a snapshot):
//!
//! | Situation | Result |
//! |---|---|
//! | same (`token`, `event_seq`) and same content | [`Notified::Duplicate`] with the earlier snapshot number |
//! | same `event_seq`, other content | [`NotifyRejection::Stale`] |
//! | `event_seq` skips ahead | [`NotifyRejection::OutOfOrder`]; consumes nothing, so the missing number is awaited |
//! | `Ack` while `pending` or `unknown` | `running` |
//! | `Ack` otherwise | [`Notified::NoChange`] |
//! | `Progress` while `running` or `unknown` | `running` with new progress |
//! | `Progress` while `pending` or terminal | [`NotifyRejection::InvalidTransition`], number consumed |
//! | terminal outcome while `pending`, `running`, `unknown` | that terminal state |
//! | the recorded terminal outcome again | [`Notified::NoChange`] |
//! | another terminal outcome after a terminal one | [`NotifyRejection::ConflictingTerminal`], number consumed |
//! | `cancelled` (no Cancel before `0.2.0`) | [`NotifyRejection::InvalidTransition`], number consumed |
//! | unknown token | [`NotifyRejection::UnknownToken`] |
//!
//! An event rejected by the transition rules (`InvalidTransition`,
//! `ConflictingTerminal`) consumes its `event_seq` (decided by Farhat on
//! 2026-11-08, status Z6). `last_event_seq` advances, no snapshot is created,
//! and the event and its rejection are logged. The same (`token`, `event_seq`)
//! with the same content returns the same rejection later, even if the state
//! has changed; other content on that number is `Stale`. The executor then
//! continues with the next number.
//!
//! Whether stale and conflicting notifications are protocol errors or ordinary
//! rejection outcomes is open (status §2.4, Z6); they are typed rejections here.

use std::collections::HashMap;

use crate::acceptance::AcceptedOperation;
use crate::executor::{ExecutionToken, Handoff};
use crate::report::{Progress, Report, State, Timestamp};
use crate::value::Value;

/// Terminal outcome an executor reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// The operation completed successfully.
    Succeeded,
    /// The attempt is deemed unsuccessful.
    Failed,
    /// Cessation caused by cancellation. Not accepted before `0.2.0`.
    Cancelled,
}

impl Outcome {
    fn state(self) -> State {
        match self {
            Self::Succeeded => State::Succeeded,
            Self::Failed => State::Failed,
            Self::Cancelled => State::Cancelled,
        }
    }
}

/// What an executor notification says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventKind {
    /// The executor confirms it holds this execution.
    Ack,
    /// Progress, with optional partial data that replaces the retained `result`.
    Progress {
        /// Current progress assessment.
        progress: Progress,
        /// Partial result obtained so far; `None` keeps the retained one.
        result: Option<Value>,
    },
    /// An authoritative terminal outcome.
    Terminal {
        /// The outcome.
        outcome: Outcome,
        /// Result data; `None` keeps the retained partial result.
        result: Option<Value>,
        /// Explanation of the outcome for a person or agent.
        explanation: String,
    },
}

/// One identified executor notification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutorEvent {
    /// Execution token Core issued at acceptance.
    pub token: ExecutionToken,
    /// Executor-assigned number, contiguous from 1 per token.
    pub event_seq: u64,
    /// The notification.
    pub kind: EventKind,
}

/// Accepted notification outcomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Notified {
    /// A new snapshot with this `sequence` was recorded.
    Applied {
        /// Number of the new snapshot.
        sequence: u64,
    },
    /// The event was seen before; no new snapshot. Carries the earlier snapshot number.
    Duplicate {
        /// Snapshot number current after the first application.
        sequence: u64,
    },
    /// The event was valid but changed nothing; no new snapshot.
    NoChange {
        /// Current snapshot number.
        sequence: u64,
    },
}

/// A notification Core refused. The record and its snapshots are unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NotifyRejection {
    /// No execution was issued this token.
    UnknownToken,
    /// This `event_seq` was already used with different content.
    Stale,
    /// `event_seq` skips a number; resend after the missing one.
    OutOfOrder {
        /// The `event_seq` Core expects next.
        expected: u64,
    },
    /// The event is not allowed in the recorded state.
    InvalidTransition {
        /// State the execution is in.
        from: State,
    },
    /// A different terminal outcome than the recorded one.
    ConflictingTerminal {
        /// The recorded terminal state.
        recorded: State,
    },
    /// Storage could not apply the change; nothing changed.
    Unavailable,
}

/// Executor events numbered so far for one execution, kept to recognise repeats.
///
/// Every `event_seq` up to `last_event_seq` is consumed, whether the event was
/// accepted or rejected by the transition rules. Each consumed number keeps its
/// content and its outcome: the snapshot number it reported, or the rejection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventLog {
    last_event_seq: u64,
    consumed: HashMap<u64, (EventKind, Result<u64, NotifyRejection>)>,
}

impl EventLog {
    /// Highest `event_seq` consumed so far; 0 before any.
    pub fn last_event_seq(&self) -> u64 {
        self.last_event_seq
    }
}

fn next_snapshot(op: &AcceptedOperation, now: Timestamp, state: State) -> Report {
    let mut next = op.snapshot.clone();
    next.sequence += 1;
    next.reported_at = now;
    next.state = state;
    next
}

fn stage(stage: &str, message: &str) -> Progress {
    Progress {
        stage: stage.to_string(),
        message: message.to_string(),
        ..Progress::default()
    }
}

fn record(op: &mut AcceptedOperation, snapshot: Report) -> u64 {
    let sequence = snapshot.sequence;
    op.snapshot = snapshot;
    sequence
}

/// Records `event` as consumed with its outcome.
fn consume(log: &mut EventLog, event: &ExecutorEvent, settled: Result<u64, NotifyRejection>) {
    log.last_event_seq = event.event_seq;
    log.consumed
        .insert(event.event_seq, (event.kind.clone(), settled));
}

/// Decides the snapshot a valid-numbered event produces: `None` when it changes nothing.
fn transition(
    op: &AcceptedOperation,
    kind: &EventKind,
    now: Timestamp,
) -> Result<Option<Report>, NotifyRejection> {
    let from = op.snapshot.state;
    Ok(match kind {
        EventKind::Ack if matches!(from, State::Pending | State::Unknown) => {
            let mut next = next_snapshot(op, now, State::Running);
            next.progress = Some(stage("started", "the executor confirmed the execution"));
            Some(next)
        }
        EventKind::Ack => None,
        EventKind::Progress { progress, result }
            if matches!(from, State::Running | State::Unknown) =>
        {
            let mut next = next_snapshot(op, now, State::Running);
            next.progress = Some(progress.clone());
            if let Some(result) = result {
                next.result = result.clone();
            }
            Some(next)
        }
        EventKind::Progress { .. } => return Err(NotifyRejection::InvalidTransition { from }),
        EventKind::Terminal { outcome, .. } if from.is_terminal() => {
            if outcome.state() == from {
                None
            } else {
                return Err(NotifyRejection::ConflictingTerminal { recorded: from });
            }
        }
        EventKind::Terminal {
            outcome: Outcome::Cancelled,
            ..
        } => return Err(NotifyRejection::InvalidTransition { from }),
        EventKind::Terminal {
            outcome,
            result,
            explanation,
        } => {
            let mut next = next_snapshot(op, now, outcome.state());
            if let Some(result) = result {
                next.result = result.clone();
            }
            next.explanation.clone_from(explanation);
            Some(next)
        }
    })
}

/// Validates and applies one executor notification to `op`.
///
/// An event whose number is the next one is consumed even when the transition
/// rules reject it: the rejection is stored and a resend with the same content
/// returns it again. A skipped number is `OutOfOrder` and consumes nothing.
pub(crate) fn apply_event(
    op: &mut AcceptedOperation,
    event: &ExecutorEvent,
    now: Timestamp,
) -> Result<Notified, NotifyRejection> {
    let last = op.events.last_event_seq;
    if event.event_seq <= last {
        return match op.events.consumed.get(&event.event_seq) {
            Some((kind, Ok(sequence))) if *kind == event.kind => Ok(Notified::Duplicate {
                sequence: *sequence,
            }),
            Some((kind, Err(rejection))) if *kind == event.kind => Err(rejection.clone()),
            _ => Err(NotifyRejection::Stale),
        };
    }
    if event.event_seq != last + 1 {
        return Err(NotifyRejection::OutOfOrder { expected: last + 1 });
    }

    let next = match transition(op, &event.kind, now) {
        Ok(next) => next,
        Err(rejection) => {
            consume(&mut op.events, event, Err(rejection.clone()));
            return Err(rejection);
        }
    };
    let (notified, sequence) = match next {
        Some(snapshot) => {
            let sequence = record(op, snapshot);
            (Notified::Applied { sequence }, sequence)
        }
        None => {
            let sequence = op.snapshot.sequence;
            (Notified::NoChange { sequence }, sequence)
        }
    };
    consume(&mut op.events, event, Ok(sequence));
    Ok(notified)
}

/// Records that the execution never reached its executor (O8, November 8, 2026).
///
/// Only `pending` may end this way. The explanation says that nothing reached
/// the executor, which is an infrastructure failure, not an executor refusal.
pub(crate) fn apply_dispatch_failure(
    op: &mut AcceptedOperation,
    reason: &str,
    now: Timestamp,
) -> Result<Notified, NotifyRejection> {
    let from = op.snapshot.state;
    if from != State::Pending {
        return Err(NotifyRejection::InvalidTransition { from });
    }
    let mut next = next_snapshot(op, now, State::Failed);
    next.explanation = format!(
        "the execution could not be handed to its executor; nothing reached the executor: {reason}"
    );
    Ok(Notified::Applied {
        sequence: record(op, next),
    })
}

/// Records the result of handing the execution to its executor.
///
/// Unlike an executor event this consumes no `event_seq`. `Acknowledged`
/// behaves like a confirming `Ack`; `Ambiguous` moves `pending` to `unknown`
/// and nothing redispatches; `Refused` is a proven failure before side effects
/// and is valid only while `pending`.
pub(crate) fn apply_handoff(
    op: &mut AcceptedOperation,
    handoff: &Handoff,
    now: Timestamp,
) -> Result<Notified, NotifyRejection> {
    let from = op.snapshot.state;
    let next = match (handoff, from) {
        (Handoff::Acknowledged, State::Pending | State::Unknown) => {
            let mut next = next_snapshot(op, now, State::Running);
            next.progress = Some(stage("started", "the executor confirmed the handoff"));
            Some(next)
        }
        (Handoff::Ambiguous, State::Pending) => {
            let mut next = next_snapshot(op, now, State::Unknown);
            next.progress = Some(stage(
                "handoff_unconfirmed",
                "the handoff may have reached the executor without an acknowledgement",
            ));
            next.explanation = "the handoff may have reached the executor but was not \
                                acknowledged; it is not redispatched"
                .to_string();
            Some(next)
        }
        (Handoff::Refused { reason }, State::Pending) => {
            let mut next = next_snapshot(op, now, State::Failed);
            next.explanation =
                format!("the executor refused the handoff before any side effect: {reason}");
            Some(next)
        }
        (Handoff::Refused { .. }, from) => return Err(NotifyRejection::InvalidTransition { from }),
        (Handoff::Acknowledged | Handoff::Ambiguous, _) => None,
    };
    Ok(match next {
        Some(snapshot) => Notified::Applied {
            sequence: record(op, snapshot),
        },
        None => Notified::NoChange {
            sequence: op.snapshot.sequence,
        },
    })
}
