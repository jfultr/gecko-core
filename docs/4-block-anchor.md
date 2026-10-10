# Block and Anchor

Version 0.7 · October 10, 2026 · [Русский](4-block-anchor.ru.md)

[Glossary](0-concepts.md) · [Mesh / App](1-mesh-app.md) · [Gecko / Axis](2-gecko-axis.md) · [Line / Point / Edge / View](3-line-point-edge-view.md)

## 1. Blocks and Anchors: Shared Services and Dependencies

**A Block is defined by a domain contract; an Anchor describes a Line’s dependency on a provider of that contract.** For example, an inference contract describes model execution, while a recording storage contract describes writing and reading video segments by camera and time. A running service, container, or Gecko-native implementation becomes a connected Block when it satisfies the contract. An arbitrary process or database nearby does not become a Block merely by exposing a port.

A Block’s contract describes available operations, inputs and results, request/response correlation, errors, and available diagnostics. For inference, it must expose models and versions, their inputs and outputs, invocation rules, and handling of overload or timeout. The exact contract format will be selected separately.

```text
Line front_camera ── Anchor detector ──→ Block inference
Line back_camera  ── Anchor detector ──→ Block inference

Anchor detector:
  required contract: inference
  required capability: selected model and version
  readiness: service reachable, contract compatible, model ready
  behavior when unavailable: explicitly selected Line policy
```

Each Line has its own Inference Point that prepares requests and receives responses. A Point uses a Block through the Line’s declared Anchor; configuration shows which Points use that dependency. The Anchor itself is not a transport: calls are implemented by a contract client or adapter. A Line can have several Anchors, and several Lines can have Anchors referencing one Block.

A Block provider binding is saved explicitly: service name, contract and version, address or built-in binding, and a supported connection implementation. The App composition mechanism may propose it, or the user may specify it. For a third-party service such as Triton, an adapter translates its interface into a Gecko contract, checks compatibility, and discovers available capabilities. A service built for Gecko can implement the contract directly. A process, container, or open port does not replace this validation.

A supported way to fulfill the contract is mandatory. An external service can use its own protocol through an adapter. Discovered service declarations participate in proposed App compositions; Anchor bindings are recorded in the selected composition rather than created merely by discovery.

Restarting a Line does not by itself restart a shared Block. A Block failure can affect all dependent Lines; each Line’s reaction is defined by its Anchors. Declared Blocks and Anchors remain in the App when a service is unavailable. Contract compatibility, service availability, and readiness of the required capability are displayed separately. An Anchor does not imply exclusive ownership of a Block.

### 1.1. An Available Provider and the Ability to Prepare One

A Block contract does not require a particular CPU, GPU, or container. Those requirements belong to the provider implementation. A node distinguishes a running contract provider from a known implementation it can prepare. Suitable hardware alone implies neither a ready Block nor the ability to start one automatically.

| State | Condition | Representation in an App Option |
|---|---|---|
| Available now | The provider is connected, the contract is compatible, and the required capability is ready | Can be used after Anchor validation |
| Can be prepared | The implementation, its requirements, and a supported preparation mechanism are known | Requires preparation with visible steps and results |
| Requires external preparation | A provider is needed, but Gecko cannot prepare it | The dependency is unsatisfied; the missing condition is explained |

An implementation can report its backend, Devices in use, location, and reasons for not being ready. This information exposes the provider’s hardware dependency without making it part of the Block contract. Unknown hardware information about an external service is marked unknown; Gecko does not infer it from a service name or a local GPU.

### 1.2. Optional Preparation and Startup Management

Service preparation is an optional Gecko capability, separate from a Block’s contract and App composition. Connecting an existing service does not require it. For the target UX of “task and input data → App option → verifiable result,” preparation of known providers must be designed alongside composition selection.

| Provider Execution Mechanism | Gecko’s Responsibility |
|---|---|
| An already running external service | Connect and validate the contract and readiness; process management is not assumed |
| A separate program or script | Use a known preparation and startup mechanism, then validate the result |
| A container | Call a supported container runtime or external manager, then validate the service |
| Gecko-native | Create a built-in service through the same contract; execution inside the Gecko process or in a separate process is implementation-defined |

A supported implementation describes requirements, installation, parameters and artifacts (including models), startup, readiness checks, logs, shutdown, and cleanup of created resources. Calling a script is not itself confirmation that a Block is ready. Gecko reports preparation steps, errors, and required actions in service terms. A limited set of known implementations with described lifecycles is preferred for the first version; arbitrary scripts and containers are not automatically supported.

An App saves the selected provider and, where needed, its preparation parameters. Deployment does not become part of the domain contract. Gecko does not have to implement its own container runtime, universal orchestrator, migration, or load balancing. A built-in provider shares resources and the failure risk of the Gecko process; that execution mechanism does not provide process isolation for the Block.

A created provider records its lifecycle owner and dependent consumers. Stopping an App does not automatically stop a shared Block. Reuse, shutdown, and cleanup rules must account for other Apps and Lines; a preparation failure must not delete an external service Gecko does not own.

## 2. Vocabulary Changes

### From Version 0.4 to 0.5

| Previously | Now |
|---|---|
| Block contract, provider, and startup are insufficiently separated | The contract defines a service capability; hardware requirements and preparation belong to the provider implementation |
| Service connection and startup are described generally | Available Blocks, providers that can be prepared, and external preparation are distinguished; external, program, container, and Gecko-native execution options are described |

### From Version 0.2 to 0.3

| Previously | Now |
|---|---|
| A managed Block must have one dedicated process | Block execution is implementation-defined; startup management is separate from the contract |
| Service dependencies have no term of their own | An Anchor explicitly describes a Line’s dependency on a Block, its requirements, and operating conditions |
| External service connection has no explicit contract validation | A Block is connected explicitly through a direct contract implementation or adapter |

## 3. Open Questions

- The first Block domain contract (inference), its direct implementations, and the Triton adapter.
- Anchor format, binding of Points that use Anchors, and concrete policies for service unavailability or lack of readiness.
- Shared Block ownership, access by multiple Apps, control permissions, and supported service startup mechanisms.
- Known service preparation mechanisms, model and artifact acquisition, progress, cleanup after failures, and process selection for Gecko-native providers.
