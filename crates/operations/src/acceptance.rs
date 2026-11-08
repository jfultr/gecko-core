//! Atomic acceptance and safe retry of Start (model sections 3.1 P4/P6 and 4).
//!
//! [`Acceptor::start`] runs one Start request in a fixed order: authorize and
//! record the decision, validate, then commit. The commit stores the retry
//! mapping, the execution record, and the initial `pending` snapshot together
//! ([`Storage::accept`]); either all three exist or none does. Work reaches an
//! executor only through the [`Dispatch`] returned for a newly created
//! execution, so a replayed or conflicting request never dispatches.
//!
//! Concurrency: the retry key is the unit of serialization. [`MemoryStorage`]
//! holds its lock only for the check-and-insert of one key, never during
//! authorization or executor handoff. The authorization guard is a short
//! critical section because the decision log is ordered.
//!
//! Durability: everything here is in memory. A restart loses all mappings,
//! records, and decisions (`0.1.0`; durable accounting is `0.3.0` work).

use std::collections::HashMap;
use std::sync::Mutex;
#[cfg(feature = "test-support")]
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::{ObservationAddress, Rejection, RejectionCode, ServiceError, StartResponse};
use crate::auth::{Authorized, Authorizer, Command, DecisionLog, Guard};
use crate::identity::{CallerContext, OperationId, Subject};
use crate::report::{Operation, Report, State, Timestamp};
use crate::request::{Catalog, RequestFingerprint, RetryClass, RetryKey, StartRequest};
use crate::value::Value;

/// Number of the first snapshot of an execution.
pub const FIRST_SEQUENCE: u64 = 1;

/// A committed execution: its owner, identity, observation address, and latest snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptedOperation {
    /// Subject that started the execution.
    pub owner: Subject,
    /// Execution identity.
    pub operation: Operation,
    /// Where to observe the execution.
    pub observation: ObservationAddress,
    /// Latest full snapshot; the initial `pending` one right after acceptance.
    pub snapshot: Report,
}

/// The commit could not be performed; nothing was stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitError(pub String);

/// Result of an atomic acceptance attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Acceptance {
    /// The candidate was committed as a new execution.
    Created(AcceptedOperation),
    /// The key was already accepted with the same content; the earlier execution.
    Replayed(AcceptedOperation),
    /// The key was already accepted with different content; nothing changed.
    Conflict,
}

/// Storage of retry mappings and execution records.
pub trait Storage: Send + Sync {
    /// Atomically resolves `key`.
    ///
    /// If the key is unknown, stores the mapping, the execution record, and the
    /// snapshot of `candidate` together and returns [`Acceptance::Created`]. If
    /// it is known, stores nothing and returns the earlier execution when
    /// `fingerprint` matches, or [`Acceptance::Conflict`] when it does not.
    /// On error nothing is stored.
    fn accept(
        &self,
        key: RetryKey,
        fingerprint: RequestFingerprint,
        candidate: AcceptedOperation,
    ) -> Result<Acceptance, CommitError>;

    /// Returns the committed execution `id`, if any.
    fn operation(&self, id: &OperationId) -> Result<Option<AcceptedOperation>, CommitError>;
}

#[derive(Debug, Default)]
struct Inner {
    mappings: HashMap<RetryKey, (RequestFingerprint, OperationId)>,
    operations: HashMap<OperationId, AcceptedOperation>,
}

/// In-memory [`Storage`]. Not durable: a restart loses everything.
#[derive(Debug, Default)]
pub struct MemoryStorage {
    state: Mutex<Inner>,
    #[cfg(feature = "test-support")]
    failures: AtomicU32,
}

fn poisoned<T>(_: T) -> CommitError {
    CommitError("storage lock poisoned".to_string())
}

impl MemoryStorage {
    /// Creates an empty storage.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of committed retry mappings.
    pub fn mapping_count(&self) -> usize {
        self.state.lock().map_or(0, |s| s.mappings.len())
    }

