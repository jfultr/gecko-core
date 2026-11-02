# Operation: model and contract

Specification draft 0.7 · module `0.1.0-dev` · November 1, 2026

[Overview](../README.md) · [Decision status](status.md) · [Working roadmap](roadmap.md) · [Security](security.md)

This document owns the common Operation design and accepted contract. [Security](security.md) owns its security requirements; [decision status](status.md) is the single register of decisions and open questions. Consumer documents specify their requirements and domain behavior, and reference this contract.

The accepted rules below are model-level decisions, not implementation evidence. The current target is integration into local Gecko 1.0.0. That product target does not assign these guarantees to a completed Operation release: the module remains `0.1.0-dev`. The first-experiment scope and release order are agreed as a working plan in [the roadmap](roadmap.md); prototype limitations must be stated explicitly.

## 1. Purpose

A request expresses intent, an Operation identifies one execution, and a report exposes its available progress and outcome. Acceptance does not promise success. Domain decisions authorizing actions are separate from these concepts.

Operation is an independent execution core for requests whose results may not be available at call time. It provides the same mechanics for Probe diagnostics, App preparation and modification, and future domain actions without incorporating their domain logic.

The module must allow a caller to:

1. submit a request with its own identity;
2. determine unambiguously whether the request was accepted or rejected;
3. observe the specific execution, its progress, partial data, and outcome after acceptance;
4. retry after a lost response without creating a second execution;
5. request cancellation and separately determine the actual outcome;
6. resume observation after a client disconnect or a normal runtime restart;
7. see an explicitly unknown outcome instead of an unsafe assumption of success, failure, or continued execution.

The architectural references are [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151). They are not runtime dependencies and do not imply API compatibility.

## 2. Responsibility boundary

Operation is responsible for:

- request acceptance, common-contract validation, and rejection;
- distinguishing the caller-provided `request_id` from the created execution's `operation.id`;
- idempotently mapping a repeated request to an already accepted operation;
- lifecycle, ordered report snapshots, progress, and terminal outcome;
- cancellation requests and observable cancellation processing;
- durable minimal accounting, restoration of observation, and explicit `unknown` outcomes;
- common enforcement points for authorization, auditing, and resource limits;
- transport-neutral client and executor contracts.

Operation is not responsible for:

- selecting the next Probe or defining the reconnaissance strategy;
- evaluating domain readiness of an App, constructing Build, or making Planner decisions;
- defining the meaning of inputs, results, blockers, artifacts, or acquired resources of a domain action;
- automatically rolling back, cleaning up, or resuming arbitrary domain actions;
- isolating untrusted executor code or securing the executor's own domain logic;
- making Zenoh an irreplaceable part of the internal core.

An executor defines the domain action and submits progress, partial results, and execution outcomes through the Executor API. Core validates transitions, records the accepted state, and assigns `sequence` to full report snapshots; domain facts remain the executor's responsibility. The authority and numbering rules are defined in [§6](#6-snapshots-and-core-authority). A consumer decides what to do with the result. For example, a Probe publishes findings through Operation, while Gecko's reconnaissance policy decides which checks to start next.

## 3. Intended module parts

These are logical boundaries, not an approved process or package layout.

| Part | Responsibility |
|---|---|
| **Core** | Identities, state validation and recording, snapshot numbering, acceptance/rejection, retries, cancellation, and invariants |
| **Executor API** | Contract for integrating a domain executor and publishing progress or results |
| **Storage API** | Atomic minimal request and operation accounting, report snapshots, and recovery |
| **Server API** | Common start, read, observe, and cancel commands |
| **Client library** | Safe `request_id` creation, retries, observation, and error interpretation |
| **Transport adapters** | Mapping the common API onto Zenoh and, where needed, other transports |
| **Contract tests** | Shared conformance tests for storage, transport, and executor implementations |

The first prototype may run in one process and use in-memory storage if those limitations are explicit and the interfaces do not bind Core to a specific transport.

### 3.1. Agreed first experiment (`0.1.0`)

