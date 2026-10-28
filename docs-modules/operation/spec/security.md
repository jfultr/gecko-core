# Operation: security

October 28, 2026 · required design track

[Overview](../README.md) · [Model](model.md) · [Decision status](status.md) · [Example roadmap](roadmap.md)

Operation is the common entry point for diagnostic and mutating Gecko actions. The module should therefore become the enforcement point where caller identification, command authorization, auditing, and load limits are applied consistently. This does not make it the only security boundary: each executor remains responsible for domain authorization, safe interaction with the operating system and hardware, and isolation of its own code.

## Design principle

Security is not a final-release task. The first API must carry trusted caller context and expose an authorization point, even if the initial test policy permits every request. Public commands, identities, and storage must not be stabilized first and then retrofitted with subjects and permissions through a breaking redesign.

## Required analysis areas

| Area | Questions that require verifiable answers |
|---|---|
| Trust boundaries | Which data comes from the client, transport, executor, and storage; what Core trusts |
| Identity | Who constructs authenticated caller context; why a payload cannot declare its own subject |
| Authorization | Who may start/get/watch/cancel; whether operation type, subject, and data scope are authorized separately |
| Idempotency | Whether another subject can guess or capture a `request_id`; what happens when permissions change between retries |
| Identifiers | Sufficient unpredictability, resistance to enumeration leaks, and correct uniqueness scope |
| Integrity | Who may publish progress and terminal results; how snapshot forgery or rollback is prevented |
| Confidentiality | Which inputs, reports, evidence, and artifacts are sensitive; filtering of reads, logs, and errors |
| Audit | Tamper-resistant linkage among subject, request, authorization decision, operation, cancellation, and administrative actions |
| Resource exhaustion | Limits on requests, active operations, subscribers, report size, history, artifacts, and update frequency |
| Executor | Type and parameter validation, least privilege, timeouts, termination, and untrusted-code isolation |
| Storage | Access, encryption where required, backup/restore, rollback, migration, and secure deletion |
| Transport adapter | Client/node authentication, channel protection, namespace isolation, and prevention of Core bypass through direct publication |
| Errors and uncertainty | Fail-closed behavior when authorization state is unknown; no unsafe automatic retry |
| Upgrades | Policy and schema compatibility, downgrade/rollback risks, key revocation, and permission changes |

## Responsibility split

Core must invoke authorization consistently and persist the audit decision before handing work to an executor. A Transport adapter must construct trusted caller context from authenticated transport identity rather than an arbitrary request field. The Executor rechecks domain constraints and operates with the minimum required privileges.

Executor sandboxing and isolation, protection of domain-action secrets, and security of a specific Zenoh deployment are designed separately. Operation provides mandatory integration points and does not claim that these concerns are automatically resolved.

## Security gate for `1.0.0`

The phrase "address all security questions" is not a verifiable completion condition. Admission to `1.0.0` requires the project to:

1. document the threat model, assets, trust boundaries, and supported deployment models;
2. define the identity source and authorization rules for every public command and Operation class;
3. verify protection of identifiers, retries, reports, artifacts, and audit records;
4. define and test limits for memory, CPU, storage, concurrency, subscribers, and message size;
5. run abuse, concurrency, fault, and restart tests at authorization and storage boundaries;
6. document safe behavior when the policy engine, storage, transport, or executor is unavailable;
7. perform an independent security review of the stable candidate;
8. close discovered risks through tests and changes, or explicitly accept residual risk with an owner, rationale, and scope.

An incomplete gate item blocks `1.0.0` but does not prohibit experimental `0.x` releases when their limitations and unsuitability for untrusted environments are stated explicitly.
