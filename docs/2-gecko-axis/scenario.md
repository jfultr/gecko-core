# Gecko / Axis: Preliminary Checks and a Client Request

[Gecko / Axis model](model.md) · [Mesh / App requirements](../1-mesh-app/lower-level-requirements.md) · [Development cycle](../development-cycle.md)

**Status:** an end-to-end scenario for developing 1.0.0 D2 contracts. It illustrates rules from [Mesh / App, §2.2–2.3 and §8.2](../1-mesh-app/model.md) and [Gecko / Axis, §3](model.md); the example introduces no new requirements. Check names below are illustrative.

## Scenario Flow

1. The user starts one Gecko. Gecko begins a preliminary Device discovery check as an ordinary Axis run under the [shared execution and management rules](model.md#34-preliminary-axes-and-check-chains). The run has its own identity and Operation Report; its context records automatic initiation at Gecko startup. Gecko's state shows that Axis work is currently in progress.
2. The check finds a Device and publishes it before completing. Gecko's reconnaissance policy matches the finding against applicability conditions of available Axes and starts a check of the discovered Device's properties or accessibility. A client can see the discovered Device while the next check is still running; the finding itself does not confirm that the Device is suitable for particular processing.
3. A client connects after Gecko has started work. It receives facts already available and the latest available Axis run reports, then observes new results. The absence of current Axis work is not evidence of complete information.
4. For its task, the client invokes an individual Axis with a specific subject and parameters, without creating an App. Gecko accepts a valid request and returns a run identifier and observation address, or rejects it with a reason. If the acceptance response is lost, the client repeats the same request with the same `request_id` and receives the previous run without a second execution.
5. During its check, the Axis reports progress and partial results. If it finds a blocker requiring an external fix, the run ends with `state: failed`, retaining the blocker and facts already obtained. Completion of this run does not mean Gecko is still executing an Axis. After fixing the problem, the client explicitly sends a new request with a new `request_id`; Gecko creates a new run, and the previous result remains distinguishable from the new one.

## What Must Be Observable

- Gecko's state answers whether Axis work is currently in progress; it does not assess App readiness.
- Each Axis run exposes its subject, parameters, progress, partial and final results, blockers, and freshness of the facts obtained.
- A client can access already published facts regardless of when it connected or whether current Axis work has completed.
- Retrying an accepted request does not run the check again; a new check after conditions change has a new identity.

## Variant: The User Changes Current Work

A client connects during preliminary checks, requests cancellation of one run, and starts an additional chain. These actions use the shared Axis run management interface. Reports reflect the actual state of the corresponding runs, while Gecko's state reflects current work after the changes. No special preliminary-check completion stage appears. Each run's initiator remains identifiable in its context.

The variant does not assume instant cancellation: propagation to child runs, handling partial results, and resource conflicts require further decisions.

Specific preliminary checks, Gecko policy rules for selecting subsequent Axes, Gecko's state format, Zenoh keys, and retention policy are defined by domain contracts and validated on the upward pass. A blocker does not become a general prohibition on the client using facts already known or requesting another check.
