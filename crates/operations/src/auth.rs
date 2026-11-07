//! Authorization hook and decision records (model section 4, security.md).
//!
//! Core calls [`Guard::check`] for every public command. The guard asks the
//! [`Authorizer`], appends a [`DecisionRecord`] to the [`DecisionLog`], and only
//! then returns an [`Authorized`] proof. Handoff to an executor requires that
//! proof ([`hand_off`]), so work cannot reach an executor before its decision is
//! recorded. The guard fails closed: a denial, an unknown decision, or a log
//! failure all yield no proof.
//!
//! The default policy is [`DenyAll`]. The permissive policy exists only behind
//! the `test-support` feature (see `testing::PermitAllForTests`) and makes no
//! production security claim. In-memory records are not persisted audit.

use crate::executor::{ExecutionToken, Executor, Handoff};
use crate::identity::{CallerContext, OperationId, RequestId, Subject};
use crate::request::{OperationType, Target};

/// A public command presented for authorization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Start an operation of `operation_type` on `target`.
    Start {
        /// Request identifier, linking the decision to the request.
        request_id: RequestId,
        /// Operation kind.
        operation_type: OperationType,
        /// Applicable target.
        target: Option<Target>,
    },
    /// Read the latest report of an execution.
    Get {
        /// Execution to read.
        operation_id: OperationId,
    },
    /// Observe snapshots of an execution.
    Observe {
        /// Execution to observe.
        operation_id: OperationId,
    },
}

/// The authorizer's answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// The command is permitted.
    Permit,
    /// The command is denied, with an explanation for the record.
    Deny {
        /// Why the command was denied.
        reason: String,
    },
    /// Authorization state is unknown. Treated as a denial (fail closed).
    Unknown {
        /// Why the state is unknown.
        reason: String,
    },
}

/// Policy decision point.
pub trait Authorizer {
    /// Decides whether `caller` may run `command`.
    fn authorize(&self, caller: &CallerContext, command: &Command) -> Decision;
}

/// The default policy: denies everything.
#[derive(Debug, Clone, Copy, Default)]
pub struct DenyAll;

impl Authorizer for DenyAll {
    fn authorize(&self, _caller: &CallerContext, _command: &Command) -> Decision {
        Decision::Deny {
            reason: "no authorization policy is configured".to_string(),
        }
    }
}

/// A preserved authorization decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecord {
    /// Position in the log, starting at 0.
    pub index: u64,
    /// Subject the decision concerns.
    pub subject: Subject,
    /// Adapter that supplied the caller context.
    pub adapter: String,
    /// The command decided.
    pub command: Command,
    /// The decision.
    pub decision: Decision,
}

/// Error from a [`DecisionLog`] that could not preserve a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogError(pub String);

/// Append-only store of decision records.
pub trait DecisionLog {
    /// Preserves `record`. On error the record must not be considered preserved.
    fn append(&mut self, record: DecisionRecord) -> Result<(), LogError>;
}

/// In-memory decision log. Not durable and not tamper-resistant.
#[derive(Debug, Default)]
pub struct MemoryDecisionLog {
    records: Vec<DecisionRecord>,
}

impl MemoryDecisionLog {
    /// Creates an empty log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the preserved records in order.
    pub fn records(&self) -> &[DecisionRecord] {
        &self.records
    }
}

impl DecisionLog for MemoryDecisionLog {
    fn append(&mut self, record: DecisionRecord) -> Result<(), LogError> {
        self.records.push(record);
        Ok(())
    }
}

/// Why a command was not authorized.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The authorizer denied the command.
    Denied {
        /// Reason from the authorizer.
        reason: String,
    },
    /// The authorizer could not decide; refused as a denial.
    Unknown {
        /// Reason from the authorizer.
        reason: String,
    },
    /// The decision could not be preserved; refused so no work starts unrecorded.
    NotRecorded(LogError),
}

/// Proof that a command was permitted and the decision was preserved.
///
/// Only [`Guard::check`] creates one.
#[derive(Debug)]
pub struct Authorized {
    index: u64,
    subject: Subject,
}

impl Authorized {
    /// Index of the preserved decision record.
    pub fn record_index(&self) -> u64 {
        self.index
    }

    /// Subject the decision concerns.
    pub fn subject(&self) -> &Subject {
        &self.subject
    }
}

/// Invokes authorization and preserves its decision.
pub struct Guard<A, L> {
    authorizer: A,
    log: L,
    next_index: u64,
}

impl<A: Authorizer, L: DecisionLog> Guard<A, L> {
    /// Creates a guard over a policy and a decision log.
    pub fn new(authorizer: A, log: L) -> Self {
        Self {
            authorizer,
            log,
            next_index: 0,
        }
    }

    /// Returns the decision log.
    pub fn log(&self) -> &L {
        &self.log
    }

    /// Authorizes `command` for `caller` and preserves the decision.
    ///
    /// Every decision, including denials, is recorded. The proof is returned
    /// only for a permitted command whose record was preserved.
    pub fn check(
        &mut self,
        caller: &CallerContext,
        command: &Command,
    ) -> Result<Authorized, Refusal> {
        let decision = self.authorizer.authorize(caller, command);
        let record = DecisionRecord {
            index: self.next_index,
            subject: caller.subject().clone(),
            adapter: caller.adapter().to_string(),
            command: command.clone(),
            decision: decision.clone(),
        };
        let index = record.index;
        self.log.append(record).map_err(Refusal::NotRecorded)?;
        self.next_index += 1;
        match decision {
            Decision::Permit => Ok(Authorized {
                index,
                subject: caller.subject().clone(),
            }),
            Decision::Deny { reason } => Err(Refusal::Denied { reason }),
            Decision::Unknown { reason } => Err(Refusal::Unknown { reason }),
        }
    }
}

/// Hands an execution to `executor`; requires the proof of a recorded permit.
pub fn hand_off(
    _proof: &Authorized,
    executor: &mut dyn Executor,
    token: &ExecutionToken,
) -> Handoff {
    executor.hand_off(token)
}