    /// Number of committed execution records.
    pub fn operation_count(&self) -> usize {
        self.state.lock().map_or(0, |s| s.operations.len())
    }

    /// Makes the next `count` commits of a new execution fail before storing anything.
    #[cfg(feature = "test-support")]
    pub fn inject_commit_failures(&self, count: u32) {
        self.failures.store(count, Ordering::SeqCst);
    }

    #[cfg(feature = "test-support")]
    fn take_injected_failure(&self) -> bool {
        self.failures
            .try_update(Ordering::SeqCst, Ordering::SeqCst, |n| n.checked_sub(1))
            .is_ok()
    }
}

impl Storage for MemoryStorage {
    fn accept(
        &self,
        key: RetryKey,
        fingerprint: RequestFingerprint,
        candidate: AcceptedOperation,
    ) -> Result<Acceptance, CommitError> {
        let mut state = self.state.lock().map_err(poisoned)?;
        let previous = state.mappings.get(&key).cloned();
        if let Some((recorded, id)) = previous {
            let existing =
                state.operations.get(&id).cloned().ok_or_else(|| {
                    CommitError("retry mapping has no execution record".to_string())
                })?;
            return match RetryClass::classify(Some(&recorded), &fingerprint) {
                RetryClass::Replay => Ok(Acceptance::Replayed(existing)),
                RetryClass::Conflict => Ok(Acceptance::Conflict),
                RetryClass::New => Err(CommitError("inconsistent retry state".to_string())),
            };
        }
        #[cfg(feature = "test-support")]
        if self.take_injected_failure() {
            return Err(CommitError("injected commit failure".to_string()));
        }
        let id = candidate.operation.id;
        state.mappings.insert(key, (fingerprint, id));
        state.operations.insert(id, candidate.clone());
        Ok(Acceptance::Created(candidate))
    }

    fn operation(&self, id: &OperationId) -> Result<Option<AcceptedOperation>, CommitError> {
        let state = self.state.lock().map_err(poisoned)?;
        Ok(state.operations.get(id).cloned())
    }
}

/// Right to hand a newly created execution to an executor.
///
/// Returned only for a request that created an execution. It carries the proof
/// of the recorded Start permit that [`hand_off`](crate::auth::hand_off) consumes.
#[derive(Debug)]
pub struct Dispatch {
    proof: Authorized,
    operation_id: OperationId,
}

impl Dispatch {
    /// The execution to hand off.
    pub fn operation_id(&self) -> OperationId {
        self.operation_id
    }

    /// Splits into the authorization proof and the execution identifier.
    pub fn into_parts(self) -> (Authorized, OperationId) {
        (self.proof, self.operation_id)
    }
}

/// Outcome of [`Acceptor::start`].
#[derive(Debug)]
pub struct StartOutcome {
    /// Typed response for the caller.
    pub response: StartResponse,
    /// Present only when this request created the execution.
    pub dispatch: Option<Dispatch>,
}

impl StartOutcome {
    fn respond(response: StartResponse) -> Self {
        Self {
            response,
            dispatch: None,
        }
    }

    fn error(message: &str) -> Self {
        Self::respond(StartResponse::Error(ServiceError {
            message: message.to_string(),
        }))
    }
}

type Clock = Box<dyn Fn() -> Timestamp + Send + Sync>;
type Addressing = Box<dyn Fn(OperationId) -> ObservationAddress + Send + Sync>;

/// Accepts Start requests: authorization, validation, and atomic commit.
pub struct Acceptor<A, L, S> {
    catalog: Catalog,
    guard: Mutex<Guard<A, L>>,
    storage: S,
    clock: Clock,
    addressing: Addressing,
}

