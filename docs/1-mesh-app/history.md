# Mesh / App: Revision History

[Active model](model.md).

The entries below preserve discussion history, including superseded decisions and former filenames. The main text defines active rules; references here do not reinstate earlier Build Plan, global App locking, or policy modes.

Earlier diagnostic-task references are normalized to Probe, the term adopted on October 28, 2026; they were called Axis / Axes when those decisions were made. This terminology update does not change the dates or scope of earlier decisions.

### From Version 0.46 to 0.47

October 28, 2026. The former Axis term is split into Probe for a diagnostic task and Capability for an environment execution possibility. §2 distinguishes their declarations and the facts obtained by Probe runs; detailed contracts remain in Gecko / Probe. Links and operation examples use the new vocabulary. The run-relationship example follows the already accepted rule that Gecko policy initiates subsequent checks and Probes do not invoke each other directly. Operation mechanics and implementation version boundaries are unchanged.

### From Version 0.45 to 0.46

October 28, 2026. Operation is accepted as an independent project foundational module shared by Probe and App operations. Alternatives were mandatory ROS 2 Actions as a foundation or an in-project implementation based on existing architectural references. The in-project implementation was selected: ROS 2 does not become a dependency; [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151) inform design. §8.2 obligations, including safe retries and recovery after normal restart, remain. A [foundational module catalog](../core-modules.md) is created; separate Operation design and prototyping may begin before D2 completion. Process division, language, storage, and transport profile are not selected.

### From Version 0.44 to 0.45

October 27, 2026. D2 refines §2.3: Init and Postinit remain descriptive concepts; the mandatory signal for completion of “necessary fact collection” is removed. Gecko exposes current Probe work and partial results; no active work does not prove completeness, and a Probe ending with a blocker does not remain running. Gecko / Probe requirements are updated and the [preliminary checks followed by a client request scenario](../2-gecko-probe/scenario.md) is selected. Earlier wording below is history rather than the active rule.

### From Version 0.43 to 0.44

October 25, 2026. [Requirements on Gecko / Probe, Line / Point / Edge / View, and Block / Anchor](lower-level-requirements.md) are collected in a separate document with expected upward-pass validation. Accepted model rules and version boundaries are unchanged.

### From Version 0.42 to 0.43

October 23, 2026. The document is divided by implementation version: 1.0.0 is a complete local cycle on one Gecko, 2.0.0 outlines multi-Gecko interaction, and 3.0.0 covers dynamic restructuring and product operation. Persistent Observer moves to 3.0.0; 1.0.0 retains current reports and subscriptions. Detailed earlier distributed rules are no longer first-version obligations or preapproved 2.0.0 contracts. Local identity, revisions, Planner-Driven Build, readiness, explicit Activation/Stop/cleanup, and Operation Protocol semantics are retained. A Point hub is a possible 2.0.0 direction. Concepts define feature delivery latency and the downward requirements/upward experiments-and-code cycle; a [separate plan](../development-cycle.md) tracks progress and Mesh / App return questions. Their implementation contracts are refined on the upward pass.

### From Version 0.41 to 0.42

October 22, 2026. An editorial review fixes links, the Zenoh expression sketch, outdated wording about open states, mandatory role assignment, and Init blocker status. New check requests are distinguished from `request_id` retries; optional Block provider management and remaining report questions are clarified. Rust API integration-map links use the stated 1.10.1 version. The [review analysis](archive/review-2026-10-22.md) lists only conflicts between outdated wording and accepted decisions. Related document links and dependent wording are reconciled with the main text; no new architectural requirements are introduced.

### From Version 0.40 to 0.41

October 21, 2026. §6.5 establishes domain data access through participant contracts exposed by an App, shared by Views, agents, and programmatic consumers. Connecting a View does not itself change execution. Domain data is separate from Operation Report and result availability from Activation success; combined data must be associable with the corresponding frame. Concrete formats and access methods remain in Line / Point / Edge / View. Plan group 8 closes at the model level; the end-to-end scenario and review are next.

### From Version 0.39 to 0.40

October 20, 2026. §2.3 establishes Postinit scope as the selected Gecko set known to the Planner at assessment time. New nodes do not reset previous results; a blocker or unavailable participant prevents overall Postinit for that set without preventing use of available facts and other participants. Postinit is not mandatory for Build or Activation; separate readiness validation remains. Work-plan group 7 closes at the model level; domain checks and formats remain in lower layers.

