# Mesh and App: Architectural Model and Implementation Versions

Document version 0.52 · November 1, 2026

[Glossary](../0-concepts/index.md) · [Goal and development cycle](../0-concepts/index.md#12-development-cycle) · [Plan progress](../development-cycle.md) · [Gecko / Probe](../2-gecko-probe/model.md) · [Line / Point / Edge / View](../4-line-point-edge-view/model.md) · [Block / Anchor](../3-block-anchor/model.md)

## 1. Purpose, Boundaries, and Status

This document defines Mesh / App entities, responsibility, preparation, execution, and observable state. Versions **1.0.0 / 2.0.0 / 3.0.0** denote implementation scope; version **0.52** is the document revision. The text describes intended behavior rather than implemented capabilities.

**1.0.0 — reduce feature delivery latency on one Gecko.** A person, agent, or program can discover capabilities, specify a local composition, validate and prepare it, obtain an observable domain result, change configuration, and repeat validation. App and a minimum Mesh layer are present from the first version. One Gecko can use multiple Devices, execute multiple Lines, and access supported network Cameras and external services. Distributing the managed composition across Gecko instances is outside this version.

**2.0.0 — distributed execution direction.** The core interaction mechanism for multiple Gecko instances extends the feature delivery cycle across available devices. An App can use several nodes; composition and placement are selected explicitly. Discovering a new node expands options without itself restructuring a running App. Concrete distributed architecture is refined later.

**3.0.0 — dynamic Mesh and product operation direction.** Gecko arrivals and departures may inform changes to App composition, placement, and responsibilities. Failure reactions, migration, redistribution, and a persistent Observer are considered. Automation, seamless transitions, and recovery guarantees are not established yet.

Feature delivery latency is defined in [concepts, §1.2](../0-concepts/index.md#12-development-cycle). The first verifiable result is an intermediate point on the path to functionality ready for its target environment; frame-processing and network-message latency measure different properties.

**Status of 1.0.0 decisions.** This document establishes Mesh / App boundaries and required behavior. [Requirements on lower levels](lower-level-requirements.md) are separate for the next step. Under the [development cycle](../development-cycle.md), requirements are refined downward to lower-level contracts. After turning upward, run experiments and write code, ascending step by step. Mesh / App implementation formats, mechanisms, and criteria are made concrete on returning to this level using the experience obtained. The [separate return-question list](../development-cycle.md#2-remaining-decisions-needed-to-make-100-specific-enough-to-implement) is not a completion condition for the current downward pass.

In each topic subsection, 1.0.0 defines local-model obligations, while 2.0.0 and 3.0.0 define directions. Domain behavior requirements are refined in corresponding lower layers. The [history](history.md) contains earlier decisions, including superseded ones; it does not expand version scope established by the main text.

## 2. Mesh: Participants, Facts, and Exchange

### 2.1. Mesh Model

**Mesh is a decentralized system for exchanging facts and capabilities among Gecko instances.** A single main Gecko is not a mandatory part of the general model.

**1.0.0.** Implement a minimum declaration namespace and API for one Gecko. It exposes identity, state, Capabilities, available Probes and their results, Devices, Cameras, and supported processing and service implementations. A client obtains published state and observes updates through the accepted Zenoh pub/sub infrastructure. Identities and declaration ownership by Gecko are preserved so future expansion does not require redefining local entities. One Gecko operates independently of others being present.

**2.0.0 — direction.** Declaration exchange among Gecko instances, a shared picture of available capabilities, and its use for distributed Apps.

**3.0.0 — direction.** Mesh membership changes during operation and use of new facts to reconsider a running composition.

### 2.2. Facts and Information Limitations

**1.0.0.** Hardware presence, an available implementation, current resources, and execution readiness are separate facts. Unknown or incomplete information differs from a negative check result. A resource reference contains **Gecko + resource**; even the local version preserves a resource's association with a particular Gecko.

Gecko obtains environment facts and measurements through Probes. A Probe is a separately addressable diagnostic task; a Device, Camera, implementation, or connection is its subject. A Capability describes an execution possibility in that Gecko's environment under stated conditions. Probe results can support or refute its availability; available Probes and advertised Capabilities are distinct. Their relationship is defined in [Gecko / Probe §2.1](../2-gecko-probe/model.md#21-capability-and-probe). An individual Probe run has its own identity, parameters, progress, results, and blockers. A check can be invoked directly through Gecko's API without an App. A new request performs the check again; retrying an accepted request with its previous `request_id` returns the previous operation (§8.2).

| Information group | Local planning need |
|---|---|
| Hardware and environment | CPU, accelerators, OS, drivers, and access conditions |
| Current resources | Available memory and disk space |
| Measured characteristics | Supported compute test and access check data |
| Tested load options | Results for a particular load under stated conditions |

A measurement is associated with its subject and check run, conditions, success criterion, and freshness. Separately tested loads are not considered tested together. Results do not reserve resources or guarantee suitability of an arbitrary composition. The concrete minimum check set and contracts are refined in Gecko / Probe; no universal additive Point cost is introduced.

**2.0.0 — direction.** Comparing facts from different Gecko instances and checking paths between nodes. Declarations of one physical Camera by different Gecko instances may remain different access paths; global resource identification is not mandatory.

**3.0.0 — direction.** Updated facts as grounds for assessing whether App restructuring is needed; freshness and consequences of changed conditions are refined through operational experience.

### 2.3. Discovery and Probe Work

**1.0.0.** Gecko may start preliminary Probes at startup; their results may lead to subsequent checks. Discovered Devices and other results become available as they arrive. Progress and result publications belong to particular runs. Gecko state lets a client understand whether Probe work is currently in progress; its absence does not mean every possible fact has been collected or the environment is ready for the selected App.

A client can connect during or after preliminary checks, obtain facts already available, observe new results, and invoke an individual Probe for its task. Blockers, errors, and uncertainty remain visible; a run ending with a blocker does not mean Probe work is still in progress. Init and Postinit describe discovery but do not prescribe mandatory completion of all checks, a list of every preliminary Probe, or admission conditions for Build and Activation. The selected composition needs sufficient facts, followed by a separate current readiness confirmation before Activation.

**2.0.0 — direction.** Comparing current Probe work and available facts for a selected Gecko set. Unavailable participants must remain visible; available information can be used independently.

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

A Probe can be invoked directly without an App. Progress, blockers, and recommendations are available structurally. Domain data is accessible through participant contracts separately from operation reports. A minimum result representation uses the same data; learning the entire execution structure must not be a prerequisite for the first experiment.

**2.0.0 — direction.** The common contract extends to distributed composition checks and management.

**3.0.0 — direction.** The same approach exposes dynamic change proposals and results, history, and notifications.

## 5. Planning and Placement

### 5.1. Task and Composition Candidate

**1.0.0.** A task specifies inputs, expected result, and constraints. An App Candidate is a proposed composition with implementations, configuration, and dependencies; it does not yet confirm readiness. An external Planner constructs it from Gecko declarations and APIs, including manual assembly. Gecko's formal input is a verifiable composition description; understanding arbitrary tasks and automatically enumerating options are not mandatory runtime features.

Reconcile the minimum candidate and revision schema with lower layers and record it on returning to Mesh / App. An agent receives the same checks and constraints as other consumers.

**2.0.0 — direction.** Placement and implementation selection across Gecko instances; automation of recurring sequences based on real scenarios.

**3.0.0 — direction.** Revising a running composition when goals, resources, or Mesh membership change; automatic experimentation limits are defined separately.

### 5.2. Placement Options and Connection Checks

**1.0.0.** A Placement Option describes a possible local placement of a Line or managed provider with Device and dependency bindings. It is not a created participant, reservation, or readiness guarantee. Selection relies on Probe results and domain checks of the selected configuration.

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

Validation includes a preparation blocker, partial results, safe request retries, restart, client disconnection, and processing independence from a View. Concrete implementations and acceptance criteria are refined when returning to this level under [plan R1–R8](../development-cycle.md). The current downward pass proceeds to Gecko / Probe without first closing that list.

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

## 8. Operation Requirements and App Integration

### 8.1. Purpose and Consumer Boundary

**Gecko 1.0.0.** Mesh / App consumes the [Operation module contract](../../docs-modules/operation/spec/model.md). Operation owns common identities, APIs, execution states, report schemas, cancellation mechanics, and recovery guarantees. This section defines requirements and App-specific use; it does not prescribe Operation internals. The [requirements register](lower-level-requirements.md#operation) traces consumer needs to module contracts and open integration questions.

App uses operations for preparation, readiness, Activation, Stop, cleanup, and supported changes. A request, its execution, an App decision permitting an action, and actual decision application remain distinct. Build is a sequence of external Planner calls (§6.1), not an Operation type, parent, or mandatory aggregate report. Individual preparation operations need not belong to an App or Coordinator.

**Gecko 2.0.0 — direction.** Extend consumption to distributed participants and acknowledgments. **Gecko 3.0.0 — direction.** Use operations for restructuring and recovery. These product directions do not approve additional module releases or universal action resumption.

### 8.2. Requirements on Operation and App-Specific Use

Mesh / App requires distinguishable accepted executions, structured progress and partial outcomes, safe retries after a lost response and normal restart, observation after reconnecting, explicit uncertainty, and cancellation whose actual outcome can be established. Detailed behavior is owned by the [Operation contract](../../docs-modules/operation/spec/model.md); requirement IDs are in [the consumer register](lower-level-requirements.md#operation).

**App identity and retention requirements.**

**Accepted for 1.0.0: records of accepted mutating App requests are retained throughout the App's existence.** The `request_id` to `operation.id` association and request-content comparison data are not automatically deleted by age, including after operation completion. Detailed Operation Reports, diagnostic data, and artifacts may have separate retention periods; deleting them does not delete the accepted request record or permit re-execution.

**Accepted: an App's request records may be deleted after the App is deleted.** An old request addressed to a deleted App is rejected without new execution or App recreation, regardless of whether the old `request_id` record remains. A missing record does not make the request admissible: the addressed App must exist first. The deleted App's identifier is not reused (§3.2), so an old request cannot reach a new App under the same identity. This retention exception applies to deleted Apps and does not permit age-based deletion while an App exists.

If the detailed report is unavailable, retrying the same request with the same content returns the previous `operation.id` and an explicit unavailable-report indication. No new execution is created; report unavailability alone does not confirm actions have completed. Other requests are checked under domain contracts without a global App lock (§4.2).

The module's common retention policy also protects standalone accepted requests; App deletion is a consumer-specific exception. A safe accounting-release interface remains open in Operation. The App domain must enforce permanent rejection of old requests independently of whether accounting remains.

**Domain report content.** App reports identify participants, subjects, selected configuration/revision, dependencies, actual actions, and available results. Common report fields are defined by Operation; App supplies their domain meaning. `acquired_resources` supports selective or overall cleanup (§7.3). Resource acquisition does not prove release or exclusive ownership; release results belong to `result`, and domain participants enforce sharing and ownership rules.

Readiness uses `status`, `checks`, and `validity` (§6.3); decision application uses `application` (§8.4). A readiness operation may succeed while returning `not_ready` or `unknown`. The latest retained report does not replace the fresh readiness request before Activation. Final nesting of App profiles within the common report is still open.

**Composite App work and cancellation.**

For a composite operation, Coordinator requests cancellation of active children and collects their actual results. Overall `cancelled` is valid only after confirmed cessation of the composite operation and its active parts. If a part continues, rejects cancellation, or does not respond, the report exposes this and the composite operation is not declared cancelled. Completed child actions are neither rewritten as cancelled nor automatically rolled back.

This is a Coordinator duty for genuinely composite App work; a parent reference alone grants no cancellation authority. Failed or unknown parts remain visible, and refusal to cancel a part does not alone determine the overall outcome. Overall cleanup follows §7.3. Cancelling Activation coordination after decision publication neither revokes permission nor proves activated processing stopped. Stop App (§6.5) and cleanup remain separate actions.

App consumes the module's retry and outcome rules: failure or uncertainty does not impose a global App lock (§4.2); domain admissibility governs a new action. After a fix, a new attempt does not imply cleanup of previous preparation. New cleanup for the remaining scope uses a new request; retransmission of an old request refers to its existing operation.

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

### 8.5. Open App Integration Decisions

For Gecko 1.0.0, Mesh / App must refine its operation kinds and domain inputs/results, readiness and application profile nesting, domain admission and safe interruption points, waiting periods, and lifecycle/accounting-release integration. These belong to the [R1–R8 return list](../development-cycle.md#2-remaining-decisions-needed-to-make-100-specific-enough-to-implement).

Common API types, identity comparison, state arbitration, snapshot persistence, and transport adapters are owned by the [Operation decision register](../../docs-modules/operation/spec/status.md). App integration validates that the selected module version satisfies consumer requirements; it does not redefine those mechanisms.

### 8.6. App Integration and Validation with Zenoh

Gecko uses Zenoh for exchange. The common operation request/report adapter is investigated in [Operation §9](../../docs-modules/operation/spec/model.md#9-transport-adapter-investigation). App integration additionally addresses publication of revisions and complete permission decisions, actual application acknowledgments, and retrieval of current participant state. Transport delivery or caching does not replace domain persistence or establish readiness or successful execution.

Minimum local integration validation:

- An App client reconnects during work or at final publication and obtains available outcomes through Operation.
- Lost responses, concurrent retries, and a normal restart preserve one accepted execution; unavailable detailed reports do not permit repetition.
- Requests to a deleted App are rejected without recreation or identity reuse, including after permitted accounting removal.
- Restart when Activation is confirmed unpublished, published, or unknown preserves §6.4 distinctions; a previous Activation does not bypass current permission validation.
- Partial startup, stopping, cleanup, and incomplete cancellation preserve actual participant states; a timeout does not prove cessation.
- Further cleanup addresses the remaining scope explicitly and preserves shared resources.

These are integration criteria, not experimental results. Module-level conformance checks are maintained in [Operation §10](../../docs-modules/operation/spec/model.md#10-validation). Gecko 2.0.0 and 3.0.0 extend domain validation to distributed and dynamic scenarios; they introduce no additional current guarantees.

[Revision history](history.md).
