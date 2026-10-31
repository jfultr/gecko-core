# Gecko and Probe

Version 0.15 · October 31, 2026

[Glossary](../0-concepts/index.md) · [Mesh / App](../1-mesh-app/model.md) · [Scenario](scenario.md) · [D2 plan](work-plan.md) · [Line / Point / Edge / View](../4-line-point-edge-view/model.md) · [Block / Anchor](../3-block-anchor/model.md)

## 1. Gecko

The user starts Gecko instances on available machines and platforms. Each Gecko instance discovers accessible Devices, exposes discovered or explicitly connected Cameras, advertises Capabilities, known processing implementations, and connected services, and exposes available Probes for discovery and diagnostics. Gecko instances that can reach one another form a Mesh; supported connection checks provide bandwidth and latency measurements.

“The Gecko project” refers to the whole system, while “a Gecko instance” refers to a particular runtime. A machine provides the environment for Gecko. This document does not constrain the number of instances on one machine.

## 2. Capabilities and Execution Requirements

**Gecko advertises Capabilities.** A Capability describes an execution possibility in a particular Gecko's environment, considering hardware, software, and access conditions. A Device describes particular hardware; a Probe is a diagnostic task that obtains facts about a subject. Points describe the local Capability requirements of their selected implementations. A Line’s own requirements and those of its Points and internal Edges must all hold; placement validation considers the whole Line. If Gecko prepares a Block provider, that implementation’s requirements are checked separately at its execution location. A Line’s domain requirements for a Block are described by Anchors, not local Capabilities.

```text
Gecko jetson-01 advertises Capabilities: camera, gstreamer, tensorrt
Gecko desktop-01 advertises Capabilities: ui.slider, ui.video_view
Gecko jetson-01 exposes Probes: device discovery, camera access, inference test

Inference Point requires: a supported inference backend
Line camera_front requires: Capabilities for all its Points
```

Requirements belong to a particular implementation. An Inference Point with a local model and an Inference Point using a Block through an Anchor may have different local requirements. A remote Block does not turn its GPU into a local Capability of another Gecko instance.

Capability and current resource availability are distinct: having TensorRT does not confirm free GPU memory, and having a camera does not confirm access at launch. Resources, Anchor conditions, and Edge support are checked separately.