### From Version 0.38 to 0.39

October 20, 2026. §6.5 introduces explicit Stop App: the creating Gecko coordinates publication of a decision withdrawing execution permission and obtains participant stop acknowledgments. Success requires every mandatory participant; unavailable parts remain unconfirmed. Stop is separate from operation cancellation and overall cleanup; a Block shared by other consumers is not automatically stopped. Another operational start requires a new explicit Activation. Coordinator responsibilities extend to Stop; plan group 4 closes at the model level.

### From Version 0.37 to 0.38

October 20, 2026. §6.5 establishes preparation of the next revision during current execution: it does not itself change or restart running parts. If concurrent preparation is impossible, a participant exposes the stop requirement and affected part; the Planner chooses an explicit stop or another preparation option. Arbitrary parallel preparation is not guaranteed. Plan group 4 is updated; explicit stopping remains its next question.

### From Version 0.36 to 0.37

October 20, 2026. Remaining group 5 rules are established: incomplete cancellation preserves actual operation state and separate part results; `cancelled` requires confirmed cessation of the whole operation and active parts, without rewriting completed success or failure (§8.2). After partial cleanup, remaining scope is released by a new explicit request with a new `request_id`; an old retry returns the previous operation (§7.3). Plan group 5 closes at the model level; group 4 is next.

### From Version 0.35 to 0.36

October 20, 2026. §7.3 establishes overall cleanup outcomes for differing participant results: confirmed failure of a mandatory part gives `failed`, complete requested release gives `succeeded`, continuing actions and waiting give `running`, and an unresolved outcome without confirmed failure gives `unknown`. The overall outcome neither replaces participant states nor confirms remaining actions stopped. Plan group 5 is updated; cancellation and further explicit actions remain separate questions.

### From Version 0.34 to 0.35

October 19, 2026. First-version concurrent operation boundaries are established (§4.1–4.2): Authority atomically checks revision, participants determine admissibility and concurrency, and Coordinator participates in Activation and explicit overall cleanup. It does not manage all Planner calls; no universal App lock or common call-reconciliation mechanism is introduced. Concrete resource and locking rules remain in domain contracts. The resolved question is removed; work-plan group 3 closes at the model level.

### From Version 0.33 to 0.34

October 19, 2026. The first version establishes no automatic deletion of minimum records of other accepted requests, including standalone Probes (§8.2). Operation identity and request comparison data persist; reports and artifacts may be deleted separately. The deleted-App exception remains. Concurrent retries are clarified as providing the same one-execution guarantee. Resolved minimum-record retention questions are removed; plan group 2 closes at the model level. Concrete mechanisms and detailed report retention periods remain integration work.

### From Version 0.32 to 0.33

October 19, 2026. A separate detailed report and artifact retention policy is established for operations, including standalone Probes (§8.2). Indefinite availability is unnecessary; deletion does not automatically delete request records or restart the operation. With preserved records a retry returns the previous `operation.id` and explicit report unavailability. Concrete retention periods belong to implementation; retention of other accepted-request records remains in plan group 2.

### From Version 0.31 to 0.32

October 19, 2026. Post-deletion App rules are accepted: identity is not reused, and requests to the former App are rejected without recreation or new execution (§3.2). Deleted-App request records may be removed; while the App exists, age-based deletion of mutating-request records remains prohibited (§8.2). This rule does not define App deletion or resource cleanup procedures. Plan group 2 and integration scenarios are refined.

### From Version 0.30 to 0.31

October 19, 2026. §8.2 establishes Operation Report snapshot numbering preservation after normal Gecko restart. `sequence` is persisted with the snapshot; repeating it preserves number and content, while a new snapshot receives a greater number. Report clarification does not resume the operation. Open questions, integration requirements, and temporary-plan group 2 are updated.

### From Version 0.29 to 0.30

October 19, 2026. The success criterion for accepting a new App description is established (§4.2): a revision is accepted after reliable local persistence; the operation receives `succeeded` after confirmed publication. Success does not confirm actual participant application. The resolved question is removed; temporary-plan group 1 closes at the model level. Concrete persistence, publication confirmation, and permission freshness mechanisms remain integration work.

