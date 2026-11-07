//! Transport-neutral core of the Gecko Operation module.
//!
//! This is the `0.1.0-dev` crate. It contains the typed contracts (Start, Get,
//! Observe inputs and outcomes, full reports), UUIDv4 identities, request
//! validation and retry comparison, the authorization hook with decision
//! records, and the provisional executor contract. It has no dependency on
//! Zenoh, Probe, App, or other domain entities. Atomic acceptance, storage, the
//! state machine, and the transport adapter are not implemented yet. The
//! contract is owned by `docs-modules/operation/spec/model.md`.

/// Typed Start, Get, and Observe inputs and outcomes.
pub mod api;
/// Authorization hook and decision records.
pub mod auth;
/// Provisional executor contract: execution tokens and handoff outcomes.
pub mod executor;
/// Request and execution identities and the trusted caller context.
pub mod identity;
/// Full Operation Report types.
pub mod report;
/// Start request, validation catalog, and retry comparison.
pub mod request;
/// Typed structured values.
pub mod value;

/// Test support (enabled by the `test-support` feature). Not for production use.
#[cfg(feature = "test-support")]
pub mod testing;

pub use api::{
    GetRequest, GetResponse, InputError, ObservationAddress, ObserveRequest, ObserveResponse,
    Rejection, RejectionCode, ServiceError, StartResponse, Unavailable, Violation,
};
pub use auth::{
    Authorized, Authorizer, Command, Decision, DecisionLog, DecisionRecord, DenyAll, Guard,
    MemoryDecisionLog, Refusal,
};
pub use executor::{ExecutionToken, Executor, Handoff};
pub use identity::{CallerContext, OperationId, RequestId, Subject};
pub use report::{Blocker, Operation, ProposedAction, Report, State};
pub use request::{
    Catalog, OperationSpec, OperationType, ParameterSpec, RequestFingerprint, RetryClass, RetryKey,
    StartRequest, Target, TargetRule, ValidatedStart,
};
pub use value::{Parameters, Value, ValueKind};
