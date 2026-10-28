# Mesh / App: Requirements on Lower Levels

[Active Mesh / App model](model.md) · [Development cycle](../development-cycle.md)

This document passes requirements from the local 1.0.0 model to subsequent documents at steps D2–D4 of the development cycle. Each domain document must define a supported contract and its limitations; after the downward pass, an experiment will test the contract's applicability. These requirements follow from the [Mesh / App model](model.md), without selecting concrete formats, storage mechanisms, or implementations. Return questions R1–R8 remain in the cycle plan and do not delay moving to the next level.

## Gecko / Axis

The [domain document](../2-gecko-axis/model.md) must define the minimum local facts and checks; preliminary Axis runs after Gecko startup, conditions for invoking subsequent Axes, and direct invocation of an individual Axis by a client. Required capabilities include run identity, progress, results, blockers, freshness, the state of current Axis work on Gecko, and access to partial results for a client that connects later. Local requests and reports preserve the meaning of `request_id`, Operation Report, and the distinction between unknown and negative results (Mesh / App, §2.2–2.3, §8.2).

On the upward pass, validate the [Gecko startup and client connection scenario](../2-gecko-axis/scenario.md): preliminary and child Axes, partial and final results, direct invocation before an App exists, retrying an accepted request without a second execution, and a new run after conditions change. A blocker in one check does not hide facts already available; the absence of current Axis work does not confirm their completeness.

## Line / Point / Edge / View

The [domain document](../4-line-point-edge-view/model.md) must define supported built-in Points and connections for the local scenario; inputs, outputs, control, data, and diagnostics without requiring a View. Domain readiness checks, conditions for changing and stopping a Line, and minimum policies for performance, overload, and unavailable dependencies are required (Mesh / App, §3.3–3.4, §4.4, §5.4, §6.3–6.5).

On the upward pass, process a file or Camera, obtain a domain result through a participant's contract, change configuration, and observe the actual result. Connecting or disconnecting a View does not control execution.

## Block / Anchor

The [domain document](../3-block-anchor/model.md) must define minimum provider and dependency contracts for the chosen scenario: compatibility, availability, readiness, failure, supported preparation operations, and limits of external service management. Ownership, reuse, and release conditions for a shared Block are required (Mesh / App, §5.5, §6.2).

On the upward pass, prepare a provider or use an existing one, check dependency readiness, and release resources following an explicit action. Stopping one consumer does not stop another consumer's shared Block.

## End-to-End Validation

Before turning upward, select the minimum set of Points, Edges, and Blocks for one reproducible local scenario (Mesh / App, §6.6). The experiment must cover preparation through separate Planner calls, a fresh readiness check, Activation, an observable result, a change, Stop, and explicit cleanup. Validate blockers, partial results, request retries, and restart without treating an unknown outcome as success or absence of action (Mesh / App, §6.6, §8.6). Results and limitations return to Mesh / App for R1–R8 decisions; this list does not claim that validation has already taken place.
