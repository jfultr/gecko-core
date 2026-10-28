# Gecko / Probe: Decision History

The active rules are in the [model](model.md); decision scope and status are in the [D2 plan](work-plan.md). This file preserves the reasons for choices and does not define a separate contract.

Earlier entries use the current Probe terminology for diagnostic tasks; the original name was Axis / Axes. Dates and decision scope are preserved. The distinction from Capability was accepted on October 28, 2026, as recorded below.

## 2026-10-27: Shared Rules for Preliminary and Client Runs

**ID:** parts of B1, B2, C3, and D2. **Status:** accepted at the model level.

**Question:** are chains at Gecko startup a special execution phase, or ordinary Probe runs initiated automatically at startup?

**Significant alternatives:** separate preliminary-phase rules, or a shared execution and management contract for preliminary and client runs.

**Choice:** a shared contract. A user can request cancellation of a preliminary check and start additional chains; Gecko reflects the changed current work. The run's origin is preserved in its context. The active rule is in [model §3.4](model.md#34-preliminary-probes-and-check-chains).

**Reason:** the initiator explains why work began but does not change the meaning of execution, observation, results, or cancellation. User intervention does not require transitions between special Gecko phases.

**Affected documents:** model 0.11, the scenario variant for managing preliminary checks, the D2 plan, and the development cycle log.

**Questions open at the time:** selection of initial chains and responsibility for subsequent invocations, cancellation propagation to child runs, resource contention, and the fate of partial results after cancellation. Responsibility for subsequent invocations was defined on October 28, 2026, below; the other parts require further D2 work.

**Hypothesis and validation:** at U2, confirm shared management of preliminary and client runs and a unified account of Gecko's current work using the scenario variant. This record contains no experiments or implementation.

## 2026-10-28: Gecko Policy Selects Subsequent Checks

**ID:** parts of A3 and B1. **Status:** accepted at the model level.

**Question:** does an individual Probe invoke the next check based on its own findings, or only publish results from which Gecko's reconnaissance policy selects subsequent runs?

**Significant alternatives:** direct invocations between Probes, or centralized selection by Gecko policy based on Probe results.

**Choice:** a Probe performs its check and publishes findings; Gecko policy selects and initiates subsequent checks using the results and diagnostic context. Individual Probes do not invoke each other directly. Applicability rules were selected to implement the policy: an available Probe describes the necessary facts and conditions, and the policy matches results against them as they appear. The active rule is in [model §3.4](model.md#34-preliminary-probes-and-check-chains).

**Policy implementation approaches considered:**

| Option | Meaning | Decision |
|---|---|---|
| Fixed scenarios | Chains predefined in code for typical diagnostic tasks | Not selected: new reconnaissance conditions and options would require scenario changes |
| Applicability rules | The Probe description states required facts and conditions; policy selects subsequent checks as results appear | Selected: adaptive selection remains in one policy, and checks do not manage each other |
| Full task graph | Dependencies, branches, result merging, and management of the entire chain are specified in advance | Not selected for the current scope: this would require defining a graph before enough domain checks exist |

**Reason:** the goal is maximally useful diagnostics without decentralized chain management inside individual checks. One policy is sufficient to select further work from findings.

**Affected documents:** model 0.12, the scenario, the D2 plan, and the development cycle log.

**Open questions:** initial check selection, the concrete form of applicability conditions, diagnostic depth, load limits, run relationships, and protection against cycles and unintended repeats. Selecting applicability rules does not prescribe a particular description language or policy execution mechanism.

**Hypothesis and validation:** at U2, check that a finding published before a Probe finishes allows Gecko policy to start an applicable subsequent check and preserve the link between that run and its originating finding. This remains a model decision without an experiment.

## 2026-10-28: Axis Split into Probe and Capability

**ID:** A1. **Status:** accepted at the model level.

**Question:** should the same term describe both an advertised environment possibility and the diagnostic task that obtains facts about it?

**Significant alternatives:** retain Axis for both roles, rename diagnostic tasks while leaving environment capabilities implicit, or define separate Probe and Capability concepts.

**Choice:** diagnostic tasks are Probe / Probes, and environment execution possibilities are Capability / Capabilities. A Probe run is an individual execution through Operation. Results can support or refute Capability availability under stated conditions; there is no required one-to-one relationship. An available Probe does not imply that the checked Capability is available. The active rule is in [model §2.1](model.md#21-capability-and-probe).

**Reason:** Axis no longer explained the diagnostic role, and using one term for both a check and an environment possibility made requirements and invocation ambiguous. Separate terms make “requires a Capability” and “invokes a Probe” explicit.

**Affected documents:** model 0.14, concept map 0.11, Mesh / App 0.47 and requirements down, D2 plan, related models and diagrams, Operation documents, links, and development cycle. The directory is renamed from `2-gecko-axis` to `2-gecko-probe`. Earlier terminology is normalized where its meaning is clear, with the original name retained in migration notes.

**Open questions:** Capability identifiers and lifecycle (A2), evidence and publication rules (C1), freshness, revision and withdrawal (C2), and client descriptions (D1). These do not reopen the terminology choice or establish a separate foundational module.

**Hypothesis and validation:** at U2, check that a client can distinguish the available check catalog from advertised Capabilities and trace availability evidence to the corresponding run and conditions. A negative check or an expired observation must not be mistaken for absence of the Probe itself. Concrete freshness behavior remains for C2. No implementation or experiment is claimed by this change.
