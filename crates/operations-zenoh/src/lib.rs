//! Minimal Zenoh adapter for the Gecko Operation module (`0.1.0-dev`).
//!
//! Maps the transport-neutral Core API to Zenoh `1.10.2` as chosen in decisions
//! Z1 and Z3 ([status §2.4](../../../docs-modules/operation/spec/status.md)):
//!
//! | Action | Zenoh | Reply |
//! |---|---|---|
//! | Start | query on `<prefix>/start` with a JSON `StartRequest` payload | one typed `StartResponse` |
//! | Get | query on `<prefix>/<operation id>/report` | one typed `GetResponse` |
//! | Observe | streaming query on `<prefix>/<operation id>/observe?after=<n>` | frames: `response`, `snapshot`*, `end` |
//!
//! Every Operation outcome, including invalid input, is a typed `Ok` reply. A
//! Zenoh error reply, zero replies, or a stream that closes without an `end`
//! frame is "no answer": the [`Client`] reports [`NoAnswer`] and the caller
//! retries (Start with the same `request_id`, Observe with the last `sequence`).
//!
//! The adapter owns the caller identity: [`ServerConfig::caller`] is applied to
//! every request. Zenoh peers are not authenticated in this deployment, so any
//! participant that can reach the endpoint acts as that subject. It is a
//! restricted local test path, not a security claim.
//!
//! Core stays free of Zenoh: this crate depends on `operations`, never the
//! reverse.

mod client;
mod keys;
mod server;
mod wire;

/// Link-cut proxy and the scripted scenario. Not for production use.
#[cfg(feature = "test-support")]
pub mod demo;
/// A TCP proxy whose link can be cut, to test reconnection.
#[cfg(feature = "test-support")]
pub mod proxy;

pub use client::{
    Client, ClientConfig, FollowError, Followed, NoAnswer, ObserveItem, ObserveStream,
};
pub use keys::Keys;
pub use server::{Server, ServerConfig};
pub use wire::{EndReason, ObserveFrame};