impl<A, L, S> Acceptor<A, L, S>
where
    A: Authorizer + Send,
    L: DecisionLog + Send,
    S: Storage,
{
    /// Creates an acceptor. Snapshot times come from the system clock and the
    /// observation address is the provisional `operations/<id>/observe`.
    pub fn new(catalog: Catalog, guard: Guard<A, L>, storage: S) -> Self {
        Self {
            catalog,
            guard: Mutex::new(guard),
            storage,
            clock: Box::new(|| {
                let millis = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX));
                Timestamp(millis)
            }),
            addressing: Box::new(|id| ObservationAddress(format!("operations/{id}/observe"))),
        }
    }

    /// Replaces the snapshot clock.
    pub fn with_clock(mut self, clock: impl Fn() -> Timestamp + Send + Sync + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    /// Replaces the observation address format.
    pub fn with_observation_address(
        mut self,
        addressing: impl Fn(OperationId) -> ObservationAddress + Send + Sync + 'static,
    ) -> Self {
        self.addressing = Box::new(addressing);
        self
    }

    /// Returns the storage.
    pub fn storage(&self) -> &S {
        &self.storage
    }

    /// Reads the decision log under the guard's lock; `None` if the lock is poisoned.
    pub fn with_decision_log<R>(&self, read: impl FnOnce(&L) -> R) -> Option<R> {
        self.guard.lock().ok().map(|guard| read(guard.log()))
    }

    /// Runs Start for `caller`.
    ///
    /// Order: authorize and record, validate, atomically commit. A denied
    /// request is not validated, so a caller cannot probe the catalog.
    pub fn start(&self, caller: &CallerContext, request: &StartRequest) -> StartOutcome {
        let command = Command::Start {
            request_id: request.request_id,
            operation_type: request.operation_type.clone(),
            target: request.target.clone(),
        };
        let checked = match self.guard.lock() {
            Ok(mut guard) => guard.check(caller, &command),
            Err(_) => return StartOutcome::error("authorization is unavailable"),
        };
        let proof = match checked {
            Ok(proof) => proof,
            Err(refusal) => return StartOutcome::respond(refusal.start_response()),
        };

        let validated = match self.catalog.validate(request) {
            Ok(validated) => validated,
            Err(rejection) => return StartOutcome::respond(StartResponse::Rejected(rejection)),
        };
        let request = validated.request();

        let candidate = self.candidate(caller, request);
        let key = RetryKey::new(caller, request.request_id);
        match self.storage.accept(key, request.fingerprint(), candidate) {
            Ok(Acceptance::Created(accepted)) => {
                let operation_id = accepted.operation.id;
                StartOutcome {
                    response: accepted_response(accepted),
                    dispatch: Some(Dispatch {
                        proof,
                        operation_id,
                    }),
                }
            }
            Ok(Acceptance::Replayed(accepted)) => {
                StartOutcome::respond(accepted_response(accepted))
            }
            Ok(Acceptance::Conflict) => {
                StartOutcome::respond(StartResponse::Rejected(Rejection::new(
                    RejectionCode::IdentifierConflict,
                    "this request_id was already used with different content",
                    Vec::new(),
                )))
            }
            Err(_) => {
                StartOutcome::error("the acceptance could not be committed; nothing was started")
            }
        }
    }

    fn candidate(&self, caller: &CallerContext, request: &StartRequest) -> AcceptedOperation {
        let id = OperationId::generate();
        let operation = Operation {
            id,
            operation_type: request.operation_type.clone(),
            parent_id: None,
        };
        let mut snapshot = Report::new(
            operation.clone(),
            FIRST_SEQUENCE,
            (self.clock)(),
            State::Pending,
        );
        snapshot.subject = request
            .target
            .as_ref()
            .map_or(Value::Null, |t| Value::Text(t.as_str().to_string()));
        snapshot.context = Value::Map(
            [(
                "parameters".to_string(),
                Value::Map(request.parameters.clone()),
            )]
            .into_iter()
            .collect(),
        );
        snapshot.explanation = "accepted; waiting for the executor to confirm".to_string();
        AcceptedOperation {
            owner: caller.subject().clone(),
            operation,
            observation: (self.addressing)(id),
            snapshot,
        }
    }
}

fn accepted_response(accepted: AcceptedOperation) -> StartResponse {
    StartResponse::Accepted {
        operation: accepted.operation,
        observation: accepted.observation,
    }
}
