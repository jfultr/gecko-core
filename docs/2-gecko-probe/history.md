# Gecko / Probe: Decision History

The active rules are in the [model](model.md); decision scope and status are in the [D2 plan](work-plan.md). This file preserves the reasons for choices and does not define a separate contract.

Earlier entries use the current Probe terminology for diagnostic tasks; the original name was Axis / Axes. Dates and decision scope are preserved. The distinction from Capability was accepted on October 28, 2026, as recorded below.

## 2026-11-23: Names for Check and Fact Roles

**ID:** parts of A2 and C1. **Status:** partially accepted at the model level.

**Question:** should the working English labels for what a run checks and what a Fact describes become the Probe domain terms?

**Significant alternatives:** leave the labels provisional or accept the four names for the agreed roles.

**Choice:** **Check Input**, **Check Target**, **Search Scope**, and **Fact Subject** are accepted English domain terms with the meanings in [model §3.2](model.md#32-data-blockers-and-reports). This names the concepts; it does not select API field names or reference encodings.

**Reason:** the four names make the accepted distinction usable in subsequent contracts without using `subject` ambiguously for a run's scope and each resulting Fact.

**Affected documents:** model 0.22, scenario, D2 plan, and development cycle log.

**Open questions:** mapping Check Input to the common Operation Report `subject` field (B2), concrete references and identity for Check Targets and Fact Subjects (A2/C1), and comparison across runs (C2).

**Hypothesis and validation:** at U2, review a discovery run and a focused check for consistent use of the terms in requests, reports, and independently available Facts. Naming itself does not claim implementation evidence.

## 2026-11-23: Distinguish Run Check From Fact Description

**ID:** parts of A2 and C1. **Status:** partially accepted at the model level.

**Question:** does one `subject` term describe both what a Probe run checks and what each resulting Fact is about?

**Significant alternatives:** use one subject reference for a run and all of its Facts; or distinguish the run's check input from each Fact's subject.

**Choice:** what a run is asked to check and what each resulting Fact describes have different roles. A run may check a particular object or examine a wider area and produce Facts about multiple objects found there. The [model §3.2](model.md#32-data-blockers-and-reports) uses working English labels to explain these roles; their exact names are not accepted by this decision.

**Reason:** a discovery run can examine one scope and find several Devices. Reusing one `subject` term for the scope and every discovered object obscures the relationship between the run, its findings, and the independently available Facts.

**Affected documents:** model 0.21, scenario, D2 plan, and development cycle log.

**Open questions:** final domain terms and concrete reference formats and identity (A2/C1), the Probe mapping to Operation Report's common `subject` field (B2), matching the same object across runs and restarts (C2), and which findings become Facts (C1). The working labels select no wire field names.

**Hypothesis and validation:** at U2, run discovery over a scope that yields multiple objects, expose each Fact with its own subject and a link to the supporting run, and verify that a late client can distinguish those objects from the check scope. This decision has no implementation evidence.

## 2026-11-21: Check a Client-Supplied Subject

**ID:** parts of A2 and D1. **Status:** partially accepted at the model level.

**Question:** must Gecko discover or register a subject before a client can invoke a Probe for it?

**Significant alternatives:** require prior discovery or registration; or let a Probe that supports client-supplied subjects check one directly.

**Choice:** a client may invoke a supported Probe for a supplied subject that Gecko has not yet discovered or registered, such as a manually provided RTSP source. Prior discovery or registration is not a general prerequisite. The active rule is in [model §3.6](model.md#36-probe-in-geckos-zenoh-api).

**Reason:** a client should be able to check a source it knows about even when limited reconnaissance has not found it. This is consistent with the independent manual-configuration path.

**Affected documents:** model 0.20, scenario, D2 plan, and development cycle log.

**Open questions:** whether subject is a typed reference, a common entity, or a Probe-specific description (A2); representation and validation of supplied subjects (D1); subject identity after discovery and comparison across checks or restarts (A2/C2). This decision does not make every Probe accept arbitrary input or identify an RTSP URL with a persistent camera identity.

**Hypothesis and validation:** at U2, invoke a supported Probe for a manually supplied source before discovery or registration and observe an associated report. This is a model decision without an experiment.

## 2026-11-08: Probe Run Uses Operation Identity

**ID:** parts of A2 and D2. **Status:** partially accepted at the model level.

**Question:** does a Probe run reuse its Operation identity or receive a separate domain identifier?

**Significant alternatives:** use `operation.id` as the run ID with explicit Gecko and Probe context; or add a distinct `probe_run_id` and maintain a mapping to `operation.id`. A compound run reference containing Gecko, Probe, and `operation.id` does not introduce another ID.

**Choice:** `operation.id` is the Probe run identifier. A run reference includes its Gecko and Probe context; no separate `probe_run_id` is defined. `request_id` remains the caller's retry identity, not a run ID. The active rule is in [model §3.6](model.md#36-probe-in-geckos-zenoh-api).

**Reason:** the accepted model defines a Probe run as one execution through Operation. Reusing its identity keeps the request, report, and run associated without a second identifier mapping.

**Affected documents:** model 0.19, scenario, D2 plan, and development cycle log.

**Open questions:** concrete request and observation addressing (D2), `operation.id` uniqueness scope and representation under the common Operation contract, Probe and Gecko identifier rules (A2), concurrency (B3), and report/fact retention (C4). This choice does not select Zenoh keys or address encoding.

**Hypothesis and validation:** at U2, check that a direct Probe invocation, retry after a lost response, report retrieval, and partial-result observation all refer to the same `operation.id` in the correct Gecko and Probe context. This is a model decision without an experiment.

## 2026-11-08: Probe Separate from Its Implementation

**ID:** part of A2. **Status:** partially accepted at the model level.

**Question:** does a client-addressable Probe denote the check contract or a particular implementation, and should clients select implementations directly?

**Significant alternatives:** treat Probe and implementation as one entity; separate the client-addressable Probe contract from its internal implementation; or expose multiple selectable implementations to clients.

**Choice:** Probe is the client-addressable description and invocation contract. Its implementation is a separate internal concept bound to the Probe to perform the check; clients invoke the Probe. This does not require multiple implementations, runtime replacement, or a client-facing implementation catalog. The active rule is in [model §3.1](model.md#31-purpose-and-invocation-form).

**Reason:** the task a client requests remains conceptually separate from the code that performs it. The distinction permits implementation provenance and future variants to be considered without making them part of the current client interaction.

**Affected documents:** model 0.18, D2 plan, and development cycle log.

**Open questions:** implementation identity and version, how a Probe binds to an implementation and whether that binding can change (A2); catalog descriptions (D1); executor and adapter responsibilities (A3, E2); how implementation details appear in run evidence (C1). No implementation selection or compatibility rule is accepted.

**Hypothesis and validation:** at U2, check that a client can invoke a Probe through its address while the bound internal implementation performs the check and the report remains associated with the requested Probe. This is a model decision without implementation evidence.

## 2026-11-08: Fact Separate from a Run Finding

**ID:** part of A2. **Status:** partially accepted at the model level.

**Question:** are facts available to other flows only as content inside a particular Probe run's Operation Report, or are they separate logical entities linked to run evidence?

**Significant alternatives:** keep findings only within reports and have consumers inspect those reports; or expose Facts independently while preserving their connection to the supporting run evidence.

**Choice:** a Fact is a separate logical entity. A finding remains part of the particular run's report; an available Fact can be accessed and referenced independently by clients and other checks, with a link to supporting run evidence. The active rule is in [model §3.2](model.md#32-data-blockers-and-reports). This does not allow a Fact to replace an explicitly requested new check.

**Reason:** clients and subsequent checks need to use available environment knowledge without treating a run report as the only access path. The evidence link retains the context of what was checked. Separating the concepts allows applicability and retention to be specified independently later.

**Affected documents:** model 0.17, D2 plan, and development cycle log.

**Open questions:** the complete entity map, Fact identity and lifecycle (A2); which findings become Facts, publication responsibility, and the fact contract (C1); applicability, freshness, replacement, and conflicts (C2); storage and retention relative to reports (C4). No storage mechanism, update model, or indefinite validity is selected.

**Hypothesis and validation:** at U2, check that a client connecting after a finding is published and a subsequent Probe can access the available Fact and trace it to its supporting run evidence under the eventual C1–C2 contract. This decision has no implementation or experimental evidence yet.

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
