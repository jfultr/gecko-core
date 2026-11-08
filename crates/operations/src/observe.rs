//! Get and Observe at Core (model sections 3.1, 4, and 6; decisions Z1 and Z5).
//!
//! **Get** returns the latest available full snapshot of an execution the
//! caller owns. Reading never starts, resumes, or cancels anything.
//!
//! **Observe** attaches an observer and reads the latest snapshot in one step
//! under the same lock that publishes every new snapshot
//! ([`Acceptor`](crate::acceptance::Acceptor) holds it across the storage
//! update and the publication). A snapshot is therefore either the observer's
//! `current` or arrives on its stream, never neither, so a final publication
//! cannot fall between "read" and "subscribe".
//!
//! Resume: the observer passes the last `sequence` it holds. Core does not
//! redeliver a snapshot the observer already has (`current` is `None` when
//! `after_sequence` equals the latest) and discloses a [`Gap`] when
//! intermediate snapshots were skipped. The latest available snapshot is still
//! delivered, so a gap never hides current state. Full history is not kept.
//!
//! Slow observers: each observer has a bounded buffer and Core never waits for
//! one. A full buffer ends that stream with [`End::Lagged`]; the observer
//! resumes with its last `sequence`. Dropping an [`Observation`] only removes
//! the observer; Core removes it at the next attach or publication of that
//! execution. A terminal snapshot ends its streams with [`End::Terminal`],
//! and attaching to a terminal execution yields no stream.
//!
//! Visibility (Z5): an execution the caller does not own, a nonexistent one,
//! and one whose report is no longer retained all give the same
//! [`Unavailable`].
//!
//! Limit: one lock serializes every publication and attach, for all
//! executions. That is fine for the in-memory first experiment; it would not
//! survive a slow storage or many observers.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock, PoisonError, mpsc};
use std::time::Duration;

use crate::acceptance::{AcceptedOperation, Acceptor, CommitError, Storage};
use crate::api::{
    Gap, GetRequest, GetResponse, InputError, ObserveRequest, ObserveResponse, ServiceError,
    Unavailable, Violation,
};
use crate::auth::{Authorizer, Command, DecisionLog};
use crate::identity::{CallerContext, OperationId};
use crate::report::Report;

/// Snapshots an observer may leave unread before its stream ends with [`End::Lagged`].
pub const DEFAULT_OBSERVER_CAPACITY: usize = 64;

/// Why an observation stream ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The terminal snapshot was the last one delivered.
    Terminal,
    /// The observer did not keep up; resume with the last `sequence` held.
    Lagged,
    /// The report is no longer retained.
    Unavailable,
    /// Core went away without saying why. Treat as "no answer" and resume.
    Closed,
}

/// Result of polling an [`Observation`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Poll {
    /// The next snapshot, in `sequence` order.
    Snapshot(Box<Report>),
    /// Nothing new yet.
    Empty,
    /// The stream is over; every snapshot sent before the end has been read.
    Ended(End),
}

/// Live stream of snapshots newer than the one given at attach time.
#[derive(Debug)]
pub struct Observation {
    receiver: Receiver<Report>,
    end: Arc<OnceLock<End>>,
    closed: Arc<AtomicBool>,
}

impl Drop for Observation {
    fn drop(&mut self) {
        // Marks the watcher dead so that the next attach or publication of its
        // execution removes it. A `SyncSender` has no cheap liveness check.
        self.closed.store(true, Ordering::SeqCst);
    }
}

impl Observation {
    fn ended(&self) -> Poll {
        Poll::Ended(self.end.get().copied().unwrap_or(End::Closed))
    }

    /// Returns the next snapshot without waiting.
    pub fn try_next(&self) -> Poll {
        match self.receiver.try_recv() {
            Ok(report) => Poll::Snapshot(Box::new(report)),
            Err(TryRecvError::Empty) => Poll::Empty,
            Err(TryRecvError::Disconnected) => self.ended(),
        }
    }

    /// Waits up to `timeout` for the next snapshot.
    pub fn next_timeout(&self, timeout: Duration) -> Poll {
        match self.receiver.recv_timeout(timeout) {
            Ok(report) => Poll::Snapshot(Box::new(report)),
            Err(RecvTimeoutError::Timeout) => Poll::Empty,
            Err(RecvTimeoutError::Disconnected) => self.ended(),
        }
    }
}

#[derive(Debug)]
struct Watcher {
    sender: SyncSender<Report>,
    end: Arc<OnceLock<End>>,
    closed: Arc<AtomicBool>,
}

/// Registered observers by execution. Only reachable through the hub lock.
#[derive(Debug, Default)]
pub(crate) struct Watchers(HashMap<OperationId, Vec<Watcher>>);

impl Watchers {
    /// Removes the observers of `id` whose [`Observation`] was dropped, and the
    /// key when none is left.
    fn prune_closed(&mut self, id: &OperationId) {
        if let Some(list) = self.0.get_mut(id) {
            list.retain(|w| !w.closed.load(Ordering::SeqCst));
            if list.is_empty() {
                self.0.remove(id);
            }
        }
    }