The first experiment uses Rust on Ubuntu-24.04 in WSL, one Gecko, a test executor, and in-memory accounting. It exercises **Start, Get, and Observe through a minimal Zenoh adapter** as well as Core directly. Core remains transport-neutral. Cancel is the next-stage `0.2.0` result, not a `0.1.0` command implementation. The first Zenoh adapter is an experimental local path; its SDK version, keys, and delivery guarantees are selected and validated during implementation. The test deployment is restricted to trusted local participants until transport authentication and production authorization are validated.

The observable scenario covers accepted and rejected Start, a lost Start response followed by same-content retry, a changed-content conflict, concurrent retries, progress and a terminal result, and uncertainty after lost executor acknowledgement without automatic redispatch. Observe must work for an already running or completed operation, after reconnection, and when attachment races with final publication. A slow observer or a missed intermediate snapshot must be detectable; full history is not promised. These checks distinguish demonstrated behavior from model assumptions.

The prototype decisions for the six [pre-code packages](status.md#22-minimum-decisions-before-the-first-prototype) are:

| Package | Accepted first-experiment rule |
|---|---|
| P1: boundary | Start, Get, Observe and the minimal Zenoh path above; in-process test executor and in-memory Storage implementation. No restart-durability claim for `0.1.0`. |
| P2: states | Acceptance creates a visible `pending` snapshot before executor acknowledgement. Confirmed execution enters `running`; an ambiguous handoff or loss of reliable execution knowledge enters `unknown`. Terminal states require an authoritative outcome. No queue is introduced. |
| P3: public API | Start takes a caller-supplied `request_id`, operation type, applicable target, and typed parameters, then returns accepted identity and observation address or a typed rejection. Get takes `operation.id` and returns the latest available full snapshot or explicit unavailability. Observe takes `operation.id` and an optional last known `sequence`, delivers the available current snapshot and subsequent snapshots, and indicates a gap when intermediate snapshots were missed. Unauthorized access and invalid input are distinct errors. Concrete serialization and transport keys are selected during implementation and conformance testing. |
| P4: identity and retry | The experiment uses random UUIDv4 values for both identifiers. A retry key is scoped to the receiving Gecko and trusted caller subject. The operation type, applicable target, and normalized typed parameters are compared; tracing metadata is excluded. Concurrent same-content retries resolve to one accepted execution; changed content conflicts. The normalized representation and UUID implementation are verified in code. |
| P5: executor | A single assigned test executor receives an execution token, acknowledges handoff separately, and sends identified, ordered progress and terminal notifications. Core validates and numbers accepted full snapshots. Duplicate notifications create no new snapshot; stale progress and conflicting terminal notifications are protocol errors. An ambiguous handoff is not retried automatically. Exact event schema and competing-notification handling are verified in the prototype. |
| P6: accounting and trust | The request mapping, execution record, and first snapshot commit together in memory before handoff. If this commit fails, Start does not accept or dispatch work. The adapter supplies trusted caller context rather than taking identity from the request body; Core calls authorization and records its decision before handoff. A permissive test policy is allowed only in the restricted test deployment. Durable accounting, persisted audit, and recovery are `0.3.0` work. |

For Observe, attaching a subscriber and reading the latest snapshot must reconcile atomically at Core so a final publication cannot be lost between the two. Reconnection uses `sequence` for deduplication and gap disclosure; it never restarts execution. The Zenoh adapter must demonstrate this behavior end to end rather than assuming transport history or cache supplies it. A gap does not imply loss of the latest available full snapshot. Concrete wire responses and unavailable-report behavior remain to be finalized in the first implementation contract.

## 4. Requests, identity, and public API

**Accepted: operation request acceptance.** The receiving Gecko validates a request and responds with acceptance or rejection.

| Request response | Meaning and content |
|---|---|
| **Accepted** | Gecko created a particular Operation execution, assigned `operation.id`, and returns its identifier and Operation Report observation address |
| **Rejected** | No execution was created; the response contains a reason and an explanation understandable to a person and agent |

The receiving Gecko generates the execution identifier and returns it on acceptance. An unknown operation type or invalid parameters may cause rejection. Acceptance confirms neither successful execution nor participant readiness. A blocker discovered while executing an accepted check appears in its Operation Report rather than turning earlier acceptance into rejection.

Acceptance or rejection describes request handling. Operation Report describes a created execution; a rejected request receives no fictitious execution or `state: failed`. Response and observation address formats remain to be refined. A lost acceptance response does not mean rejection. A retry uses the same `request_id` under the following rules.

**Accepted: request retry through `request_id`.** The caller supplies `request_id`. It differs from `operation.id`, which the receiving Gecko assigns to the created execution. Calling the same Gecko again with the same `request_id` and request content returns the previously created operation and observation address without creating another execution. This restores the operation association after a lost acceptance response.

| Situation | Behavior |
|---|---|
| Retrying an accepted request with the same `request_id` and content | Return the previously created operation; do not start another execution |
| The same `request_id` with different parameters or request content | Reject as an identifier conflict; leave the previous operation unchanged |
| An intentional new attempt, including after fixing a blocker | Use a new `request_id`; on acceptance Gecko creates a new `operation.id` |

**Accepted: operation identity.** `operation` is an object with these fields:

| `operation` field | Meaning |
|---|---|
| `id` | Identifier of a particular operation execution |
| `type` | Domain operation kind; concrete values belong to the consumer contract |
| `parent_id` | Optional parent-operation reference when the execution belongs to a composite operation |

A request and all progress and result messages of one execution share one `operation.id`. A new run after a fix receives a new identifier. A standalone operation has no parent. Each nested operation has its own identity; `parent_id` and `children` relate executions without implying ownership, dependency, cancellation propagation, or composite atomicity.

The receiving runtime assigns the execution identifier on acceptance. For the first experiment, both identifiers are random UUIDv4 values; the retry key is scoped to the receiving Gecko and trusted caller subject, and request comparison uses the normalized operation type, applicable target, and typed parameters rather than tracing metadata. The precise canonical encoding, production security of this scope, and parent-reference transfer still require validation or later decisions.

**Accepted: minimum Operation API.** Action meaning is defined independently of concrete Zenoh integration keys and mechanisms ([§9](#9-transport-adapter-investigation)).

| Action | Result |
|---|---|
| **Start** | Acceptance with `operation.id` and observation address, or rejection with a reason; retries follow `request_id` rules |
| **Get report** | Latest available full Operation Report for a particular execution by `operation.id` |
| **Observe** | Subscribe to new full snapshots of this operation, ordered by `sequence` |
| **Cancel** | Cancellation acceptance or rejection, current cancellation status on repetition, or the actual outcome of a terminal operation; acceptance alone does not confirm cessation |

Retrieving a report neither creates a new execution, repeats a check, nor resumes an operation. For a running operation, return the latest available snapshot; for a completed one, the final snapshot if available. If the report is unavailable, the API explicitly says so; absence of a report does not prove the operation never ran or is not running. Reading does not change retained results.

Client disconnection does not cancel an accepted operation. Reading or reconnecting neither starts nor resumes domain work.

## 5. Report content

A report belongs to a particular operation and participant. Several messages can progressively expose one execution's progress and result; these are not separate accepted decisions. A composite operation links its report to participant reports, allowing navigation from an overall problem to concrete diagnostics.

| Field | Meaning |
|---|---|
| `operation` | Object with `id`, `type`, and optional `parent_id`; associates a particular execution's request, progress, and result with a parent operation if present |
| `sequence` | Increasing snapshot number within a particular operation; determines update order |
| `reported_at` | Core-side snapshot generation time; used for diagnostics rather than replacing `sequence` for ordering |
| `subject` | Participant and operation subject, including configuration or revision where applicable |
| `context` | Inputs, parameters, dependencies, and App association if the operation relates to an App |
| `state` | Execution state: `pending`, `running`, `succeeded`, `failed`, `cancelled`, `unknown`; does not replace participant readiness or decision application results |
| `progress` | Object with `stage`, `message`, and optional numeric `completed`, `total`, `unit`; exposes the executor's available progress assessment |
| `result` | Data actually obtained and actions performed, including partial results; domain content is defined in the relevant section |
| `acquired_resources` | What preparation acquired, created, or reserved, including before failure; not an estimate of future consumption |
| `blockers` | List of objects with `code`, `subject`, `message`, `evidence`, `next_actions`: obstacles and possible remedies |
| `limitations` | Known limitations and boundaries of the result obtained |
| `explanation` | Substantive explanation of completed work, outcome reasons, and significant context for a person or agent |
| `next_actions` | List of objects with `code`, `message`, `subject`, `instructions`, and optional `request`; recommendations remain separate from performed actions |
| `evidence` | Supporting measurements, Probe results, diagnostic data, and artifacts |
| `children` | Relationships to nested operations and participant reports of a composite operation |
| `details` | Domain information specific to the operation or layer |

Populate applicable report parts. The common core does not require a diagnostic Probe to have an App, create resources, or apply a decision. Significant errors and blockers must be structured; domain details must not be the only place to discover the overall operation outcome.

`acquired_resources` belongs to specific preparation and supports selective or overall explicit domain cleanup. Release results appear separately in `result`: a list of previously acquired resources does not itself confirm cleanup. Using shared or preexisting resources does not mean exclusive ownership. Domain layers define record structures, use/acquisition distinctions, and local release operations. Reporting readiness does not itself imply new resource acquisition.

**Accepted: `next_actions` structure.** The field contains a list of proposed actions. The same structure is used at report level and inside an individual blocker.

| Proposed action field | Meaning |
|---|---|
| `code` | Machine-readable action name; concrete values belong to the domain layer |
| `message` | What is proposed and why |
| `subject` | Participant or resource the action concerns |
| `instructions` | Detailed execution instructions and conditions for a person or agent |
| `request` | Optional prepared API request description when the action is available through Gecko |

For example, installing a missing driver can be described as instructions for an agent using OS tools, without `request`. A subsequent Probe check may contain a prepared Gecko request description. Concrete `request` schemas, API parameters, and condition forms are refined separately.

A proposed action is not an accepted decision, issued command, or execution confirmation. Execution requires a separate call to the relevant tools or API. Starting a new action through Operation Protocol creates a new `request_id` and, on acceptance, a new `operation.id`. Redelivery of one already sent request preserves its `request_id` under duplicate-execution protection rules. External-tool instructions do not themselves create a Gecko operation.

**Accepted: `blockers` structure.** The field is a list of blocking problems, each represented by an object:

| Blocker field | Meaning |
|---|---|
| `code` | Machine-readable reason code, such as `driver_missing`; concrete codes belong to the domain layer |
| `subject` | Device, participant, or dependency the problem concerns |
| `message` | Obstacle explanation understandable to a person and agent |
| `evidence` | Supporting data or diagnostic and related-report references |
| `next_actions` | Possible remedies and their execution conditions |

For example, a Probe reports a missing NVIDIA driver blocker, attaches diagnostics, and instructs an agent to install the driver and then explicitly start a new check. Recommendations themselves neither authorize nor perform remediation; actual actions appear in corresponding reports.

Multiple blockers remain separate list entries. Their subjects may differ from the report's overall `subject`: an Activation report can expose a particular participant's problem. Blocker evidence and recommendations detail the report's common `evidence` and `next_actions` fields without requiring large materials to be copied. Proposed action structure is defined above; exact reference formats, domain identifiers, and API requests remain to be refined. A blocker does not substitute for `state`: a readiness request can successfully return `not_ready` with a reason.

**Accepted: `progress` structure.** Progress is an object rather than a single number from 0 to 1.

| `progress` field | Meaning |
|---|---|
| `stage` | Machine-readable current stage name; concrete names belong to the operation kind |
| `message` | Work description understandable to a person and agent |
| `completed` | Completed amount in the stated units, if measurable |
| `total` | Total amount in the same units, if known |
| `unit` | Measurement unit, such as frames, bytes, or checks |

Numeric indicators are optional. When the amount is unknown, expose stage and description without inventing a percentage. If `completed` and `total` are both present, they refer to the same measured amount and unit; their scope follows the current stage and domain contract. Changing stage need not preserve the previous counter. `sequence`, rather than progress, remains the strictly increasing snapshot number.

A domain executor may expose a composite operation's stage and link child reports through `children`. Overall completion is not automatically computed by averaging participant progress: durations and work amounts differ. If no overall estimate is defined, do not claim numeric progress. Progress does not replace `state`, readiness, or decision application; reaching the stated amount does not itself confirm successful completion.

## 6. Snapshots and Core authority

**Accepted: full Operation Report snapshots.** Every message contains a full snapshot of the particular operation's available state at generation time rather than only changes since the previous message. A recipient can interpret it without replaying all earlier messages of that execution.

Subsequent snapshots retain results obtained, actions performed, acquired-resource information, and applicable blockers and limitations. A state change or refined conclusion appears in a new snapshot; this does not keep a resolved blocker active. Unavailable or unknown facts explicitly remain unknown rather than being reconstructed by the recipient from a missing message. Large artifacts, nested reports, and supporting materials may be references with clear operation ownership.

Completeness means available state, rather than a full event log or every diagnostic file copied into each message. It does not define serialization, storage under one key, latest-value retention, history, or new-subscriber delivery. Zenoh mechanisms and linked-material availability rules are separate. `sequence` determines update order and newest snapshot selection as below.

**Accepted: state and snapshot authority.** The domain executor submits progress, partial results, and execution outcomes through the Executor API. Operation Core validates state transitions, records the accepted operation state, and assigns `sequence` to the resulting full report snapshots. The executor supplies domain facts; Core enforces the common lifecycle rules rather than independently judging domain success. Executor notifications and transport delivery do not bypass this authority. Core is the logical authority for each operation; this decision does not prescribe process boundaries or a concurrency mechanism. Arbitration rules for competing notifications and their validation remain to be specified and tested.

**Accepted: snapshot order and time.** Core increments `sequence` for each new snapshot of one operation. Redelivery of the same snapshot preserves its number. A recipient uses the greatest known `sequence`; a late or duplicate snapshot with a lower or equal number does not replace current state. Different content under one number is not a valid new update.

**Accepted: snapshot numbering survives normal Gecko restart.** Core persists `sequence` with its report snapshot through the Storage API. Republishing the same retained snapshot preserves number and content; a new snapshot of the same operation receives a greater number. The counter does not reset after restart: after snapshot 12, the same 12 may be republished, while a new result clarification receives the next number. Report clarification neither resumes domain execution nor creates a new execution. Concrete snapshot and number persistence and loading mechanisms are defined during integration and must preserve this order.

Each nested operation has its own `operation.id` and counter. The domain executor supplies a composite operation's summary through the Executor API; Core numbers its snapshots separately from child snapshots. Numbers of different operations are not compared; `parent_id` and `children` relate them without a global counter.

`reported_at` is snapshot generation time at Core, rather than executor event time or subscriber receipt time. Order follows `sequence` and requires no synchronized Gecko clocks. Snapshot generation time does not replace domain freshness conditions. Number preservation after normal restart is accepted above; exact time format, initial counter value, and persistence mechanisms remain to be defined.

## 7. Lifecycle and cancellation

**Accepted: execution `state`.** The common core uses these values independently of domain readiness or decision application results.

| `state` | Meaning |
|---|---|
| `pending` | Accepted and recorded, but executor acknowledgement has not established that execution began; this is not a queue claim |
| `running` | The operation is executing |
| `succeeded` | The operation completed successfully |
| `failed` | The attempt is deemed unsuccessful, including due to a blocker requiring an external fix. The overall outcome of an actually composite operation does not replace actual part states |
| `cancelled` | The executor confirmed cessation; a cancellation request alone does not establish this state |
| `unknown` | The observer cannot reliably establish execution state; this does not confirm stopping, failure, or completion |

**Accepted: uncertainty and later confirmation.** `unknown` is not a terminal outcome. It records the lack of reliable knowledge about the same execution; later confirmation may refine its state without changing `operation.id`.

Acceptance first creates `pending` with the initial full snapshot. A confirmed handoff of the same execution changes it to `running`. If handoff may have reached the executor but its acknowledgement is lost, `pending` changes to `unknown` without another dispatch. A proven failure before any side effect can end as `failed` with an infrastructure explanation; an ambiguous handoff cannot be reported as proven failure. Later reliable evidence can refine `unknown` to the state of that same execution. Confirmation evidence and complete notification arbitration remain under ST2–ST3.

| From / to | `running` | `unknown` | `succeeded` | `failed` | `cancelled` |
|---|---|---|---|---|---|
| `pending` | Confirmed handoff | Handoff may have occurred, but execution cannot be established | Confirmed completion, even if acknowledgement was lost | Confirmed failure, including proven failure before side effects | Confirmed cessation caused by cancellation after cancellation is supported |
| `running` | Progress without a state change | Execution can no longer be reliably established | Confirmed success | Confirmed failure | Confirmed cessation as a result of cancellation |
| `unknown` | Executor confirms the same execution continues | Uncertainty remains | Confirmed success | Confirmed failure | Confirmed cessation as a result of cancellation |
| `succeeded` | Forbidden | Forbidden | State unchanged | Forbidden | Forbidden |
| `failed` | Forbidden | Forbidden | Forbidden | State unchanged | Forbidden |
| `cancelled` | Forbidden | Forbidden | Forbidden | Forbidden | State unchanged |

An unchanged state does not itself require a new snapshot. The rules for accepting repeated executor notifications or enriching a terminal report remain open under ST3. Redelivery of an existing snapshot always preserves its content and `sequence`; a new snapshot receives a greater number.

These transitions refine knowledge and do not start or resume work. A timeout alone proves neither failure nor cancellation. `succeeded`, `failed`, and `cancelled` are immutable terminal outcomes. Partial results survive uncertainty; a new clarification uses a full snapshot with a greater `sequence`.

No `blocked` state waiting for resumption is introduced for the local Gecko target. `pending` is accepted as the initial state; the full lifecycle, cancellation races, and notification arbitration remain incomplete; see [ST1–ST5](status.md#21-lifecycle-decisions).

Execution state is distinct from domain conclusions. A successfully performed check can report a negative or unknown domain answer. Nested operation outcomes are not replaced by an overall composite outcome.

**Accepted: execution cancellation.** Cancellation is requested by `operation.id` from the operation executor. It accepts or rejects the cancellation request; acceptance does not yet confirm cessation. `state: cancelled` is published in a full Operation Report only after confirmed cessation. If an operation has already completed, cancellation does not replace its actual outcome. If an action cannot be interrupted safely, cancellation is rejected with an explanation.

Cancellation neither rolls back completed actions nor triggers automatic cleanup. The report preserves actual actions, partial results, and acquired-resource information; cleanup is requested separately under the domain contract. Cancelling observation or a caller-side waiting timeout does not itself cancel execution.

**Accepted: cancellation racing with completion.** Acceptance of a cancellation request does not reserve `cancelled` as the eventual outcome. If execution completes successfully or unsuccessfully before cancellation takes effect, its actual outcome is `succeeded` or `failed`, even when cancellation was already accepted. `cancelled` requires confirmed cessation as a result of cancellation. A late cancellation response cannot rewrite a confirmed terminal outcome. While the outcome is not established, the existing `running` or `unknown` rules apply. This settles outcome semantics; atomic arbitration of competing executor notifications and response formats remain to be specified and tested.

**Accepted: repeated and late Cancel.** Repeating Cancel for the same operation returns the current status of the already requested cancellation rather than creating another stop attempt. If the operation has reached a confirmed terminal state, Cancel returns its actual outcome without issuing another stop attempt or changing that outcome. A repeated response reflects current information rather than necessarily replaying the original response. This does not make cancellation acceptance proof of cessation. Exact response schemas, concurrent-request deduplication, persistence of cancellation accounting, and whether a previously rejected cancellation may be explicitly reconsidered remain to be specified.

**Accepted: incomplete cancellation does not replace actual operation state.** The overall report preserves the operation's own state; cancellation results and part states appear separately. While execution is known to continue and the final outcome is not established, the operation remains `running`; if state cannot be established, expose `unknown`. Confirmed `succeeded` or `failed` is not rewritten as cancellation. A part refusing cancellation does not alone determine the entire operation's outcome: it is considered with actual results and overall outcome rules. `cancelled` requires confirmed cessation of the operation itself and all active parts; an already completed actual outcome is preserved.

Request and response formats, concrete safe interruption points, and acknowledgment waiting periods are refined in domain contracts and integration. A common request mechanism does not require every domain operation to support safe cancellation at every stage.

**Accepted for the local Gecko target: a new run after an external fix.** If execution cannot continue due to a blocker and requires an external action, the current attempt ends with a report of the blocker and actual results, including partial ones. It does not remain an execution waiting to resume. After a fix, a user or agent explicitly creates a new run with a new identity; no automatic continuation or retry occurs. No universal resume-from-saved-step mechanism is introduced for the local Gecko target.

A new run does not imply automatic cleanup of earlier preparation. Use of existing state follows the domain contract; an explicit Probe check executes again. A blocker in data does not always mean the operation itself failed: a readiness request can successfully return `not_ready`. This rule concerns ending an attempt requiring an external fix to continue, without conflating `state` and domain results.

The rule does not permit blindly repeating an action with an unknown result: after partial decision application or a lost response, first establish available actual state. Conditions for a safe new request in such cases remain open.

A parent reference alone does not cause cascading cancellation. Domains define owned active parts and cancellation scope; a composite operation may be confirmed cancelled only after it and its active parts have ceased. The common relationship model and ownership interface remain open.

## 8. Accounting, retention, and recovery

**Accepted: protection against duplicate operation creation survives normal Gecko restart.** The receiving Gecko reliably persists the `request_id` to assigned `operation.id` association, data needed to compare request content, and the accepted operation record before starting domain actions, including sending mutating requests to participants. A normal restart with preserved records does not turn an accepted request into a new one.

After such a restart, retrying with the same `request_id` and content returns the previous `operation.id`, observation address, and available report information; no second execution is created. Retrieving the previous operation does not resume preparation or other domain actions. If the outcome is unknown, Gecko exposes that uncertainty and establishes available actual participant state. A persisted `running` snapshot alone does not confirm execution continues after restart. Domain contracts determine admission of new requests; uncertainty does not imply a global lock.

**Accepted retention boundary for the local Gecko target.** Minimum accepted-request records are not automatically deleted by age or completion, including standalone diagnostic runs. They preserve the request-to-operation mapping and comparison data independently of detailed-report retention. No automatic minimum-record expiry protocol is introduced.

Consumers can impose lifecycle requirements on accounting. Mesh / App permits removal after an App is deleted only while old requests remain permanently inadmissible and its identity is never reused. That consumer exception is not a generic permission to delete deduplication records. The safe release interface and allocation of enforcement responsibility remain open in [decision status](status.md#2-open-decisions); no deletion API is approved here.

**Local Gecko target boundary: automatic recovery of completely lost request records is not provided.** Persisted records are a mandatory part of the receiving Gecko's state. Normal recovery after ordinary restart relies on their preservation. Automatically reconstructing a lost `request_id` → `operation.id` mapping from published reports is outside the local Gecko target.

Participant reports for particular preparation calls or other operations may remain accessible through Mesh independently of local request records. Observing execution does not itself restore the original user-request association. Lost records do not justify repeating an old request with an unknown outcome; complete record loss is outside normal supported recovery.

This protects against duplicate operation creation on retries rather than promising atomic domain actions. Concrete persistence and reload mechanisms, request-content comparison, and concurrent retry handling are defined during integration. They must uphold previous-operation retrieval and changed-content conflict rules. Concurrent retries of one request must not create several executions. Concrete report retention periods follow a separate policy; report unavailability is defined below and snapshot numbering in [§6](#6-snapshots-and-core-authority).

**Accepted: detailed reports and artifacts may be deleted under a separate retention policy.** Indefinite availability of full Operation Reports, diagnostic data, and files is not required. Their retention policy is separate from accepted request records, including standalone diagnostic Probe operations. Deleting a report or artifact does not itself delete the `request_id` to `operation.id` association or permit re-execution.

With preserved records, retrying the original request returns the previous `operation.id` and explicitly states that its report is no longer available; no check or other action automatically runs again. An intentional new execution uses a new request. Domain admission requirements continue to apply independently of retained operation records. Concrete report and artifact retention periods and deletion mechanisms are defined during implementation; they do not change retry protection or snapshot numbering preservation semantics.

“Latest available” does not mean guaranteed fresh readiness or current executor availability: a consumer considers `sequence`, `reported_at`, domain freshness, and presence information. Retrieving a retained report and starting a new check are different API actions.

Detailed report and artifact deletion and an explicit unavailable response are accepted above. Report source, exact unavailable-response formats, concrete retention periods, authority among multiple sources, and read/subscription reconciliation remain integration details validated under the adapter investigation in §9.

## 9. Transport adapter investigation

Gecko selects Zenoh for integration. The first `0.1.0` experiment includes a minimal adapter for Start, Get, and Observe; Core remains transport-neutral. The following mapping candidates and references are unvalidated investigation material, not selected SDK versions or proven guarantees. Their viability is tested in the first experiment, while the broader transport contract is stabilized later.

| Local-version need | Integration candidate and validation subject |
|---|---|
| Request and acceptance response | Query / Queryable; Operation creation and deduplication remain Gecko logic |
| Full report observation | Publisher / Subscriber; ordinary delivery does not establish durable storage |
| Current report retrieval | Queryable or supported storage; select an authoritative source and explicit unavailable response |
| Late connection and resuming observation | Validate built-in history/cache/recovery capabilities of the selected SDK, their limits, and compatibility |
| Persistence after restart | Validate local storage for request records and snapshots; transport cache does not replace reliable persistence |
| Ordering | Preserve domain `sequence` and snapshot selection rules independently of transport numbering |
| Timeout and cancellation | Distinguish response waiting, execution time, and confirmed cancellation |

Initial reference materials: [Zenoh abstractions](https://zenoh.io/docs/manual/abstractions/), [AdvancedSubscriber 1.10.1](https://docs.rs/zenoh-ext/1.10.1/zenoh_ext/struct.AdvancedSubscriber.html), [filesystem backend](https://github.com/eclipse-zenoh/zenoh-backend-filesystem). These are further-research links rather than a mandatory dependency set. SDKs, versions, and guarantees are checked during implementation.

Spike evidence for `0.1.0`: the [ROB-60 report](../research/zenoh-spike/REPORT.md) measures Zenoh `1.10.2` query outcomes, attach races, reconnection, and slow observers on one host. It is investigation material and proposes choices Z1–Z6 in [decision status](status.md#23-choices-raised-by-the-zenoh-spike-rob-60). It does not change the rules above.

## 10. Validation

This is a test outline, not existing tests or experimental evidence.

| Case | Required observation |
|---|---|
| Reject an invalid Start | No operation is created |
| Retry Start after a lost response, including concurrently | One execution; the previous identity is returned |
| Reuse request identity with changed content | Conflict; existing execution unchanged |
| Lose execution knowledge, then obtain reliable confirmation | `running` → `unknown` → confirmed state on the same identity; no work is restarted |
| Lose contact after a terminal outcome | Terminal state unchanged |
| Accept cancellation, then receive actual successful or unsuccessful completion | Preserve the actual outcome; cancellation acceptance does not force `cancelled` |
| Confirm cessation due to cancellation | `cancelled`, preserving partial results |
| Repeat Cancel or call it after completion | No new stop attempt; return current cancellation information or actual terminal outcome |
| Deliver snapshots late or more than once | Lower or equal `sequence` does not replace current state |
| Restart and republish a retained snapshot | Same content and number; a new clarification has a greater number |
| Inject failures around handoff and recording | Expected outcomes to be completed after ST1–ST5 are resolved |

Additional common contract checks:

- A late subscriber, including one attaching during final publication, obtains the available outcome; older snapshots do not replace it.
- Deleting detailed reports and artifacts preserves retry protection and returns explicit report unavailability with the previous operation identity.
- Partial results and structured blockers remain meaningful independently of execution outcome.
- Authorization, audit, and resource-limit checks satisfy the [security gate](security.md#security-gate-for-100).

Domain integration checks (App deletion, readiness, Activation, Stop, shared-resource cleanup, and Probe fact freshness) belong to the consuming contracts. These tests do not establish a queue, cancellation ownership graph, or complete crash-consistency mechanism in advance.

## 11. Versioning

The module version follows SemVer and applies to its public contract, compatible libraries, and conformance tests. The `-dev` suffix denotes an unreleased version under development.

The document version identifies a text revision and does not prove that an implementation exists. Breaking changes are allowed before `1.0.0`, but they must be recorded explicitly in the decision status and migration notes. The stability criteria for `1.0.0` cover not only API shape but also recovery, failure behavior, and [security](security.md).
