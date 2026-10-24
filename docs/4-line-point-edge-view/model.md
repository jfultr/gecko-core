# Line, Point, Edge, and View

Version 0.7 · October 10, 2026 · [Русский](model.ru.md)

[Glossary](../0-concepts/index.md) · [Mesh / App](../1-mesh-app/model.ru.md) · [Gecko / Axis](../2-gecko-axis/model.md) · [Block / Anchor](../3-block-anchor/model.md)

## 1. A Line and Its Points

A Line is the main operational unit for processing: `start`, `stop`, `pause`, `resume`, `reload`, `restart`, state, health, and logs. Operation availability and exact semantics depend on the implementation.

A Point groups elements that implement one understandable function:

```text
Inference Point
├── preprocessing
├── inference client or local inference
└── postprocessing
```

A Point can be complex and composite, but remains within its Line. It does not hide additional independent Lines or Blocks. If its function uses a shared service, the dependency is declared through its Line’s Anchor and displayed explicitly.

| Point Property | Example |
|---|---|
| Inputs and outputs | Video input, detections output |
| Parameters | Model, threshold |
| Implementation requirements | A supported inference backend |
| Diagnostics | Latency, errors, input and output previews |

Point addressability allows changing `front_camera.detector.threshold` or inspecting its metrics. It does not imply an independent `restart(detector)`. A change may apply while running or require rebuilding or restarting the Line; this must be visible before applying it.

A Line and a GStreamer pipeline also describe different levels. A Line is a Gecko object with a stable name, lifecycle, and dedicated process with the Gecko runtime; a GStreamer pipeline is a concrete implementation of its processing within that runtime. They may correspond directly in the first scenarios.

### 1.1. Point Contract: Control, View, and Trace

**Control, data presentation, and tracing are designed alongside processing.** A Point declares its capabilities so that Gecko can expose available data, parameters, commands, and their usage conditions to any App user. These descriptions are part of the Point contract from the first version; specific capability sets depend on the implementation. UI hints describe possible presentation, but access to the contract does not require a visual interface.

| Contract Part | What the Point Declares | Possible UI Elements |
|---|---|---|
| **Control plane** | Parameters, types, ranges or choices, available commands, application conditions, and result acknowledgment | Slider, input field, model selector, action button |
| **View plane** | Available data representations, their formats and meaning: previews, images, detections, tables | VideoView, image viewer, overlay, results table |
| **Trace plane** | Processing events, errors, durations, metrics, and identifiers relating events to inputs and results | Timeline, latency graph, frame or request path inspector |

A View as an App object is distinct from the view plane: the plane declares available data representations, while a View assembles a user-facing screen from capabilities of all three planes.

The control plane describes whether a change can apply while running, whether it requires rebuilding or restarting the Line, and when it is considered applied. Point commands do not introduce an independent lifecycle: `restart(detector)` does not appear automatically. Lifecycle operations belong to the Line.

The view plane describes available representations, not a finished screen. A Point may suggest a preferred UI element, such as a Slider for a numeric parameter or an overlay for detections; the View selects the actual presentation. Formats and coordinate systems must allow bindings to be validated, such as the compatibility of detections with the displayed image.

The trace plane relates events to a particular Line, Point, and processed frame or request. Across Point and Line boundaries, the contract must allow event correlation to be preserved. Unrelated logs do not replace a trace. Collection completeness, sampling, and history storage will be defined separately.

```text
Inference Point detector

Control:
  threshold: number 0…1, change while running
  model: select from a list, change requires Line restart

View:
  input_preview: image
  detections: results with coordinates and frame identifier

Trace:
  inference_started / completed / failed
  duration, frame_id / request_id
```

This is an example contract, not a universal requirement for every inference implementation. The Point declares capabilities; the Line runtime exposes them; Gecko makes them available through the App for observation and permitted actions. A View uses the same contract for data presentation and control. Configuration preserves explicit UI bindings to parameter, command, and representation addresses. These bindings are neither processing Edges nor service dependency Anchors. API formats and transports for these planes have not been selected.

### 1.2. Selecting a Point Implementation

A Point’s function and contract differ from its execution mechanism. For example, an Inference Point may have implementations for a CPU, an NVIDIA dGPU, the Jetson platform, or calling an inference Block through an Anchor. These are alternatives with their own conditions, rather than a flat list of interchangeable devices. Options must satisfy the required Point contract; compatibility with neighboring Points and Edges is checked separately.

During App composition, the selection mechanism chooses compatible implementations and placement for the whole Line from known options, considering the task, constraints, and user preferences. The App saves the selected option or an explicitly permitted selection policy. Gecko validates the configuration and binds execution to particular Devices. The user can select an option manually or pin an implementation and placement.

A Line’s own requirements, such as the architectures supported by its runtime, apply together with Point and internal Edge requirements. Runtime support for x86_64 and arm64 does not mean every Line composition supports both architectures. Moving from local inference to a Block changes dependencies and requires an Anchor; selection before startup does not imply automatic switching while running.

## 2. Edges: Supported Processing Connections

**An Edge predetermines a supported way to connect one Point’s output to another Point’s input.** It defines interface compatibility, the data transfer mechanism, and constraints. The supported mechanism and a particular connection using it are distinct: during assembly, each connection records the source Point and output, destination Point and input, and selected Edge mechanism and settings. Multiple such connections can form a many-to-many topology.

