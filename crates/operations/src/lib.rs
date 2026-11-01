//! Transport-neutral core of the Gecko Operation module.
//!
//! This is the `0.1.0-dev` harness skeleton. It contains only the executor
//! contract shape needed to drive controllable tests. It has no dependency on
//! Zenoh, Probe, App, or other domain entities. Start, Get, and Observe are not
//! implemented yet. The contract is owned by `docs-modules/operation/spec/model.md`.

/// Provisional executor contract: execution tokens and handoff outcomes.
pub mod executor;

/// Test support (enabled by the `test-support` feature). Not for production use.
#[cfg(feature = "test-support")]
pub mod testing;

pub use executor::{ExecutionToken, Executor, Handoff};
