# Gecko Foundational Modules

November 1, 2026 · dependency and work catalog for foundational modules

[Documentation](README.md) · [Development cycle](development-cycle.md) · [Mesh / App](1-mesh-app/model.md) · [Gecko / Probe](2-gecko-probe/model.md)

This catalog lists modules with their own contracts used by several domain areas of the project. It shows dependency direction and work status; it does not introduce a new domain contract or imply a division into processes, services, or repositories. A module is added after its boundary has been agreed. Operation is currently the only module defined this way.

| ID | Module | Responsibility and active contract | Dependencies on other foundational modules | Consumers | Status and next outcome |
|---|---|---|---|---|---|
| CORE-OP | **Operation** | Request acceptance, execution identity, reports and progress, safe retries, cancellation, preservation of operation records, and resuming observation. [Module contract](../docs-modules/operation/spec/model.md) | No agreed dependencies | Gecko runtime; [Probe](2-gecko-probe/model.md#32-data-blockers-and-reports) runs; App operations | Boundary and first experiment accepted at the model level; a build and test harness skeleton exists in [`crates/operations`](../crates/operations/README.md), without Start/Get/Observe. Next outcome: validate Start/Get/Observe through minimal Zenoh with a test executor |

Dependencies point **from the consumer to the foundational module**: Probe and App operations use Operation; Operation does not depend on their domain entities. Gecko supplies the execution environment and storage and transport adapters. Defining the module does not select a language, package structure, storage system, or Zenoh keys.

## CORE-OP: Boundary and Work

**Accepted:** implement the Operation module within this project. [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151) are direct architectural references; ROS 2 does not become a project dependency. Similar interfaces do not replace Gecko's agreed rules for retries, preserving operation records, and unknown outcomes.

The module provides a common execution lifecycle and access to reports. Domain executors define inputs, results, blockers, and actual actions. The reconnaissance policy decides which Probe to run; the Planner and App participants determine preparation, readiness, and application of decisions. Operation does not make these domain judgments or start the next check itself.

Operation owns its [common contract](../docs-modules/operation/spec/model.md) and [decision register](../docs-modules/operation/spec/status.md). Mesh / App and Gecko / Probe supply consumer requirements and domain integrations. Their documents reference the module instead of defining its internal design. The ownership correction was recorded on November 1, 2026.

Work on the module can begin independently of completing D2. The [first experiment](../docs-modules/operation/spec/model.md#31-agreed-first-experiment-010) validates Start/Get/Observe through minimal Zenoh with a test executor, including late observation, safe retry, and uncertainty. Cancellation follows in `0.2.0`; preserved records and recovery after restart follow in `0.3.0`. Integration with Probe, App, and domain storage is validated as their contracts become available. Separate CORE-OP work does not declare the entire upward pass of the [cycle](development-cycle.md) complete.

Further foundational modules, their dependencies, and development order are added here after their boundaries are agreed. Candidates from the [Gecko / Probe plan](2-gecko-probe/work-plan.md#6-documents-and-future-delegation) are not yet approved modules.
