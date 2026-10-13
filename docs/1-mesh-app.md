# Mesh and App

Version 0.9 · October 13, 2026 · [Русский](1-mesh-app.ru.md)

[Glossary](0-concepts.md) · [Gecko / Axis](2-gecko-axis.md) · [Line / Point / Edge / View](3-line-point-edge-view.md) · [Block / Anchor](4-block-anchor.md)

**Status:** a working design framework. Decentralized fact exchange, access to partial discovery results, and a shared App contract for different users are established. The four stages frame the discussion; task reception, plan construction, and App responsibility remain open decisions. The document does not describe implemented capabilities.

**Document boundary:** this document defines the pub/sub broker's role in the Mesh, its minimum requirements, how Mesh capabilities are used, and how an App is built and coordinated. Pub/sub is an established architectural foundation; message update and delivery mechanisms belong to the broker. Requirements for lower-layer domain capabilities are listed in section 6, and the Mesh / App decision queue is in section 7.

## 1. Mesh

**A Mesh is a decentralized system for exchanging facts and capabilities among Gecko instances.** A single main Gecko or centralized manager is not a required part of the model. Each node exposes its identity, state, and known capabilities; participants construct their view of the Mesh from available declarations. A new participant expands the available App options, but does not change a running composition by itself.

The technical exchange model is a subscription through a pub/sub broker to a publication space for Gecko identities, statuses, discovery results, Devices, Cameras, Axes, known processing and service implementations, and connection test results. Each Gecko publishes information relative to its own node; the Mesh uses these declarations to build and validate Apps. The broker provides access to published state and exchanges updates under the required guarantees. This is an initial Mesh architecture decision; designing custom message update mechanisms is not part of the App decision queue.