### From Version 0.28 to 0.29

October 19, 2026. Returning Gecko executor behavior is accepted (§7.3): before a new start after restart, it obtains current execution permission from the shared App namespace; previous local Activation is insufficient. If permission cannot be established, processing does not restart and unconfirmed state is exposed. Permission is separate from the latest accepted description revision; this does not stop continuing execution merely on a communication interruption. Related Activation provisions, an integration scenario, and work-plan group 1 are updated.

### From Version 0.27 to 0.28

October 18, 2026. Activation behavior after Coordinator restart is established (§6.4): a confirmed unpublished decision requires a new explicit attempt and readiness poll; a published decision resumes application observation; an unknown outcome exposes uncertainty and establishes state without blindly granting permission again. Accepted App description publication recovery does not automatically extend to Activation. Publication confirmation mechanisms and other partial activation questions remain open.

### From Version 0.26 to 0.27

October 18, 2026. Group 5 establishes that the Planner assesses combined preparation from participant structured reports and readiness and may produce a user summary. No mandatory Coordinator Build Report is introduced. §6.1 removes overall Build execution, system state, and Coordinator step-management rules. Dependent responsibility, Block preparation, readiness, and operation protocol wording is reconciled; the pre-Activation poll and explicit overall cleanup remain separate contracts. The work plan marks reconciliation of groups 6, 3, and 5 complete; other group questions remain open.

### From Version 0.25 to 0.26

October 18, 2026. The universal one-mutating-operation-per-App rule is abandoned, along with overall busy state, rejection on that basis, and waiting for all earlier actions to finish (§4.2). The Planner selects calls; the receiver checks each request under its contract. The system need not repair contradictory Planner intentions. Authority's atomic revision check and reliable completion information remain. Dependent references and the current work-plan step are updated.

### From Version 0.24 to 0.25

October 18, 2026. Build meaning in Planner-Driven Build is recorded: preparation through separate Planner calls to participants, rather than a separate Coordinator operation with its own identifier and mandatory overall report (§6.1). The work plan records group 6 acceptance and limits the current reconciliation step: regulated-operation boundaries in group 3, need for summary assessment in group 5, then dependent text correction.

### From Version 0.23 to 0.24

October 18, 2026. §6.1 records Planner-Driven Build as the foundation: the Planner directly calls participants and selects subsequent actions, while Coordinator regulates App operations. A separate Build Plan entity and Coordinator-Orchestrated Build are rejected. Subsequent rules and dependent sections are marked for reconciliation with this foundation; they are not fully rewritten at this step.

### From Version 0.22 to 0.23

October 18, 2026. Successful Build criteria are accepted: mandatory Build Plan actions completed successfully, mandatory participant readiness confirmed for the selected revision and configuration and current at completion (§6.1). A successful readiness request with `not_ready` does not mean Build success; an optional check failure alone does not fail Build and remains a limitation. Activation remains separate with a fresh readiness poll. Open questions are narrowed and the temporary plan updated.

### From Version 0.21 to 0.22

October 18, 2026. First-version behavior after confirmed Build failure is accepted: Coordinator starts no further preparation actions, lets started actions finish without automatic cancellation, and collects results (§6.1). The report distinguishes actions never started because of failure from actual states of started actions. Preparation is neither automatically rolled back nor cleaned up; cleanup remains explicit. Open questions and the temporary plan are refined.

### From Version 0.20 to 0.21

October 17, 2026. Build establishes: confirmed failure of a mandatory part determining an unsuccessful attempt gives overall `failed`, even while other actions run. Without such confirmed failure, an outcome that cannot be established after waiting gives `unknown`. The report retains actual child action states; the App becomes available only after their completion is confirmed (§6.1, §4.2). `failed` meaning, open questions, and the temporary plan are refined.

### From Version 0.19 to 0.20

October 17, 2026. The first-version boundary is selected: persisted request records are mandatory for normal restart recovery; automatic recovery of completely lost records is not provided (§8.2). Participant report availability in Mesh is separate from preserving the user-request/operation association; record loss does not permit re-executing an old request with an unknown outcome. Open questions and integration test boundaries are refined.

### From Version 0.18 to 0.19

