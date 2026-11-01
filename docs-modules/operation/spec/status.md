# Operation: decision status

November 1, 2026

[Overview](../README.md) · [Model](model.md) · [Example roadmap](roadmap.md) · [Security](security.md)

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

## 2. Open decisions

This is a working register, not an attempt to predetermine the entire implementation.

Accepted rules live in the module contract. This register owns all open Operation questions, including ST1–ST5 below. `pending` is only a proposal; the migration does not approve it or complete the minimum prototype contract.

| Area | Question to resolve | Required result |
|---|---|---|
| States | Partially accepted: nonterminal `unknown`, its evidence-based transitions, terminal-state immutability, and cancellation/completion outcome semantics; consolidated matrix available. Remaining: [ST1–ST5](#21-lifecycle-decisions), including initial state, evidence validation, notification arbitration, cancellation accounting, and recording/recovery | Complete transition table and invariant tests |
| Identities | Format and uniqueness scope of `request_id` and `operation.id`; canonical request comparison | Versioned schema and conflict rules |
| API | Partially accepted: Start/Get/Observe/Cancel semantics in the module contract. Remaining: typed commands, response schemas, errors, and observation reconnection details | Transport-neutral public contract |
| Executor | Partially accepted: executors submit progress and outcomes; Core validates transitions, records state, and numbers snapshots. Remaining: work handoff, acknowledgement, notification validation and arbitration, and concrete progress/result API and errors | Executor API and error model |
| Storage | Atomic record before side effects, journal/snapshots, recovery, detailed-report retention, and safe accounting release under consumer lifecycle constraints; see ST5 | Storage API, crash-consistency model, and contract tests |
| Cancellation | Partially accepted: accepted cancellation does not override actual completion; `cancelled` requires confirmed cessation as a result of cancellation; repeated Cancel returns current cancellation status without a new stop attempt, and terminal outcomes are returned unchanged. Remaining: who may cancel, response schemas, concurrent deduplication, cancellation accounting persistence, explicit reconsideration after rejection, cooperative or forced stop, and atomic arbitration of competing executor notifications | Standalone cancellation contract |
| Composite operations | Causality, links, parent/child relations, ownership, and independent cancellation scope | Relationship model without implicit cascading semantics |
| Concurrency | Admission control, limits, queues, and conflicting actions without a global lock | Common hooks and domain conflict rules |
| Reports | Partially accepted: Core owns snapshot numbering; `reported_at` is generation time at Core. Remaining: minimum typed schema, extensions, and removal of details or artifacts | Versioned Operation Report schema |
| Transport | Map the API and observation onto Zenoh without leaking its details into Core | First adapter and common transport tests |
| Compatibility | Whether ROS 2 Actions or Google LRO mappings and adapters are required | Explicit compatibility targets or an explicit decision not to provide them |
| Observability | Correlation with traces, logs, and metrics without making telemetry the source of truth | Correlation context and export rules |
| Implementation | First language, package/process boundaries, schema format, and supported platforms | ADR after validating the minimum prototype |
| Security | Caller identity, authorization for every command, auditing, data protection, and resource exhaustion | Threat model and verifiable security gates before `1.0.0` |

### 2.1. Lifecycle decisions

| ID | Question | Status | Completed work and remaining work |
|---|---|---|---|
| ST1 | Acceptance and entry into execution | Partially accepted | Acceptance creates one execution before side effects. Decide its initial state, what is observable before executor acknowledgement, and whether a separate pending state or handoff field is needed. Do not silently describe queued work as executing. |
| ST2 | Executor evidence and loss of execution knowledge | Partially accepted | Evidence-based uncertainty transitions are accepted. Define what confirms the same execution is active, which errors prove failure, and when an exception, lost acknowledgement, or lost executor instead requires `unknown`. |
| ST3 | Concurrent, duplicate, and late executor notifications | Partially accepted | Core owns validation and numbering; terminal outcomes are immutable. Define notification identity/order, competing final-outcome arbitration, stale progress handling, and whether terminal report details may be enriched. Arrival order alone has not been approved as proof of actual outcome. |
| ST4 | Cancellation accounting and reconsideration | Partially accepted | Repeats do not create another stop attempt; actual completion remains authoritative. Define cancellation status fields, behavior while execution is `unknown`, responses to concurrent Cancel calls, and explicit reconsideration after a rejection. |
| ST5 | Atomic recording and recovery boundaries | Partially accepted | Acceptance precedes side effects; snapshot numbering survives normal restart. Define which state, result, snapshot, counter, and cancellation records commit together, and what callers and executors observe if recording fails. |

ST1–ST5 refine the areas above rather than add a second backlog. Safe consumer-driven accounting release is also open: the deletion of an App must not become knowledge required by Core.

## 3. Current status

- The module boundary is accepted.
- The active common contract is owned by [Operation model](model.md), with security requirements in [security](security.md).
- Accepted model rules and explicitly open implementation details coexist in the module specification.
- Uncertainty transitions and terminal-state immutability are accepted at the model level; the full state machine remains incomplete.
- Cancellation/completion outcome semantics are accepted at the model level; atomic arbitration and cancellation response details remain open.
- Repeated and late Cancel semantics are accepted at the model level; response schemas, concurrency, persistence, and reconsideration after rejection remain open.
- Core authority over state validation, recording, and snapshot numbering is accepted at the model level; executor notification and arbitration details remain open.
- Implementation, prototype, and contract tests have not started.
- The exact release breakdown is not approved.
- November 1, 2026: common design migrated from Mesh / App into Operation; consumer requirements remain in domain documents. Historical links above record provenance, not contract authority.

The next substantive result is to approve a minimum state machine and transport-neutral API, then validate them with an in-memory prototype, a test executor, and fault and concurrency tests.