The `gecko/**` space illustrates this model for Zenoh. Minimum Mesh requirements are defined here; specific keys, QoS settings, and delivery mechanisms are configured during broker integration. See the [official Zenoh abstractions documentation](https://zenoh.io/docs/manual/abstractions/).

### 1.1. Facts, Capabilities, and Potential Slots

A Mesh view contains what nodes have discovered and reported so far. Incomplete or unknown information must be distinguishable from a negative test result. Hardware presence, an available implementation, service readiness, and current resource availability are also distinct.

Discovery results allow potential execution locations for Lines and Block providers, and supported connections between them, to be described. **Potential slot** is a working description of the possibility of placing processing when its requirements are met. It is not yet a created Line, a ready Block, or a reserved resource, and is not added to the core vocabulary as a separate entity.

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

An App candidate description exists before its participants are created. The `App` stage below denotes the selected composition being ready for use, not the first appearance of the App concept.

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

## 3. Discovery and App Preparation Stages

These are four stages of the target scenario, not a single global lifecycle for the entire Mesh. New nodes may be undergoing discovery while one App is being built and another is running. Readiness is evaluated for a particular set of participants, facts, and a selected composition.

| Stage | Activities | Result |
|---|---|---|
| **Init** | Running Gecko instances publish identity, statuses, and available discovery results as they emerge | Known Devices, Cameras, and capabilities are already visible even if some discovery is unfinished |
| **Postinit** | Known participants have completed their current discovery pass or reported its limitations; resource and supported connection information is available | A picture of potential slots and connections from which App options for a future task can be built |
| **Build** | For a selected candidate, connections are refined, Points tested, Lines and required Block providers prepared and placed, and Anchors validated | A prepared composition and validation results for its parts and the App as a whole |
| **App** | The prepared composition is available for startup, observation, View attachment, and input data use | A verifiable result and a cycle of use, modification, and repeat evaluation |

### 3.1. Init and Postinit

Init does not require all discovery to finish before exposing available facts. The Gecko / Axis layer performs discovery, checks, and capability declarations. The Mesh receives their results and discovery completion information; the internal implementation of those checks is defined in the lower layer.

Postinit means the current pass has completed for a specified set of known participants. It does not promise discovery of every possible node or information about unreachable participants. A failed check or unknown characteristic remains an explicit limitation. Lower layers report discovery completion and information freshness; selecting participants sufficient for a particular App belongs to its planning and coordination.

Supported preliminary connection checks and a picture of potential execution can be produced at this stage. Deploying processing Lines and Blocks for the new task is not yet required. Existing services and Apps remain Mesh facts; their presence does not violate this discovery stage.

### 3.2. Build

Build applies to a particular App candidate and includes:

1. Checking selected implementation requirements and current resource availability.
2. Measuring required connection segments under the selected formats and load conditions. Individual measurements are supplemented by testing the composition's combined load.
3. Testing Points in isolation in a suitable test environment. This does not introduce an independent process or lifecycle for a Point during normal Line execution.
4. Preparing artifacts, implementations, and service providers, placing Lines, and validating their connections.
5. Validating Block contracts, readiness of required models, and Anchor conditions.
6. Testing the whole App against selected inputs and criteria, and saving results and limitations.

Preparation order within Build depends on the dependency graph. Tests require the providers they use to be ready; unused services need not be prepared in advance. Successful Build criteria, temporary test execution, resource reservation, and cleanup after failure remain to be defined. Passing a test does not guarantee readiness indefinitely: dependencies are rechecked before operational startup.

### 3.3. App: Readiness and Use

At the App stage, the user can start and stop processing, observe results, attach supported Views, provide samples, and use a real Camera. Readiness, current execution, and validation results are distinct states.

Switching from a sample to a Camera, changing processing mode, or attaching a View may change load and requirements. Seamless changes are not assumed. Gecko must report what can be applied immediately, what requires a restart, and which Build results need to be revalidated; exact rules have not been selected.

## 4. Placement, Constraints, and Interaction

### 4.1. Composition and Execution Boundaries

Gecko's target capability is to expose the Mesh and obtain compatible App options for a task, including Point implementations, Line placement, and Block providers. Where options are built, who accepts the request, and which part an agent performs are open questions. Available now is distinguished from requiring preparation and an unsatisfied dependency. This information, selection actions, and validation results are available regardless of interaction mechanism. Selecting an option records the composition; manual assembly and constraint changes are also available.

Nodes automatically perform supported checks of paths between them to assess bandwidth and latency. Planning considers stream requirements and measurement freshness; an unchecked path is not assumed suitable. Measuring one connection does not guarantee bandwidth for all simultaneous streams or reserve resources. Current resource availability and dependency readiness are rechecked before startup.

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

### 4.2. Node Participation Characteristics

App composition depends on performance and on a node's willingness to participate under a particular load profile. At least the following scenarios must be distinguished:

| Participation Characteristics | Implication for App Composition |
|---|---|
| Permanent node | A candidate for sustained operation if required capabilities and resources are confirmed |
| Powerful node for bounded tasks | Can perform heavy tests or individual jobs, but continuous load is not assumed |
| Intermittently available node | Can help with a task, but future availability is unknown |
| Node used only for user interaction | Is not assumed to participate in computation merely because it is present in the Mesh |

These are participation scenarios, not approved mutually exclusive node types. They may overlap and change. Declared willingness to operate continuously differs from observed availability and a guarantee of uninterrupted execution. Who sets participation policy, what the node declares itself, and how planning validates these conditions remain undecided.

### 4.3. Line and Block Execution Policies

Minimum latency (`zero latency` as a name for the intent) and maximum throughput (`max throughput`) depend on settings of Lines and the Blocks they use: queues, batching, frame processing, and resources. Absolute zero latency is not a system promise.

For tasks with many cameras or SGIE, batching is an inference requirement. Requests from multiple cameras are batched through Triton, the Block inference provider. Inference Points access it through their Lines' Anchors; batching settings on the Triton side belong to the Block provider. A Line's selected policy must therefore be compatible with the model's capabilities and the shared provider's execution mode, including batch size and the permitted wait for batch formation.

Policy affects connection requirements, Blocks, Anchor conditions, and Build criteria. Candidate selection and validation account for the combined load of all Lines using a Block and the effects of batching on latency and throughput. Testing a single Line is insufficient to assess a shared provider's execution mode.

Switching after App readiness depends on the Line and Block provider implementations: some settings may change while running, while others require rebuilding, restarting, or new placement. Changing a shared Block's execution mode requires accounting for all its consumers, including other Apps. Policies, parameters, and revalidation rules remain to be defined with the [Line / Point / Edge](3-line-point-edge-view.md) and [Block / Anchor](4-block-anchor.md) layers.

### 4.4. Anchors and Block Preparation

Required contracts and Anchor conditions are described in the App candidate so its feasibility can be checked. An Anchor describes a dependency; it is not an installable service. A Block provider may already be running or may require preparation; startup rules belong to the [Block / Anchor](4-block-anchor.md) layer.

Required providers must be ready before tests or operational execution that use them. Preparation follows the selected Build plan's dependencies. When a provider is created, who manages it, how it is reused, and when it is released are separate decisions; preparing every potential Block during Postinit is not required.

### 4.5. Agent Systems

An agent uses the same capability declarations, validation results, and App operations as a human. Agent interaction alone does not confer extra permissions, resources, or guarantees. The shared contract must support a programmatic cycle of “propose an option → test → observe → modify.”

An agent may be able to explore hypotheses faster, but its place in transforming a task into an App plan has not been selected. It is undecided whether it is an optional assistant, a separate planning participant, or part of a supported implementation of option generation. Experiment limits, conditions for starting Build, and result selection criteria also need definition.

### 4.6. Responsibility for an App

Decentralized fact exchange does not automatically determine who accepts an App change, stores its current revision, and coordinates Build or startup. Building options and managing a selected composition are distinct responsibilities.

A coordinator for an individual App remains one possible solution, not an agreed part of the model. A main Gecko for the entire Mesh, leader election, a separate App construction service, and automatic control transfer have not been established. Responsibility and behavior under concurrent changes, participant loss, and partially completed operations must be defined.

## 5. Target Workflow and Early Versions

1. The user specifies a task, inputs, and constraints, for example by providing a video to test an idea. Gecko instances must already be running on available machines and platforms; the need to start additional participants is reported as a condition of the selected option.
2. Gecko exposes discovered Mesh participants, their Devices, Cameras, Axes, known Point implementations, running Block providers, and service preparation capabilities. For options using connections between nodes, it automatically checks supported paths and saves bandwidth, latency, measurement time, and conditions.
3. App candidates are built from Mesh facts and the task, with compatible Point implementations, Line composition, Edges, Anchors, Block providers, available data and actions, Views, and placement. The proposal mechanism and agent role remain open.
4. For each option, Gecko reports what is ready now, what requires preparation, and which dependencies are unsatisfied. The user selects an option or changes constraints and composition; the App saves the selected composition and bindings.
5. Perform Build for the selected candidate: refine connections, test Points, prepare participants in dependency order, and validate Anchors and the whole App. Node participation and execution policies are part of the test conditions.
6. Save the selected composition and Build results. If required preparation is unavailable, report the unmet condition; such an option is not ready.
7. At the App stage, recheck readiness before starting Lines and provide results, data, and traces. The user can attach Views and select supported inputs, including samples and Cameras.
8. Change processing and repeat evaluation. Gecko reports change scope and required revalidation. A Mesh change may produce new proposals, but does not rebuild a running App by itself.

This is the target scenario, not a claim of an existing implementation. Early versions may limit tasks, implementations, and preparation mechanisms; manual selection remains available. The initial practical scenario is a file or Camera and one Line with local processing, or shared inference through a Block. This scenario can proceed without waiting for a global Postinit. Selection before startup does not promise migration or failover while running.

## 6. Mesh / App Requirements for Lower Layers

These are expected domain capabilities on which Mesh / App relies, not a list of reopened architecture questions. The respective layers define their contracts and implementations. This list specifies required outcomes. Broker role and requirements are addressed separately in section 1.2.

| Lower Layer | Requirement | Purpose for Mesh / App |
|---|---|---|
| **[Gecko / Axis](2-gecko-axis.md), Devices, and Cameras** | Each Gecko exposes its own resources, sources, capabilities, and access conditions through addressable references | Build a picture of available execution and select a particular binding; global camera identification is not a prerequisite |
| **Gecko / Axis** | Publishes discovery results progressively, reports completion and limitations, and distinguishes unknown, unavailable, and confirmed information | Use partial information during Init and evaluate known participant readiness during Postinit |
| **[Line / Point / Edge](3-line-point-edge-view.md)** | Expose known implementations, inputs and outputs, requirements, supported connections, and execution constraints | Check App candidate feasibility and potential placement without creating every participant in advance |
| **Gecko / Axis and Line / Point / Edge** | Provide resource availability, connection measurements, and supported execution policies with their usage conditions | Evaluate options and account for node participation, latency/throughput, and Build results |
| **[Block / Anchor](4-block-anchor.md)** | Expose contracts, available providers, preparation capabilities, readiness, and dependency conditions | Select providers and include preparation and validation in the Build plan |
| **Gecko / Axis, Line, and Block** | Provide supported validation, preparation, startup, shutdown, and change operations, with results and limitations | Execute the selected plan, observe progress, and handle partially completed operations |
| **[Point / View](3-line-point-edge-view.md)** | Provide control/view/trace, observable results, and presentation binding conditions | Evaluate hypotheses and use an App through a shared contract for a human or an agent |

For example, Camera declarations from Gecko A and Gecko B remain information relative to their respective nodes. Mesh / App uses them to select a source and connection mechanism. Global camera identification is not required to publish this information.

## 7. Mesh / App Architecture Decision Queue

Decisions should be made in the following order, relying on the requirements in section 6. App responsibility is defined first; specific operations can then be assigned. Options in this queue are not yet selected architecture.

| Step | Problem | Decision Output |
|---|---|---|
| **1** | How are task reception, candidate construction, storage of the selected App, and management of its operations assigned? Is an App coordinator needed, who performs that role, and how is it reached? | A responsibility model without a required main Gecko for the entire Mesh; defines which roles are combined or separate, how changes are accepted, and what happens when a managing participant is lost |
| **2** | How does the user specify a task and success criteria; who turns them into a candidate, scores options, and selects one? How does an agent participate? | One complete scenario from task to selected composition, with responsibilities assigned according to step 1; a manual path remains |
| **3** | How does composition selection account for node participation, resources, and latency/throughput goals? Which set of known participants is sufficient? | Admission and placement selection policy based on lower-layer declarations, with explicit candidate constraints |
| **4** | How does the participant responsible for an App organize Build: preparation dependencies, tests, reservation, combined load evaluation, and failure cleanup? | A Build plan with operation assignments, progress, results, and readiness criteria |
| **5** | How is a ready App used and how are inputs, Views, policies, and composition changed; which changes require revalidation or another Build? | App operation rules and a reproducible hypothesis evaluation cycle accounting for lower-layer capabilities |
| **6** | When are automatic recovery, control transfer, migration, and balancing needed? | An explicitly selected automation scope; basic failure behavior is defined already in steps 1 and 4–5 |

The discussion can use an example with one permanent node, one node for temporary computation, a file or Camera, a detector, and a result accessible to both a human and an agent. Nodes already publish capabilities through a pub/sub broker meeting section 1.2 requirements. This example first establishes the responsibility model, then assigns candidate construction, Build, and repeat evaluation. Domain contracts are developed in lower layers according to section 6 requirements; exchange mechanisms belong to the broker.

## 8. Vocabulary Changes

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
