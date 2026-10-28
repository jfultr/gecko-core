# Gecko Foundational Modules

October 28, 2026 · dependency and work catalog for foundational modules

[Documentation](README.md) · [Development cycle](development-cycle.md) · [Mesh / App](1-mesh-app/model.md) · [Gecko / Probe](2-gecko-probe/model.md)

This catalog lists modules with their own contracts used by several domain areas of the project. It shows dependency direction and work status; it does not introduce a new domain contract or imply a division into processes, services, or repositories. A module is added after its boundary has been agreed. Operation is currently the only module defined this way.

| ID | Module | Responsibility and active contract | Dependencies on other foundational modules | Consumers | Status and next outcome |
|---|---|---|---|---|---|
| CORE-OP | **Operation** | Request acceptance, execution identity, reports and progress, safe retries, cancellation, preservation of operation records, and resuming observation. [Common core and 1.0.0 rules](1-mesh-app/model.md#82-operation-report-core) | No agreed dependencies | Gecko runtime; [Probe](2-gecko-probe/model.md#32-data-blockers-and-reports) runs; App operations | Boundary accepted at the model level; implementation has not started. Next outcome: a separate module contract and a verifiable prototype with a test executor |

Dependencies point **from the consumer to the foundational module**: Probe and App operations use Operation; Operation does not depend on their domain entities. Gecko supplies the execution environment and storage and transport adapters. Defining the module does not select a language, package structure, storage system, or Zenoh keys.

## CORE-OP: Boundary and Work

**Accepted:** implement the Operation module within this project. [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151) are direct architectural references; ROS 2 does not become a project dependency. Similar interfaces do not replace Gecko's agreed rules for retries, preserving operation records, and unknown outcomes.

The module provides a common execution lifecycle and access to reports. Domain executors define inputs, results, blockers, and actual actions. The reconnaissance policy decides which Probe to run; the Planner and App participants determine preparation, readiness, and application of decisions. Operation does not make these domain judgments or start the next check itself.

The active rules currently reside in [Mesh / App §8.2](1-mesh-app/model.md#82-operation-report-core). When designing a separate module contract, the common semantics can be moved out of that section, leaving only domain additions in Mesh / App and Gecko / Probe. Until that migration, this catalog and the future module contract must not define a second, divergent set of rules.

Work on the module can begin independently of completing D2: refine interface boundaries, then verify a prototype with a test executor. Minimum validation must cover acceptance and rejection, a lost response and retry with the same `request_id`, a new request, progress and results, cancellation request and confirmation, restart with preserved operation records, and an explicitly unknown outcome. Integration with Probe, App, Zenoh, and domain storage is validated as their contracts become available. Separate CORE-OP work does not declare the entire upward pass of the [cycle](development-cycle.md) complete.

Further foundational modules, their dependencies, and development order are added here after their boundaries are agreed. Candidates from the [Gecko / Probe plan](2-gecko-probe/work-plan.md#6-documents-and-future-delegation) are not yet approved modules.
