# Operation

An independent foundational Gecko module for starting and observing long-running operations.

Operation accepts or rejects a request, creates a distinct execution, publishes its progress and result, handles cancellation requests, and prevents duplicate execution after a lost response or a normal restart. The module has no knowledge of domain entities such as Axis, App, Block, or Line: domain executors integrate with it through a separate contract.

Current version: [`0.1.0-dev`](VERSION). Implementation has not started yet; this is the version of the contract and first prototype under development, not a completed release.

## Contents

- [Model and boundaries](spec/model.md) — purpose, responsibilities, and intended module structure.
- [Decision status](spec/status.md) — accepted decisions, open questions, and links to their original context.
- [Example roadmap](spec/roadmap.md) — a preliminary outline from `0.1.0` to `1.0.0`; the versions and their scope are not approved yet.
- [Security](spec/security.md) — required areas of security work and the admission gate for `1.0.0`.

## Source-of-truth status

Until the contract is migrated and reconciled, the active common Operation rules remain in [Mesh / App §8](../../docs/1-mesh-app/model.md#8-operation-protocol), primarily [§8.2](../../docs/1-mesh-app/model.md#82-operation-report-core). The files in this module define its boundary, collect its context, and prepare a standalone specification. If they conflict, Mesh / App §8.2 takes precedence.

Moving the source of truth must be an explicit decision. After that move, domain documents will reference the versioned module contract and retain only their Axis- or App-specific extensions.
