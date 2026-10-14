# Mesh and App

Version 0.13 · October 14, 2026 · [Русский](1-mesh-app.ru.md)

[Glossary](0-concepts.md) · [Gecko / Axis](2-gecko-axis.md) · [Line / Point / Edge / View](3-line-point-edge-view.md) · [Block / Anchor](4-block-anchor.md)

**Status:** material for architectural decisions. Decentralized fact exchange through pub/sub, access to partial discovery results, and the intent to provide a shared App contract for different users are established. The local App creation entry point, authority to accept changes in the first version, independent App identity, optional permanent observation, and separation of Build from commissioning have been defined. Accepted decisions and participant state are exposed in a shared App publication space. Assignment of planner and coordinator responsibilities, commissioning details, and recovery remain open. Other stages, scenarios, and expected lower-layer capabilities are working hypotheses for discussion; the target workflow and domain contract requirements have not yet been approved. The document does not describe implemented capabilities.

**Document boundary:** this document defines the pub/sub broker's role in the Mesh and its minimum requirements, and discusses the use of Mesh capabilities, App construction, and coordination. Pub/sub is an established architectural foundation; message update and delivery mechanisms belong to the broker. Accepted answers and remaining questions are collected in the temporary architecture decision queue. New decisions are recorded at the Mesh / App level; lower-layer requirements are not being changed yet and will be agreed after the entire queue has been discussed.

## 1. Mesh

**A Mesh is a decentralized system for exchanging facts and capabilities among Gecko instances.** A single main Gecko or centralized manager is not a required part of the model. Each node exposes its identity, state, and known capabilities; participants construct their view of the Mesh from available declarations. A new participant expands the available App options, but does not change a running composition by itself.

The technical exchange model is a subscription through a pub/sub broker to a publication space for Gecko identities, statuses, discovery results, Devices, Cameras, Axes, known processing and service implementations, and connection test results. Each Gecko publishes information relative to its own node; the Mesh uses these declarations to build and validate Apps. The broker provides access to published state and exchanges updates under the required guarantees. This is an initial Mesh architecture decision; designing custom message update mechanisms is not part of the App decision queue.

