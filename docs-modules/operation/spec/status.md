# Operation: decision status

November 2, 2026

[Overview](../README.md) · [Model](model.md) · [Working roadmap](roadmap.md) · [Security](security.md)

## 1. Accepted decisions

| Decision | Status | Contract and decision provenance |
|---|---|---|
| Operation is an independent foundational module, not part of the reconnaissance system | Accepted | [Module boundary](model.md#2-responsibility-boundary); provenance: [research](../../../docs/2-gecko-probe/researches/operations-and-reconnaissance.md#question-and-brief-conclusion) |
| The module is implemented within Gecko; ROS 2 Actions and Google LRO are direct references, not dependencies | Accepted | [Operation contract](model.md), [comparison of existing solutions](../../../docs/2-gecko-probe/researches/operations-and-reconnaissance.md#1-how-similar-is-operation-to-ros-2-action) |
| Operation does not depend on Probe, App, or their domain entities | Accepted | [CORE-OP catalog entry](../../../docs/core-modules.md#core-op-boundary-and-work), [Operation contract](model.md) |
| A request, an execution, and a report are distinct concepts; acceptance does not promise success | Accepted | [Requests and identity](model.md#4-requests-identity-and-public-api) |
| The client supplies `request_id`, the receiver supplies `operation.id`, and a safe retry returns the previous operation | Accepted | [Operation contract](model.md) |
| Protection against duplicate execution survives a normal restart when accounting is preserved | Accepted | [Operation contract](model.md) |
| Client disconnection does not cancel execution, and an unknown outcome is represented explicitly | Accepted | [Lifecycle](model.md#7-lifecycle-and-cancellation), [recovery](model.md#8-accounting-retention-and-recovery) |
| `unknown` is nonterminal uncertainty about the same execution; reliable confirmation may resolve it without restarting work. Confirmed `succeeded`, `failed`, and `cancelled` outcomes are immutable | Accepted at the model level, October 31, 2026; not yet tested | [Operation contract](model.md), [decision history](../../../docs/1-mesh-app/history.md#from-version-047-to-048) |
| Cancellation is a request to stop, not immediate proof of `cancelled`; partial results remain meaningful | Accepted | [Operation contract](model.md) |
| Repeated Cancel returns current cancellation status without another stop attempt; Cancel on a terminal operation returns its actual outcome unchanged | Accepted at the model level, November 1, 2026; not yet tested | [Operation contract](model.md), [decision history](../../../docs/1-mesh-app/history.md#from-version-050-to-051) |
| The executor submits progress and outcomes; Core validates transitions, records state, and assigns snapshot `sequence` | Accepted at the model level, November 1, 2026; not yet tested | [Operation contract](model.md), [decision history](../../../docs/1-mesh-app/history.md#from-version-049-to-050) |
| Accepted cancellation does not reserve the terminal outcome; actual completion before cancellation takes effect remains `succeeded` or `failed`, and `cancelled` requires confirmed cessation as a result of cancellation | Accepted at the model level, November 1, 2026; not yet tested | [Operation contract](model.md), [decision history](../../../docs/1-mesh-app/history.md#from-version-048-to-049) |
| A parent relationship must not automatically imply ownership, dependency, or cancellation propagation | Accepted as a boundary | [Identity and relationships](model.md#4-requests-identity-and-public-api); provenance: [research §3](../../../docs/2-gecko-probe/researches/operations-and-reconnaissance.md#3-run-tree-and-dependency-graph) |
| Probe uses Operation for an individual run but does not select the next check itself | Accepted at the domain level | [Probe §3.4](../../../docs/2-gecko-probe/model.md#34-preliminary-probes-and-check-chains), [Probe §3.6](../../../docs/2-gecko-probe/model.md#36-probe-in-geckos-zenoh-api) |
| Module work may proceed independently of completing the Probe domain design | Accepted | [Development cycle progress](../../../docs/development-cycle.md#1-route-and-outcomes), [CORE-OP catalog entry](../../../docs/core-modules.md#core-op-boundary-and-work) |
| The `0.1.0` experiment includes Start, Get, Observe, and a minimal Zenoh adapter; Core remains transport-neutral | Accepted as a working release scope, November 1, 2026; not yet tested | [First experiment](model.md#31-agreed-first-experiment-010), [roadmap](roadmap.md) |
| `pending` is the visible initial state before executor acknowledgement; ambiguous handoff becomes `unknown` without automatic redispatch | Accepted at the model level, November 1, 2026; not yet tested | [First experiment](model.md#31-agreed-first-experiment-010), [lifecycle](model.md#7-lifecycle-and-cancellation) |
| Cancel is the `0.2.0` result and durable accounting is the `0.3.0` result | Accepted as a working release order, November 1, 2026; not yet tested | [Roadmap](roadmap.md) |

## 2. Open decisions

This is a working register, not an attempt to predetermine the entire implementation.

Accepted rules live in the module contract. This register owns all open Operation questions, including ST1–ST5 below. The first-experiment scope and `pending` initial state are now accepted at the model level; exact schemas, transport behavior, persistence, and test results still require implementation work.

| Area | Question to resolve | Required result |
|---|---|---|
| States | `pending` is accepted as the initial state; nonterminal `unknown`, its evidence-based transitions, terminal-state immutability, and cancellation/completion outcome semantics are accepted. Remaining: [ST1–ST5](#21-lifecycle-decisions), especially complete evidence, notification arbitration, cancellation accounting, and recording/recovery | Complete transition table and invariant tests |
| Identities | Random UUIDs, scope to receiving Gecko and trusted caller, and normalized request comparison are selected for the first experiment; exact representation and production verification remain | Versioned schema and conflict rules |
| API | Start/Get/Observe semantics and the initial observation/reconnection behavior are selected for `0.1.0`; Cancel follows in `0.2.0`. Remaining: exact typed commands, response schemas, errors, and transport-independent conformance details | Transport-neutral public contract |
| Executor | Partially accepted: executors submit progress and outcomes; Core validates transitions, records state, and numbers snapshots. Remaining: work handoff, acknowledgement, notification validation and arbitration, and concrete progress/result API and errors | Executor API and error model |
| Storage | Atomic record before side effects, journal/snapshots, recovery, detailed-report retention, and safe accounting release under consumer lifecycle constraints; see ST5 | Storage API, crash-consistency model, and contract tests |
| Cancellation | Partially accepted: accepted cancellation does not override actual completion; `cancelled` requires confirmed cessation as a result of cancellation; repeated Cancel returns current cancellation status without a new stop attempt, and terminal outcomes are returned unchanged. Remaining: who may cancel, response schemas, concurrent deduplication, cancellation accounting persistence, explicit reconsideration after rejection, cooperative or forced stop, and atomic arbitration of competing executor notifications | Standalone cancellation contract |
| Composite operations | Causality, links, parent/child relations, ownership, and independent cancellation scope | Relationship model without implicit cascading semantics |
| Concurrency | Admission control, limits, queues, and conflicting actions without a global lock | Common hooks and domain conflict rules |
| Reports | Partially accepted: Core owns snapshot numbering; `reported_at` is generation time at Core. Remaining: minimum typed schema, extensions, and removal of details or artifacts | Versioned Operation Report schema |
| Transport | Minimal Zenoh Start/Get/Observe adapter is in `0.1.0`; select and validate SDK, keys, late-attachment behavior, and gaps without leaking transport details into Core | First adapter and common transport tests |
| Compatibility | Whether ROS 2 Actions or Google LRO mappings and adapters are required | Explicit compatibility targets or an explicit decision not to provide them |
| Observability | Correlation with traces, logs, and metrics without making telemetry the source of truth | Correlation context and export rules |
| Implementation | Rust on Ubuntu-24.04 in WSL and a one-process first experiment are selected. Provisional layout since November 2, 2026 (ROB-59): a root Cargo workspace with Rust crates under `crates/`; the first crate is [`operations`](../../../crates/operations/README.md), without dependencies, with test support behind the `test-support` feature, and with the toolchain pinned to 1.99.1. Remaining: whether the Zenoh adapter is a feature of `operations` or a separate `operations-zenoh` crate; a synchronous or asynchronous Executor API; whether to add CI (the repository has none) and which runner; the toolchain upgrade policy; schema format and later supported platforms | ADR after validating the minimum prototype |
| Security | Caller identity, authorization for every command, auditing, data protection, and resource exhaustion | Threat model and verifiable security gates before `1.0.0` |

### 2.1. Lifecycle decisions

| ID | Question | Status | Completed work and remaining work |
|---|---|---|---|
| ST1 | Acceptance and entry into execution | Accepted for the first experiment; broader handoff cases open | Acceptance creates one execution and visible `pending` snapshot before side effects; executor acknowledgement establishes `running`. No queue is implied. Validate failed and ambiguous handoff paths. |
| ST2 | Executor evidence and loss of execution knowledge | Partially accepted | A lost acknowledgement after possible handoff yields `unknown`, without redispatch; confirmation must identify the same execution token. Define the complete evidence and error taxonomy, including executor loss and recovery. |
| ST3 | Concurrent, duplicate, and late executor notifications | Partially accepted | First experiment uses identified, ordered notifications: duplicates do not advance `sequence`; stale progress and conflicting terminal events are protocol errors. Validate arbitration under concurrency and decide terminal-detail enrichment. Arrival order alone is not proof of actual outcome. |
| ST4 | Cancellation accounting and reconsideration | Partially accepted | Repeats do not create another stop attempt; actual completion remains authoritative. Define cancellation status fields, behavior while execution is `unknown`, responses to concurrent Cancel calls, and explicit reconsideration after a rejection. |
| ST5 | Atomic recording and recovery boundaries | Partially accepted | Acceptance precedes side effects; snapshot numbering survives normal restart. Define which state, result, snapshot, counter, and cancellation records commit together, and what callers and executors observe if recording fails. |

ST1–ST5 refine the areas above rather than add a second backlog. Safe consumer-driven accounting release is also open: the deletion of an App must not become knowledge required by Core.

### 2.2. Minimum decisions before the first prototype

The following six packages are the agreed checklist for the first prototype. Their first-experiment answers and release scope are now recorded in [the model](model.md#31-agreed-first-experiment-010) at the architectural level. This approves the direction for implementation, not an untested concrete schema, selected Zenoh SDK, completed code, or a release. Finalize exact types and wire forms during implementation and validate each package separately.

| ID | Package | Accepted first-experiment decision | Remaining validation or detail |
|---|---|---|---|
| P1 | Prototype boundary and stack | Rust on Ubuntu-24.04 in WSL; one Gecko, test executor, in-memory storage; Start/Get/Observe through minimal Zenoh adapter and direct Core checks. Cancel follows in `0.2.0`. | Run the accepted scenario and failure checks; document SDK choice, test topology, and no restart guarantee. |
| P2 | State machine | Initial `pending`; confirmed handoff gives `running`, ambiguous handoff `unknown` without redispatch; terminal outcome needs authoritative evidence. | Test transitions and invariants; finish ST2–ST3 evidence and arbitration details. |
| P3 | Public API | Start has request identity, type, target, and typed parameters; Get returns latest full snapshot or explicit unavailability; Observe supplies current snapshot and later updates with cursor and gap disclosure. | Specify concrete types, errors, serialization, and Zenoh mapping; verify late attachment and reconnect races. |
| P4 | Identity and retry | Random UUIDs; retry scoped to receiving Gecko and trusted caller; normalized type, target, and typed parameters determine content equality; concurrent retries create one execution. | Verify normalization and conflict behavior; validate production identity/security and restart behavior later. |
| P5 | Executor contract | One assigned test executor, execution token, separate acknowledgement, identified ordered notifications, Core validation/numbering, duplicate suppression, stale/conflicting event errors. | Finalize event types and competing-notification behavior in code; cancellation interaction follows in `0.2.0`. |
| P6 | Accounting and trust | Atomic in-memory request mapping, operation record, and initial snapshot before handoff; trusted caller context, authorization point, and recorded decision; restricted permissive test policy. | Verify failed-commit behavior; durable accounting, persisted audit, and recovery follow in `0.3.0`; full security gate remains for `1.0.0`. |

The six packages map to the existing areas and ST1–ST5; they do not create a second set of rules. They settle the first-experiment direction while leaving wider lifecycle and implementation questions open with validation steps. In-memory storage does not satisfy or waive the target contract's durable retry and normal-restart guarantees. A permissive test policy does not establish production security; trusted context, authorization, and audit responsibilities remain governed by [security](security.md).

The first-experiment scope and checks are accepted at the model level. Before implementation is considered validated, concrete types, Zenoh mapping, and contract tests must demonstrate the stated behavior. Observe is in `0.1.0`; Cancel is in `0.2.0`. Broader integration, persistence, and security gates remain open independently.

## 3. Current status

- The module boundary is accepted.
- The active common contract is owned by [Operation model](model.md), with security requirements in [security](security.md).
- Accepted model rules and explicitly open implementation details coexist in the module specification.
- Uncertainty transitions and terminal-state immutability are accepted at the model level; the full state machine remains incomplete.
- Cancellation/completion outcome semantics are accepted at the model level; atomic arbitration and cancellation response details remain open.
- Repeated and late Cancel semantics are accepted at the model level; response schemas, concurrency, persistence, and reconsideration after rejection remain open.
- Core authority over state validation, recording, and snapshot numbering is accepted at the model level; executor notification and arbitration details remain open.
- The first-experiment direction, including Observe and minimal Zenoh, and the working release order are accepted at the model level. A Rust build and test harness skeleton exists (ROB-59, November 2, 2026); Start, Get, Observe, the Zenoh adapter, and contract tests are not implemented.
- Exact schemas, SDK choices, transport guarantees, and later release contents remain subject to experiments.
- November 1, 2026: common design migrated from Mesh / App into Operation; consumer requirements remain in domain documents. Historical links above record provenance, not contract authority.

The next substantive result is to finalize the first experiment's concrete API and event types, implement the selected path through Zenoh, and evaluate the six [prototype packages](#22-minimum-decisions-before-the-first-prototype). The accepted model decisions must be corrected if experiments disprove them.
