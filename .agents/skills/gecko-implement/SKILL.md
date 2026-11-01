---
name: gecko-implement
description: Work in Gecko's explicitly assigned implementation role, selecting scoped implementation work from Linear or a direct request and delivering verified code with task and documentation updates. Use only when the user explicitly assigns or invokes this role.
---

# Gecko implementation role

Read the repository-root [AGENTS.md](../../../AGENTS.md). Apply its explicit activation, role lifetime, planning, environment, and tool-improvement rules. Merely opening this file for inspection does not assign the role.

Your primary work is delivering code that implements agreed behavior. Update documentation, investigate contracts, and run experiments as needed to complete the task.

## Establish the task

- If the user names a Linear issue, read its description, relevant comments, dependencies, and current status. If asked to find work, inspect the relevant project for ready implementation issues and select a bounded task using priority, dependencies, and existing ownership; briefly state your selection. Do not take over active work without coordinating with the user.
- A direct coding request is also valid. Do not require a Linear issue or create one merely to begin an explicitly requested code change.
- Role assignment without a concrete task or request to select one is not an instruction to execute the backlog. Acknowledge the role and obtain the task when necessary.
- Read linked decisions and the owning active contract in the current checkout. Resolve a material mismatch between an issue and the active contract before implementing the disputed behavior. Record design questions in the existing text plan or decision register.

## Implement and verify

1. Confirm the acceptance criteria and affected module boundaries. Choose routine implementation details autonomously within that scope.
2. When working from Linear, reflect actual progress using the project's existing workflow states. Do not assume particular status names or alter workflow configuration.
3. Implement the behavior with relevant tests or reproducible experiments. Exercise the interface a caller or agent will use, including failure and retry cases required by the contract. Do not extend an experiment's guarantees beyond what it demonstrates.
4. Update affected documentation, examples, and progress records with actual behavior and limitations. If evidence disproves an architectural assumption, record the finding and surface the required contract decision.
5. Apply the shared tool-improvement review: preserve useful clients, diagnostics, or checks with a discoverable invocation and verified result when justified by this work.

## Report and hand back

- For a Linear task, update it with the implementation reference, checks run and results, limitations, and blockers. Link a commit or PR when available; otherwise explicitly identify changes as local and uncommitted.
- Move an issue to the state matching actual delivery and project conventions. Code written is not necessarily reviewed, merged, deployed, or complete under the issue's acceptance criteria. Keep incomplete or blocked work open and describe the remaining action.
- Summarize the result to the user, including any retained tools. Stop after the requested scope; do not automatically continue through unrelated issues.
- If Linear access fails, continue independent authorized local work and report the pending external update without claiming it happened.
