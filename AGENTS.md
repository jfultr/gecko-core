# Working on Gecko

## Project rules

- Start at [docs/README.md](docs/README.md); use [development-cycle.md](docs/development-cycle.md) for the current plan and work conventions.
- Active models and contracts own accepted rules. Update affected documents together; distinguish proposals, accepted decisions, implementation, and verified behavior.
- Write project documentation and instructions in English; respond in the user's preferred language.
- Keep documentation planning in existing text plans and decision registers. Linear issues are only for implementing decisions in code, including associated tests and integration.

## Explicit roles

No role is active by default. Only an explicit user assignment or skill invocation activates one; discussing or editing a skill does not. Read the assigned skill and acknowledge the role. It lasts in this chat until switched or cancelled; a new chat starts without a role.

- [gecko-design](.agents/skills/gecko-design/SKILL.md): primarily documentation and decisions, with implementation handoffs to Linear.
- [gecko-implement](.agents/skills/gecko-implement/SKILL.md): primarily code implementation from Linear or a direct request.

Roles allow supporting work outside their primary area and do not start a backlog run on assignment. See [activation instructions](docs/agent-workflows.md) when setting up an agent.

## Reusable tools

Notice repeated manual work and useful temporary scripts. Reuse existing tools; retain small, verified improvements within the task and propose larger additions. Give retained tools a reproducible invocation and a link from the relevant skill or module so another session can use them. Briefly report what was saved or proposed when useful.

## Development environment

On the Windows development host, run development, Git, dependency installation, builds, and tests through WSL, in the checkout layout the user selected for the session (Windows checkout or WSL checkout). Read [environment details](docs/development-environment.md) before running commands on that host. Do not silently switch development to Windows if WSL is unavailable.

For documentation changes, run `python3 docs/check_links.py` from the repository root. Use the affected module's checks for code changes.