Both endpoints are always Point interfaces. Within a Line, Edges define allowed Point connections implemented by the Gecko runtime. At App level, they connect Points belonging to different Lines through interfaces explicitly exposed by those Lines. For example, `front_camera.video` may expose `front_camera.source.video`: the Line-level name refers to its Source Point’s output. Exposing an interface does not create a separate processing endpoint or change the Point’s membership in its Line. The exposed Point need not be the first or last Point in the Line; its selected input or output must be explicitly available for external connections.

```text
Within one Line:
front_camera.source.video ── Edge ──→ front_camera.detector.video_in

Across Line boundaries:
capture.source.video ── Edge ──→ analysis.detector.video_in
```

A Point has inputs and outputs, but matching data types alone does not permit a connection: a supported Edge must exist. Gecko exposes allowed connections and explains constraints before execution. Fan-out, multiple senders to one input, and stream mixing require explicit support.

| Context | What the Edge Must Support |
|---|---|
| Between Points in one Line | Implementation compatibility within the shared chain and process |
| Between Points in different Lines on one Gecko instance | A concrete interprocess data transfer mechanism |
| Between Points in Lines on different Gecko instances | A concrete network path, formats, and transfer conditions |

`unsupported connection` is a valid validation result. Distinguish supported; unsupported; supported but conditions are unsuitable; and not yet checked. Available Axes do not guarantee a connection. The status of a particular connection is diagnostic information, not the definition of an Edge.

A Line’s dependency on a Block is described by an Anchor. Checks such as “is Triton reachable?” and “is the model ready?” belong to the Block and Anchor. Block calls are implemented by a domain contract client or adapter; they do not need to be represented as Edges.

Concrete transports will be selected separately for supported connection mechanisms. Bandwidth and latency measurements are tied to their time and test conditions.

## 3. View: A User-Facing App Representation

An App can have several Views. An operator View displays video, detections, and the necessary controls. A diagnostic View displays Line state, Anchor readiness, Point metrics, and event traces. Both use declared capabilities of the same App.

A View consists of UI elements with explicit bindings to data, parameters, and commands. For example, a VideoView displays `front_camera.detector.input_preview`, a Slider changes `front_camera.detector.threshold`, and a Button invokes `front_camera.start`. A trace presentation uses declared events and their identifiers.

Declarations allow a basic View to be assembled automatically or a custom screen to be built using the same interfaces. Gecko reports operation availability, acknowledgment of changes, restart requirements, and lost connections; the View presents this information. A preferred widget does not grant additional control permissions.

A View executes outside Point processing. Its definition does not promise a separate process or a particular UI framework. Placement, preview and trace delivery, update rules, and reconnection behavior are selected separately; an available data transfer mechanism must support each particular binding.

## 4. Process and Concrete Line Execution

Process is not part of the core domain vocabulary, but a Line’s execution boundary is concrete:

```text
1 running Line → 1 dedicated OS Process with the Gecko runtime
```

The runtime executes Points and supported connections within the Line. A Line retains its identity across restarts; the PID changes. Configuration and diagnostics belong to the stable name, while the PID is shown as technical information.

```text
Line camera_front → PID 1234
       restart
Line camera_front → PID 5678
```

`pause` means suspending processing according to runtime rules, rather than necessarily suspending the process through the OS. `reload` means applying configuration and may require a restart. These operations do not promise seamless application of every change.

A Block has no universal process-count rule or mandatory Gecko runtime. Its contract can be fulfilled by an external service, program, container, or Gecko-native service, including inside the Gecko process. A connected Block’s stable name does not depend on the PID of its implementation.

The dedicated-process rule for a Line does not describe the internals of the Gecko instance itself or View execution. Process isolation also does not remove shared dependencies on devices and services.

## 5. Vocabulary Changes

### From Version 0.5 to 0.6

| Previously | Now |
|---|---|
| Control/view/trace are described mainly for external UI | Contracts are available to all App users; Views present the same capabilities visually |

### From Version 0.3 to 0.4

| Previously | Now |
|---|---|
| The App user interface has no term of its own | A View is a user-facing App representation with explicit UI bindings |
| A Point lists parameters and diagnostics without a shared contract for external UI | A Point declares control/view/trace capabilities, usage conditions, and suitable UI elements from the outset |
| The text lists transport options | Transport selection remains open |

### From Version 0.2 to 0.3

| Previously | Now |
|---|---|
| Lines and Blocks are symmetric managed execution units | A Line is a concrete process with the Gecko runtime; a Block is a service fulfilling a domain contract |
| An Edge is a logical connection, including connections to Blocks | An Edge defines a supported way to connect processing between Points or Lines; Block dependencies are described by Anchors |

### Retained Changes in Version 0.2 from 0.1

An App combines Lines and UI rather than an arbitrary graph of Points. A Point is a logical group within a Line; a Line is the placement and lifecycle unit. Process is an execution mechanism and diagnostic detail.

Earlier names Component / Dot correspond to Point only in the context of internal Line processing. Capability corresponds to Axis. Earlier Connection denoted a particular connection; a supported Edge is now selected for it. Qualify “node”: Gecko is a runtime instance; Point is an element within a Line.

## 6. Open Questions

- Rules for exposing Point interfaces through a Line; the distinction between parameters and control inputs.
- Formats for control/view/trace declarations, UI hints, and View bindings; View placement and execution.
- Command and change acknowledgments, representation updates, and UI behavior on connection loss.
- Trace correlation across Points and Lines, sampling, history storage, and data delivery for each plane.
- Concrete Point and Edge implementations for the first version.
- Pause/reload semantics, policy for loss of a source or data consumer, and the order of App operations.
