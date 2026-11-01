# Operation: working roadmap from 0.1.0 to 3.0.0

November 1, 2026 · **working release order accepted at the model level; no releases implemented**

[Overview](../README.md) · [Model](model.md) · [Decision status](status.md) · [Security](security.md)

The common contract is owned by [Operation model](model.md). Version numbers below refer to Operation releases, not the product Gecko version. `0.1.0` includes a minimal Zenoh adapter from the start, while Core retains a transport-neutral contract. The `0.1.0`–`0.3.0` sequence is agreed for the first experiments; later rows are directions to refine from observed results.

This roadmap orders verifiable results. A version represents a result only when the corresponding implementation and tests exist, not when its description is written. The later boundaries and number of intermediate releases may change after experiments.

| Version | Objective | Verifiable result |
|---|---|---|
| `0.1.0` | First end-to-end experiment | Rust on Ubuntu-24.04 in WSL; Start, Get, and **Observe over minimal Zenoh** with a test executor and in-memory accounting. Validate acceptance/rejection, concurrent safe retry, changed-content conflict, `pending`/`unknown`, progress and terminal outcome, late attachment, reconnect, gap disclosure, and attachment racing with final publication. Core is tested directly as well. No restart-durability claim. |
| `0.2.0` | Cancellation | Cancel over Zenoh; repeated/concurrent cancellation, cancel/complete races, status separate from execution outcome, and preserved partial results. |
| `0.3.0` | Durable accounting | Storage API, persistence before side effects, retry after a lost response and restart, cancellation accounting and recovery, and explicit `unknown` |
| `0.4.0` | Integration boundaries | Refine and stabilize Client, Executor, Storage, and Transport APIs and the existing Zenoh adapter against shared contract tests, including read/subscription reconciliation. |
| `0.5.0` | Access and load control | Production identity and authorization path, persisted audit, quotas, and protection against primary resource-exhaustion scenarios. Trusted caller context and authorization hooks already exist in the first API. |
| `0.6.0` | First domain integration | One Probe runs through the module without reconnaissance concepts leaking into Core |
| `0.7.0` | Second domain integration | One App operation uses the same module; validate parent/link/ownership/cancellation scope only where a real scenario requires it. |
| `0.8.0` | Failure hardening | Fault injection, crash points, concurrent retries, recovery, and load boundaries |
| `0.9.0` | Stable-contract candidate | Frozen schemas and APIs, storage migrations, compatibility suite, and integration tests for Probe and an App operation |
| `1.0.0` | Stable foundational module | Compatibility, recovery, operations, and security criteria are met; guarantees and limitations are published |
| `1.x` | Compatible extensions | Add operation types and client conveniences without weakening the stable lifecycle, identity, or recovery guarantees. |
| `2.0.0` | Distributed-operation direction | Explore coordination across Gecko instances when a distributed App needs it; define identity, authority, and recovery guarantees from real scenarios before approving scope. |
| `3.0.0` | Dynamic-Mesh direction | Explore operation responsibility transfer under Mesh membership changes when required by product scenarios; do not infer automatic migration or recovery from the version number. |

`Observe` is the operation-specific read/subscribe action and begins in `0.1.0`. The optional persistent product **Observer** for independent history and notifications remains a [Mesh / App 3.0.0 direction](../../../docs/1-mesh-app/model.md#1-purpose-boundaries-and-status); it is a different responsibility.

## Conditions for changing this plan

The roadmap will be revised after `0.1.0` according to observed risks. Later releases may be combined, divided, or reordered. Security is not postponed until `0.5.0`: that row identifies a dedicated integration result, while required security properties must be designed into the first public contract. The initial Zenoh deployment is a restricted test path, not a production security claim.

No intermediate version is required to support multiple languages. The project first needs one reference stack and language-neutral schemas and contract tests. An additional SDK should appear only for a real consumer and must not precede semantic stabilization.

## Preliminary `1.0.0` gate

Before the first stable release, the project must at least:

- implement and test Start, Get, Observe, and Cancel against the public contract, including concurrent calls, repeated requests, observation reconnection, and recovery after restart;
- approve and test the public state machine and compatibility rules;
- demonstrate safe retry after a lost response and a normal restart;
- define behavior at every supported crash point and document the limits of its guarantees;
- test concurrent start/get/watch/cancel operations and resource-exhaustion protection;
- have at least two domain-distinct integrations, or explicitly justify why one is sufficient;
- complete the [security gate](security.md#security-gate-for-100);
- provide operational guidance for storage, upgrades, backup, and diagnostics;
- publish the validated module contract and its compatibility guarantees from this documentation center.
