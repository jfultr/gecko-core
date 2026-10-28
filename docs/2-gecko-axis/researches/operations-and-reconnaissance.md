# Operations and Environment Reconnaissance: Review of Existing Solutions

October 28, 2026 · status: research, not an accepted architectural decision

[D2 plan](../work-plan.md) · [Gecko / Axis model](../model.md) · [Operation Report core](../../1-mesh-app/model.md#82-operation-report-core)

## Question and Brief Conclusion

Gecko documents already show two related mechanisms: common execution of observable operations, and collection of environment facts through Axis. Their responsibilities should be distinguished. Operation handles request acceptance, execution identity, progress, results, cancellation, and retries. Reconnaissance handles the check set, invocation conditions, diagnostic results, and environment picture. An Axis check uses the operation mechanism; Mesh / App operations use the same core without reconnaissance domain concepts.

This proposes a logical boundary rather than splitting parts into processes, packages, or repositories. In particular, it does not close A1–A3 or B1 in the D2 plan.

## 1. How Similar Is Operation to ROS 2 Action

[ROS 2 Action](https://design.ros2.org/articles/actions.html) already defines a named action, goal submission, acceptance or rejection, goal identity, a feedback stream, final result, and cancellation request. The described implementation uses three services (`send_goal`, `get_result`, `cancel_goal`) and two topics (`feedback`, `status`). Accepted cancellation does not yet mean actual cessation: `CANCELING` is separate from `CANCELED`. This is a useful operation-interface reference rather than a reason to reinvent those mechanics.

Gecko's current contract goes beyond basic Action in significant areas. It distinguishes client-supplied `request_id` from Gecko-assigned `operation.id`, preserves protection against duplicate operation creation after normal restart, explicitly allows an unknown outcome, exposes partial results, and separates operation execution from domain conclusions. ROS 2 Action has configurable result retention, but that alone does not satisfy our durable accepted-request record contract. This compares basic contracts; additional guarantees can be built on ROS 2.

Similar solutions exist outside ROS. [Google AIP-151](https://google.aip.dev/151) defines a Long-running Operation resource for a lengthy invocation; [AIP-155](https://google.aip.dev/155) separately describes client `request_id` for safe request retries. This pair is closer to our request/execution identity separation.

If ROS 2 becomes an interoperability target, exposing a real Action interface can be investigated. Using Zenoh does not make an arbitrary Gecko API ROS-compatible: [ROS `rmw_zenoh`](https://github.com/ros2/rmw_zenoh/blob/rolling/docs/design.md) carries ROS interfaces over Zenoh, and Actions are composed of ROS services and publications on top of it.

**Working conclusion:** logically separating a common operation core makes sense, but its contract should be compared point by point with ROS Actions and Google LRO. Copying the ROS protocol is unnecessary without a ROS compatibility need; similar models do not automatically establish interoperability.

## 2. Reconnaissance References and Check Sources

Among the systems studied, none was found whose published contract simultaneously covers local media/AI environment reconnaissance, active tests, addressed invocation, partial facts, agent explanations, and our accepted operation guarantees. Useful parts appear in different projects:

| System | Implemented part | What to study for Gecko |
|---|---|---|
| [osquery](https://osquery.readthedocs.io/en/stable/deployment/configuration/) | System facts are accessible through SQL-like queries; scheduling and *discovery queries* enable check sets only on suitable hosts | A source of some OS facts and an example of conditional checks. Active camera and inference tests remain Gecko domain executors |
| [Velociraptor](https://docs.velociraptor.app/docs/artifacts/) | Named collection procedures contain parameters, sources, applicability conditions, external tool descriptions, and composition mechanisms | A reference for Axis catalog form. Its *artifact* means a collection recipe, whereas an artifact in our model is an obtained result; the terms must not be equated |
| [Kubernetes Node Feature Discovery](https://kubernetes-sigs.github.io/node-feature-discovery/stable/get-started/introduction.html) | Feature sources discover node properties; separate rules turn features into Kubernetes declarations | The boundary between a machine observation and a conclusion about its capabilities. Publishing into Kubernetes is not mandatory for Gecko |
| [Nmap Scripting Engine](https://nmap.org/book/nse-script-format.html) | A script contains an applicability condition, action, and where necessary an execution-order dependency; the condition uses an already discovered subject | A reference for progressively starting specialized checks on discovered objects |
| [Temporal](https://docs.temporal.io/activity-definition) | Durable workflows, history, and action retries | A recovery and composition reference if a simple local executor proves insufficient. Retrying an Activity can repeat an actual action; the documentation recommends idempotency. This requires separate reconciliation with our prohibition on blind re-execution |

Narrower libraries and tools can execute individual checks. [hwloc](https://www.open-mpi.org/projects/hwloc/) exposes processor, memory, and some device topology; [NVIDIA DCGM](https://developer.nvidia.com/dcgm) offers monitoring and diagnostics for supported data-center GPUs. They can be Axis adapters or sources without defining the common operation protocol or reconnaissance policy.

Initial references for detailed comparison: Velociraptor for check descriptions; Nmap NSE and osquery for applicability and invocation; NFD for the transition from facts to advertised capabilities. This assesses fit for Gecko rather than deciding to use their code.

## 3. Run Tree and Dependency Graph

Progressive reconnaissance is useful: a cheap check discovers a subject, then only applicable specialized checks run. Conditional execution appears in osquery, Velociraptor, and Nmap NSE. A run-origin tree need not be the only model for every relationship between checks.

| Relationship | Example | Model implication |
|---|---|---|
| Reason for starting | Device discovery found a GPU, so an access test started | A parent reference can explain origin |
| Information dependency | An inference test requires facts about a GPU, driver, and backend | A check can have several incoming dependencies: a graph, often acyclic, rather than a tree |
| Work and cancellation ownership | Cancelling a composite test must affect its owned subtasks | Cancellation scope is explicit rather than automatically following the launch-reason reference |

These relationships can diverge. A discovery run can finish while a test triggered after it is still running. Several checks can use one discovery result. A combined-load check can depend on facts about several devices. [Dask Task Graphs](https://docs.dask.org/en/stable/graphs.html) uses data dependency graphs to determine order and concurrency; this is a dependency-form reference rather than a proposal to adopt Dask as Gecko's runtime.

A possible model to consider: retain an origin tree or forest for observation, express dependencies through separate references, and specify cancellation scope separately. New tasks can be added as subjects are discovered without building the whole graph in advance. A discovered fact makes a check a candidate, but actual invocation must consider user intent and permitted load. Under the current model, two different explicit user requests remain two distinguishable runs; matching inputs do not make them retries of one request.

The key B1 question remains: does Axis initiate subsequent checks itself, or report findings from which Gecko selects subsequent runs? A coordinated approach is also possible. This choice affects depth and run-count limits, resource management, composite results, and cancellation.

## 4. Consensus and Interoperability

No single common standard was found for adaptive machine reconnaissance with media/AI checks. Recurring choices in the examined systems are: distinguish a check description from its run; specify applicability separately from the action; retain fact provenance; explicitly manage dependencies and load; separately describe request acceptance, actual execution, and outcome. This is a synthesis of sources rather than a normative standard.

Standards and conventions cover individual boundaries:

| Boundary | Reference | Applicability limit |
|---|---|---|
| Long-running action | [ROS 2 Actions](https://design.ros2.org/articles/actions.html), [Google Long-running Operations](https://google.aip.dev/151) | Invocation and result models differ; select target compatibility or provide an adapter |
| Tracing | [OpenTelemetry](https://opentelemetry.io/docs/concepts/signals/traces/) | Spans, parent relationships, and links expose causality; tracing does not own execution or replace an authoritative Operation Report |
| Event publication | [CloudEvents](https://github.com/cloudevents/spec/blob/v1.0.2/cloudevents/spec.md) | Common event envelope; diagnostic fact meaning, freshness, and applicability remain a domain contract |
| Server hardware facts | [DMTF Redfish](https://www.dmtf.org/standards/redfish) | Data source and potential integration point for supporting hardware |
| Security and configuration checks | [SCAP / OVAL / XCCDF](https://www.open-scap.org/features/scap-components/) | An interoperable language in its domain; does not cover arbitrary media/AI tests |

**Preliminary architectural hypothesis:** a small common operation core, with a check catalog and execution above it, subsequent-check selection rules, and a separate fact picture. External tools connect as data sources or executors of individual Axes. Test this hypothesis with a scenario where device discovery leads to several independent checks and one combined check, while the client cancels one and starts another during execution.
