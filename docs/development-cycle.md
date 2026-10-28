# Gecko: Development Cycle Progress

Version 0.6 · October 28, 2026

[Cycle and concepts](0-concepts/index.md#12-development-cycle) · [Mesh / App](1-mesh-app/model.md) · [Foundational modules](core-modules.md)

This plan tracks the architectural-level pass for 1.0.0. On the downward pass, refine requirements on lower levels until reaching domain contracts. At the lowest level required by the selected scenario, turn upward: run real experiments, write code, and ascend while refining each level from the results. Directions for 2.0.0 and 3.0.0 retain the longer-term perspective without expanding this pass's obligations.

**Current position:** Mesh / App version boundaries are defined at the model level; the Gecko / Axis downward step is in progress. The [preliminary checks and subsequent client request scenario](2-gecko-axis/scenario.md) is selected, and a [D2 plan by branch and decision](2-gecko-axis/work-plan.md) is prepared; definitions and the responsibility map are next. Domain contracts are not yet complete. The upward pass has not started. This status describes work under this plan, rather than the presence or absence of code throughout the repository.

## 1. Route and Outcomes

Document order establishes the initial investigation route rather than a strict execution hierarchy: Line / Point / Edge / View and Block / Anchor are related through contracts. If deeper work on a runtime, adapter, or transport is needed, add a corresponding step before turning upward. Basic operations need sufficiently concrete contracts to execute and evaluate an experiment.

| Step | Direction and level | Expected outcome | Status |
|---|---|---|---|
| D0 | Goals and concepts | Feature delivery latency definition, version boundaries, and pass rules | Recorded in documents |
| D1 | Down: Mesh / App | 1.0.0 scope, 2.0.0/3.0.0 directions, [requirements on lower levels](1-mesh-app/lower-level-requirements.md), separate return list | Recorded at the model level |
| D2 | Down: [Gecko / Axis](2-gecko-axis/model.md) | Discovery, local operations, facts, checks, and execution requirements | In progress: scenario and [plan](2-gecko-axis/work-plan.md) prepared; block A is next |
| D3 | Down: [Line / Point / Edge / View](4-line-point-edge-view/model.md) | Supported processing composition, interfaces, lifecycle, data, diagnostics, and reactions | Not started in this cycle |
| D4 | Down: [Block / Anchor](3-block-anchor/model.md) and required mechanisms | Domain provider and dependency contracts, ownership, preparation and release; concrete experiment boundaries | Not started in this cycle |
| T | Turn upward at the selected scenario's lowest level | Testable hypotheses, inputs, outcome criteria, and minimum contracts for code selected | Not started |
| U4 | Up: foundational mechanisms and Block / Anchor | Experiments, code, validated contracts, and limitations | Not started |
| U3 | Up: Line / Point / Edge / View | Working local processing and validated usage rules | Not started |
| U2 | Up: Gecko / Axis | Validated discovery, diagnostics, management, and lower-layer integration | Not started |
| U1 | Up: Mesh / App | Refined decisions from section 2, integration, and a validated 1.0.0 end-to-end scenario | Not started; return questions await this step |
| U0 | Goal evaluation | Assessment of the path from a task to verified functionality, identified limitations, and the next pass | Not started |

For each step, retain links to document changes, an experiment or code, the observed result, and limitations. A decision on paper does not imply implementation validation. If an experiment disproves an assumption, correct affected contracts and requirements, record the return in progress, and continue the pass; do not preserve an unsupported promise for the sake of the earlier plan.

Foundational modules with independent contracts are listed in the [catalog](core-modules.md), with dependencies and work status. Their separate design and prototyping may precede completion of the domain downward pass if the module boundary is accepted; integration checks remain part of the corresponding upward steps. Operation is the first accepted module. This parallel work does not itself change D2–D4 or U4–U1 status.

## 2. Remaining Decisions Needed to Make 1.0.0 Specific Enough to Implement

**Separate Mesh / App return list, step U1.** These questions do not currently need to be closed before moving to Gecko / Axis. On the downward pass, pass necessary requirements down; lower layers make their own contracts concrete. On the upward pass, collect validated results and use them to refine Mesh / App decisions. Early observations can be recorded here without declaring a question resolved.

| ID | Return question | Outcome at U1 | Status |
|---|---|---|---|
| R1 | Minimum App schema, revision, and supported operations | Consistent local App composition description, versions, and API | Awaiting the upward pass |
| R2 | Concrete end-to-end scenario capabilities: Points, connections, Blocks, inputs, and result | A validated minimum set and its limitations | Awaiting the upward pass |
| R3 | Rules for changing, stopping, deleting, and cleaning up a local App | Domain transitions, resource ownership, and safe explicit actions | Awaiting the upward pass |
| R4 | Request, report, and identifier formats and API addressing | Typed contracts consistent with lower layers | Awaiting the upward pass |
| R5 | Storage of revisions, execution permissions, request records, and latest reports | Validated persistence, publication, and recovery mechanisms and retention policy | Awaiting the upward pass |
| R6 | Readiness conditions, timeouts, and post-restart behavior | Validated admission, waiting, and recovery conditions for local operations | Awaiting the upward pass |
| R7 | Minimum Zenoh profile and validation of its guarantees | Selected SDKs, keys, settings, and confirmed exchange guarantees | Awaiting the upward pass |
| R8 | Acceptance criteria for the complete local cycle | A reproducible scenario, failure checks, and feature delivery latency evaluation | Awaiting the upward pass |

## 3. Preparing Subsequent Documents

These rules aim to produce a contract sufficient for the next step sooner and reduce repeated text revision. Mesh / App work showed that late version-boundary refinement, revising Build responsibility, and combining several drafts into one document required repeated changes to related sections. Scope and disputed decisions are therefore established before writing detailed descriptions.

### 3.1. Work Order

1. For each level, first select one 1.0.0 end-to-end scenario, the step's outcome, and the criterion for moving to the next level. Record 2.0.0 and 3.0.0 directions separately, without detailing their contracts in this pass.
   A level plan may describe development within 1.x: near-term scope and later extensions at different depths. Assigning each direction a release number in advance is unnecessary; accepted Mesh / App boundaries remain in effect.
2. List questions that actually change the scenario contract. Briefly record alternatives, the accepted rule, its reason, and affected files for each. Only then expand the rule in active documents. An example or proposed schema alone does not constitute an accepted decision.
3. On the downward pass, define inputs, results, states, errors, responsibility, and lower-level requirements specifically enough to choose and evaluate an experiment. Leave storage and transport mechanisms, numeric parameters, and complete schemas until validation if they do not change that contract.
4. Before moving on, perform one end-to-end review of the scenario and consistency with neighboring documents: terms, links, responsibilities, request retries, partial results, and failures. Correct contradictions in active rules; add new questions to the return list with a specified resolution step.
5. On the upward pass, test assumptions through experiments and code. Record the observed result and limitation in the log, then update affected contracts. Maintain documentation in English and update related pages together.

For Gecko / Axis, it is sufficient to describe preliminary and subsequent Axis runs on one Gecko, its state of current Axis work, client connection to facts already available, direct invocation, progress, results, errors, freshness, and safe retries, and to pass execution requirements down. Closing all R1–R8 questions is not required before this transition.

### 3.2. Level File Format

Use a topic folder for each level. Separate active rules and working materials by purpose so that changing one contract does not require rereading history and the entire scenario. Proposed structure:

| File | Purpose |
|---|---|
| `model.md` | Entry point: 1.0.0 status and boundaries, future-version directions, principal entities, and accepted rules. Links to detailed contracts and the scenario. |
| `scenario.md` | One end-to-end scenario: action sequence, observable result, and rule validation. An example introduces no new requirements. |
| `work-plan.md` | Questions and tasks for developing the level, dependencies, and completion criteria. Begins with a progress table for all items under §3.3. |
| `contracts/<interaction>.md` | One coherent interaction contract: participants and responsibility, inputs, result, states, errors, and retries. Create a separate file for a contract needing detailed description; a short rule can remain in the model. |
| `lower-level-requirements.md` | Requirements on the next level with links to rules or the scenario, as in [Mesh / App](1-mesh-app/lower-level-requirements.md). |
| `open-questions.md` | Questions, why answers are needed, resolution step, and status. A short list can remain in the model; deferred items do not become active promises. |
| `history.md` and `archive/` | Revision history, rejected alternatives, temporary plans, and reviews. They do not define the active contract. |

These are file roles, rather than a requirement to create empty files in advance. `model.md` remains the reader's entry point; the [general index](README.md) links to level models. Each active rule has one primary location: the model or a contract file. Other files link to it rather than retelling it. Add tables and diagrams where they clarify the contract.

### 3.3. Tracking Progress

At the start of each level plan, maintain a common progress table covering all items, including open ones. Required columns: **ID, area or question, status, completed work and remaining work**. Keep IDs stable across discussions to connect decisions, history, and tasks. State the update date and nearest next step beside the table. The [Gecko / Axis progress table](2-gecko-axis/work-plan.md#d2-progress) is an example.

Architectural development statuses: `open`, `under discussion`, `partially accepted`, `accepted at the model level`, `deferred until a specified step`. For a partial decision, explicitly identify the accepted part and remaining questions. An item becomes fully accepted only when its entire scope is closed; a deferred item states the reason and return step. Previously accepted rules also apply to open items.

After each accepted decision, scope change, or revision, update the relevant table rows together with active documents. The table gives a concise current state; decision details reside in the model or contract, alternatives and reasons in history. The general cycle log contains significant changes and links to level plans.

A summary may count items by status. These counts describe the state of questions rather than a completion percentage: item sizes differ. Track architectural acceptance, implementation availability, and experimental confirmation separately; on the upward pass, record validation evidence and its limitations.

## 4. Progress Log

| Date | Step | Change and evidence | Next action |
|---|---|---|---|
| 2026-10-23 | D0–D1 | The [concepts](0-concepts/index.md) describe the goal and cycle. [Mesh / App 0.43](1-mesh-app/model.md) is split into local 1.0.0, distributed 2.0.0, and dynamic 3.0.0; persistent Observer belongs to 3.0.0. Model documentation, without experiments or implementation | Continue down to Gecko / Axis; retain R1–R8 until U1 |
| 2026-10-25 | D2 preparation | Work order and active document format recorded in §3; the [Mesh / App model](1-mesh-app/model.md), [requirements down](1-mesh-app/lower-level-requirements.md), and [history](1-mesh-app/history.md) are already separate | Apply the format to Gecko / Axis and continue downward |
| 2026-10-27 | D2 | The [Gecko / Axis scenario](2-gecko-axis/scenario.md) is selected: preliminary Axis chains, partial facts, a late client connection, and an addressed check. [Mesh / App §2.3](1-mesh-app/model.md#23-discovery-and-axis-work) and [Gecko / Axis §3.4](2-gecko-axis/model.md#34-preliminary-axes-and-check-chains) are refined: current Axis work state does not signal discovery completeness. Recorded in documents, without an experiment | Define domain contracts for Gecko state and Axis runs for this scenario |
| 2026-10-27 | D2 plan | An [architectural development plan](2-gecko-axis/work-plan.md) is recorded: branches A–F, decisions and dependencies, scenario variants, 1.x horizon, delegated task requirements, and completion criterion. Writing the plan does not close its listed decisions | Analyze A1–A3: Axis meaning, entities, and responsibility |
| 2026-10-27 | D2: parts of B1, B2, C3, and D2 | [Gecko / Axis model 0.11](2-gecko-axis/model.md#34-preliminary-axes-and-check-chains) accepts shared rules for preliminary and client runs; a scenario variant adds cancellation and an additional chain. The [plan](2-gecko-axis/work-plan.md#41-accepted-parts-of-decisions) distinguishes accepted parts from open questions; reasons are retained in [history](2-gecko-axis/history.md). Model decision without an experiment | Continue A1–A3; refine tree cancellation, contention, and partial results in B–D; validate the shared interface at U2 |
| 2026-10-28 | D2: parts of A3 and B1 | [Gecko / Axis model 0.12](2-gecko-axis/model.md#34-preliminary-axes-and-check-chains) accepts: Axis publishes findings; Gecko's reconnaissance policy selects and starts subsequent checks under applicability rules. The [plan](2-gecko-axis/work-plan.md#41-accepted-parts-of-decisions) and [history](2-gecko-axis/history.md#2026-10-28-gecko-policy-selects-subsequent-checks) reflect responsibility boundaries, three considered approaches, and open details. Model decision without an experiment | Refine applicability condition form and run relationships; validate starting a subsequent check from a partial finding at U2 |
| 2026-10-28 | Cycle rules | §3.3 requires a progress table at the start of each level plan: all items, statuses, accepted parts and remaining questions, updated with decisions. Applied in the [Gecko / Axis plan](2-gecko-axis/work-plan.md#d2-progress) | Maintain tables as decisions proceed and apply the rule to subsequent levels |
| 2026-10-28 | Operation foundational module | A [foundational module catalog](core-modules.md) is created; Operation is defined as an independent in-project implementation with ROS 2 Actions and Google LRO as references. Active rules remain in [Mesh / App §8.2](1-mesh-app/model.md#82-operation-report-core). Separate development is allowed before D2 completion; implementation and experiments have not started | Prepare a module contract and prototype with a test executor; validate integration on the upward pass |

The earlier [temporary Mesh / App plan](1-mesh-app/archive/work-plan.md) and [revision 0.42 review](1-mesh-app/archive/review-2026-10-22.md) reflect the preceding discussion stage. Their statuses do not imply completion of the current cycle or readiness to implement 1.0.0.
