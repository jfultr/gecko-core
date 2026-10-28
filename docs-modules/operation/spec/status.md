# Operation: decision status

October 28, 2026

[Overview](../README.md) · [Model](model.md) · [Example roadmap](roadmap.md) · [Security](security.md)

## 1. Accepted decisions

| Decision | Status | Original context |
|---|---|---|
| Operation is an independent foundational module, not part of the reconnaissance system | Accepted | [Foundational module catalog](../../../docs/core-modules.ru.md#core-op-граница-и-работа), [research](../../../docs/2-gecko-axis/researches/operations-and-reconnaissance.ru.md#вопрос-и-краткий-вывод) |
| The module is implemented within Gecko; ROS 2 Actions and Google LRO are direct references, not dependencies | Accepted | [Mesh / App §8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report), [comparison of existing solutions](../../../docs/2-gecko-axis/researches/operations-and-reconnaissance.ru.md#1-насколько-operation-похожа-на-ros-2-action) |
| Operation does not depend on Axis, App, or their domain entities | Accepted | [CORE-OP catalog entry](../../../docs/core-modules.ru.md#core-op-граница-и-работа), [Mesh / App §8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report) |
| A request, an execution, and a report are distinct concepts; acceptance does not promise success | Accepted | [Mesh / App §8.1–8.2](../../../docs/1-mesh-app/model.ru.md#81-назначение-и-основные-понятия) |
| The client supplies `request_id`, the receiver supplies `operation.id`, and a safe retry returns the previous operation | Accepted | [Mesh / App §8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report) |
| Protection against duplicate execution survives a normal restart when accounting is preserved | Accepted | [Mesh / App §8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report) |
| Client disconnection does not cancel execution, and an unknown outcome is represented explicitly | Accepted | [Mesh / App §7.3](../../../docs/1-mesh-app/model.ru.md#73-недоступность-и-восстановление), [§8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report) |
| Cancellation is a request to stop, not immediate proof of `cancelled`; partial results remain meaningful | Accepted | [Mesh / App §8.2](../../../docs/1-mesh-app/model.ru.md#82-ядро-operation-report) |
| A parent relationship must not automatically imply ownership, dependency, or cancellation propagation | Accepted as a boundary | [Research §3](../../../docs/2-gecko-axis/researches/operations-and-reconnaissance.ru.md#3-дерево-запусков-и-граф-зависимостей), [Axis §3.4](../../../docs/2-gecko-axis/model.ru.md#34-предварительные-axis-и-цепочки-проверок) |
| Axis uses Operation for an individual run but does not select the next check itself | Accepted at the domain level | [Axis §3.4](../../../docs/2-gecko-axis/model.ru.md#34-предварительные-axis-и-цепочки-проверок), [Axis §3.6](../../../docs/2-gecko-axis/model.ru.md#36-axis-в-zenoh-api-gecko) |
| Module work may proceed independently of completing the Axis domain design | Accepted | [Development cycle progress](../../../docs/development-cycle.ru.md#1-маршрут-и-результаты), [CORE-OP catalog entry](../../../docs/core-modules.ru.md#core-op-граница-и-работа) |

## 2. Open decisions

This is a working register, not an attempt to predetermine the entire implementation.

| Area | Question to resolve | Required result |
|---|---|---|
| States | Complete state machine for acceptance, execution, cancellation request, completion, and `unknown`; permitted races | Transition table and invariant tests |
| Identities | Format and uniqueness scope of `request_id` and `operation.id`; canonical request comparison | Versioned schema and conflict rules |
| API | Start/get/watch/cancel commands, acceptance/rejection responses, errors, and connection loss | Transport-neutral public contract |
| Executor | Work handoff, executor acknowledgement, progress, partial result, and final result | Executor API and error model |
| Storage | Atomic record before side effects, journal/snapshots, recovery, and detailed-report retention | Storage API, crash-consistency model, and contract tests |
| Cancellation | Who may cancel, repeated and late cancellation, cooperative or forced stop, and completion races | Standalone cancellation contract |
| Composite operations | Causality, links, parent/child relations, ownership, and independent cancellation scope | Relationship model without implicit cascading semantics |
| Concurrency | Admission control, limits, queues, and conflicting actions without a global lock | Common hooks and domain conflict rules |
| Reports | Minimum common schema, extensions, snapshot ordering, and removal of details or artifacts | Versioned Operation Report schema |
| Transport | Map the API and observation onto Zenoh without leaking its details into Core | First adapter and common transport tests |
| Compatibility | Whether ROS 2 Actions or Google LRO mappings and adapters are required | Explicit compatibility targets or an explicit decision not to provide them |
| Observability | Correlation with traces, logs, and metrics without making telemetry the source of truth | Correlation context and export rules |
| Implementation | First language, package/process boundaries, schema format, and supported platforms | ADR after validating the minimum prototype |
| Security | Caller identity, authorization for every command, auditing, data protection, and resource exhaustion | Threat model and verifiable security gates before `1.0.0` |

## 3. Current status

- The module boundary is accepted.
- An active common contract exists in Mesh / App §8.2.
- The standalone specification is a draft.
- Implementation, prototype, and contract tests have not started.
- The exact release breakdown is not approved.
- The source of truth has not yet moved from Mesh / App into this directory.

The next substantive result is to approve a minimum state machine and transport-neutral API, then validate them with an in-memory prototype, a test executor, and fault and concurrency tests.
