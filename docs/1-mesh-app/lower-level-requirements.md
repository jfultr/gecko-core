# Mesh / App: Requirements on Lower Levels

[Active Mesh / App model](model.md) · [Development cycle](../development-cycle.md)

This document passes requirements from the local 1.0.0 model to subsequent documents at steps D2–D4 of the development cycle. Each domain document must define a supported contract and its limitations; after the downward pass, an experiment will test the contract's applicability. These requirements follow from the [Mesh / App model](model.md), without selecting concrete formats, storage mechanisms, or implementations. Return questions R1–R8 remain in the cycle plan and do not delay moving to the next level.

## Operation

Mesh / App consumes the independent [Operation contract](../../docs-modules/operation/spec/model.md). Requirements below describe what the application needs for local Gecko 1.0.0; implementation, common API, and lifecycle design belong to Operation. Module release scope remains separately tracked in its [decision register](../../docs-modules/operation/spec/status.md).

| ID | Consumer requirement | Contract or integration work |
|---|---|---|
| APP-OP1 | Distinguish acceptance from execution success; identify and observe each accepted action; expose rejection reasons | [Requests and API](../../docs-modules/operation/spec/model.md#4-requests-identity-and-public-api) |
| APP-OP2 | Lost responses, concurrent retries, and normal restarts must not repeat accepted domain work | [Identity](../../docs-modules/operation/spec/model.md#4-requests-identity-and-public-api), [accounting and recovery](../../docs-modules/operation/spec/model.md#8-accounting-retention-and-recovery); mechanisms remain open |
| APP-OP3 | Reconnecting clients obtain available progress, partial results, blockers, and outcomes; unavailable detail must be explicit | [Reports](../../docs-modules/operation/spec/model.md#5-report-content), [snapshots](../../docs-modules/operation/spec/model.md#6-snapshots-and-core-authority), [adapter investigation](../../docs-modules/operation/spec/model.md#9-transport-adapter-investigation) |
| APP-OP4 | Cancellation and waiting timeouts must not falsely establish cessation or overwrite actual completion; partial actions remain visible | [Lifecycle and cancellation](../../docs-modules/operation/spec/model.md#7-lifecycle-and-cancellation); App Coordinator duties in [model §8.2](model.md#82-requirements-on-operation-and-app-specific-use) |
| APP-OP5 | Preserve mutating App request records throughout App existence; after deletion, old requests remain permanently inadmissible even if records are removed | [App retention and deletion](model.md#82-requirements-on-operation-and-app-specific-use); safe module accounting-release interface remains open in [Storage](../../docs-modules/operation/spec/status.md#2-open-decisions) |
| APP-OP6 | Represent App revision, readiness, permission application, resources, and participant results without putting their domain logic into Core | [Common report](../../docs-modules/operation/spec/model.md#5-report-content); domain profiles in [§6.3](model.md#63-app-readiness-readiness-profile-in-the-operation-protocol), [§8.3–8.4](model.md#83-decision-and-its-content) |
| APP-OP7 | Expose uncertainty without a global App lock or automatic repetition, rollback, or cleanup | [Lifecycle](../../docs-modules/operation/spec/model.md#7-lifecycle-and-cancellation); domain admission and cleanup in [§4.2](model.md#42-creation-and-change-acceptance) and [§7.3](model.md#73-unavailability-and-recovery) |
| APP-OP8 | Apply access checks consistently; UI, role, or agent interaction must not grant extra authority | [Module security](../../docs-modules/operation/spec/security.md), App [§7.4](model.md#74-deferred-capabilities); concrete policy and integration remain open |

Validate these requirements through the [App integration scenarios](model.md#86-app-integration-and-validation-with-zenoh). Common conformance checks belong to the module; App lifecycle checks remain the consumer's responsibility. Satisfaction at the model level does not prove implementation conformance.

## Gecko / Probe

The [domain document](../2-gecko-probe/model.md) must define the minimum local facts and checks; preliminary Probe runs after Gecko startup, conditions for invoking subsequent Probes, and direct invocation of an individual Probe by a client. It must distinguish available Probes from advertised environment Capabilities and define how facts support Capability availability, conditions, and freshness. Required behavior includes run identity, progress, results, blockers, freshness, the state of current Probe work on Gecko, and access to partial results for a client that connects later. Local requests and reports use the [Operation contract](../../docs-modules/operation/spec/model.md), preserving request identity and the distinction between unknown and negative results; Mesh / App §2.2–2.3 supplies the domain needs.

On the upward pass, validate the [Gecko startup and client connection scenario](../2-gecko-probe/scenario.md): preliminary and child Probes, partial and final results, direct invocation before an App exists, retrying an accepted request without a second execution, and a new run after conditions change. A blocker in one check does not hide facts already available; the absence of current Probe work does not confirm their completeness.

## Line / Point / Edge / View

The [domain document](../4-line-point-edge-view/model.md) must define supported built-in Points and connections for the local scenario; inputs, outputs, control, data, and diagnostics without requiring a View. Domain readiness checks, conditions for changing and stopping a Line, and minimum policies for performance, overload, and unavailable dependencies are required (Mesh / App, §3.3–3.4, §4.4, §5.4, §6.3–6.5).

On the upward pass, process a file or Camera, obtain a domain result through a participant's contract, change configuration, and observe the actual result. Connecting or disconnecting a View does not control execution.

## Block / Anchor

The [domain document](../3-block-anchor/model.md) must define minimum provider and dependency contracts for the chosen scenario: compatibility, availability, readiness, failure, supported preparation operations, and limits of external service management. Ownership, reuse, and release conditions for a shared Block are required (Mesh / App, §5.5, §6.2).

On the upward pass, prepare a provider or use an existing one, check dependency readiness, and release resources following an explicit action. Stopping one consumer does not stop another consumer's shared Block.

## End-to-End Validation

Before turning upward, select the minimum set of Points, Edges, and Blocks for one reproducible local scenario (Mesh / App, §6.6). The experiment must cover preparation through separate Planner calls, a fresh readiness check, Activation, an observable result, a change, Stop, and explicit cleanup. Validate blockers, partial results, request retries, and restart without treating an unknown outcome as success or absence of action (Mesh / App, §6.6, §8.6). Results and limitations return to Mesh / App for R1–R8 decisions; this list does not claim that validation has already taken place.