Hardware is described in the [Devices section](../0-concepts/index.md#4-devices).

### 2.1. Capability and Probe

**Accepted:** Capability and Probe are separate concepts. A Capability describes what Gecko's environment supports under stated conditions; a Probe is a separately addressable diagnostic task that can be invoked to discover, validate, or measure facts. A Probe run is one execution of that task through Operation. Availability of a Probe says that the check can be requested, not that the checked Capability is available.

Probe results provide evidence that can confirm or refute a Capability's availability under the check's conditions. There is no required one-to-one relationship: several Probes can contribute evidence for one Capability, and one Probe can obtain facts relevant to several Capabilities or facts that do not describe a Capability. Gecko advertises Capabilities using available facts; completion of a run does not by itself establish a Capability or readiness of a selected App.

Capability identifiers, descriptions, evidence references, freshness, and rules for publication, revision, or withdrawal remain open. These are developed with the entity, fact, and client contracts in D2; the terminology decision does not define a new foundational module. Lowercase “capability” also retains its ordinary meaning in Point, Block, control/view/trace, and UI contracts; those uses do not automatically denote an environment Capability.

### 2.2. Reconnaissance and Manual Configuration

**Accepted:** Points and Blocks can be created and configured independently of reconnaissance. Creation does not require a Probe invocation, completion of preliminary checks, or a positive Capability finding. The user can supply source and implementation parameters manually. Reconnaissance simplifies and accelerates this process by exposing discovered sources and available environment facts for selection and parameter reuse.

A discovered camera is visible to the client and provides quick access to configuring a camera Point from the available source parameters. Discovery alone does not create a configured Point. If the camera is not discovered, the user can configure its source manually, for example with an RTSP address. Hardware and processing-backend findings similarly help configure Points and Blocks; limited discovery coverage does not limit which configurations the user can create.

An absent finding does not by itself establish that a Device or Capability is absent or unusable. A negative finding, an unavailable check, and incomplete information must not become a general prohibition on creating Points or Blocks.

Creation and configuration are distinct from execution at a selected location. The selected implementation's execution requirements still apply: an unsupported CPU architecture or an unavailable required backend may prevent execution there without prohibiting creation of the Point or Block. The timing of validation, evidence required for launch, and treatment of unknown compatibility remain open in the corresponding placement, preparation, and execution contracts. This decision does not require a universal blocking/non-blocking classification of Capabilities.

## 3. Preliminary Probe Model

**Status:** a working model for further analysis, with accepted rules marked explicitly. Probe and Capability are distinct as defined in §2.1. This section develops diagnostic tasks and their runs; operation sets, result formats, and execution rules are not yet complete.

### 3.1. Purpose and Invocation Form

**A Probe is a separately addressable diagnostic task for discovery, validation, search, testing, and obtaining useful artifacts or cache.** Its main purpose is to identify problems, blocking conditions, and bottlenecks promptly, either in advance or on request, and provide results to Gecko and an agent.

The working interaction form is **one invocation → observable progress → results as data, artifacts, or cache**. A Probe run has a bounded goal and a clear end; specific durations, limits, and interruption rules remain undefined.

A result must explain what was checked, what was obtained, and how fresh the facts are. Artifacts and cache can be used by subsequent checks. Retention, reuse, freshness, and invalidation rules remain to be defined.

### 3.2. Data, Blockers, and Reports

A Probe invocation can produce more than retained facts. Possible results in the working model are:

| Result | Purpose |
|---|---|
| **Data** | Discovered devices, environment properties, check results, and measurements |
| **Artifacts and cache** | Prepared or obtained materials suitable for subsequent use |
| **Blocker** | A discovered condition preventing further checking, preparation, or execution, with an explanation and remediation instructions |
| **Report** | An account of completed work, including a successful check, useful for agent context and subsequent decisions |

These result types can coexist: discovering a blocker does not make data already obtained useless. Result schemas and rules for representing partially completed work remain open.

**Operation Report: a Probe run.** Invocation, intermediate messages, and the final outcome use the common [operation protocol](../1-mesh-app/model.md#82-operation-report-core) core. A report is associated with a particular Probe run, rather than just a check type. This is a working form of message content; exact structures and keys are refined separately.

Common execution mechanics belong to the [Operation foundational module](../core-modules.md#core-op-boundary-and-work). Probe defines the check's subject, results, constraints, and blockers; Gecko's reconnaissance policy selects subsequent checks. The common core does not depend on these domain rules.

| Report field | What a Probe run records |
|---|---|
| `operation`, `subject` | Run identity, Gecko and Probe, and check subject: a Device, environment, implementation, or another supported subject |
| `sequence`, `reported_at` | Number of this run's full snapshot and its generation time; an older or duplicate snapshot does not replace a newer one; order follows the number rather than clocks on different Gecko instances |
| `context` | Parameters, inputs, run conditions, and relationships to dependencies used; App and revision only when related |
| `state`, `progress` | `running`, `succeeded`, `failed`, `cancelled`, or `unknown` under the common protocol (Mesh / App §8.2), the current stage and available progress; a blocker preventing continuation without an external fix ends the attempt with `failed` rather than leaving it waiting to resume |
| `result` | Discovered facts and measurements as they appear, then the final outcome; a discovered Device is visible before the whole run finishes |
| `acquired_resources` | Resources actually acquired or created if the task performed such preparation; ordinary discovery need not acquire anything |
| `blockers`, `limitations` | A missing driver or access, unavailable dependency, incomplete facts, and limitations of test applicability |
| `explanation` | What was sought or checked, what was found, why a blocker arose, and what supports the conclusion; text suitable for agent context |
| `next_actions` | Instructions and possible agent or user actions, such as installing a driver and then explicitly running the check; instructions do not mean a fix has been performed |
| `evidence` | Measurement conditions and criteria, diagnostic data, references to artifacts and useful Gecko cache, and information about result freshness |
| `children` | Relationships to child Probe runs, where the domain contract defines them; Gecko policy initiates subsequent checks rather than Probes invoking each other; provisionally the tree belongs to one Gecko |
| `details` | Domain characteristics and data of the particular Probe; a readiness assessment is added only if the task actually provides one under its contract |

Every Probe run message is a full snapshot of the available Operation Report state, rather than only newly discovered data. Subsequent snapshots retain results already obtained and reflect current blockers, progress, and the final outcome; artifact references do not require copying artifact contents. This rule does not define latest-value storage or history in Zenoh.

Identity and context are available at execution start. Progress, discovered data, and possible blockers appear during work; the final report exposes the actual result, including partial results. Not every run must have progress percentages, child tasks, or resources. A standalone Probe need not have an `application` or App decision. Run completion does not automatically mean a successful check or readiness of an arbitrary composition.

### 3.3. Interface for an AI Agent

The AI agent interface is a substantial part of the Probe model. Gecko can detect a problem without necessarily having the means to fix it. A Probe must give the agent understandable context: the discovered condition, its impact, facts obtained, and instructions for further action. A successful report is also useful as context and a basis for subsequent decisions.

For example, a Probe discovers a missing required NVIDIA driver and returns a blocker with diagnostics and installation instructions. An agent has a broader set of tools for interacting with the OS and user and can use these instructions to fix the problem. This does not require the Probe itself to install the driver. After an external fix, the user or agent explicitly runs the check again: a new run with a new identity, rather than resuming the previous one. The current run ends with the blocker and obtained data; it does not automatically wait for a fix and continue. The agent's specific authority remains open. The common operation rule is defined in Mesh / App, §8.2.

### 3.4. Preliminary Probes and Check Chains

After startup, Gecko may start preliminary Probes. Their findings provide grounds for further checks. Provisionally, these chains are considered within one Gecko; this model does not approve Probe invocations across Gecko instances. For example:

```text
Gecko starts Device discovery
Device discovery publishes a discovered Device
Gecko policy starts Device inference test for that Device
```

Discovering a Device provides grounds for testing inference on it. Obtained data, artifacts, cache, and blockers form the environment diagnostic result. Checks may run in advance at Gecko startup or as needed during subsequent work.

**Accepted:** an individual Probe performs its check and publishes findings and a report; it does not invoke other Probes directly. Available Probes describe the facts and applicability conditions required to start them. As results appear, Gecko's reconnaissance policy matches them against those conditions, selects subsequent checks, and initiates them. It can use a finding before the originating check completes because partial results are published as they appear. The policy considers the current diagnostic context: automatic reconnaissance at Gecko startup or a client request. The concrete condition description, initial check selection, permitted load, and protection against cycles remain undefined.

**Accepted:** preliminary checks are ordinary Probe runs initiated automatically at Gecko startup. Preliminary and client-requested runs share an execution, observation, results, and cancellation contract. A user can request cancellation of a preliminary check and start additional chains through the same run management interface. These actions change current work without putting Gecko into a special mode. A run's origin is preserved in its context: automatic Gecko startup, a client request, or policy initiation following another Probe result; common execution rules do not depend on the initiator.

This decision does not define initial check selection, cancellation propagation to related runs, admission of competing checks, or the fate of partial results after cancellation. These parts of the common contract remain open.

Run relationships, subsequent-check selection conditions, progress and result aggregation, parallel execution, permitted load from heavy tests, and repeated runs require separate design. This model does not require unconditional execution of every possible test at every startup.

**Accepted:** Gecko's state lets a client understand whether Probe work is currently in progress. Discovered data and individual run reports are available as they appear. A client connecting later receives facts already available and can observe subsequent results.

The absence of current Probe work does not confirm that everything possible has been checked or that Gecko is ready for a particular App. A Probe that ended with a blocker is no longer running; its blocker and partial results remain available under report retention rules. Gecko or a client may start new checks later. Init and Postinit remain descriptive discovery concepts, rather than separate operations or a mandatory barrier for subsequent actions ([Mesh / App, §2.3](../1-mesh-app/model.md#23-discovery-and-probe-work)).

The [1.0.0 end-to-end scenario](scenario.md) connects preliminary checks, client connection, and an individual Probe invocation. Gecko's state format, specific preliminary checks, and policy rules for selecting subsequent Probes are refined during domain contract development.

### 3.5. Boundary with Mesh / App

The tree structure and execution of diagnostic tasks belong to Gecko / Probe. Mesh requirements on their progress, results, blockers, and freshness are defined in Mesh / App, §2.2–2.3 and §8; concrete domain schemas and methods are refined in Gecko / Probe. The Mesh layer uses reported results; diagnostic task rules and agent instructions are defined in detail in this document.

The preliminary model does not itself approve the Mesh / App contract or establish readiness of an arbitrary composition from a universal device test.

**Requirement from Mesh:** before Build, Probe results expose hardware and platform environment facts, current resources, measured characteristics, and tested load options. A measurement is associated with its subject and check run, test conditions, success criterion, and freshness; separate and combined load tests must be distinguished. Concrete data schemas and methods remain for lower layers to define. Resource budgeting and processing cost calculation will be investigated through real tests; no finished calculation model is currently prescribed.

### 3.6. Probe in Gecko's Zenoh API

At the Mesh level before Build, each Gecko can be addressed directly to request individual checks through its Zenoh API (edge communication). A Probe must appear explicitly in this API as a separately addressable entity. Addressing a check preserves its association with a particular Gecko and Probe; it cannot be reduced to an anonymous check operation on Gecko itself.

**Accepted:** a Probe and an individual Probe run are distinct. Each run has its own identity in the context of a particular Gecko and Probe. A newly accepted request creates a distinguishable run. Parameters, state, progress, results, artifacts, and blockers belong to the corresponding run. Retrying the same request with the previous `request_id` returns the previous operation without new execution (Mesh / App, §8.2). This decision does not determine whether several runs can execute concurrently.

| Addressing level | Meaning |
|---|---|
| **Gecko** | The particular instance providing the API |
| **Probe** | A separately addressable diagnostic task |
| **Probe run** | A particular task execution with its own identity, parameters, progress, and result |

Run identity distinguishes repeated check results. An explicit new request performs the check again rather than substituting a previously cached result for execution. Retrying an accepted request with the same `request_id` is not a new run. Cache in the preliminary model means useful artifacts of Gecko's own work; it does not mean a mechanism for answering an explicit check request. Cache details belong to Gecko / Probe internals. A run's own identity does not define result retention time or require indefinite history storage.

**Accepted:** each Probe has its own topic within a particular Gecko's namespace and publishes progress there. Discovered data is exposed as it arrives: a discovered Device becomes visible immediately, without waiting for the Probe or the whole check tree to complete. Subscribing to a particular Gecko's Probe namespace allows observation of their work and available results.

The user's group subscription sketch is `/gecko/213123/probe/*`: observe Probes of Gecko instance `213123`. This illustrates a logical subscription scope, rather than an approved Zenoh key syntax. Concrete keys and the subscription pattern must be reconciled with namespace nesting.

Progress and result publications preserve their association with an individual Probe run. The run identifier must distinguish repeated executions; placement in the key or publication content has not been selected. Immediate visibility of a discovered Device does not mean subsequent suitability checks have completed.

The topic system must expose the check subject, state and progress, results and limitations, blockers, and remediation instructions. The check request API and observation namespace must be consistent; their exact relationship remains to be defined.

**Open decisions**

| Open questions | Decision outcome |
|---|---|
| What is the concrete Zenoh key and group subscription format in the Gecko → Probe → run namespace? How are available Probes discovered and related to checked Devices, environments, implementations, or connections? | A scheme for keys, Probe discovery, and check-subject references. Association with a particular Gecko, separate Probe addressing, distinct runs, and progress publication are already defined. |
| How is an individual check request addressed, a run identifier issued, and observation topics associated with it? Are concurrent runs of one Probe permitted, and under what execution rules? | A Gecko → Probe → run addressing scheme, request mechanism, and concurrency rules. Distinct runs are accepted; identifier and key formats remain open. |
| How are state, progress, data, limitations, artifact and cache references, blockers, and agent instructions organized within a Probe namespace? How is result freshness conveyed? | Topic and message schemas allowing a subscriber to interpret a particular check's result. |

Concrete keys and formats have not been selected. This decision establishes check and observation addressability without defining the internal Probe tree structure or Build protocol.

### 3.7. Limited Reconnaissance in 1.0.0

**Accepted:** reconnaissance is required in 1.0.0 with a limited set of useful checks that reduces manual discovery and configuration work. The release does not require a comprehensive reconnaissance system. Its domain coverage is:

| Check area | Intended result | Boundary |
|---|---|---|
| Basic machine information | Basic CPU information, including architecture, memory information, and disk information | A basic environment picture; exhaustive hardware assessment and performance testing are not required by this scope |
| Accelerator discovery | Discover and identify GPUs and NPUs through the supported checks | Discovery does not establish execution support. NPU execution support and support for additional accelerator types are deferred beyond 1.0.0 |
| DeepStream and OpenVINO support | Check and report environment support, with evidence and limitations of the performed checks | Exact validation depth remains open. Installed software alone does not confirm successful execution of a selected workload or support for every accelerator |
| Camera discovery | Discover cameras through a limited set of supported interfaces, including scenarios involving RTSP sources, and expose findings for convenient configuration | Specific interfaces and RTSP discovery methods remain open; finding every camera or stream is not required |

The [scenario](scenario.md#100-domain-cases-limited-reconnaissance) illustrates these cases and the independent manual configuration path. Exact check profiles, inputs, outputs, applicability conditions, limits, and success criteria remain to be defined. The coverage decision does not prescribe which checks run at startup, require every available check to run, or approve a general extensible reconnaissance framework.

The broader Probe model permits diagnostics and measurements beyond this limited set; that generality does not make every possible check a 1.0.0 obligation. Concrete load-test coverage remains to be reconciled with the Mesh requirements in §3.5. Reconnaissance completeness is not a prerequisite for creating Points or Blocks under §2.2; readiness of a selected composition remains a separate concern.

## 4. Vocabulary Changes

### From Version 0.14 to 0.15

Limited reconnaissance is included in 1.0.0: basic machine information, GPU/NPU discovery, DeepStream/OpenVINO support checks, and camera discovery including RTSP scenarios. Points and Blocks can be created and configured manually regardless of reconnaissance results; findings simplify configuration. Execution requirements remain separate, with validation and launch policy still open.

### From Version 0.13 to 0.14

The former **Axis / Axes** vocabulary is split by responsibility: diagnostic tasks are **Probe / Probes**, while advertised environment possibilities are **Capability / Capabilities**. A Probe run uses Operation and produces facts; it is distinct from a Capability. The documentation directory is renamed from `2-gecko-axis` to `2-gecko-probe`, and references, examples, and diagrams use the new terms. Earlier entries below use the current terms where the meaning is unambiguous; the original terminology is identified where needed.

### From Version 0.12 to 0.13

Common Operation mechanics are separated into an [independent foundational module](../core-modules.md) used by Probe runs and App operations. The active common contract remains in [Mesh / App §8.2](../1-mesh-app/model.md#82-operation-report-core).

### From Version 0.11 to 0.12

Gecko's reconnaissance policy selects subsequent checks from Probe findings and available checks' applicability conditions. An individual Probe publishes its check results and does not invoke other Probes directly. Accepted parts of A3 and B1 and open questions are recorded in the [D2 plan](work-plan.md#41-accepted-parts-of-decisions).

### From Version 0.10 to 0.11

Preliminary checks are established as ordinary Probe runs with a shared execution and management contract. Users can request their cancellation and start additional chains; run origin is preserved in context. Resolved parts of B1, B2, C3, and D2 and remaining questions are recorded in the [D2 plan](work-plan.md#41-accepted-parts-of-decisions).

### From Version 0.9 to 0.10

| Previously | Now |
|---|---|
| A common status declared completion of the current fact collection's necessary checks | Gecko's state exposes current Probe work; no separate complete-collection boundary is defined |
| The scenario began with a Probe tree during Init | The [scenario](scenario.md) covers Gecko startup, partial facts, a late client connection, and an addressed Probe invocation |

### From Version 0.8 to 0.9

| Previously | Now |
|---|---|
| Diagnostic message form remains a general working model | Probe use of Operation Report, full snapshots, `sequence` and `reported_at`, execution states, and an explicit new run after an external fix are described; the common core is in Mesh / App, §8 |

### From Version 0.7 to 0.8

| Previously | Now |
|---|---|
| Axis originally describes an available environment capability (now Capability) | Diagnostic tasks (now Probes) are added under the same former name; the relationship remained open at that revision and is resolved in §2.1 |
| Diagnostic result acquisition mechanics are not exposed | Invocation with progress, data and artifacts, result freshness, blockers and agent reports, and a task tree during Init are proposed |

### From Version 0.5 to 0.6

| Previously | Now |
|---|---|
| App selection and capability presentation are assigned to Admin / Editor | Gecko provides these capabilities regardless of interaction mechanism |

### Retained Changes in Version 0.2 from 0.1

The Kubernetes comparison retains its original meaning: Gecko understands media/AI processing, supported connections, and domain dependencies. Kubernetes can provide an execution environment for part of the system. This map does not establish a direct correspondence between a Line or Block and a Pod.