October 17, 2026. The first version accepts retaining accepted mutating-request records throughout an App's existence without automatic age-based deletion (§8.2). Detailed reports and artifacts may be stored separately; a retry with an unavailable report returns the former `operation.id` without another execution. Open questions and an integration scenario are refined.

### From Version 0.17 to 0.18

October 17, 2026. The `request_id`/operation association and request comparison data are persisted before domain actions. After normal Gecko restart, retrying an accepted request returns the previous operation without a second execution or resumption; an unknown outcome is explicit and does not free the App (§8.2). Open questions and integration scenarios are refined. Storage loss, record expiry, report retention periods, and `sequence` recovery remain open.

### From Version 0.16 to 0.17

October 17, 2026. A new App description revision is accepted after reliable local persistence at its creating Gecko. After interrupted publication and restart, the same Gecko publishes the persisted revision and decision with their previous identity, without another user request. Description acceptance, publication, and actual application are separate (§4.2, §7.1). Storage mechanisms, publication confirmation, and other decision rules remain open.

### From Version 0.15 to 0.16

October 17, 2026. Conflicting mutating requests are rejected without a queue; the next operation is admitted only after confirming all previous actions, including children, completed (§4.2). Timeout, unknown outcome, and restart do not themselves free the App. Concrete confirmation and recovery mechanisms remain open.

### From Version 0.14 to 0.15

October 17, 2026. Sequential execution of mutating operations on one App is accepted for the first version (§4.2). Boundaries for different revisions, child actions, reads, and observation are refined. Conflicting-request handling, next-operation admission under unknown outcomes, and other concurrency questions remain open.

### From Version 0.13 to 0.14

October 17, 2026. Version 0.14 records a working revision organized by concept; the original version 0.13 file `1-mesh-app.ru.md` was retained separately. Earlier version history follows without revising previous entries.

| Previously | Now |
|---|---|
| Discovery, preparation stages, responsibilities, and limitations are mixed; questions share one queue | Text is organized by concept; open questions are distributed across domain sections and updated as decisions are accepted |
| Axis originally describes mainly an environment capability (now Capability) | Its relationship to diagnostic tasks (now Probes) is explored: individual runs, Gecko's direct Zenoh API, progress, and immediate publication of discovered data; resources use Gecko + resource addressing |
| Postinit boundaries need refinement | A common explicit completion status for each Gecko's fact collection is accepted; Postinit is a stage without a separate code entity; blocker effects remain a working hypothesis |
| Responsibility distribution and node participation are partly described | Planner is external; the creating Gecko coordinates individual operations; Observer is 0 or 1, Interaction 0…N; descriptive roles need no UI display; Participation Profile uses free-form tags without enforced constraints |
| Point sources and the performance model remain broadly open | 1.0.0 uses built-in Points; a hub is a 2.0.0 direction; dynamic restructuring is a 3.0.0 direction. Line defines policy requirements and reactions; concrete budgets are refined in lower layers and tests |
| Readiness has no common response form or combination rule | A three-status readiness profile with context and freshness is accepted; participants own internal checks; one explicit bounded poll precedes Activation, without automatic retries |
| Decision application and cleanup have only broad boundaries | Activation success requires all mandatory participant acknowledgments; partial results return a report without automatic rollback. Cleanup is explicit at a participant or through Coordinator for all preparation, considering shared Blocks |
| Detailed complex-operation responses lack a common model | Operation Protocol with Operation Report is added: common core, explanations, evidence, resources, decisions, and `application`; report-content tables reside in domain sections and the Probe document |
| Identity, updates, and execution management are undefined | Accepted: `operation.id/type/parent_id`, safe-retry `request_id`, full snapshots, `sequence`, `reported_at`, five states, progress/blocker/recommendation structures; start/read/observe/cancel API; a new run after an external fix |
| Integration with transport is unexplored | ROS 2 Actions is a reference; a Zenoh mechanism, responsibility-boundary, and future integration-test map is added without claiming untested guarantees |
| Remaining significant decisions lack a separate plan | An eight-group temporary plan is retained, with discussion order, deferred details, and subsequent end-to-end scenario validation and review |

This version records accepted provisions and a working model rather than a completed implementation. Remaining architectural decisions are in the [temporary plan](archive/work-plan.md).

### From Version 0.12 to 0.13

