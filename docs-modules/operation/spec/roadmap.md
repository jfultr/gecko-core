# Operation: example roadmap to 1.0.0

October 28, 2026 · **preliminary example, not an approved plan**

[Overview](../README.md) · [Model](model.md) · [Decision status](status.md) · [Security](security.md)

This document illustrates a possible order for reducing risk and producing verifiable results. Version numbers, boundaries, and the number of intermediate releases will be revised after the state machine and first prototype are designed. A version represents a result only when the corresponding implementation and tests exist, not when its description is written.

| Version | Possible objective | Example verifiable result |
|---|---|---|
| `0.1.0` | Minimum contract | In-memory Core and test executor: start, accepted/rejected, get, progress, terminal result, and basic identity and transition tests |
| `0.2.0` | Cancellation and observation | Observation after late attachment, repeated cancellation, cancel/complete races, and partial results |
| `0.3.0` | Durable accounting | Storage API, persistence before side effects, retry after a lost response, restart, and explicit `unknown` |
| `0.4.0` | Integration boundaries | Stabilized Client, Executor, Storage, and Transport APIs plus the first Zenoh adapter with common contract tests |
| `0.5.0` | Access and load control | Trusted caller context, authorization hooks, audit events, quotas, and protection against primary resource-exhaustion scenarios |
| `0.6.0` | First domain integration | One Probe runs through the module without reconnaissance concepts leaking into Core |
| `0.7.0` | Related operations | Validated parent/link/ownership/cancellation-scope model, if justified by a real scenario |
| `0.8.0` | Failure hardening | Fault injection, crash points, concurrent retries, recovery, and load boundaries |
| `0.9.0` | Stable-contract candidate | Frozen schemas and APIs, storage migrations, compatibility suite, and integration tests for Probe and an App operation |
| `1.0.0` | Stable foundational module | Compatibility, recovery, operations, and security criteria are met; guarantees and limitations are published |

## Conditions for changing this example

The roadmap will be revised after `0.1.0` according to observed risks. Releases may be combined, divided, or reordered. In particular, security is not postponed until `0.5.0`: that row identifies a dedicated integration result, while required security properties must be designed into the first public contract.

No intermediate version is required to support multiple languages. The project first needs one reference stack and language-neutral schemas and contract tests. An additional SDK should appear only for a real consumer and must not precede semantic stabilization.

## Preliminary `1.0.0` gate

Before the first stable release, the project must at least:

- approve and test the public state machine and compatibility rules;
- demonstrate safe retry after a lost response and a normal restart;
- define behavior at every supported crash point and document the limits of its guarantees;
- test concurrent start/get/watch/cancel operations and resource-exhaustion protection;
- have at least two domain-distinct integrations, or explicitly justify why one is sufficient;
- complete the [security gate](security.md#security-gate-for-100);
- provide operational guidance for storage, upgrades, backup, and diagnostics;
- move the source of truth for the common contract into the versioned module specification.
