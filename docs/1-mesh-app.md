# Mesh and App

Version 0.7 · October 10, 2026 · [Русский](1-mesh-app.ru.md)

[Glossary](0-concepts.md) · [Gecko / Axis](2-gecko-axis.md) · [Line / Point / Edge / View](3-line-point-edge-view.md) · [Block / Anchor](4-block-anchor.md)

## 1. Mesh

A Mesh is a collection of Gecko instances that have discovered one another and can interact. Supported connection checks provide bandwidth and latency measurements. A new participant expands the available App options, but does not change a running composition by itself.

## 2. App

An App defines a composition for a task and a shared experience for interacting with its capabilities, actions, and results. Its general role is described in the [glossary](0-concepts.md).

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
    └── Anchor storage → Block recordings
                          └── provider: separate implementation requirements
```

A Line combines its own requirements with those of its Points and internal Edges. When using a Block, the Line requires its contract and readiness through an Anchor; a remote provider’s hardware requirements do not become local Line requirements. Shared diagnostics can expose Device relationships reported by the provider. No additional term is introduced for a hardware dependency: requirements and concrete bindings belong to execution configuration.

## 3. Placement, Distributed Processing, and UX

Gecko exposes Mesh capabilities and proposes compatible App options for a task, including Point implementations, Line placement, and Block providers. Available now is distinguished from requiring preparation and an unsatisfied dependency. This information, selection actions, and validation results are available regardless of interaction mechanism. Selecting an option records the composition; manual assembly and constraint changes are also available.

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

## 4. Target Workflow and Early Versions

1. The user specifies a task, inputs, and constraints, for example by providing a video to test an idea. Gecko instances must already be running on available machines and platforms; the need to start additional participants is reported as a condition of the selected option.
2. Gecko exposes discovered Mesh participants, their Devices, Cameras, Axes, known Point implementations, running Block providers, and service preparation capabilities. For options using connections between nodes, it automatically checks supported paths and saves bandwidth, latency, measurement time, and conditions.
3. Gecko proposes App options with compatible Point implementations, Line composition, Edges, Anchors, Block providers, available data and actions, Views, and placement.
4. For each option, Gecko reports what is ready now, what requires preparation, and which dependencies are unsatisfied. The user selects an option or changes constraints and composition; the App saves the selected composition and bindings.
5. Check each Line’s combined requirements, Devices and resource availability, every Edge, Anchor conditions, and View bindings. Explain limitations before startup.
6. Prepare required providers using supported mechanisms. Validate their contracts and readiness of required capabilities; external preparation remains an explicit condition when Gecko cannot perform it.
7. Recheck readiness before starting Lines. Provide the first observable processing result, available data and traces for evaluating the hypothesis, along with Line and Block state, Anchor readiness, Device use, and service preparation progress.
8. Change processing and repeat evaluation on the selected input data. Gecko reports the scope of the change: a Point parameter, implementation change, Line restart, Anchor change, or Block provider operation. The evaluated composition can be saved and prepared for deployment to the operational environment. A Mesh change may produce new proposals, but does not rebuild a running App by itself.

This is the target scenario, not a claim of an existing implementation. Early versions may limit tasks, implementations, and preparation mechanisms; manual selection remains available. The initial practical scenario is a camera or file and one Line with local processing, or shared inference through a Block. Discovery, proposed compositions, preparation, and execution are separate steps; selection before startup does not promise migration or failover while running.

## 5. Vocabulary Changes

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

## 6. Open Questions

- Configuration and interface formats, their versions, and identity.
- A catalog of Point and Block provider implementations, App selection algorithms, task constraints, and option selection policy.
- Task and processing behavior description format; shared access to App capabilities, actions, conditions, and results for human and programmatic users.
- Repeat evaluation on selected data, experiment reproducibility, and comparison of results after App changes.
- Supported automatic network checks, measurement freshness, and assessment of combined load.
- When migration, load balancing, and recovery of running participants are needed.
