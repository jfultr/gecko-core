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

### 2.2. Minimum decisions before the first prototype

The following six packages are the agreed checklist for deciding when to begin writing the first prototype. They identify the decisions needed for that experiment; they do not approve the answers, a release scope, or implementation readiness. Each package is complete for this purpose when its stated criterion is accepted at the model level. Record its accepted rule in [model](model.md) or [security](security.md), and update this register. Implementation and experimental validation are tracked separately.

| ID | Package | Status and accepted basis | Remaining decision and sufficient outcome before code |
|---|---|---|---|
| P1 | Prototype boundary and stack | Open. An in-process prototype with in-memory storage and a test executor is permitted; the example [roadmap](roadmap.md) is not an approved plan. | Select the experiment's observable scenario, included commands and failure cases, language and execution environment, and explicit limitations. State how its result will be evaluated. |
| P2 | State machine | Partially accepted: `unknown`, terminal outcomes, Core authority, and cancellation/completion semantics are established in [lifecycle](model.md#7-lifecycle-and-cancellation). | For the P1 scope, define the initial state and the transitions and evidence needed for its scenario, including uncertainty and relevant cancellation races. Identify which ST1–ST3 questions the experiment leaves open or will investigate. |
| P3 | Public API | Partially accepted: Start/Get/Observe/Cancel have common meanings in [the contract](model.md#4-requests-identity-and-public-api). | Define the inputs, responses, and errors for each command included in P1, including accepted/rejected Start and unavailable reports. State explicitly how commands outside the experiment are deferred, without changing the target contract. |
| P4 | Identity and retry | Partially accepted: client `request_id`, receiver `operation.id`, retry and changed-content conflict semantics are established. | Choose identifier representation and uniqueness scope, request-content comparison, and concurrent retry behavior sufficient to exercise the P1 scenario; retain open security and cross-restart questions for later validation. |
| P5 | Executor contract | Partially accepted: the executor supplies facts; Core validates transitions and numbers snapshots. | Define handoff, acknowledgement, progress and outcome notifications, errors, and the cancellation interaction if included in P1. Specify how the prototype treats relevant duplicate or competing notifications; link unresolved ST2–ST4 details to later work. |
| P6 | Accounting and trust | Partially accepted: accepted-request accounting precedes domain side effects, and the first API carries trusted caller context and an authorization point. | Define the prototype's accounting boundary and storage interface, including how acceptance and deduplication are recorded before handoff, and trusted caller context, authorization, and audit integration. An in-memory implementation and test policy are possible if their limitations, including lack of restart durability, are explicit. Identify which ST4–ST5 recovery and persistence cases require a later implementation. |

The six packages map to the existing areas and ST1–ST5; they do not create a second set of accepted rules. A decision may settle only the portion required by P1 and leave the rest open with a validation step. In particular, choosing in-memory storage for an experiment does not satisfy or waive the target contract's durable retry and normal-restart guarantees. A permissive test policy does not establish production security; trusted context, authorization, and audit responsibilities remain governed by [security](security.md).

Before implementation starts, all six prototype criteria must be accepted and P1 must specify the checks that will distinguish observed results from assumptions. The accepted checklist itself does not determine whether Observe or Cancel is in the first experiment; their common contract remains in force, and the P1 decision must say what is included. Broader release, integration, persistence, and security gates remain open independently.

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

The next substantive result is to resolve the six [minimum prototype decision packages](#22-minimum-decisions-before-the-first-prototype), then implement and evaluate the selected experiment. The checklist is accepted; its package-level answers remain open or partially accepted as shown.
