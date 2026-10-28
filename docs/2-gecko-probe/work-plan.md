# Gecko / Probe: Architectural Development Plan

October 28, 2026 · step D2 · status: A1 accepted; parts of A3, B1, B2, C3, D2, and F3 accepted; other decisions open

[Model](model.md) · [Scenario](scenario.md) · [Mesh / App requirements](../1-mesh-app/lower-level-requirements.md) · [Development cycle](../development-cycle.md)

## D2 Progress

Updated October 28, 2026. Of 21 plan items, **1 is accepted at the model level, 6 are partially accepted, and 14 are open**. This tracks architectural development; implementation and experimental validation are recorded on the upward pass.

“Partially accepted” means part of a decision is already established in the model and remaining questions are listed in [§4.1](#41-accepted-parts-of-decisions). “Open” means the item is not yet fully developed and no separate decision for it is recorded in this plan; previously accepted model rules and Mesh / App requirements still apply.

| ID | Area | Status | Completed work and next work |
|---|---|---|---|
| A1 | Probe and Capability | Accepted at the model level | Separate diagnostic tasks from environment possibilities; definitions, examples, and references reconciled. Detailed Capability contracts remain in A2, C1–C2, and D1 |
| A2 | Entities and their lifecycle | Open | Extend the accepted Probe/run/Capability distinction into a complete entity map, identifiers, and lifecycles |
| A3 | Component responsibility | Partially accepted | Probe checks and publishes findings; Gecko policy selects subsequent checks; common Operation mechanics are a foundational module. Allocate remaining executor and adapter responsibilities |
| B1 | Run selection and relationships | Partially accepted | Policy based on applicability rules selected. Initial checks, condition form, run relationships, limits, and protection against cycles and repeats remain |
| B2 | Common execution contract | Partially accepted | Preliminary and client runs have equal standing. Refine combinations of results, errors, blockers, and cancellation |
| B3 | Concurrency and resources | Open | Define permitted concurrency, waiting or rejection, and load limits |
| B4 | Gecko shutdown and restart | Open | Specify cessation, resuming observation, and establishing actual run and resource state |
| C1 | Facts and environment picture | Open | Define a fact contract and responsibility for publishing and updating the environment picture and advertised Capabilities |
| C2 | Fact applicability and staleness | Open | Define result comparison, replacement, conflicts, and effects on Capability availability when conditions change |
| C3 | Gecko state | Partially accepted | Reflects all current Probe work. Refine waiting, child work, and uncertainty |
| C4 | Result retention | Open | Separate retention periods and responsibilities for facts, reports, artifacts, and cache |
| D1 | Probe catalog and Capability descriptions | Open | Define client descriptions of checks, subjects, and parameters, considering applicability conditions; distinguish the Probe catalog from advertised Capabilities |
| D2 | Run invocation and management | Partially accepted | Preliminary runs use the common management interface. Complete the acceptance, reading, observation, cancellation, and retry profile |
| D3 | Late connection and reconnection | Open | Define reconciliation of reading and subscribing, snapshot order, and disclosure of gaps |
| E1 | Minimum check set | Open | Select domain profiles for discovery, access or environment, and load |
| E2 | Lower-level contracts | Open | Define executor and adapter operations, resource ownership, and release |
| E3 | Information for planning and readiness | Open | Refine the boundary between Probe measurements and composition readiness conclusions |
| F1 | Near-term scope and 1.x horizon | Open | Consider §5 candidates and define obligations and directions |
| F2 | End-to-end architecture review | Open | Walk through the scenario and variants using agreed A–E contracts |
| F3 | Future implementation tasks | Partially accepted | Operation is in the [foundational module catalog](../core-modules.md) and may be developed separately; other work packages and validation criteria remain to be defined |
| F4 | Passing requirements down | Open | Consolidate active rules and prepare D3–D4 requirements |

After every accepted decision, update the corresponding row and §4.1 details. Preserve reasons and alternatives in [history](history.md); active rules belong in the model or contract. The next planned block is **A2–A3: entity lifecycles and responsibility map**, using the accepted A1 distinction.

## 1. Goal and Depth of Work

D2 produces an agreed Gecko / Probe architecture: entities, responsibility, interaction boundaries, required behavior, and extension directions. It must make clear which contracts to pass to lower levels, which assumptions to test on the upward pass, and how to divide future implementation into small agent tasks.

Develop the entire layer in terms of responsibilities and relationships. For the nearest local scenario, define inputs, results, states, errors, and retries in enough detail that independent implementations agree on contract meaning. For subsequent 1.x, record the need, possible extension, and affected boundaries. Assign future release numbers after selecting scope; Mesh / App 2.0.0 and 3.0.0 directions remain the project's broader horizon.

At this pass, evidence consists of scenario and contract consistency. Performance, transport guarantees, and recovery guarantees are validated by experiments after descending. Each significant assumption receives a validation method and return step. Language, SDK, storage, exact keys, numeric limits, and the full message set need selection now only if they change an architectural boundary.

This file defines work order. Active rules reside in the model and domain contracts. A question or alternative in the plan does not itself become an accepted decision.

## 2. Basis and Scenarios

The basis is already selected: [Gecko starts → Probes publish findings → Gecko policy starts subsequent checks → a client connects → it starts an addressed check](scenario.md). Gecko's state exposes current Probe work; absence of work does not imply complete information. This plan introduces no mandatory Init/Postinit boundary.

One main scenario is sufficient to begin. Review it with short variants touching different boundaries:

| Variant | Question tested | Branches |
|---|---|---|
| A client connects during work or after completion | How to obtain available facts and continue observing | C, D |
| A Probe ends with a blocker and partial results | How to distinguish execution, check outcome, and Gecko state | B, C |
| The acceptance response is lost; the request is retried | How to find the previous run without a second execution | B, D |
| Conditions change; a new check is performed | How to distinguish old and new facts and their applicability | C, D |
| Preliminary and addressed checks compete for one resource | How run admissibility is determined and conflicts exposed | B, E |
| Cancellation is requested or Gecko restarts | What is known about actual execution, results, and resources | B, C, D, E |

Add variants to the scenario as corresponding rules are accepted. A new independent scenario is needed when a different responsibility or interaction boundary cannot be tested by these variants. A list of devices or error combinations alone does not require separate scenarios.

## 3. Branches and Decision Blocks

A–F denote architectural development branches. They define work and possible delegation areas without creating Git branches at this step. The following tables define the full question scope; accepted parts appear in §4.1. Other parts remain open, with existing accepted rules as constraints.

### A. Entities and Responsibility

**Goal:** obtain a common vocabulary and responsibility map for other branches.

| ID | Decision needed | Outcome |
|---|---|---|
| A1 | Accepted: Probe is a diagnostic task; Capability is an environment execution possibility; Probe runs obtain supporting or refuting facts without a required one-to-one mapping | Distinct definitions and corrected model and glossary examples; active rule in model §2.1 |
| A2 | Boundaries of Gecko, an available Probe, its implementation, an individual run, a check subject, a result, and an advertised Capability | Entity map, identifier ownership, and lifecycle responsibility |
| A3 | Which responsibilities remain in Gecko, belong to Probe, or belong to lower-level executors and adapters | Logical component and interaction map; future task boundaries |

**Dependency:** initial Mesh / App requirements. **Done when:** every scenario participant has clear responsibility and the same words do not denote incompatible entities. Concrete process and package division can be refined later.

### B. Probe Runs and Execution

**Goal:** define how work starts and ends, including preliminary check chains.

| ID | Decision needed | Outcome |
|---|---|---|
| B1 | How Gecko policy selects initial and subsequent Probes from available results and diagnostic context | Run, context transfer, and run relationship rules; protection against infinite chains and unintended repeats |
| B2 | How preliminary and client runs share a contract; how results, errors, blockers, and cancellation combine | Execution contract applying Operation Report; a negative domain answer is distinct from an execution error |
| B3 | Which concurrent runs are allowed; how busy resources and heavy checks are handled; what current work means during waiting or a child run | Admission and observable state rules; queue or rejection choice where it changes client behavior |
| B4 | What happens to runs and their resources during Gecko shutdown and normal restart | Cessation and recovery boundary, unknown outcome, and responsibilities for establishing actual state |

**Dependency:** A. **Done when:** each scenario variant clearly identifies who accepts the request, executes the work, and establishes its outcome. Mesh / App already requires protection against repeating accepted requests after normal restart; the persistence mechanism remains to be validated.

### C. Facts and Gecko State

**Goal:** define what a client knows about the environment and current work, including after a Probe completes.

| ID | Decision needed | Outcome |
|---|---|---|
| C1 | How individual run results form the available environment picture and provide evidence for advertised Capabilities; who publishes and updates these | Fact contract: subject, source, run association, conditions, limitations, and Capability evidence references |
| C2 | How results of different checks on one subject compare; what happens with changed conditions, conflicts, or staleness, including effects on Capability availability | Fact applicability and replacement rules; unknown, negative, and stale results distinguished; Capability revision or withdrawal rules |
| C3 | How Gecko state reflects current Probe work and uncertainty without becoming a discovery completeness assessment | Gecko state semantics and relation to run reports; consistency with B3–B4 |
| C4 | What a client can obtain after run completion, report deletion, or restart; how facts, reports, artifacts, and cache differ | Retention and explicit unavailability obligations; artifact ownership and reuse boundaries |

**Dependency:** A; C3–C4 require B decisions. **Done when:** a late client can interpret available state and the origin and limitations of facts. An explicit new check request executes again under the active model rule; useful cache does not substitute for that run.

### D. Client API and Observation

**Goal:** connect discovery of available actions, invocation, and result retrieval into an understandable client contract.

| ID | Decision needed | Outcome |
|---|---|---|
| D1 | How a client discovers available Probes, applicable subjects, accepted parameters, and advertised Capabilities as distinct descriptions | Probe catalog and invocation semantics; Capability descriptions and their evidence references |
| D2 | How request, acceptance, run identity, report reading, observation, and cancellation relate | A domain profile of the common operation protocol, including `request_id` conflict and unavailable reports |
| D3 | How a client obtains current state and continues observation on late connection and reconnection | Requirements for read/subscription reconciliation, snapshot order, and gap disclosure |

**Dependency:** A, B, C. **Done when:** client actions and the meaning of each response can be described without selecting concrete Zenoh keys. Use the Mesh / App Operation Report core; transport guarantees are passed to U2/U1 for validation.

### E. Domain Checks and Lower-Level Boundaries

**Goal:** connect the model to a small real check set and prepare the next downward step.

| ID | Decision needed | Outcome |
|---|---|---|
| E1 | Which minimum check set demonstrates discovery, access or environment validation, and concrete load | A small set of domain profiles with inputs, results, and criteria; diagnostic load and operational execution boundaries |
| E2 | Which operations Probe obtains from runtime, adapters, Line / Point / Edge / View, and Block / Anchor; who owns temporary resources | Requirements down: execution, observation, errors, cancellation, release, and sharing |
| E3 | What Probe information goes to planning and readiness checks, and which conclusions remain theirs | Explicit boundary between measurement, result applicability, and readiness of the selected composition |

**Dependency:** A; domain profiles are refined with B and C, final handoff follows D. **Done when:** lower levels have concrete requirements and future experiment criteria. The full Point, Block, and end-to-end App set is selected at D3–D4; D2 need not settle their architecture in advance.

### F. Reconciliation, Evolution, and Implementation Tasks

**Goal:** combine branch results into a consistent architecture with a clear continuation.

| ID | Work needed | Outcome |
|---|---|---|
| F1 | Reconcile near-term and future 1.x scope under §5 | Obligations, directions, and extension points |
| F2 | Walk through the main scenario and §2 variants; check neighboring documents | Corrected contradictions and return questions with assigned steps |
| F3 | Divide future implementation along agreed boundaries | Work packages with input contracts, dependencies, outcomes, and validation methods |
| F4 | Consolidate active rules and pass requirements down | Updated model, necessary contracts, D3–D4 requirements, and a development cycle entry |

**Dependency:** A–E. **Done when:** §7 criteria are met. Prepared implementation tasks are refined on the upward pass after lower-level experiments.

## 4. Order and Decision Recording

1. Continue A2–A3 using accepted A1: distinguish Probe, Probe run, Capability, and fact, then define entity lifecycles and responsibility.
2. Develop B and C, reconciling Gecko state and run lifecycle. E1 domain check candidates can be prepared alongside this work; the final set depends on accepted boundaries.
3. Use B and C to define D client interactions and complete E requirements.
4. Perform F: review the scenario and 1.x perspective, and obtain tasks for further descent and the upward pass.

The plan permits starting without answering every question in advance. A dependent block is not complete while a decision material to it remains open. For a new question, first determine whether it changes responsibility or observable behavior; direct mechanism details to the relevant experiment.

A short decision record is sufficient: **ID → question → significant alternatives → accepted rule → reason → affected documents → open hypothesis and validation step**. Keep alternatives and reasons in history, active rules in the model or contract. Work statuses: `open`, `under discussion`, `partially accepted`, `accepted at the model level`, `deferred until a specified step`. `partially accepted` requires the accepted part and remaining questions. All items' current state appears in the [progress table](#d2-progress). Code validation is recorded separately on the upward pass.

### 4.1. Accepted Parts of Decisions

On October 27–28, 2026, rules for [ordinary preliminary runs and Gecko reconnaissance policy](model.md#34-preliminary-probes-and-check-chains) were accepted at the model level. On October 28, the [Probe/Capability distinction](model.md#21-capability-and-probe) was also accepted as A1. Reasons and considered alternatives are in [decision history](history.md). A partial decision does not close the entire block.

| ID | Accepted at the model level | Remaining open questions |
|---|---|---|
| A1 | Probe is a diagnostic task, Capability is an environment execution possibility, and Probe run is an execution through Operation; results can support or refute availability without a required one-to-one mapping; vocabulary is reconciled | None within the terminology decision. Identifiers and lifecycles remain in A2, evidence and publication/freshness rules in C1–C2, and client descriptions in D1 |
| A3 | Probe performs a domain check and publishes findings; Gecko's reconnaissance policy selects and starts subsequent checks; common Operation mechanics are outside Probe domain logic | Remaining responsibility map for Gecko, Probe, executors, and adapters |
| B1 | Gecko startup may automatically initiate ordinary Probe chains; available Probes describe necessary facts and applicability conditions; as results appear, Gecko policy selects subsequent checks considering these conditions and diagnostic context; Probes do not invoke each other directly; run origin is preserved | Initial Probe selection, concrete applicability condition form, run relationships, diagnostic depth and load, protection against infinite chains and repeats |
| B2 | Preliminary and client runs share an execution, observation, results, and cancellation contract; a user can request cancellation of a preliminary check and start additional chains | Complete contract for results, errors, blockers, and cancellation together, cancellation propagation to child runs, and partial results after cancellation |
| C3 | Gecko state reflects current Probe work regardless of initiator; cancelling and adding checks change current work without a separate preliminary-check completion stage | Exact waiting, child work, and uncertainty semantics; consistency with B3–B4 |
| D2 | Preliminary runs are accessible through the common run management interface | Full acceptance, identity, reading, observation, and cancellation profile, `request_id` conflict, and unavailable reports |

Scenario review: [a finding satisfies the next check's conditions, and the user changes current work](scenario.md). At U2, validate in code that Gecko policy can start an applicable check from a partial Probe result, preliminary runs use the common observation and cancellation interface, and client-added chains appear in Gecko state under the same rules. This is a model decision without an experiment so far.

## 5. The 1.x Horizon

This table provides design perspective without approving future features or release order. During F1, each candidate receives a decision: include in near-term scope, retain as a direction, or exclude with a reason.

| Scope | Under consideration | Boundaries to consider now |
|---|---|---|
| Near-term core | One Gecko, preliminary chains, current Probe work, partial facts, late client, addressed invocation, and minimum domain checks | Probe/run/Capability distinction, fact/source association, common protocol, resource conflicts, and normal restart |
| Subsequent 1.x: new checks | New Devices, environments, adapters, and load methods; Probes with new parameters and results | Catalog, domain profiles, and client compatibility. Loading external code and plugins requires a separate decision |
| Subsequent 1.x: fact updates | More convenient addressed rechecks, staleness handling, and changed environment conditions | Fact origin and result applicability. Automatic triggers and periodic checks are separate candidates |
| Subsequent 1.x: diagnostic work management | Richer priorities, load limits, composite check observation, and artifact handling | Admission contract, resource ownership, and independence of domain reports from execution mechanisms |
| Longer-term Mesh horizon | Multiple Gecko instances and dynamically changing environments under Mesh / App | Addresses and facts belong to a particular Gecko; the local model does not promise distributed Probe execution |

Mandatory current protocol properties, including safe retries and behavior after normal restart, remain in the near-term core. 1.x extensions do not justify deferring already accepted obligations. Persistent Observer and automatic App restructuring retain Mesh / App version boundaries.

## 6. Documents and Future Delegation

After development, the model contains an entity and responsibility map, concise accepted rules, and links. Move a detailed contract to a separate file when its size justifies it. Expected contract areas: Probe execution, facts and Gecko state, and client interactions. Create files as content develops; format requirements down and return questions under the [cycle rules](../development-cycle.md#32-level-file-format).

The first approved foundational module is [Operation](../core-modules.md), shared by Probe and App operations. Separate design and prototyping can start before D2 is complete. Possible domain implementation packages follow the branches but are refined after A3: Probe description and catalog; Probe integration with Operation and run relationships; facts and Gecko state; API and observation; domain Probes and adapters; integration checks. These are responsibility-area candidates rather than approved code packages.

Each coding-agent task must contain:

- a link to the agreed contract and dependent decisions;
- a concrete outcome and change scope;
- inputs and outputs connecting it to other tasks;
- dependencies and integration order;
- an observable acceptance criterion and significant failure checks;
- hypotheses the agent must test and feed back into the architecture.

Independent areas can be delegated after their interfaces are agreed. If several tasks must still invent their shared interface independently, the delegation boundary is not ready. Actual agent assignment and branch creation occur when moving to those tasks.

## 7. D2 Completion Criterion

- Gecko / Probe entities, responsibility, and boundaries are unambiguous.
- Accepted contracts explain the main scenario and significant variants, including partial results, retries, cancellation, and normal restart.
- Every A–E decision has an accepted rule or justified deferral with a validation step. An open question changing the next level's contract is not hidden as an implementation detail.
- Near-term scope is defined and 1.x directions considered; future capabilities do not implicitly expand current obligations.
- Concrete requirements are passed to lower levels, with testable hypotheses and implementation packages prepared for the upward pass.
- The model, scenario, and Mesh / App requirements agree; D2 is marked complete at the design level. Experiments and code continue through the common cycle after descent.

**Current next step: A2–A3 — entity lifecycles and responsibility map, using accepted A1.**