The `gecko/**` space illustrates this model for Zenoh. Minimum Mesh requirements are defined here; specific keys, QoS settings, and delivery mechanisms are configured during broker integration. See the [official Zenoh abstractions documentation](https://zenoh.io/docs/manual/abstractions/).

### 1.1. Facts, Capabilities, and Placement Options

A Mesh view contains what nodes have discovered and reported so far. Incomplete or unknown information must be distinguishable from a negative test result. Hardware presence, an available implementation, service readiness, and current resource availability are also distinct.

Discovery results allow **Placement Options** for Lines and Block providers on particular Gecko instances, and supported connections between them, to be described. A Placement Option is a possibility of placing processing when its requirements are met. It does not mean a created Line, a ready Block, or a reserved resource; suitability for a particular task requires validation. An Axis describes an execution environment capability; a Placement Option describes a way to use that environment for processing placement. The term replaces the working name “potential slot” and is used at the Mesh / App level.

Without a task, available resources and constraints, and possible graph fragments, can be described. A particular App composition, its suitability for the task, and option evaluation require the user's inputs, behavior, and constraints. Resource scoring alone does not define the actual task.

### 1.2. Broker Role and Minimum Requirements

The broker is the Mesh communication infrastructure. The name denotes the pub/sub system's role; its topology must support decentralized Gecko participation and an edge environment. Using a broker does not appoint a main Gecko or an App coordinator: application management responsibilities are considered separately.

Minimum broker requirements:

- An addressable publication space attributing information to a particular Gecko, with subscriptions to key groups including `gecko/**`.
- Access for a joining participant to current published state and subsequent changes.
- Declaration updates and withdrawals with the delivery and ordering guarantees needed to represent current state.
- Publisher availability information and visibility of state retrieval limitations during connection loss and reconnection.
- Configurable exchange guarantees and edge operation; Mesh policy does not require a single management center.

Specific state retention, synchronization, redelivery mechanisms, and QoS settings belong to broker implementation and integration. Mesh / App relies on these capabilities as requirements for the selected infrastructure. Gecko supplies declaration contents and domain validation results: message delivery alone does not confirm resource readiness or successful execution of an App operation.

## 2. App

An App defines a composition for a task and a shared experience for interacting with its capabilities, actions, and results. Its general role is described in the [glossary](0-concepts.md).

Access to App capabilities and operations must have the same meaning for an engineer, an agent, and a programmatic consumer. Discovery, compatibility validation, and explanations of missing readiness belong to Gecko. The division between task formulation, graph proposal, scoring, and option selection has not yet been defined.

In the model under discussion, an App candidate description exists before its participants are created. Planning and preparation distinguish the following Mesh / App terms:

| Term | Meaning |
|---|---|
| **App Candidate** | A proposed composition: implementations, placement, and dependencies still to be selected and validated. It may use Placement Options; it does not imply prepared execution. |
| **App Revision** | A recorded version of the App description against which changes are accepted, checks performed, and operations executed. A revision alone does not imply a successful Build or operational startup. |
| **Build Plan** | A plan for validating and preparing the selected revision according to its dependencies. It differs from the composition description and the results of executing the plan. |
| **App Readiness** | An assessment of the selected revision's readiness for a particular operation, under stated conditions and with regard to check freshness. It differs from actual execution and does not replace a separate commissioning decision. |

The provisional `App` stage below denotes the selected composition being ready for use, not the first appearance of the App concept. App Candidate, App Revision, and Build Plan formats, and App Readiness criteria, remain to be defined. These names do not introduce additional lower-layer entities.

### 2.1. The App Dependency Graph

App composition can be expanded as a tree, but execution dependencies form a graph: several Lines may share a Block or Device. App validation combines participant results while preserving each dependency’s execution location.

```text
App inspection
├── Line camera: its own runtime requirements
│   ├── Source Point: implementation requirements, binding to Camera entrance
│   ├── Inference Point: selected option requirements
│   │   └── local binding to Device gpu-0
│   └── internal Edges: compatibility and resource requirements
└── Line archive
    ├── Recorder Point: receiving video, recording through Anchor storage
    └── Anchor storage → Block recordings
                          └── provider: separate implementation requirements
```

In this example, Recorder Point receives video and calls Block recordings operations through Anchor storage, declared by Line archive. The Anchor describes the Line's dependency; the Point performs the processing.

A Line combines its own requirements with those of its Points and internal Edges. When using a Block, the Line requires its contract and readiness through an Anchor; a remote provider’s hardware requirements do not become local Line requirements. Shared diagnostics can expose Device relationships reported by the provider. No additional term is introduced for a hardware dependency: requirements and concrete bindings belong to execution configuration.

## 3. Working Model of Discovery and App Preparation

Init, Postinit, Build, and App are provisional names for discussing discovery, preparation, and use of a composition. The separation of Build completion from App commissioning has been accepted (§3.4); other boundaries, activities, and transition criteria depend on the remaining questions in the temporary architecture decision queue. The descriptions below show the proposed content of this model, not a complete approved workflow or requirements for lower-layer operations. New nodes may be undergoing discovery while one App is being built and another is running. Readiness is evaluated for a particular set of participants, facts, and a selected composition.

| Stage | Activities | Result |
|---|---|---|
| **Init** | Running Gecko instances publish identity, statuses, and available discovery results as they emerge | Known Devices, Cameras, and capabilities are already visible even if some discovery is unfinished |
| **Postinit** | Known participants have completed their current discovery pass or reported its limitations; resource and supported connection information is available | A picture of Placement Options and connections from which App Candidates for a future task can be built |
| **Build** | For a selected App Revision, following a Build Plan, connections are refined, Points tested, Lines and required Block providers prepared and placed, and Anchors validated | A prepared composition and validation results for its parts and the App as a whole |
| **App** | The prepared composition is available for startup, observation, View attachment, and input data use | A verifiable result and a cycle of use, modification, and repeat evaluation |

### 3.1. Init and Postinit

Init does not require all discovery to finish before exposing available facts. The Gecko / Axis layer performs discovery, checks, and capability declarations. The Mesh receives their results and discovery completion information; the internal implementation of those checks is defined in the lower layer.

Postinit means the current pass has completed for a specified set of known participants. It does not promise discovery of every possible node or information about unreachable participants. A failed check or unknown characteristic remains an explicit limitation. Lower layers report discovery completion and information freshness; selecting participants sufficient for a particular App belongs to its planning and coordination.

Supported preliminary connection checks and a picture of potential execution can be produced at this stage. Deploying processing Lines and Blocks for the new task is not yet required. Existing services and Apps remain Mesh facts; their presence does not violate this discovery stage.

### 3.2. Build

In the working model, Build applies to a selected App Revision formed from an App Candidate. A Build Plan describes its validation and preparation. The following activities are proposed for discussion; whether each is mandatory, who performs it, and under what conditions have not yet been selected:

1. Checking selected implementation requirements and current resource availability.
2. Measuring required connection segments under the selected formats and load conditions. Individual measurements are supplemented by testing the composition's combined load.
3. Testing Points in isolation in a suitable test environment. This does not introduce an independent process or lifecycle for a Point during normal Line execution.
4. Preparing artifacts, implementations, and service providers, placing Lines, and validating their connections.
5. Validating Block contracts, readiness of required models, and Anchor conditions.
6. Testing the whole App against selected inputs and criteria, and saving results and limitations.

Preparation order within Build depends on the dependency graph. Tests require the providers they use to be ready; unused services need not be prepared in advance. Successful Build criteria, temporary test execution, resource reservation, and cleanup after failure remain to be defined. Passing a test does not guarantee readiness indefinitely: dependencies are rechecked before operational startup.

### 3.3. App: Readiness and Use

For the provisional App stage, starting and stopping processing, observing results, attaching supported Views, providing samples, and using a real Camera are under discussion. The set of available operations remains to be selected. Readiness, current execution, and validation results are distinct states.

Switching from a sample to a Camera, changing processing mode, or attaching a View may change load and requirements. Seamless changes are not assumed. The proposal for discussion is to expose what can be applied immediately, what requires a restart, and which validation results must be obtained again; exact rules and responsibility for this information have not yet been selected.

### 3.4. Build Completion and Activation

Build completion records a prepared composition for a particular App revision, validation results, and limitations. It does not yet authorize operational execution. Temporary test runs belong to validation and do not mean the App has been commissioned; their rules remain open.

**Activation**, or commissioning, is a separate operation on the selected App Revision that authorizes its operational execution. Preparation and commissioning progress are exposed in the Mesh. Before the Activation decision, participants do not begin operational execution or declare themselves participants in a running App. Being "prepared for an App" differs from "executing an App."

Operational execution is authorized by a commissioning decision for a particular revision published as one complete record in the shared App space (§4.8). Before this decision is recorded, an interrupted operation does not authorize participants to complete commissioning or start processing independently. Completed activities and their results remain visible in the Mesh.

After the decision is recorded, its application and the transition to a working state are tracked separately through participant declarations. Accepting a decision does not imply simultaneous receipt by all nodes or simultaneous physical startup. Delivery and retrieval of published state belong to broker integration; confirmation that publication has completed, behavior during partial startup, recovery of an unfinished operation, and retention or release of prepared resources remain to be defined. These boundaries do not approve a complete commissioning protocol or introduce new lower-layer requirements.

## 4. Placement, Constraints, and Interaction

### 4.1. Composition and Execution Boundaries

Gecko's target capability is to expose the Mesh and obtain compatible App options for a task, including Point implementations, Line placement, and Block providers. The user creates an App through a local Gecko (§4.6); where options are built and which part an agent performs remain open questions. Available now is distinguished from requiring preparation and an unsatisfied dependency. This information, selection actions, and validation results are available regardless of interaction mechanism. Selecting an option records the composition; manual assembly and constraint changes are also available.

Assessing bandwidth and latency may require checks of paths between nodes. Who initiates and performs them, when they are needed, and whether automated checks are appropriate remain open questions. Measurements must be used with regard to stream requirements and result freshness; an unchecked path is not guaranteed suitable. Measuring one connection does not guarantee bandwidth for all simultaneous streams or reserve resources. The need for and scope of repeat checks before startup are determined together with App readiness rules.

An App can run across several Gecko instances. Each Line is placed entirely on one Gecko instance. Moving a Point’s function beyond its Line requires an explicit structural change: using a Block’s service contract through an Anchor, or extracting part of the processing into another Line with a supported Edge.

```text
jetson-01                         gpu-01
Line capture                     Line analysis
  Point source.video ── Edge ───→   Point detector.video_in

or:
Line camera ── Anchor detector ──→ Block inference
```

The first case requires processing connection support. The second requires a contract, connection implementation, and fulfillment of Anchor conditions. A Block’s location is determined by its deployment; Gecko may support managing that deployment, but this does not follow from the Block’s role.

One App does not imply a shared process or one atomic pause: an App operation coordinates its participants according to a chosen policy. A shared Block should not automatically stop if other applications use it.

An App exposes Lines, connections through Edges, and dependencies through Anchors referencing Blocks. A Line exposes its Points. Lifecycle and placement are available at Line level; parameters and diagnostics are available at Point level. An Anchor exposes its required contract, referenced Block, readiness, and policy for unavailability. A Block exposes its capabilities, state, and management operations. Gecko reports required restarts before changes and dependent consumers before restarting a Block. This is a shared interaction contract, not a mandatory sequence of screens or manual assembly steps.

Presentation and control capabilities are considered when composing Views. A user may have access only to video and the necessary actions. Control permissions are checked separately and do not follow from a UI Axis or the chosen interaction mechanism.

### 4.2. Node Participation Profile

A **Participation Profile** describes the declared conditions under which a Gecko participates in App planning and execution. Performance and actual node availability are considered separately. The profile separates expected presence, permitted use, and roles relative to a particular App:

| Dimension | Values and Meaning |
|---|---|
| **Expected presence** | **Persistent** — continuous presence; **Scheduled** — presence according to a declared schedule; **Intermittent** — irregular presence without a known schedule of future availability |
| **Permitted use** | **Continuous** — sustained execution; **Task-bound** — individual bounded jobs or tests; **No compute** — computational processing is not permitted |
| **App roles** | **Planner** — proposes composition and placement; **Coordinator** — coordinates operations; **Observer** — monitors state; **Executor** — executes processing; **Interaction** — provides user interaction |

For example, a permanent compute server may have a `Persistent + Continuous` profile, while a powerful workstation for one-off tests may use `Intermittent + Task-bound`. A node used only for user interaction has `No compute` and the `Interaction` role. Roles may overlap; assignment to a particular App differs from a node's permitted use. `No compute` limits participation in application processing and does not by itself prohibit management or observation.

A profile may change and does not guarantee uninterrupted execution. Observed availability, current resources, and actual App participation state are published separately. Who sets the profile, what the node declares itself, how roles are assigned, and how planning validates these conditions remain undecided. Role names describe architectural responsibilities rather than access permissions.

### 4.3. Performance Policy

A **Performance Policy** sets the balance between latency and throughput during App selection and validation. A **Performance Profile** names the selected mode of that policy. Three named profiles are introduced at the Mesh / App level:

| Profile | Priority |
|---|---|
| **Latency-first** | Minimize latency under specified load constraints |
| **Balanced** | Jointly meet specified latency constraints and required throughput |
| **Throughput-first** | Maximize throughput within the permitted latency limit |

The names describe intent rather than guaranteed results. A profile is refined by the measured path and latency metric, its permitted limit, required throughput, permitted batch formation wait, and queue limits. Numerical values, the default profile, and validation rules remain to be defined; a T0–T5 scale is not introduced at this stage. Absolute zero latency is not a system promise; `Latency-first` replaces `zero latency`.

An **Overload Policy** separately defines permitted App behavior when load exceeds capacity. Selecting a Performance Profile does not itself permit frame dropping, data loss, or unlimited queue growth. Permitted responses and participant support remain to be agreed with the lower layers.

For tasks with many cameras or SGIE, batching is an inference requirement. Requests from multiple cameras are batched through Triton, the Block inference provider. Inference Points access it through their Lines' Anchors; batching settings on the Triton side belong to the Block provider. The Performance Profile selected for a Line must therefore be compatible with the model's capabilities and the shared provider's execution mode, including batch size and the permitted wait for batch formation.

Performance Policy affects the choice of connections, Blocks, and Anchor conditions, as well as Build and App Readiness criteria. App Candidate selection and validation account for the combined load of all Lines using a Block and the effects of batching on latency and throughput. Testing a single Line is insufficient to assess a shared provider's execution mode. Individual Line needs are checked against the shared Block's agreed mode; each Line cannot independently change its profile.

Switching after App readiness depends on the Line and Block provider implementations: some settings may change while running, while others require rebuilding, restarting, or new placement. Changing a shared Block's execution mode requires accounting for all its consumers, including other Apps. Policy and profile names are recorded here at the Mesh / App level; concrete execution settings, supported responses, and revalidation rules remain to be defined with the [Line / Point / Edge](3-line-point-edge-view.md) and [Block / Anchor](4-block-anchor.md) layers.

### 4.4. Anchors and Block Preparation

Required contracts and Anchor conditions are described in the App candidate so its feasibility can be checked. An Anchor describes a dependency; it is not an installable service. A Block provider may already be running or may require preparation; startup rules belong to the [Block / Anchor](4-block-anchor.md) layer.

At App level, a **Dependency Failure Policy** names the selected composition behavior when a required participant or service is not ready or becomes unavailable. It differs from Performance Policy and Overload Policy. Concrete dependent Line behavior is described by its Anchors; App policy must account for these conditions and a shared Block's effect on all consumers. Supported responses, their coordination, and recovery rules remain open; no new Anchor contract is defined here.

Required providers must be ready before tests or operational execution that use them. Preparation follows the selected Build plan's dependencies. When a provider is created, who manages it, how it is reused, and when it is released are separate decisions; preparing every potential Block during Postinit is not required.

### 4.5. Agent Systems

An agent uses the same capability declarations, validation results, and App operations as a human. Agent interaction alone does not confer extra permissions, resources, or guarantees. The shared contract must support a programmatic cycle of “propose an option → test → observe → modify.”

An agent may be able to explore hypotheses faster, but its place in transforming a task into an App plan has not been selected. It is undecided whether it is an optional assistant, a separate planning participant, or part of a supported implementation of option generation. Experiment limits, conditions for starting Build, and result selection criteria also need definition.

### 4.6. Responsibility for an App

The user creates an App through a Gecko instance physically located on the same machine. The Mesh may contain several such user nodes; their presence does not require a single main Gecko. A user node may disconnect and is not assumed to be a permanent observation center for the Mesh.

An App has its own stable identity, independent of the Gecko that created it. The **App Authority** is the Gecko that stores the current App Revision and atomically checks the base revision and saves the accepted next revision. In the first version, the creating Gecko is the App Authority. A concurrent change based on an outdated revision is not accepted. Accepted decisions are published in the shared App space (§4.8). The mechanism connecting local persistence and publication, including recovery from interruption between them, remains open.

While the App Authority is unavailable, new App changes are not accepted. Commissioned execution does not require its continuous presence and continues when dependencies are available, according to participant policies. A user on another machine can observe available declarations and results, but in the first version cannot change this App through another Gecko. An optional observer (§4.7) does not gain authority to accept changes by being appointed.

Candidate construction, operation coordination, and permanent observation are distinct responsibilities. A planner proposes composition and placement; a coordinator organizes application of the selected decision and collects operation results; an observer monitors state, retains history, and sends notifications. Assignment of these responsibilities among Gecko instances and their relationship to Participation Profiles in §4.2 have not yet been defined. Assigning authority to accept changes does not itself make the App Authority responsible for all these duties.

Authorization is deferred until release **1.0.0**, and ACLs until **2.0.0**. User identity, its binding to a node, and transfer of management authority remain future decisions. These responsibilities do not yet define an access role model.

### 4.7. Optional Permanent Observation

A permanent observer can be appointed for a particular App: a Gecko that monitors state, records history, and sends notifications independently of user node availability. The observer is selected from available nodes whose Participation Profile declares Persistent presence. When a suitable choice is obvious, the system proposes its appointment; the user is asked to choose when the choice is not obvious. If no suitable node exists, the user can explicitly select a node for observation or use the App without an observer. Suitability criteria and what makes a choice obvious remain to be defined.

An observer is optional: a one-off test or processing a video segment may run without permanent history and notifications. Appointment as an observer does not authorize App changes or imply automatic recovery or composition restructuring. Its exact operations, behavior when it fails, and division of responsibilities with the planner and coordinator remain open.

Dynamic App restructuring when new permanent nodes appear is deferred until **2.0.0**. Until then, a new node expands the available options but does not automatically change a running composition. This restriction does not define other recovery, migration, or balancing policies.

### 4.8. App Decisions and State in the Mesh

An App has a shared addressable publication space for accepted decisions and participant state. A decision refers to a particular operation and App revision and is published as one complete record linking its contents to the selected revision and participant set. Publication integrity differs from the atomic revision check used to accept a change (§4.6) and does not imply simultaneous execution by all participants.

Each Gecko declares which Apps it serves, its Participation Profile, assigned roles, and actual participation state. Preparation progress, execution participation, and observation are distinct; a prepared part of the composition is not declared running merely because Build activities have completed.

The shared decision and the actual outcome of applying it are exposed separately. Publishing permission to execute does not confirm successful startup of all participants. Direct node checks supplement declarations when readiness or state needs clarification; polling alone does not replace a commissioning decision.

Exchange relies on the accepted model of access to current published state and subsequent changes (§1.2). Specific keys, record formats, placement of retained state, and delivery mechanisms are defined during integration. Behavior after reconnection and during incomplete publication still needs to be agreed with the App operation protocol. Domain contracts and new lower-layer requirements are not established here.

## MESH / APP ARCHITECTURE DECISION QUEUE

This temporary list retains questions about the App scenario and necessary lower-layer capabilities. Accepted parts of decisions are noted after the table; remaining questions continue to be discussed. Responsibility and task specification are determined first, followed by App operations and necessary information. The target workflow and first-version scope are recorded after decisions have been checked; requirements for lower-layer domain contracts will be agreed after the entire queue has been discussed. Expected discussion outputs are not lower-layer commitments. Broker role and minimum requirements are addressed separately in section 1.2.

| Step | Open Questions | Decision Output |
|---|---|---|
| **1** | How are task reception, candidate construction, storage of the selected App, and management of its operations assigned? Is an App coordinator needed, who performs that role, and how is it reached? Who accepts changes and stores the current revision? | A responsibility model without a required main Gecko for the entire Mesh; role assignment, change acceptance, and behavior when a managing participant is lost |
| **2** | What does the user provide: a task description, a ready composition, or parameters for a supported template? How are input data, constraints, and success criteria specified? Who accepts this input? | Selected ways to specify tasks and success criteria, with responsibilities assigned according to step 1 |
| **3** | Who transforms a task into a candidate, proposes implementations and placement, scores options, and selects one, and on what information? Are candidate exploration, manual selection, and an agent needed; what is each one's role? | Composition construction and selection method, candidate format, and planning automation boundaries |
| **4** | What information about resources, Devices, Cameras, Axes, capabilities, and access conditions is needed from Gecko / Axis? How is it addressed and a particular binding selected? Are Camera declarations from individual nodes sufficient, or is global camera identification needed? | Minimum execution and source information agreed with Gecko / Axis, and rules for references and binding selection |
| **5** | How are partial discovery results used, current pass completion, limitations, and freshness reported, and unknown, unavailable, and confirmed information distinguished? Which known participant set is sufficient for an App; are separate Init and Postinit stages needed and what are their boundaries? | Rules for incomplete information and discovery sufficiency, with agreed Gecko / Axis information |
| **6** | What information about known Line / Point / Edge implementations, inputs and outputs, requirements, supported connections, and constraints is needed to validate a candidate without creating every participant in advance? | Minimum implementation and compatibility descriptions agreed with the Line / Point / Edge layer |
| **7** | How does placement account for node Participation Profiles, available resources, and Performance Profiles? Which connection measurements are needed, who initiates and performs checks, and when is automation appropriate? How are measurement conditions and freshness, combined load, Performance Policy, and Overload Policy considered? | Admission and placement selection policy with explicit limitations; information and checks agreed with Gecko / Axis and Line / Point / Edge |
| **8** | What information about Block contracts, available providers, preparation capabilities, readiness, and Anchor conditions is needed to select a composition? Who prepares, manages, reuses, and releases a provider? | Dependency information and rules for provider participation in App preparation and execution agreed with Block / Anchor |
| **9** | Which checks and preparation are necessary and who performs them? Is a separate Build needed? How are preparation dependencies, Point and Anchor checks, temporary test runs, reservation, combined load assessment, and cleanup after failure organized? What is sufficient for startup and what is checked again? | Selected App preparation and validation model, activity assignments, readiness criteria, and repeat check rules |
| **10** | Which validation, preparation, startup, shutdown, and change operations are needed from Gecko / Axis, Line, and Block? How are they assigned between layers, progress, results, and limitations obtained, and partially completed operations handled? | Agreed minimum operations and results with responsible layers; coordination rules for the selected composition |
| **11** | How do users, agents, and programmatic consumers obtain results and use an App through a shared contract? Which control/view/trace and View binding conditions are needed from Point / View? How are files, samples or Cameras, processing, presentations, policies, and composition changed; what requires repeat validation, restart, or Build? | App use and change rules, result observation, and presentation binding agreed with lower layers |
| **12** | How are unavailable dependencies, failed checks, partial preparation, and participant loss reported? What happens with concurrent changes and failure of a managing participant or shared Block provider? | Basic failure and conflict behavior, diagnostics linked to App state, and responsibility assignment |
| **13** | When are automatic recovery, control transfer, migration, and balancing needed? Which experiment limits and startup conditions are needed for an agent? | Explicitly selected automation scope and experiment limits; basic failure behavior is defined in preceding steps |
| **14** | Are the decisions sufficient for an end-to-end scenario from user input to an observable result and subsequent change? Which lower-layer capabilities are necessary and compatible with their models, and which are optional, deferred, or rejected? | An agreed workflow with executors, inputs, outputs, and failure behavior; first-version scope and confirmed requirements with responsible layers |

### ACCEPTED PARTS OF DECISIONS AND REMAINING QUESTIONS

No step is fully closed yet. The table above retains the complete set of questions; the table below identifies the parts already decided.

| Step | Accepted | Still to Be Defined |
|---|---|---|
| **1** | Creation through a local Gecko; independent App identity; App Authority assigned to the creating Gecko in the first version, change acceptance, and current App Revision storage; execution without its continuous presence (§4.6) | Planning and coordination assignments, how to reach them, relationship of responsibilities to node Participation Profiles; recovery of unfinished operations |
| **2** | The local Gecko accepts user input when creating an App | Input forms, input data, constraints, and success criteria |
| **9** | Build completion is separate from commissioning; preparation does not authorize operational execution (§3.4) | Checks, executors, completion criteria, repeat checks, test runs, reservation, and cleanup |
| **10** | Decisions are published as complete records in the shared App space; participant states and direct checks expose application outcomes (§4.8) | Operations and their assignment, exact commissioning protocol, publication confirmation, partial execution, and recovery |
| **11** | Permanent observation, history, and notifications are optional (§4.7); available state can be observed from another user node | Shared use contract, Views, change rules, and exact observer capabilities |
| **12** | One party accepts changes in the first version, with an atomic revision check; while it is unavailable, no new changes are accepted, and commissioned execution continues when dependencies are available | Recovery between persistence and publication, partial commissioning, failures of executors, dependencies, shared Blocks, and the observer |
| **13** | Transfer of management authority is outside the first version; dynamic restructuring when new permanent nodes appear is deferred until 2.0.0 | Other recovery, migration, and balancing mechanisms; agent experiment limits |

Authorization is deferred until 1.0.0, and ACLs until 2.0.0. These dates do not resolve architectural responsibility assignments. Steps **3–8** and **14** remain open; selecting where authority to accept changes resides does not determine who performs planning.

Decisions can be checked against an example: processing video from a file or Camera in one Line with local processing, then a variant with inference through a Block or an additional compute node. The example may include a permanent node, a node for temporary computation, a detector, and a result accessible to both a human and an agent, as well as operation without an observer and user node disconnection before and after a commissioning decision. It does not prescribe an operation sequence or first-version scope. Declaration exchange through a pub/sub broker remains the initial architecture decision; domain contracts and lower-layer requirements will be agreed after the entire queue has been discussed.

## 5. Vocabulary Changes

### From Version 0.12 to 0.13

| Previously | Now |
|---|---|
| Potential execution locations are called “potential slots” | A Placement Option names a placement possibility without promising readiness or reservation |
| Composition options, revisions, preparation plans, and readiness lack consistent names | App Candidate, App Revision, Build Plan, and App Readiness are distinguished; Activation names commissioning a revision |
| Participation characteristics mix presence, load, and role | A Participation Profile separates expected presence, permitted use, and App roles |
| The latency/throughput balance has no name or modes | Performance Policy has Latency-first, Balanced, and Throughput-first profiles; Overload Policy and Dependency Failure Policy are named separately |
| The party accepting changes is described only through App creation | App Authority names that responsibility; it is assigned to the creating Gecko in the first version |

These terms are introduced only at the Mesh / App level. They do not close remaining queue questions, define lower-layer contracts, or change the shared glossary.

### From Version 0.11 to 0.12

| Previously | Now |
|---|---|
| App responsibility is entirely open | Local creation, App identity, and authority to accept changes assigned to the creating Gecko in the first version are defined; planning and coordination assignments remain open |
| Permanent observation is not distinguished | An optional observer for state, history, and notifications is added; the user is asked to choose when a suitable option is not obvious |
| The boundary between Build and operational execution is not established | Build completion is separate from commissioning; a published decision authorizes execution, with outcomes tracked by participant |
| Publications are primarily considered Gecko declarations | A shared App decision and state space is added without selecting specific keys or formats |
| Partially accepted answers are not noted in the queue | Decision status is added; no step is declared fully closed, and lower-layer requirements are not being changed yet |
| Authorization, ACL, and dynamic restructuring dates are not specified | Authorization is deferred until 1.0.0; ACLs and dynamic restructuring when new permanent nodes appear until 2.0.0 |

### From Version 0.10 to 0.11

| Previously | Now |
|---|---|
| A scenario for checking decisions and lower-layer hypotheses are described in separate sections 5 and 6 | Both sections are removed; their open questions are included in a single decision queue |
| The decision queue is in section 7 | A single queue combines responsibility, App scenario, and lower-layer capability agreement questions |

### From Version 0.9 to 0.10

| Previously | Now |
|---|---|
| The target workflow is described before responsibility and App operation rules are selected | The example is used to check open decisions; the workflow is recorded after they are agreed |
| A list of lower-layer capabilities establishes outcome requirements | Capabilities are described as hypotheses whose necessity and assignment are still being checked |
| Init, Postinit, Build, and App are presented as target scenario stages | Stages are described as a working model with unapproved boundaries and transitions |
| The decision queue relies on already established lower-layer requirements | The queue ends with scenario validation and joint agreement on lower-layer requirements |

### From Version 0.8 to 0.9

| Previously | Now |
|---|---|
| Fact contracts and exchange details come first in the Mesh / App queue | Pub/sub is accepted as the Mesh foundation; broker role and requirements are separated from requirements for lower-layer domain capabilities |
| App responsibility is discussed after candidate construction | The responsibility model, including the coordinator question, is decided first |
| The queue combines lower-layer definitions and composition decisions | Six consecutive decisions address App planning, coordination, and use |

### From Version 0.7 to 0.8

| Previously | Now |
|---|---|
| Mesh is described as a collection of discovered Gecko instances | Decentralized fact exchange, partial discovery, and potential execution locations are clarified |
| The workflow does not distinguish discovery from preparation of a selected composition | Init, Postinit, Build, and App are described with scope and outputs |
| App proposal is stated without responsibility separation | Task reception, graph construction, scoring, agent involvement, and App responsibility remain open |
| Open questions are listed without an order | Nine decisions are ordered with an expected output for each step |

### From Version 0.5 to 0.6

| Previously | Now |
|---|---|
| An App describes and represents a selected composition | An App also defines a shared task experience: capability discovery, validation, startup, observation, and modification |
| The workflow starts with node startup and network checks | The user starts with a task and inputs; Gecko exposes available execution and required conditions |

### From Version 0.4 to 0.5

| Previously | Now |
|---|---|
| An App is assembled manually before placement | An App is a task composition proposed from Mesh capabilities or assembled manually |
| Automatic placement is left for the future | Proposed implementations and placement before startup are part of the target UX; migration and failover remain separate tasks |
| Line requirements come from Points and Edges | A Line’s own requirements are also checked; App dependencies form a graph |
