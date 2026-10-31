# Gecko / Probe: Preliminary Checks and a Client Request

[Gecko / Probe model](model.md) · [Mesh / App requirements](../1-mesh-app/lower-level-requirements.md) · [Development cycle](../development-cycle.md)

**Status:** an end-to-end scenario for developing 1.0.0 D2 contracts. The execution flow illustrates rules from [Mesh / App, §2.2–2.3 and §8.2](../1-mesh-app/model.md) and [Gecko / Probe, §3](model.md). The domain cases below record the agreed direction for limited 1.0.0 reconnaissance and independent manual configuration; their implications for the model remain to be reconciled separately. Check names are illustrative.

## Scenario Flow

1. The user starts one Gecko. Gecko begins a preliminary Device discovery check as an ordinary Probe run under the [shared execution and management rules](model.md#34-preliminary-probes-and-check-chains). The run has its own identity and Operation Report; its context records automatic initiation at Gecko startup. Gecko's state shows that Probe work is currently in progress.
2. The check finds a Device and publishes it before completing. Gecko's reconnaissance policy matches the finding against applicability conditions of available Probes and starts a check of the discovered Device's properties or accessibility. A client can see the discovered Device while the next check is still running; the finding itself does not confirm that the Device is suitable for particular processing.
3. A client connects after Gecko has started work. It receives facts already available and the latest available Probe run reports, then observes new results. The absence of current Probe work is not evidence of complete information.
4. For its task, the client invokes an individual Probe with a specific subject and parameters, without creating an App. Gecko accepts a valid request and returns a run identifier and observation address, or rejects it with a reason. If the acceptance response is lost, the client repeats the same request with the same `request_id` and receives the previous run without a second execution.
5. During its check, the Probe reports progress and partial results. If it finds a blocker requiring an external fix, the run ends with `state: failed`, retaining the blocker and facts already obtained. Completion of this run does not mean Gecko is still executing a Probe. After fixing the problem, the client explicitly sends a new request with a new `request_id`; Gecko creates a new run, and the previous result remains distinguishable from the new one.

## 1.0.0 Domain Cases: Limited Reconnaissance

Reconnaissance is part of the intended 1.0.0 scope, with a limited set of useful checks that saves manual discovery and configuration time. This release does not require a comprehensive reconnaissance system. The cases below define the intended coverage; exact interfaces, check depth, and success criteria remain to be specified.

| Case | Scenario and useful result | Scope boundary |
|---|---|---|
| Basic machine information | Gecko obtains basic CPU information, including architecture, memory information, and disk information. A client can use the available facts when configuring work on that Gecko. | A basic environment picture, without an exhaustive hardware assessment or performance test suite |
| Accelerator discovery | Gecko discovers and identifies GPUs and NPUs where supported by the limited discovery checks, and exposes the findings to the client. | Discovering an accelerator does not establish that Gecko can use it. NPU execution support and support for additional accelerator types are deferred beyond 1.0.0 |
| DeepStream and OpenVINO support | Gecko checks the environment for DeepStream and OpenVINO support and exposes the findings and the limits of what was checked. A client can use these results to simplify processing configuration. | The exact level of validation remains to be defined; detecting installed software does not by itself confirm successful execution of a selected workload or support for every accelerator |
| Camera discovery | Gecko discovers cameras through a limited set of supported interfaces, including scenarios involving RTSP sources. A client sees discovered sources and can quickly use a finding to configure a camera Point. | Supported interfaces and RTSP discovery methods remain to be selected. The case does not promise to find every camera or RTSP stream |

## Variant: Create Points and Blocks with or without Reconnaissance

1. A user creates and configures Points and Blocks without requiring a reconnaissance run, its completion, or a positive finding. The user can supply the necessary source and implementation parameters manually.
2. If Gecko discovers a camera, the client exposes that finding and provides quick access to configuring a Point from it, reusing the available source parameters. Discovery does not by itself create a configured Point.
3. If Gecko does not discover the camera, the user can configure its source manually, for example by supplying an RTSP address. An absent finding does not establish that the source is absent or unusable.
4. Available hardware and DeepStream/OpenVINO findings similarly help the user select and configure processing Points or Blocks. Incomplete or unavailable reconnaissance results do not prohibit their creation.
5. Whether the resulting configuration can execute at a selected location is a separate question. An unsupported CPU architecture or unavailable required backend may prevent that execution; it does not prohibit creating the Point or Block. The precise validation and launch rules belong to the corresponding contracts and are not settled by this scenario.

Reconnaissance simplifies and accelerates configuration. Its coverage does not define which Points and Blocks the user is allowed to create. Manual configuration remains available when discovery is incomplete or does not cover the user's environment.

## What Must Be Observable

- Gecko's state answers whether Probe work is currently in progress; it does not assess App readiness.
- Each Probe run exposes its subject, parameters, progress, partial and final results, blockers, and freshness of the facts obtained.
- A client can distinguish available Probes from advertised environment Capabilities. A Probe result can provide evidence about Capability availability; the check's presence alone does not establish it ([model §2.1](model.md#21-capability-and-probe)).
- A client can access already published facts regardless of when it connected or whether current Probe work has completed.
- Retrying an accepted request does not run the check again; a new check after conditions change has a new identity.

## Variant: The User Changes Current Work

A client connects during preliminary checks, requests cancellation of one run, and starts an additional chain. These actions use the shared Probe run management interface. Reports reflect the actual state of the corresponding runs, while Gecko's state reflects current work after the changes. No special preliminary-check completion stage appears. Each run's initiator remains identifiable in its context.

The variant does not assume instant cancellation: propagation to child runs, handling partial results, and resource conflicts require further decisions.

Specific preliminary checks, Gecko policy rules for selecting subsequent Probes, Gecko's state format, Zenoh keys, and retention policy are defined by domain contracts and validated on the upward pass. A blocker does not become a general prohibition on the client using facts already known or requesting another check.
