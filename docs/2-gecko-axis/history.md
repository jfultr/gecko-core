# Gecko / Axis: Decision History

The active rules are in the [model](model.md); decision scope and status are in the [D2 plan](work-plan.md). This file preserves the reasons for choices and does not define a separate contract.

## 2026-10-27: Shared Rules for Preliminary and Client Runs

**ID:** parts of B1, B2, C3, and D2. **Status:** accepted at the model level.

**Question:** are chains at Gecko startup a special execution phase, or ordinary Axis runs initiated automatically at startup?

**Significant alternatives:** separate preliminary-phase rules, or a shared execution and management contract for preliminary and client runs.

**Choice:** a shared contract. A user can request cancellation of a preliminary check and start additional chains; Gecko reflects the changed current work. The run's origin is preserved in its context. The active rule is in [model §3.4](model.md#34-preliminary-axes-and-check-chains).

**Reason:** the initiator explains why work began but does not change the meaning of execution, observation, results, or cancellation. User intervention does not require transitions between special Gecko phases.

**Affected documents:** model 0.11, the scenario variant for managing preliminary checks, the D2 plan, and the development cycle log.

**Questions open at the time:** selection of initial chains and responsibility for subsequent invocations, cancellation propagation to child runs, resource contention, and the fate of partial results after cancellation. Responsibility for subsequent invocations was defined on October 28, 2026, below; the other parts require further D2 work.

**Hypothesis and validation:** at U2, confirm shared management of preliminary and client runs and a unified account of Gecko's current work using the scenario variant. This record contains no experiments or implementation.

## 2026-10-28: Gecko Policy Selects Subsequent Checks

**ID:** parts of A3 and B1. **Status:** accepted at the model level.

**Question:** does an individual Axis invoke the next check based on its own findings, or only publish results from which Gecko's reconnaissance policy selects subsequent runs?

**Significant alternatives:** direct invocations between Axes, or centralized selection by Gecko policy based on Axis results.

**Choice:** an Axis performs its check and publishes findings; Gecko policy selects and initiates subsequent checks using the results and diagnostic context. Individual Axes do not invoke each other directly. Applicability rules were selected to implement the policy: an available Axis describes the necessary facts and conditions, and the policy matches results against them as they appear. The active rule is in [model §3.4](model.md#34-preliminary-axes-and-check-chains).

**Policy implementation approaches considered:**

| Option | Meaning | Decision |
|---|---|---|
| Fixed scenarios | Chains predefined in code for typical diagnostic tasks | Not selected: new reconnaissance conditions and options would require scenario changes |
| Applicability rules | The Axis description states required facts and conditions; policy selects subsequent checks as results appear | Selected: adaptive selection remains in one policy, and checks do not manage each other |
| Full task graph | Dependencies, branches, result merging, and management of the entire chain are specified in advance | Not selected for the current scope: this would require defining a graph before enough domain checks exist |

**Reason:** the goal is maximally useful diagnostics without decentralized chain management inside individual checks. One policy is sufficient to select further work from findings.

**Affected documents:** model 0.12, the scenario, the D2 plan, and the development cycle log.

**Open questions:** initial check selection, the concrete form of applicability conditions, diagnostic depth, load limits, run relationships, and protection against cycles and unintended repeats. Selecting applicability rules does not prescribe a particular description language or policy execution mechanism.

**Hypothesis and validation:** at U2, check that a finding published before an Axis finishes allows Gecko policy to start an applicable subsequent check and preserve the link between that run and its originating finding. This remains a model decision without an experiment.