    fn attach(&mut self, id: OperationId, capacity: usize) -> Observation {
        self.prune_closed(&id);
        let (sender, receiver) = mpsc::sync_channel(capacity.max(1));
        let end = Arc::new(OnceLock::new());
        let closed = Arc::new(AtomicBool::new(false));
        self.0.entry(id).or_default().push(Watcher {
            sender,
            end: Arc::clone(&end),
            closed: Arc::clone(&closed),
        });
        Observation {
            receiver,
            end,
            closed,
        }
    }

    /// Number of registered observers of `id`, including dropped ones not yet pruned.
    #[cfg(feature = "test-support")]
    fn count(&self, id: &OperationId) -> usize {
        self.0.get(id).map_or(0, Vec::len)
    }

    /// Sends `snapshot` to every observer of `id` without waiting for any.
    pub(crate) fn publish(&mut self, id: &OperationId, snapshot: &Report) {
        let Some(list) = self.0.get_mut(id) else {
            return;
        };
        list.retain(|w| match w.sender.try_send(snapshot.clone()) {
            Ok(()) => true,
            Err(TrySendError::Full(_)) => {
                let _ = w.end.set(End::Lagged);
                false
            }
            Err(TrySendError::Disconnected(_)) => false,
        });
        if snapshot.state.is_terminal() {
            self.end_all(id, End::Terminal);
        } else if list.is_empty() {
            self.0.remove(id);
        }
    }

    fn end_all(&mut self, id: &OperationId, reason: End) {
        for watcher in self.0.remove(id).unwrap_or_default() {
            let _ = watcher.end.set(reason);
        }
    }
}

/// The lock that orders publication and attachment.
#[derive(Debug, Default)]
pub(crate) struct Hub(Mutex<Watchers>);

impl Hub {
    pub(crate) fn lock(
        &self,
    ) -> Result<MutexGuard<'_, Watchers>, PoisonError<MutexGuard<'_, Watchers>>> {
        self.0.lock()
    }
}

/// Outcome of [`Acceptor::observe`].
#[derive(Debug)]
pub struct ObserveOutcome {
    /// Typed response for the caller.
    pub response: ObserveResponse,
    /// Present only when attached to an execution that is not yet terminal.
    pub stream: Option<Observation>,
}

impl ObserveOutcome {
    fn respond(response: ObserveResponse) -> Self {
        Self {
            response,
            stream: None,
        }
    }

    fn error(message: &str) -> Self {
        Self::respond(ObserveResponse::Error(ServiceError {
            message: message.to_string(),
        }))
    }
}