| Previously | Now |
|---|---|
| Potential execution locations are called “potential slots” | Placement Option denotes a placement possibility without promising readiness or reservation |
| Composition option, revision, preparation plan, and readiness lack agreed names | App Candidate, App Revision, Build Plan, and App Readiness are distinct; Activation means putting a revision into operation |
| Participation characteristics mix presence, load, and role | Participation Profile separates expected presence, allowed use, and App roles |
| Latency/throughput balance has no name or modes | Performance Policy has Latency-first, Balanced, and Throughput-first profiles; Overload Policy and Dependency Failure Policy are named separately |
| Change acceptance is identified only through App creation | App Authority names the responsibility; in the first version it belongs to the creating Gecko |

These terms are introduced only at Mesh / App level. They do not close remaining queued questions, define lower-layer contracts, or change the general glossary.

### From Version 0.11 to 0.12

| Previously | Now |
|---|---|
| App responsibility is entirely open | Local creation, App identity, and the creating Gecko's first-version change-acceptance authority are defined; planning and coordination distribution remain open |
| Persistent observation is not distinguished | An optional observer for state, history, and notifications is added; the user is offered the choice when no suitable option is obvious |
| Build and operational execution boundaries are undefined | Build completion is separate from activation; published decisions authorize execution, with results tracked per participant |
| Publications are mainly Gecko declarations | A shared App decision and state namespace is added without selecting concrete keys and formats |
| Partially accepted answers are not marked in the queue | Decision status is added; no step is declared fully closed, and lower-layer requirements remain unchanged for now |
| Timing of authorization, ACLs, and dynamic restructuring is unstated | Authorization is deferred to 1.0.0; ACLs and dynamic restructuring when new persistent nodes appear to 2.0.0 |

### From Version 0.10 to 0.11

| Previously | Now |
|---|---|
| A decision-validation scenario and lower-layer hypotheses appear in separate sections 5 and 6 | Both sections are removed; their open questions join one decision queue |
| The decision queue is in section 7 | One queue combines responsibility, App scenario, and lower-layer capability reconciliation questions |

### From Version 0.9 to 0.10

| Previously | Now |
|---|---|
| The target workflow precedes responsibility and App operation rules | The example tests open decisions; workflow is recorded after they are agreed |
| A lower-layer capability list fixes result requirements | Capabilities are hypotheses whose necessity and allocation remain to be checked |
| Init, Postinit, Build, and App are target scenario stages | Stages are a working model with unapproved boundaries and transitions |
| The decision queue assumes predetermined lower-layer requirements | The queue ends with scenario validation and joint lower-layer requirement reconciliation |

### From Version 0.8 to 0.9

| Previously | Now |
|---|---|
| Fact contracts and exchange details lead the Mesh / App queue | Pub/sub is accepted as Mesh foundation; broker role and requirements are separated from lower-layer domain capability requirements |
| App responsibility is discussed after candidate construction | Responsibility, including the Coordinator question, is settled first |
| The queue combines lower-layer definitions and composition decisions | Six sequential decisions concern App planning, coordination, and use |

### From Version 0.7 to 0.8

| Previously | Now |
|---|---|
| Mesh is a collection of discovered Gecko instances | Decentralized fact exchange, partial discovery, and potential execution locations are clarified |
| Workflow does not distinguish discovery from selected-composition preparation | Init, Postinit, Build, and App are described with applicability scope and results |
| App proposals lack responsibility separation | Task acceptance, graph construction, scoring, agent role, and App responsibility remain open |
| Open questions have no order | A nine-decision queue states each step's expected outcome |

### From Version 0.5 to 0.6

| Previously | Now |
|---|---|
| App describes and presents the selected composition | App also defines a shared task experience: capability discovery, validation, startup, observation, and change |
| Workflow begins with node startup and network checks | The user begins with a task and inputs; Gecko exposes available execution and necessary conditions |

### From Version 0.4 to 0.5

| Previously | Now |
|---|---|
| App is assembled manually, then placed | App is a task composition proposed from Mesh capabilities or assembled manually |
| Automatic placement is left to the future | Proposing implementations and placement before startup is part of target UX; migration and failover remain separate tasks |
| Line requirements are described through Points and Edges | A Line's own requirements are checked too; App dependencies form a graph |
