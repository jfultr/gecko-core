# Mesh and App: Architectural Model and Implementation Versions

Document version 0.46 · October 28, 2026

[Glossary](../0-concepts/index.md) · [Goal and development cycle](../0-concepts/index.md#12-development-cycle) · [Plan progress](../development-cycle.md) · [Gecko / Axis](../2-gecko-axis/model.md) · [Line / Point / Edge / View](../4-line-point-edge-view/model.md) · [Block / Anchor](../3-block-anchor/model.md)

## 1. Purpose, Boundaries, and Status

This document defines Mesh / App entities, responsibility, preparation, execution, and observable state. Versions **1.0.0 / 2.0.0 / 3.0.0** denote implementation scope; version **0.46** is the document revision. The text describes intended behavior rather than implemented capabilities.

**1.0.0 — reduce feature delivery latency on one Gecko.** A person, agent, or program can discover capabilities, specify a local composition, validate and prepare it, obtain an observable domain result, change configuration, and repeat validation. App and a minimum Mesh layer are present from the first version. One Gecko can use multiple Devices, execute multiple Lines, and access supported network Cameras and external services. Distributing the managed composition across Gecko instances is outside this version.

**2.0.0 — distributed execution direction.** The core interaction mechanism for multiple Gecko instances extends the feature delivery cycle across available devices. An App can use several nodes; composition and placement are selected explicitly. Discovering a new node expands options without itself restructuring a running App. Concrete distributed architecture is refined later.

**3.0.0 — dynamic Mesh and product operation direction.** Gecko arrivals and departures may inform changes to App composition, placement, and responsibilities. Failure reactions, migration, redistribution, and a persistent Observer are considered. Automation, seamless transitions, and recovery guarantees are not established yet.

Feature delivery latency is defined in [concepts, §1.2](../0-concepts/index.md#12-development-cycle). The first verifiable result is an intermediate point on the path to functionality ready for its target environment; frame-processing and network-message latency measure different properties.

**Status of 1.0.0 decisions.** This document establishes Mesh / App boundaries and required behavior. [Requirements on lower levels](lower-level-requirements.md) are separate for the next step. Under the [development cycle](../development-cycle.md), requirements are refined downward to lower-level contracts. After turning upward, run experiments and write code, ascending step by step. Mesh / App implementation formats, mechanisms, and criteria are made concrete on returning to this level using the experience obtained. The [separate return-question list](../development-cycle.md#2-remaining-decisions-needed-to-make-100-specific-enough-to-implement) is not a completion condition for the current downward pass.

In each topic subsection, 1.0.0 defines local-model obligations, while 2.0.0 and 3.0.0 define directions. Domain behavior requirements are refined in corresponding lower layers. The [history](history.md) contains earlier decisions, including superseded ones; it does not expand version scope established by the main text.

## 2. Mesh: Participants, Facts, and Exchange

### 2.1. Mesh Model

**Mesh is a decentralized system for exchanging facts and capabilities among Gecko instances.** A single main Gecko is not a mandatory part of the general model.

**1.0.0.** Implement a minimum declaration namespace and API for one Gecko. It exposes identity, state, Axis results, Devices, Cameras, and supported processing and service implementations. A client obtains published state and observes updates through the accepted Zenoh pub/sub infrastructure. Identities and declaration ownership by Gecko are preserved so future expansion does not require redefining local entities. One Gecko operates independently of others being present.

**2.0.0 — direction.** Declaration exchange among Gecko instances, a shared picture of available capabilities, and its use for distributed Apps.

**3.0.0 — direction.** Mesh membership changes during operation and use of new facts to reconsider a running composition.

### 2.2. Facts and Information Limitations

**1.0.0.** Hardware presence, an available implementation, current resources, and execution readiness are separate facts. Unknown or incomplete information differs from a negative check result. A resource reference contains **Gecko + resource**; even the local version preserves a resource's association with a particular Gecko.

Gecko obtains environment facts and measured capabilities through Axis. Axis is the checking mechanism; a Device, Camera, implementation, or connection is its subject. An individual Axis run has its own identity, parameters, progress, results, and blockers. A check can be invoked directly through Gecko's API without an App. A new request performs the check again; retrying an accepted request with its previous `request_id` returns the previous operation (§8.2).

| Information group | Local planning need |
|---|---|
| Hardware and environment | CPU, accelerators, OS, drivers, and access conditions |
| Current resources | Available memory and disk space |
| Measured characteristics | Supported compute test and access check data |
| Tested load options | Results for a particular load under stated conditions |

A measurement is associated with its subject and check run, conditions, success criterion, and freshness. Separately tested loads are not considered tested together. Results do not reserve resources or guarantee suitability of an arbitrary composition. The concrete minimum check set and contracts are refined in Gecko / Axis; no universal additive Point cost is introduced.

**2.0.0 — direction.** Comparing facts from different Gecko instances and checking paths between nodes. Declarations of one physical Camera by different Gecko instances may remain different access paths; global resource identification is not mandatory.

**3.0.0 — direction.** Updated facts as grounds for assessing whether App restructuring is needed; freshness and consequences of changed conditions are refined through operational experience.

### 2.3. Discovery and Axis Work

**1.0.0.** Gecko may start preliminary Axes at startup; their results may lead to subsequent checks. Discovered Devices and other results become available as they arrive. Progress and result publications belong to particular runs. Gecko state lets a client understand whether Axis work is currently in progress; its absence does not mean every possible fact has been collected or the environment is ready for the selected App.

A client can connect during or after preliminary checks, obtain facts already available, observe new results, and invoke an individual Axis for its task. Blockers, errors, and uncertainty remain visible; a run ending with a blocker does not mean Axis work is still in progress. Init and Postinit describe discovery but do not prescribe mandatory completion of all checks, a list of every preliminary Axis, or admission conditions for Build and Activation. The selected composition needs sufficient facts, followed by a separate current readiness confirmation before Activation.

**2.0.0 — direction.** Comparing current Axis work and available facts for a selected Gecko set. Unavailable participants must remain visible; available information can be used independently.

**3.0.0 — direction.** Revising the participant set and running new checks without resetting applicable results already obtained. A node's arrival must not automatically make previous facts false.

### 2.4. Broker and Minimum Requirements

**1.0.0.** Broker denotes a communication infrastructure role and does not appoint a main Gecko or Planner. Minimum integration must provide addressing for declarations and operations, retrieval of current published state, change subscriptions, and explicit disclosure of unavailable information. A client must be able to resume observation after reconnecting.

Zenoh is selected as the exchange foundation; concrete SDKs, keys, storage, and settings are validated during implementation (§8.6). The `gecko/**` namespace illustrates publication ownership without approving a final schema. Message delivery does not prove resource readiness or successful operation execution. No custom retransmission mechanism is designed in advance.

**2.0.0 — direction.** Node-to-node exchange, participant presence, state retrieval after network interruptions, and distributed guarantee boundaries.

**3.0.0 — direction.** Integrating exchange with dynamic membership and long-term Mesh operation requirements.

## 3. App: Composition and Boundaries

### 3.1. App Purpose

**1.0.0.** App defines a local task's composition and shared access to its capabilities, actions, and results. Its description may exist before preparation; having a description does not confirm readiness or execution. App is not a stage name.

**2.0.0 — direction.** Shared composition and access to results of parts executed by several Gecko instances.

**3.0.0 — direction.** Preserving task meaning and result access while App composition and placement change.

### 3.2. Identity and Description

**1.0.0.** An App has a stable identity separate from Gecko identity. An App Revision is a recorded version of the composition, configuration, placement, and dependency description. A revision does not confirm preparation or startup. After App deletion its identifier is not reused; old requests are rejected without recreating the App. Request record rules are in §8.2; deletion procedures are refined on the upward pass.

**2.0.0 — direction.** A revision describes participants across Gecko instances while preserving independent App identity.

**3.0.0 — direction.** App identity survives supported composition and placement changes; configuration transition protocols are defined later.

### 3.3. Dependency Graph

**1.0.0.** An App contains one or more local Lines, Points, supported Edges, Anchors to Blocks, and View bindings. Dependencies form a graph: several Lines may share a Block or Device. One Gecko does not imply one Line or Device.

Only **built-in Points** shipped with Gecko are used. An implementation's presence does not prove compatibility or readiness. A Line has its own requirements and combines those of its Points and internal Edges. An Anchor describes a dependency on a Block contract; a Point performs calls. External provider requirements do not become a Line's local hardware requirements.

```text
Gecko local
App inspection
├── Line camera: Source Point → Inference Point → Output Point
│   └── Anchor inference → Block detector
└── Line archive: Recorder Point
    └── Anchor storage → Block recordings
```

Concrete built-in implementations, connections, and contracts for the end-to-end scenario are selected during lower-layer design; their validated set returns to Mesh / App.

**2.0.0 — direction.** A distributed graph and implementation and provider selection across nodes. An external Point source may expand the set; installed implementations must be distinguished from those available to obtain.

**3.0.0 — direction.** Changing a running App's graph and dependencies in response to new capabilities and failures.

### 3.4. Execution Boundaries

**1.0.0.** All App Lines are placed on one Gecko. Each Line executes entirely there within its own execution boundary. Supported network Cameras and external Blocks are allowed: using an external service does not imply managing another Gecko or owning the provider process. Managed provider preparation is limited to supported local operations.

**2.0.0 — direction.** Placing Lines and managed providers across Gecko instances. Processing between nodes uses supported Edges between Lines or a Block service contract through an Anchor. A Point does not move independently of its Line without an explicit structural change.

**3.0.0 — direction.** Placement changes preserving the Line boundary; migration and seamless transitions require independent decisions and tests.

## 4. Responsibility and Management

### 4.1. App Responsibilities

**1.0.0.** The Planner is external: a person, agent, or their tools construct compositions, select calls, and assess results. No separate Planner inside Gecko is required.

| Responsibility | 1.0.0 scope |
|---|---|
| App Authority | The local Gecko that created the App stores its revision, atomically checks the base revision, and accepts the next one |
| Coordinator | The same Gecko coordinates Activation, Stop App, and explicitly requested overall cleanup of local participants |
| Executor | Describes actual execution; no separate assignment is required |
| Interaction | Describes an interaction channel; client disconnection does not itself stop processing |

Responsibilities do not require separate classes, services, or role-management UI and grant no additional access rights. Coordinator does not execute Build for the Planner. There is no persistent Observer in 1.0.0.

**2.0.0 — direction.** Execution and interaction split across Gecko instances, coordination of shared distributed operations, and a defined change-acceptance party. Concrete responsibility distribution is refined later; persistent Observer remains outside this version.

**3.0.0 — direction.** Persistent Observer, possible responsibility reassignment, and control transfer when Mesh membership changes. Automatic authority transfer is not yet promised.

### 4.2. Creation and Change Acceptance

**1.0.0.** An App is created through Gecko on the user's machine. That Gecko is the sole App Authority and performs shared local operations. It atomically checks the base revision; a change based on a stale revision is rejected.

A new revision is accepted after reliable local persistence. The description acceptance operation receives `succeeded` after confirmed publication of the accepted revision in the App namespace. Acceptance, publication, and actual application are separate facts. If publication is not yet confirmed, the report says so. After restart with preserved state, Gecko publishes the same accepted revision with its former identity; this does not automatically prepare or apply it.

The Planner selects and orders calls. The receiving participant checks each request's admissibility and local resource limits; the system does not repair contradictory Planner intentions. **No global App lock, universal queue, or one-mutating-operation rule is introduced.** An unknown operation outcome does not imply the whole App is busy or permit blindly repeating actions. `request_id` governs safe retries (§8.2).

When Gecko is unavailable, new changes through it are impossible. Execution independence from UI does not promise continued processing if the runtime or machine fails. Persistence, publication confirmation, and domain operation admission mechanisms are refined through the development cycle.

**2.0.0 — direction.** A defined Authority accepts distributed description changes, participants apply them, and behavior during managing-node unavailability is defined. Description and actual application remain distinct.

**3.0.0 — direction.** Control transfer and change acceptance in a dynamic environment with explicit authority and recovery rules.

### 4.3. Persistent Observation

**1.0.0.** Current-state reads, subscriptions, diagnostics, and necessary Operation Report retention are available. An assigned persistent Observer, independent observation history, and notification service are outside scope.

**2.0.0 — direction.** Observing available distributed App state through a common contract. Persistent Observer remains deferred; the number of state clients is not determined by the number of Observers.

**3.0.0 — direction.** An optional persistent Observer for history and notifications independently of user connection. Placement selection, observation gaps, return, and possible replacement are considered. Observer count, assignment criteria, retention, and notification delivery guarantees are not fixed yet. Observation alone grants no App modification rights.

### 4.4. Common Interaction Contract

**1.0.0.** Capabilities, checks, operations, and results have the same meaning for a person, agent, or program. Gecko exposes Lines, Points, Edges, Anchors, Blocks, and supported actions. Changes expose required stops or restarts and affected consumers of a shared provider. Rights do not follow from having a View or agent client.

An Axis can be invoked directly without an App. Progress, blockers, and recommendations are available structurally. Domain data is accessible through participant contracts separately from operation reports. A minimum result representation uses the same data; learning the entire execution structure must not be a prerequisite for the first experiment.

**2.0.0 — direction.** The common contract extends to distributed composition checks and management.

**3.0.0 — direction.** The same approach exposes dynamic change proposals and results, history, and notifications.

## 5. Planning and Placement

### 5.1. Task and Composition Candidate

**1.0.0.** A task specifies inputs, expected result, and constraints. An App Candidate is a proposed composition with implementations, configuration, and dependencies; it does not yet confirm readiness. An external Planner constructs it from Gecko declarations and APIs, including manual assembly. Gecko's formal input is a verifiable composition description; understanding arbitrary tasks and automatically enumerating options are not mandatory runtime features.

Reconcile the minimum candidate and revision schema with lower layers and record it on returning to Mesh / App. An agent receives the same checks and constraints as other consumers.

**2.0.0 — direction.** Placement and implementation selection across Gecko instances; automation of recurring sequences based on real scenarios.

**3.0.0 — direction.** Revising a running composition when goals, resources, or Mesh membership change; automatic experimentation limits are defined separately.

### 5.2. Placement Options and Connection Checks

**1.0.0.** A Placement Option describes a possible local placement of a Line or managed provider with Device and dependency bindings. It is not a created participant, reservation, or readiness guarantee. Selection relies on Axis results and domain checks of the selected configuration.

Lower layers validate supported connections and combined-load options. No universal numeric Point cost, additive consumption model, or general placement optimizer is introduced. A user or agent can arrange composite local checks; temporary actions and cleanup need explicit domain contracts.

**2.0.0 — direction.** Inter-node bandwidth and latency checks, coordination of composite check steps, partial execution, and release of temporary resources. A path measurement does not reserve bandwidth or prove combined throughput for all streams.

**3.0.0 — direction.** Reassessing placement and connections to adapt a running App.

### 5.3. Participation Profile: Node Participation Tags

**1.0.0.** A profile is unnecessary for choosing between nodes and is not mandatory local App machinery. Availability, load, and permissions are determined separately.

**2.0.0 — direction.** Free-form hints to the Planner about participation, such as Persistent, Scheduled, Intermittent, and No compute. They describe intended use without guaranteeing availability or replacing ACLs or measurements. Tags and how they are set are refined later.

**3.0.0 — direction.** Practical use of participation characteristics during restructuring and reactions to node departure or failure, without preapproved automatic migration.

### 5.4. Line Requirements and Policies

**1.0.0.** Each supported Line implementation exposes permitted requirements, their scope, validation conditions, and reactions when they cannot be met. The lower layer defines a minimum concrete set of performance, overload, and unavailable-dependency policies. Mesh uses this contract without introducing universal parameters for arbitrary Points or Lines.

Performance Policy denotes performance requirements and preferences; Overload Policy denotes overload behavior. Analysis rate, result latency, processing completeness, stale-frame dropping, and priority can be domain requirements. Numeric values and Latency-first / Balanced / Throughput-first modes are not a mandatory schema or defaults. A requirement must make sense for specific inputs and configuration. Checking one Line does not confirm every Line's requirements under combined load.

Shared Block parameters belong to its provider; changing one Line does not permit implicitly changing service mode for other consumers. Reactions are defined together with Anchor conditions.

**2.0.0 — direction.** Checking compatibility of distributed Line, connection, and shared-provider requirements using real measurements.

**3.0.0 — direction.** Using policies to select composition and placement changes during operation.

### 5.5. Block Contracts and Anchor Conditions

**1.0.0.** An Anchor describes the required contract and dependency conditions rather than a service to install. Before Build, distinguish a running provider, a known preparation method, and an unsatisfied dependency. Neither a provider's presence nor a preparation method alone confirms suitability or readiness. Not finding a provider does not prove the contract cannot be supplied.

Minimum declarations, compatibility and availability checks, and failure behavior are defined in Block / Anchor. For an external service, explicitly expose the absence of lifecycle management if Gecko can only call it.

**2.0.0 — direction.** Selecting and checking providers across Gecko instances and external services.

**3.0.0 — direction.** Replacing providers and changing running App dependency bindings under defined transition conditions.

## 6. Preparation, Activation, and Use

### 6.1. Build: Planner-Driven Build

**1.0.0.** Build is preparation of a selected revision through separate external Planner calls. Participants perform supported actions and provide reports. The Planner selects order and subsequent steps, compares results with the task, and assesses preparation sufficiency.

Build Plan is not a separate system entity. Build receives no `request_id`, `operation.id`, mandatory overall Operation Report, or execution state of its own. Preparation calls have their own identities and need not form a tree rooted at Build. Coordinator does not accept a preparation graph for execution or repair Planner calls.

Preparation can include resource, implementation, connection, and dependency checks, local participant creation, and supported test runs. A preparation operation can execute without an App. Its report exposes configuration, actual actions, limitations, blockers, acquired resources, and available next actions. A temporary test does not mean operational App activation; its conditions belong to the domain contract.

A failure does not automatically trigger overall rollback, cancellation of other calls, or cleanup. Accepted operations follow their own contracts. The Planner selects subsequent calls; resources are retained or released under local lifecycle rules. Sufficient preparation does not permit operational startup: Activation is required.

**2.0.0 — direction.** Separate preparation calls to participants across Gecko instances, preserving domain results and limitations.

**3.0.0 — direction.** Preparing running App changes when Mesh changes; whether preparation can coexist with execution is refined per implementation.

### 6.2. Preparing Block Providers

**1.0.0.** If Gecko can prepare a provider, it exposes a direct API operation with progress and results, also available outside an App. The Planner calls it according to dependencies; discovery need not prepare every possible Block. An already running external service can be used without authority to manage startup.

A Block's participation in an App does not confer exclusive provider ownership. Stopping one App must not stop a service shared by other consumers. A preparation report distinguishes creation, reuse, resources actually acquired, and unfinished actions. Ownership, reuse, and release rules are refined in Block / Anchor.

**2.0.0 — direction.** Preparing providers through selected Gecko APIs and using shared services in a distributed composition.

**3.0.0 — direction.** Supported provider replacement and relocation considering consumers and continuity requirements.

### 6.3. App Readiness: Readiness Profile in the Operation Protocol

**1.0.0.** App Readiness assesses whether a particular revision is ready for a specified operation in a given context. A participant owns internal checks of its part. Mesh / App uses a common profile without managing the internal check list.

| Field | Meaning |
|---|---|
| `status` | `ready`, `not_ready`, or `unknown`; domain readiness, separate from operation `state` |
| `checks` | Results and evidence of performed checks |
| `validity` | Context, freshness, and conditions requiring a new check |

An applicable `not_ready` from a mandatory participant gives overall `not_ready`. Without such a blocking result, a missing, stale, or `unknown` mandatory answer gives `unknown`. Overall `ready` requires current applicable `ready` from every mandatory local participant. Limitations are retained; an optional check does not replace a mandatory conclusion.

Before Activation, Gecko makes one new explicit readiness request to each mandatory local participant with bounded waiting. Old publications are insufficient. If readiness is unconfirmed, activation is not performed; the report retains reasons and unknown parts. There is no automatic wait for fixes or repeated polling. A fix requires a new explicit Activation attempt.

Readiness does not reserve resources or guarantee conditions remain unchanged until startup. A successful request can return `state: succeeded` with `status: not_ready` or `unknown`. Domain check sets, `validity`, and waiting periods are refined from the bottom upward.

**2.0.0 — direction.** Obtaining and combining readiness from participants across Gecko instances, accounting for connectivity and freshness.

**3.0.0 — direction.** Checking readiness of running App changes and possibly optimizing repeated checks using validated freshness guarantees.

### 6.4. Activation: Permission for Operational Execution

**1.0.0.** Activation is a separate operation permitting operational execution of the selected revision after a fresh readiness check (§6.3). Preparation and temporary tests do not grant this permission. Operational startup is based on a decision published as a complete record in the App namespace; local participants confirm actual application through `application` (§8.4).

Success requires confirmation from every mandatory local participant. Partial failure or an unconfirmed result is exposed per participant; a timeout does not prove startup did not occur or revoke a published decision. There is no automatic rollback. A decision, its application, and the first domain data appearing are separate facts.

After the managing component restarts with preserved state, distinguish three cases:

| Decision publication outcome | Local Gecko behavior |
|---|---|
| Confirmed not published | No automatic continuation; a new explicit attempt with new identifiers checks readiness again |
| Confirmed published | Establish actual application results for the previous decision; resuming observation creates no new Activation |
| Unknown | Expose uncertainty; establish facts without blindly granting permission again |

Retrying an old request returns the previous operation without resuming domain actions. The accepted-description publication recovery rule (§4.2) does not automatically apply to unpublished startup permission. Before a new start after restart, an executor checks current permission rather than only a historical record (§7.3). Concrete confirmation and recovery mechanisms are validated on the upward pass.

**2.0.0 — direction.** Distributed startup permission and application acknowledgments, including partial startup and unavailable Coordinator. A common decision does not imply physically simultaneous startup.

**3.0.0 — direction.** Activating changed composition and transition rules during dynamic restructuring; seamless behavior is not promised in advance.

### 6.5. Using and Changing a Composition

**1.0.0.** Video, detections, and other domain results are available through participant contracts, including without a screen. A View displays these data, diagnostics, and available actions. Connecting a View does not itself start or stop processing. When combining images and detections, the contract relates them to the corresponding frame. Operation Report describes operation execution rather than replacing the domain data stream.

Accepting or preparing a new revision does not implicitly switch or restart running processing. **An explicit Stop → preparation → Activation cycle is sufficient for composition changes in 1.0.0.** Supported parameter changes are defined by domain contracts. Arbitrary concurrent preparation and seamless switching are not guaranteed; if a preliminary stop is needed, the participant identifies the affected part and the Planner chooses further action.

Stop App is an explicit operation withdrawing operational execution permission. The local Coordinator publishes the corresponding decision and collects mandatory participant stop acknowledgments. Request acceptance or decision publication does not prove stopping occurred. Overall success requires actual acknowledgments; continuing and unknown parts are exposed separately.

Stop is not cancellation of a particular Operation and does not start overall cleanup. A Block shared by other consumers is not stopped automatically. The next operational start requires a new explicit Activation with a new readiness check. A change report shows initial and actually obtained configurations, restart requirements, completed and unfinished work; accepting a description does not substitute for application.

**2.0.0 — direction.** Accessing distributed App data and explicitly changing composition, including coordinated stopping and reactivation.

**3.0.0 — direction.** Dynamic changes to inputs, composition, placement, and providers; transition guarantees are selected for specific scenarios.

### 6.6. Model Reconciliation and First-Version Boundaries

**1.0.0.** End-to-end validation must follow inputs and local capabilities through composition, preparation, and Activation to an observable result, then change, revalidation, stop, and cleanup. The reference scenario processes a file or Camera using built-in Points, local Devices, and supported dependencies. Multiple local Lines and a shared Block test composition and ownership boundaries when their contracts are ready.

Validation includes a preparation blocker, partial results, safe request retries, restart, client disconnection, and processing independence from a View. Concrete implementations and acceptance criteria are refined when returning to this level under [plan R1–R8](../development-cycle.md). The current downward pass proceeds to Gecko / Axis without first closing that list.

**2.0.0 — direction.** Scenarios with multiple Gecko instances, distributed dependencies, and explicit composition changes when connectivity or participants fail.

**3.0.0 — direction.** Scenarios for node arrival, departure, and replacement, App restructuring, and persistent observation.

## 7. Decisions, State, and Failures

### 7.1. Shared App Publication Namespace

**1.0.0.** An App has an addressable namespace for its description, decisions, and local participant state. A decision is published as one complete record associating its content, operation, revision, and participants. A prepared participant is not declared running merely because preparation finished. Actual application is exposed separately.

Revision persistence, publication, and application differ; the current description revision is not necessarily the executing revision. A client obtains current available state and subsequent changes. The publication namespace does not require persistent Observer or complete history. Authoritative state storage and freshness confirmation are defined during implementation (§8.6).

**2.0.0 — direction.** Decision and participation publications from multiple Gecko instances, state retrieval after reconnection, and distinct fact sources.

**3.0.0 — direction.** Exposing dynamic composition, placement, and responsibility transitions; developing history and observation.

### 7.2. Decision and Actual Execution

**1.0.0.** Description, preparation, readiness, execution permission, and actual results differ even on one Gecko. A shared operation on several local participants can complete partially. The report retains each result; one failure does not prove the others stopped. There is no automatic rollback or general ban on new requests due to an unknown outcome; participants determine action admissibility.

**2.0.0 — direction.** The same fact distinctions when applying decisions across Gecko instances, accounting for delivery, partial execution, and unavailable participants.

**3.0.0 — direction.** Observable transition state between App compositions and actual recovery or redistribution results.

### 7.3. Unavailability and Recovery

**1.0.0.** Basic error handling is mandatory. After restart and before a new operational start, a local executor establishes current permission and its associated revision. An old Activation and the latest accepted description revision alone are insufficient: Stop may have occurred. If permission is unconfirmed, a new start does not begin and uncertainty is exposed. This does not promise automatic resumption; other conditions belong to the domain contract.

Client disconnection neither cancels accepted operations nor stops the App. Failure of Gecko itself, a Line process, or an external dependency is considered separately according to actual state. Dependency Failure Policy denotes unavailable-dependency behavior; a Line's domain contract defines its concrete reaction considering Anchor conditions. Automatic recovery on another node is outside this version.

Cleanup is invoked explicitly: directly on an individual participant, or through local Coordinator for preparation of the selected composition. `acquired_resources` links it to particular preparation results but grants no right to delete shared resources. Participants determine ownership, safe release, and local lifecycle. Failed Activation does not trigger immediate overall cleanup or promise indefinite resource retention.

| Overall local cleanup result | `state` |
|---|---|
| A mandatory part is confirmed unsuccessful | `failed`; other part states are preserved |
| Release of everything requested is confirmed | `succeeded` |
| No confirmed failure; actions or waiting continue | `running` |
| No confirmed failure, but the outcome cannot be established after waiting | `unknown` |

The report shows released, remaining, in-progress, and unknown resources. Partial cleanup is not rolled back. For the remaining scope, the Planner makes a new explicit request with a new `request_id`; retrying the old request returns the previous operation. Unknown outcomes are first investigated from available facts; local contracts check the next action's admissibility. Incomplete cancellation is not declared overall cancellation (§8.2).

**2.0.0 — direction.** Operation when the managing Gecko, remote executor, connection, or shared provider is lost; resuming observation and explicit actions after partial results. Basic distributed error rules must be defined before implementation.

**3.0.0 — direction.** Automatic reactions, participant replacement, migration and redistribution, and possible control transfer. Guarantees and boundaries are established through experiments and operational scenarios.

### 7.4. Deferred Capabilities

**1.0.0.** Built-in Points, local management, and a minimum common contract are used. The local API access boundary and allowed-action validation must be defined; user identity mechanisms and concrete access controls are refined during implementation. Having UI, a role, or agent interaction grants no additional authority.

**2.0.0 — direction.** Inter-node identity, trust, and access rights. A Point hub is a possible extension: developing and testing external implementations, obtaining Points, and feedback. The hub is not a prerequisite for multi-Gecko interaction; its architecture and protocols are not fixed.

**3.0.0 — direction.** Dynamic Mesh, App restructuring, failure reactions, possible migration and load balancing, and persistent Observer. Automatic recovery and seamless behavior do not follow from a version name and require separate validated decisions.

## 8. Operation Protocol

### 8.1. Purpose and Core Concepts

**1.0.0.** Operation Protocol defines common access to Axis diagnostics, participant preparation, readiness, Activation, Stop, cleanup, and supported changes. Operation Report exposes state, actual results, blockers, and explanations for a person, agent, or program. An individual operation requires neither App nor Coordinator; Build is not an operation type.

A request expresses intent, Operation is a particular execution, a decision permits an action where required, and Operation Report is observable progress and outcome. Recommendations are separate from completed actions and are not executed automatically.

[ROS 2 Actions](https://design.ros2.org/articles/actions.html) remains a domain-model reference: request, distinguishable execution, intermediate feedback, and result. This is neither a ROS 2 dependency nor adoption of all its rules; Gecko transport is Zenoh. Contract formats are refined through the development cycle.

**2.0.0 — direction.** Applying the common core to operations involving participants across Gecko instances and distributed acknowledgments.

**3.0.0 — direction.** An App restructuring and recovery operation protocol; universal resumption of arbitrary actions is not promised in advance.

### 8.2. Operation Report Core

**1.0.0.** This core applies to operations of one Gecko, including composite operations on its local participants. Fields are populated where applicable; a field's existence does not require every operation to create an App, child actions, or resources. Semantics apply from the local version onward; schemas and mechanisms are refined from lower-level results.

**Accepted:** implement common Operation mechanics as an [independent foundational module](../core-modules.md#core-op-boundary-and-work) used by Axis and App operations. The module does not depend on their domain entities. [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151) are architectural references rather than runtime dependencies; ROS 2 is not part of the project foundation. This section remains the active common-rule source until a separate module contract is established. Module work may start independently of completing D2 domain design; integration is validated against corresponding contracts.

A report belongs to a particular operation and participant. Several messages can progressively expose one execution's progress and result; these are not separate accepted decisions. A composite operation links its report to participant reports, allowing navigation from an overall problem to concrete diagnostics.

| Field | Meaning |
|---|---|
| `operation` | Object with `id`, `type`, and optional `parent_id`; associates a particular execution's request, progress, and result with a parent operation if present |
| `sequence` | Increasing snapshot number within a particular operation; determines update order |
| `reported_at` | Executor-side snapshot generation time; used for diagnostics rather than replacing `sequence` for ordering |
| `subject` | Participant and operation subject, including configuration or revision where applicable |
| `context` | Inputs, parameters, dependencies, and App association if the operation relates to an App |
| `state` | Execution state: `running`, `succeeded`, `failed`, `cancelled`, `unknown`; does not replace participant readiness or decision application results |
| `progress` | Object with `stage`, `message`, and optional numeric `completed`, `total`, `unit`; exposes the executor's available progress assessment |
| `result` | Data actually obtained and actions performed, including partial results; domain content is defined in the relevant section |
| `acquired_resources` | What preparation acquired, created, or reserved, including before failure; not an estimate of future consumption |
| `blockers` | List of objects with `code`, `subject`, `message`, `evidence`, `next_actions`: obstacles and possible remedies |
| `limitations` | Known limitations and boundaries of the result obtained |
| `explanation` | Substantive explanation of completed work, outcome reasons, and significant context for a person or agent |
| `next_actions` | List of objects with `code`, `message`, `subject`, `instructions`, and optional `request`; recommendations remain separate from performed actions |
| `evidence` | Supporting measurements, Axis results, diagnostic data, and artifacts |
| `children` | Relationships to nested operations and participant reports of a composite operation |
| `details` | Domain information specific to the operation or layer |

Populate applicable report parts. The common core does not require a diagnostic Axis to have an App, create resources, or apply a decision. Significant errors and blockers must be structured; domain details must not be the only place to discover the overall operation outcome.

`acquired_resources` belongs to specific preparation and supports selective or overall cleanup (§7.3). Release results appear separately in `result`: a list of previously acquired resources does not itself confirm cleanup. Using shared or preexisting resources does not mean exclusive ownership. Domain layers define record structures, use/acquisition distinctions, and local release operations. Reporting readiness does not itself imply new resource acquisition.

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

**Accepted: protection against duplicate operation creation survives normal Gecko restart.** The receiving Gecko reliably persists the `request_id` to assigned `operation.id` association, data needed to compare request content, and the accepted operation record before starting domain actions, including sending mutating requests to participants. A normal restart with preserved records does not turn an accepted request into a new one.

After such a restart, retrying with the same `request_id` and content returns the previous `operation.id`, observation address, and available report information; no second execution is created. Retrieving the previous operation does not resume preparation or other domain actions. If the outcome is unknown, Gecko exposes that uncertainty and establishes available actual participant state. A persisted `running` snapshot alone does not confirm execution continues after restart. Uncertainty belongs to a particular operation and does not make the whole App busy; receivers determine new-request admissibility (§4.2).

**Accepted for 1.0.0: records of accepted mutating App requests are retained throughout the App's existence.** The `request_id` to `operation.id` association and request-content comparison data are not automatically deleted by age, including after operation completion. Detailed Operation Reports, diagnostic data, and artifacts may have separate retention periods; deleting them does not delete the accepted request record or permit re-execution.

**Accepted for 1.0.0: minimum records of other accepted requests are not automatically deleted.** This includes standalone Axis runs outside an App. The `request_id` to `operation.id` association and request-content comparison data are retained; age or completion does not turn an old retry into a new execution. Detailed reports and artifacts may be deleted under a separate policy. The deleted-App exception below remains; no automatic minimum-record expiry protocol is introduced in 1.0.0.

**Accepted: an App's request records may be deleted after the App is deleted.** An old request addressed to a deleted App is rejected without new execution or App recreation, regardless of whether the old `request_id` record remains. A missing record does not make the request admissible: the addressed App must exist first. The deleted App's identifier is not reused (§3.2), so an old request cannot reach a new App under the same identity. This retention exception applies to deleted Apps and does not permit age-based deletion while an App exists.

If the detailed report is unavailable, retrying the same request with the same content returns the previous `operation.id` and an explicit unavailable-report indication. No new execution is created; report unavailability alone does not confirm actions have completed. Other requests are checked under domain contracts without a global App lock (§4.2).

**1.0.0 boundary: automatic recovery of completely lost request records is not provided.** Persisted records are a mandatory part of the receiving Gecko's state. Normal recovery after ordinary restart relies on their preservation. Automatically reconstructing a lost `request_id` → `operation.id` mapping from published reports is outside the first version.

Participant reports for particular preparation calls or other operations may remain accessible through Mesh independently of local request records. Observing execution does not itself restore the original user-request association. Lost records do not justify repeating an old request with an unknown outcome; complete record loss is outside normal 1.0.0 recovery.

This protects against duplicate operation creation on retries rather than promising atomic domain actions. Concrete persistence and reload mechanisms, request-content comparison, and concurrent retry handling are defined during integration. They must uphold previous-operation retrieval and changed-content conflict rules. Concurrent retries of one request must not create several executions. Concrete report retention periods follow a separate policy; report unavailability and `sequence` preservation after normal restart are defined below.

**Accepted: minimum Operation API.** Action meaning is defined independently of concrete Zenoh integration keys and mechanisms (§8.6).

| Action | Result |
|---|---|
| **Start** | Acceptance with `operation.id` and observation address, or rejection with a reason; retries follow `request_id` rules |
| **Get report** | Latest available full Operation Report for a particular execution by `operation.id` |
| **Observe** | Subscribe to new full snapshots of this operation, ordered by `sequence` |
| **Cancel** | Cancellation acceptance or rejection; actual cessation is confirmed by a subsequent report rather than the response itself |

Retrieving a report neither creates a new execution, repeats a check, nor resumes an operation. For a running operation, return the latest available snapshot; for a completed one, the final snapshot if available. If the report is unavailable, the API explicitly says so; absence of a report does not prove the operation never ran or is not running. Reading does not change retained results.

**Accepted: detailed reports and artifacts may be deleted under a separate retention policy.** Indefinite availability of full Operation Reports, diagnostic data, and files is not required. Their retention policy is separate from accepted request records, including standalone diagnostic Axis operations. Deleting a report or artifact does not itself delete the `request_id` to `operation.id` association or permit re-execution.

With preserved records, retrying the original request returns the previous `operation.id` and explicitly states that its report is no longer available; no check or other action automatically runs again. An intentional new execution uses a new request. Requests to deleted Apps follow the separate rule above: reject without recreating the App. Concrete report and artifact retention periods and deletion mechanisms are defined during implementation; they do not change retry protection or snapshot numbering preservation semantics.

“Latest available” does not mean guaranteed fresh readiness or current executor availability: a consumer considers `sequence`, `reported_at`, domain freshness, and presence information. The explicit new readiness request before Activation (§6.3) remains required. Retrieving a retained report and starting a new check are different API actions.

Detailed report and artifact deletion and an explicit unavailable response are accepted above. Report source, exact unavailable-response formats, concrete retention periods, authority among multiple sources, and read/subscription reconciliation remain integration details validated under the Zenoh map (§8.6).

**Accepted: full Operation Report snapshots.** Every message contains a full snapshot of the particular operation's available state at generation time rather than only changes since the previous message. A recipient can interpret it without replaying all earlier messages of that execution.

Subsequent snapshots retain results obtained, actions performed, acquired-resource information, and applicable blockers and limitations. A state change or refined conclusion appears in a new snapshot; this does not keep a resolved blocker active. Unavailable or unknown facts explicitly remain unknown rather than being reconstructed by the recipient from a missing message. Large artifacts, nested reports, and supporting materials may be references with clear operation ownership.

Completeness means available state, rather than a full event log or every diagnostic file copied into each message. It does not define serialization, storage under one key, latest-value retention, history, or new-subscriber delivery. Zenoh mechanisms and linked-material availability rules are separate. `sequence` determines update order and newest snapshot selection as below.

**Accepted: snapshot order and time.** The executor producing Operation Report increments `sequence` for each new snapshot of one operation. Redelivery of the same snapshot preserves its number. A recipient uses the greatest known `sequence`; a late or duplicate snapshot with a lower or equal number does not replace current state. Different content under one number is not a valid new update.

**Accepted: snapshot numbering survives normal Gecko restart.** The executor persists `sequence` with its report snapshot. Republishing the same retained snapshot preserves number and content; a new snapshot of the same operation receives a greater number. The counter does not reset after restart: after snapshot 12, the same 12 may be republished, while a new result clarification receives the next number. Report clarification neither resumes domain execution nor creates a new execution. Concrete snapshot and number persistence and loading mechanisms are defined during integration and must preserve this order.

Each nested operation has its own `operation.id` and counter. Coordinator separately numbers summary snapshots of its composite operation. Numbers of different operations are not compared; `parent_id` and `children` relate them without a global counter.

`reported_at` is snapshot generation time at the executor, rather than subscriber receipt time. Order follows `sequence` and requires no synchronized Gecko clocks. Message generation time does not replace freshness conditions in `validity`. Number preservation after normal restart is accepted above; exact time format, initial counter value, and persistence mechanisms remain to be defined.

**Accepted: operation identity.** `operation` is an object with these fields:

| `operation` field | Meaning |
|---|---|
| `id` | Identifier of a particular operation execution |
| `type` | Operation kind: Axis check, participant preparation, readiness check, Activation, Stop App, cleanup, App change, and other supported kinds; exact values remain undefined. Build is not a separate operation kind |
| `parent_id` | Optional parent-operation reference when the execution belongs to a composite operation |

A request and all progress and result messages of one execution share one `operation.id`. A new run after a fix receives a new identifier. A standalone operation has no parent. For example, an Axis may invoke a child Axis within a local composite task; the child has its own `id`, and `parent_id` references the parent run. The parent report may refer to these calls through `children`. Separate Planner calls preparing an App need not share a Build parent.

The receiving Gecko creates the identifier and returns it on acceptance. Identifier generation, uniqueness scope, `parent_id` transfer, and concrete Zenoh keys remain undefined. `parent_id` relates executions without granting new parent authority or guaranteeing composite-operation atomicity.

**Accepted: `next_actions` structure.** The field contains a list of proposed actions. The same structure is used at report level and inside an individual blocker.

| Proposed action field | Meaning |
|---|---|
| `code` | Machine-readable action name; concrete values belong to the domain layer |
| `message` | What is proposed and why |
| `subject` | Participant or resource the action concerns |
| `instructions` | Detailed execution instructions and conditions for a person or agent |
| `request` | Optional prepared API request description when the action is available through Gecko |

For example, installing a missing driver can be described as instructions for an agent using OS tools, without `request`. A subsequent Axis check may contain a prepared Gecko request description. Concrete `request` schemas, API parameters, and condition forms are refined separately.

A proposed action is not an accepted decision, issued command, or execution confirmation. Execution requires a separate call to the relevant tools or API. Starting a new action through Operation Protocol creates a new `request_id` and, on acceptance, a new `operation.id`. Redelivery of one already sent request preserves its `request_id` under duplicate-execution protection rules. External-tool instructions do not themselves create a Gecko operation.

**Accepted: `blockers` structure.** The field is a list of blocking problems, each represented by an object:

| Blocker field | Meaning |
|---|---|
| `code` | Machine-readable reason code, such as `driver_missing`; concrete codes belong to the domain layer |
| `subject` | Device, participant, or dependency the problem concerns |
| `message` | Obstacle explanation understandable to a person and agent |
| `evidence` | Supporting data or diagnostic and related-report references |
| `next_actions` | Possible remedies and their execution conditions |

For example, an Axis reports a missing NVIDIA driver blocker, attaches diagnostics, and instructs an agent to install the driver and then explicitly start a new check. Recommendations themselves neither authorize nor perform remediation; actual actions appear in corresponding reports.

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

Coordinator may expose a composite operation's stage and link child reports through `children`. Overall completion is not automatically computed by averaging participant progress: durations and work amounts differ. If no overall estimate is defined, do not claim numeric progress. Progress does not replace `state`, readiness, or decision application; reaching the stated amount does not itself confirm successful completion.

**Accepted: execution `state`.** The common core uses these values independently of domain readiness or decision application results.

| `state` | Meaning |
|---|---|
| `running` | The operation is executing |
| `succeeded` | The operation completed successfully |
| `failed` | The attempt is deemed unsuccessful, including due to a blocker requiring an external fix. The overall outcome of an actually composite operation does not replace actual part states |
| `cancelled` | The executor confirmed cessation; a cancellation request alone does not establish this state |
| `unknown` | The observer cannot reliably establish execution state; this does not confirm stopping, failure, or completion |

No `blocked` state waiting for resumption is introduced in 1.0.0. The stopping reason appears in `blockers`; completed work and partial results appear in `result`. After an external fix, create a new run under the rule below.

A successfully executed readiness request may have `state: succeeded` and `status: not_ready` or `status: unknown`: obtaining a conclusion succeeded, but the participant is unready or information insufficient. A blocker in such a conclusion does not make the request itself unsuccessful. Nested operation and participant message `state` values are not replaced by an actually composite operation's overall state. Build is a process of separate Planner calls rather than a separate operation; the Planner assesses overall preparation without a mandatory common Operation Report (§6.1). Failure or uncertainty in one call does not generally prohibit new App operations; receivers determine admissibility (§4.2).

**Accepted: execution cancellation.** Cancellation is requested by `operation.id` from the operation executor. It accepts or rejects the cancellation request; acceptance does not yet confirm cessation. `state: cancelled` is published in a full Operation Report only after confirmed cessation. If an operation has already completed, cancellation does not replace its actual outcome. If an action cannot be interrupted safely, cancellation is rejected with an explanation.

Cancellation neither rolls back completed actions nor triggers automatic cleanup. The report preserves actual actions, partial results, and acquired-resource information; cleanup is requested separately (§7.3). Cancelling observation or a caller-side waiting timeout does not itself cancel execution. In particular, stopping Activation coordination after decision publication does not revoke permission or mean activated processing has stopped.

For a composite operation, Coordinator requests cancellation of active children and collects their actual results. Overall `cancelled` is valid only after confirmed cessation of the composite operation and its active parts. If a part continues, rejects cancellation, or does not respond, the report exposes this and the composite operation is not declared cancelled. Completed child actions are neither rewritten as cancelled nor automatically rolled back.

**Accepted: incomplete cancellation does not replace actual operation state.** The overall report preserves the operation's own state; cancellation results and part states appear separately. While execution is known to continue and the final outcome is not established, the operation remains `running`; if state cannot be established, expose `unknown`. Confirmed `succeeded` or `failed` is not rewritten as cancellation. A part refusing cancellation does not alone determine the entire operation's outcome: it is considered with actual results and overall outcome rules. `cancelled` requires confirmed cessation of the operation itself and all active parts; an already completed actual outcome is preserved.

Request and response formats, concrete safe interruption points, and acknowledgment waiting periods are refined in domain contracts and integration. A common request mechanism does not require every domain operation to support safe cancellation at every stage.

**Accepted for 1.0.0: a new run after an external fix.** If execution cannot continue due to a blocker and requires an external action, the current attempt ends with a report of the blocker and actual results, including partial ones. It does not remain an execution waiting to resume. After a fix, a user or agent explicitly creates a new run with a new identity; no automatic continuation or retry occurs. No universal resume-from-saved-step mechanism is introduced in 1.0.0.

A new run does not imply automatic cleanup of earlier preparation (§7.3). Use of existing state follows the domain contract; an explicit Axis check executes again. A blocker in data does not always mean the operation itself failed: a readiness request can successfully return `not_ready`. This rule concerns ending an attempt requiring an external fix to continue, without conflating `state` and domain results.

The rule does not permit blindly repeating an action with an unknown result: after partial decision application or a lost response, first establish available actual state. Conditions for a safe new request in such cases remain open. An individual Axis ending with a blocker neither hides facts already obtained nor means its execution continues (§2.3).

Readiness (§6.3) and decision application (§8.4) are domain report parts. Readiness preserves agreed `status`, `checks`, and `validity` fields; application uses `application`. Readiness `status` is not a universal operation execution state: `state` is dedicated to that purpose. Final nesting of profiles and the core is not yet selected.

**2.0.0 — direction.** Using the same core for participants across Gecko instances, distributed composite operations, and partial results. Concrete inter-node coordination and retention rules are defined later.

**3.0.0 — direction.** Applying reports to App restructuring and recovery while preserving distinct intent, actual action, and unknown outcome.

### 8.3. Decision and Its Content

**1.0.0.** A decision explicitly permits an action where the contract requires it, such as Activation or Stop. It exposes identity, associated operation, App and revision, participants, action content, accepting party, and basis. Diagnostics and preparation do not themselves permit operational startup.

A complete record is available in the App namespace; a report reference retrieves its content. Copying the entire decision into every snapshot or retaining an indefinite log is not required. Revision acceptance and operational startup permission have different recovery rules (§4.2, §6.4). Concrete schemas and persistence and publication mechanisms belong to return questions for this level.

**2.0.0 — direction.** Addressable decisions for distributed composition and participant rules for obtaining current permission.

**3.0.0 — direction.** Dynamic transition and responsibility-change decisions with explicitly defined authority.

### 8.4. Application: Decision Application Report

**1.0.0.** `application` associates actual application with a particular decision, revision, and local participant. Progress, acknowledgment, error, or an unconfirmed result is exposed with explanation and evidence. `ready` does not mean applied; missing acknowledgment does not prove startup did not occur.

The overall report references mandatory participant results. Activation and Stop success follow confirmed application under their contracts. Record nesting and collection access are refined during implementation; a complete log of all decisions in every response is unnecessary.

**2.0.0 — direction.** Application acknowledgments across Gecko instances and establishing partial results during network failures.

**3.0.0 — direction.** Observable restructuring and recovery results that establish actual App configuration.

### 8.5. Open Protocol Decisions

**1.0.0.** Common core meaning, retry protection, and reliability rules are established here. Exact types, required fields, `type` values, codes, parent-reference transfer, request and reference formats, time, initial `sequence`, retention, safe cancellation points, and numeric waiting periods are refined in lower layers and on the upward pass. Lower layers may define their own domain results while preserving common core meaning.

These questions belong to the [separate R1–R8 return list](../development-cycle.md); completing the entire protocol design is not required before descending to Gecko / Axis. After experiments, check local contract compatibility and record final Mesh / App specifics.

**2.0.0 — direction.** Distributed composite operations, authoritative state sources, and resuming observation across nodes.

**3.0.0 — direction.** Dynamic management and recovery contracts; scope follows real scenarios rather than protocol universality.

### 8.6. Operation Protocol Integration Map for Zenoh

**1.0.0.** Zenoh provides exchange and retrieval of published data; Gecko defines request acceptance, execution, reports, and decisions. A concrete integration profile is not yet selected or tested. Research in an earlier revision is not a test or confirmation that a particular SDK version is current.

| Local-version need | Integration candidate and validation subject |
|---|---|
| Request and acceptance response | Query / Queryable; Operation creation and deduplication remain Gecko logic |
| Full report observation | Publisher / Subscriber; ordinary delivery does not establish durable storage |
| Current report and decision retrieval | Queryable or supported storage; select an authoritative source and explicit unavailable response |
| Late connection and resuming observation | Validate built-in history/cache/recovery capabilities of the selected SDK, their limits, and compatibility |
| Persistence after restart | Validate local storage for request records, snapshots, revisions, and decisions; transport cache does not replace reliable persistence |
| Ordering | Preserve domain `sequence` and snapshot selection rules independently of transport numbering |
| Timeout and cancellation | Distinguish response waiting, execution time, and confirmed cancellation |
| Execution permission | Confirm complete publication, current permission, and actual application by local participants |

Initial reference materials: [Zenoh abstractions](https://zenoh.io/docs/manual/abstractions/), [AdvancedSubscriber 1.10.1](https://docs.rs/zenoh-ext/1.10.1/zenoh_ext/struct.AdvancedSubscriber.html), [filesystem backend](https://github.com/eclipse-zenoh/zenoh-backend-filesystem). These are further-research links rather than a mandatory dependency set. SDKs, versions, and guarantees are checked during implementation.

Minimum upward-pass validation:

- A late subscriber and a client connecting simultaneously with the final publication obtain the available outcome; an old delayed snapshot does not replace current state.
- A lost acceptance response, concurrent retries, and normal restart do not create a second execution of an accepted request.
- A retained snapshot preserves content and number after restart; a new clarification receives a greater number without resuming the action.
- Deleting a detailed App or standalone Axis report does not remove retry protection; an old request returns the previous operation with explicit report unavailability.
- A request to a deleted App is rejected; its identifier is not used for a new App.
- Restart when Activation is confirmed unpublished, published, or unknown preserves §6.4 distinctions; a previous Activation does not bypass current permission validation.
- Partial local startup, stopping, cleanup, and incomplete cancellation preserve actual participant states; a timeout does not become proof of cessation.
- Repeating cleanup for the remaining scope uses a new request; other consumers' shared resources are preserved under their contracts.

This is a test plan rather than results. Completely lost record storage is outside normal 1.0.0 recovery. Tests must validate the selected minimum profile without expanding it into the distributed version.

**2.0.0 — direction.** Testing multi-Gecko presence, network interruptions, distributed decisions, state recovery, and inter-node delivery limits.

**3.0.0 — direction.** Testing exchange and management with dynamic membership, prolonged observation, and App restructuring.

[Revision history](history.md).
