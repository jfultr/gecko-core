# Operation: model and boundaries

Specification draft 0.1 · module `0.1.0-dev` · October 28, 2026

[Overview](../README.md) · [Decision status](status.md) · [Example roadmap](roadmap.md) · [Security](security.md)

## 1. Purpose

Operation is an independent execution core for requests whose results may not be available at call time. It provides the same mechanics for Probe diagnostics, App preparation and modification, and future domain actions without incorporating their domain logic.

The module must allow a caller to:

1. submit a request with its own identity;
2. determine unambiguously whether the request was accepted or rejected;
3. observe the specific execution, its progress, partial data, and outcome after acceptance;
4. retry after a lost response without creating a second execution;
5. request cancellation and separately determine the actual outcome;
6. resume observation after a client disconnect or a normal runtime restart;
7. see an explicitly unknown outcome instead of an unsafe assumption of success, failure, or continued execution.

The architectural references are [ROS 2 Actions](https://design.ros2.org/articles/actions.html) and [Google Long-running Operations](https://google.aip.dev/151). They are not runtime dependencies and do not imply API compatibility.

## 2. Responsibility boundary

Operation is responsible for:

- request acceptance, common-contract validation, and rejection;
- distinguishing the caller-provided `request_id` from the created execution's `operation.id`;
- idempotently mapping a repeated request to an already accepted operation;
- lifecycle, ordered report snapshots, progress, and terminal outcome;
- cancellation requests and observable cancellation processing;
- durable minimal accounting, restoration of observation, and explicit `unknown` outcomes;
- common enforcement points for authorization, auditing, and resource limits;
- transport-neutral client and executor contracts.

Operation is not responsible for:

- selecting the next Probe or defining the reconnaissance strategy;
- evaluating domain readiness of an App, constructing Build, or making Planner decisions;
- defining the meaning of inputs, results, blockers, artifacts, or acquired resources of a domain action;
- automatically rolling back, cleaning up, or resuming arbitrary domain actions;
- isolating untrusted executor code or securing the executor's own domain logic;
- making Zenoh an irreplaceable part of the internal core.

An executor defines the domain action and reports observable changes. A consumer decides what to do with the result. For example, a Probe publishes findings through Operation, while Gecko's reconnaissance policy decides which checks to start next.

## 3. Intended module parts

These are logical boundaries, not an approved process or package layout.

| Part | Responsibility |
|---|---|
| **Core** | Identities, states, transitions, acceptance/rejection, retries, cancellation, and invariants |
| **Executor API** | Contract for integrating a domain executor and publishing progress or results |
| **Storage API** | Atomic minimal request and operation accounting, report snapshots, and recovery |
| **Server API** | Common start, read, observe, and cancel commands |
| **Client library** | Safe `request_id` creation, retries, observation, and error interpretation |
| **Transport adapters** | Mapping the common API onto Zenoh and, where needed, other transports |
| **Contract tests** | Shared conformance tests for storage, transport, and executor implementations |

The first prototype may run in one process and use in-memory storage if those limitations are explicit and the interfaces do not bind Core to a specific transport.

## 4. Minimum invariants

The properties below have already been accepted. The exact schema and state machine still require a separate decision.

- A rejected request does not create a synthetic Operation.
- Acceptance creates exactly one distinct execution and does not promise success.
- The same `request_id` and content within one idempotency scope identify the previous Operation.
- Reusing a `request_id` with different content is a conflict.
- An intentional new attempt uses a new `request_id`.
- The acceptance and deduplication record is stored before domain side effects begin.
- Client disconnection does not cancel an accepted Operation.
- A cancellation request is not the same as confirmed cancellation.
- A terminal report does not return to `running`; a later snapshot cannot replace an established fact with a less certain state.
- After restart, a stored `running` state alone does not prove that execution continues.
- An unknown outcome is reported explicitly and does not permit blind repetition of the domain action.
- Partial results, blockers, and limitations retain their meaning independently of the final execution state.

## 5. Versioning

The module version follows SemVer and applies to its public contract, compatible libraries, and conformance tests. The `-dev` suffix denotes an unreleased version under development.

The document version identifies a text revision and does not prove that an implementation exists. Breaking changes are allowed before `1.0.0`, but they must be recorded explicitly in the decision status and migration notes. The stability criteria for `1.0.0` cover not only API shape but also recovery, failure behavior, and [security](security.md).
