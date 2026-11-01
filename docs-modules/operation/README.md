# Operation

An independent foundational Gecko module for starting and observing long-running operations.

Operation accepts or rejects a request, creates a distinct execution, publishes its progress and result, handles cancellation requests, and prevents duplicate execution after a lost response or a normal restart. The module has no knowledge of domain entities such as Probe, App, Block, or Line: domain executors integrate with it through a separate contract.

Current version: [`0.1.0-dev`](VERSION). Implementation has not started yet; this is the version of the contract and first prototype under development, not a completed release.

## Contents

- [Model and boundaries](spec/model.md) — the authoritative common contract, responsibilities, lifecycle, reports, accounting, and validation.
- [Decision status](spec/status.md) — accepted decisions, open questions, and links to their original context.
- [Example roadmap](spec/roadmap.md) — a preliminary outline from `0.1.0` to `1.0.0`; the versions and their scope are not approved yet.
- [Security](spec/security.md) — required areas of security work and the admission gate for `1.0.0`.

## Documentation ownership

[Model and contract](spec/model.md) owns Operation design and accepted common rules. [Security](spec/security.md) owns security requirements. [Decision status](spec/status.md) is the single register of accepted decisions, their provenance, and open questions, including ST1–ST5. The roadmap remains a preliminary release outline.

Mesh / App and Gecko / Probe are consumers. They define requirements and domain-specific behavior, and use the module contract through references. Their documents do not override Operation design. A mismatch between a consumer requirement and module guarantees must be recorded and resolved explicitly.

On November 1, 2026, the common contract was migrated here from Mesh / App §8.2. Historical references to its former location are provenance, not active authority. Product Gecko 1.0.0 requirements and the unreleased Operation version `0.1.0-dev` are distinct; migration does not approve release scope or claim implementation.
