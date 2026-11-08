# Operation

An independent foundational Gecko module for starting and observing long-running operations.

Operation accepts or rejects a request, creates a distinct execution, publishes its progress and result, handles cancellation requests, and prevents duplicate execution after a lost response or a normal restart. The module has no knowledge of domain entities such as Probe, App, Block, or Line: domain executors integrate with it through a separate contract.

Current version: [`0.1.0-dev`](VERSION). Only the Rust build and test harness skeleton exists; Start, Get, Observe, and the Zenoh adapter are not implemented. This is the version of the contract and first prototype under development, not a completed release.

## Contents

- [Model and boundaries](spec/model.md) — the authoritative common contract, responsibilities, lifecycle, reports, accounting, and validation.
- [Decision status](spec/status.md) — accepted decisions, open questions, and links to their original context.
- [Working roadmap](spec/roadmap.md) — the accepted first-experiment order from `0.1.0` to `0.3.0`, later `0.x` targets, and directions through `3.0.0`; no release is implemented yet.
- [Security](spec/security.md) — required areas of security work and the admission gate for `1.0.0`.

## Code

The Rust implementation lives in the repository's root Cargo workspace rather than in this documentation center: the transport-neutral Core in [`crates/operations`](../../crates/operations/README.md) and the minimal Zenoh adapter, client and demo in [`crates/operations-zenoh`](../../crates/operations-zenoh/README.md). Run `make check` from the repository root. The crate README records commands, toolchain versions, and current limitations. The code layout is provisional; its open questions are in [decision status](spec/status.md#2-open-decisions).

## Documentation ownership

[Model and contract](spec/model.md) owns Operation design and accepted common rules. [Security](spec/security.md) owns security requirements. [Decision status](spec/status.md) is the single register of accepted decisions, their provenance, and open questions, including ST1–ST5. The roadmap records a working release order, subject to experimental revision.

Mesh / App and Gecko / Probe are consumers. They define requirements and domain-specific behavior, and use the module contract through references. Their documents do not override Operation design. A mismatch between a consumer requirement and module guarantees must be recorded and resolved explicitly.

On November 1, 2026, the common contract was migrated here from Mesh / App §8.2. Historical references to its former location are provenance, not active authority. The first Operation experiment was subsequently scoped to Start, Get, Observe, and minimal Zenoh; Cancel and durable accounting follow in `0.2.0` and `0.3.0`. Product Gecko 1.0.0 requirements and the unreleased Operation version `0.1.0-dev` are distinct. Agreed scope does not claim implementation.
