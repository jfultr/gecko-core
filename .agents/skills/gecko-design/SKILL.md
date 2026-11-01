---
name: gecko-design
description: Work in Gecko's explicitly assigned design and documentation role, maintaining contracts and text plans and handing accepted code implementation work to Linear. Use only when the user explicitly assigns or invokes this role.
---

# Gecko design and documentation role

Read the repository-root [AGENTS.md](../../../AGENTS.md). Apply its explicit activation, role lifetime, planning, environment, and tool-improvement rules. Merely opening this file for inspection does not assign the role.

Your primary work is developing coherent decisions and documentation. Read code, run experiments, or make supporting code changes when the user's task calls for them.

## Work from the current design state

1. Locate the relevant active model and current text plan through [the documentation index](../../../docs/README.md). Establish what is accepted, unresolved, implemented, and validated before proposing changes.
2. Develop the requested scenario and decisions at their owning level. Keep alternatives, reasons, open questions, and next steps in the existing plans and registers. Clearly distinguish proposals from accepted rules; obtain user direction for material choices not already authorized by the task.
3. Update affected contracts, references, decision status, and progress together. Follow [the existing development cycle](../../../docs/development-cycle.md), including its progress-table conventions. Avoid turning documentation planning into a parallel Linear backlog.

## Hand implementation to Linear

When an accepted decision is ready for code implementation within the requested work, find or create the corresponding implementation issue. Do not require the whole architecture to be finished if a bounded module experiment is already agreed.

- Discover the connected Linear workspace, project, existing related issues, and workflow states. Reuse an existing matching issue. Ask only for unresolved destination or scope information that cannot be inferred reliably.
- Include the implementation problem and intended behavior; repository paths and section or decision IDs for authoritative rules; agreed scope and exclusions; dependencies and unresolved blockers; observable acceptance criteria and relevant checks.
- Reference the source revision when available. A local path alone is not a remotely accessible document: state when the decision exists only in the working tree and provide enough context for the executor to identify it. Do not claim an uncommitted decision is available on the remote.
- Split work only where pieces have meaningful, separately verifiable outcomes. Keep pure design discussion and documentation planning in repository text plans.
- Record the issue ID/link beside the corresponding implementation item in the existing plan or register. Keep implementation status separate from architectural acceptance.
- If Linear is unavailable, provide a ready-to-publish draft and leave publication explicitly pending. Do not fabricate IDs or statuses.

Finish with the decisions changed, open questions, validation performed, and implementation issues created or updated. Apply the shared tool-improvement review when the work yielded reusable procedures or scripts.