impl<A, L, S> Acceptor<A, L, S>
where
    A: Authorizer + Send,
    L: DecisionLog + Send,
    S: Storage,
{
    /// Sets how many snapshots an observer may leave unread (at least 1).
    #[must_use]
    pub fn with_observer_capacity(mut self, capacity: usize) -> Self {
        self.observer_capacity = capacity.max(1);
        self
    }

    /// Number of observers registered for execution `id`, including dropped ones
    /// that no attach or publication has pruned yet.
    ///
    /// # Panics
    ///
    /// Panics if the observer lock is poisoned.
    #[cfg(feature = "test-support")]
    pub fn observer_count(&self, id: &OperationId) -> usize {
        self.hub.lock().expect("observer lock").count(id)
    }

    /// The execution, if `caller` may see it and its report is retained.
    fn visible(
        &self,
        caller: &CallerContext,
        id: &OperationId,
    ) -> Result<Option<AcceptedOperation>, CommitError> {
        Ok(self
            .storage()
            .operation(id)?
            .filter(|op| op.owner == *caller.subject() && op.report_retained))
    }

    /// Returns the latest available full snapshot of an execution `caller` owns.
    ///
    /// Authorization is recorded first. Unowned, nonexistent, and expired
    /// reports are indistinguishable (Z5). Reading changes nothing.
    pub fn get(&self, caller: &CallerContext, request: &GetRequest) -> GetResponse {
        let command = Command::Get {
            operation_id: request.operation_id,
        };
        match self.guard.lock() {
            Ok(mut guard) => {
                if let Err(refusal) = guard.check(caller, &command) {
                    return refusal.get_response();
                }
            }
            Err(_) => {
                return GetResponse::Error(ServiceError {
                    message: "authorization is unavailable".to_string(),
                });
            }
        }
        match self.visible(caller, &request.operation_id) {
            Ok(Some(op)) => GetResponse::Report(Box::new(op.snapshot)),
            Ok(None) => GetResponse::Unavailable(Unavailable {}),
            Err(_) => GetResponse::Error(ServiceError {
                message: "the report could not be read".to_string(),
            }),
        }
    }

    /// Attaches an observer to an execution `caller` owns.
    ///
    /// Reads the latest snapshot and registers the stream atomically with
    /// respect to publication. See the [module documentation](self).
    pub fn observe(&self, caller: &CallerContext, request: &ObserveRequest) -> ObserveOutcome {
        let command = Command::Observe {
            operation_id: request.operation_id,
        };
        match self.guard.lock() {
            Ok(mut guard) => {
                if let Err(refusal) = guard.check(caller, &command) {
                    return ObserveOutcome::respond(refusal.observe_response());
                }
            }
            Err(_) => return ObserveOutcome::error("authorization is unavailable"),
        }

        let Ok(mut watchers) = self.hub.lock() else {
            return ObserveOutcome::error("observation is unavailable");
        };
        let op = match self.visible(caller, &request.operation_id) {
            Ok(Some(op)) => op,
            Ok(None) => {
                return ObserveOutcome::respond(ObserveResponse::Unavailable(Unavailable {}));
            }
            Err(_) => return ObserveOutcome::error("the report could not be read"),
        };

        let latest = op.snapshot.sequence;
        // `Some(0)` says "I hold nothing", like no cursor.
        let after = request.after_sequence.filter(|after| *after > 0);
        let (current, gap) = match after {
            None => (Some(Box::new(op.snapshot.clone())), None),
            Some(after) if after > latest => {
                return ObserveOutcome::respond(ObserveResponse::InvalidInput(InputError {
                    message: "after_sequence is newer than any snapshot of this operation"
                        .to_string(),
                    violations: vec![Violation::new(
                        "after_sequence",
                        format!("the latest snapshot is {latest}"),
                    )],
                }));
            }
            Some(after) if after == latest => (None, None),
            Some(after) => (
                Some(Box::new(op.snapshot.clone())),
                (latest > after + 1).then_some(Gap {
                    first_missed: after + 1,
                    last_missed: latest - 1,
                }),
            ),
        };
        let stream = (!op.snapshot.state.is_terminal())
            .then(|| watchers.attach(request.operation_id, self.observer_capacity));
        ObserveOutcome {
            response: ObserveResponse::Attached {
                current,
                sequence: latest,
                gap,
            },
            stream,
        }
    }

    /// Ends the retention of an execution's report and its streams.
    ///
    /// The request mapping, state, and `sequence` counter stay, so a retry or
    /// later event still behaves as before; only reading stops. Returns whether
    /// the execution exists. This is the `0.1.0` stand-in for the retention
    /// policy, which is a separate decision.
    ///
    /// # Errors
    ///
    /// [`CommitError`] if storage or the observer lock fails; nothing changed.
    pub fn release_report(&self, id: &OperationId) -> Result<bool, CommitError> {
        let mut watchers = self
            .hub
            .lock()
            .map_err(|_| CommitError("observer lock poisoned".to_string()))?;
        let found = self
            .storage()
            .update(id, &mut |op| op.report_retained = false)?;
        watchers.end_all(id, End::Unavailable);
        Ok(found)
    }
}

/// What applying a received snapshot did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied {
    /// A newer snapshot replaced the held one. `missed` names skipped
    /// intermediate snapshots, if any.
    Replaced {
        /// Snapshots between the held one and the new one.
        missed: Option<Gap>,
    },
    /// The same snapshot was already held.
    Duplicate,
    /// A lower `sequence` than the held one; ignored.
    Stale,
    /// The same `sequence` with different content; not a valid update, ignored.
    Conflict,
    /// A snapshot of another operation; ignored.
    WrongOperation,
}

/// Observer-side rule: keep the greatest `sequence`, never let an equal or
/// lower one replace newer state, and notice skipped snapshots.
#[derive(Debug, Clone, Default)]
pub struct Follower {
    latest: Option<Report>,
}

impl Follower {
    /// Creates a follower that holds nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// The newest valid snapshot received.
    pub fn latest(&self) -> Option<&Report> {
        self.latest.as_ref()
    }

    /// `sequence` of the newest snapshot, for the next `after_sequence`.
    pub fn sequence(&self) -> Option<u64> {
        self.latest.as_ref().map(|r| r.sequence)
    }

    /// Considers a received snapshot.
    pub fn apply(&mut self, report: Report) -> Applied {
        let Some(held) = &self.latest else {
            self.latest = Some(report);
            return Applied::Replaced { missed: None };
        };
        if report.operation.id != held.operation.id {
            return Applied::WrongOperation;
        }
        match report.sequence.cmp(&held.sequence) {
            std::cmp::Ordering::Less => Applied::Stale,
            std::cmp::Ordering::Equal if report == *held => Applied::Duplicate,
            std::cmp::Ordering::Equal => Applied::Conflict,
            std::cmp::Ordering::Greater => {
                let missed = (report.sequence > held.sequence + 1).then_some(Gap {
                    first_missed: held.sequence + 1,
                    last_missed: report.sequence - 1,
                });
                self.latest = Some(report);
                Applied::Replaced { missed }
            }
        }
    }
}
