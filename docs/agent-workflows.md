# Agent role activation

The shared project instructions are in [AGENTS.md](../AGENTS.md). The role bodies live in `.agents/skills/`; application-specific entry points refer to those canonical files.

| Role | Codex | Claude Code |
|---|---|---|
| Design and documentation | `$gecko-design` | `/gecko-design` |
| Implementation | `$gecko-implement` | `/gecko-implement` |

Invoke the role with a task, for example `$gecko-implement` followed by an implementation issue ID. An assignment without a task selects the role but does not start executing the backlog.

Codex's `agents/openai.yaml` disables implicit invocation for both skills. Claude Code's `.claude/skills/` entry points set `disable-model-invocation: true`; the root `CLAUDE.md` imports `AGENTS.md`.

For another agent, explicitly ask it to read `AGENTS.md` and `.agents/skills/gecko-design/SKILL.md` (or `gecko-implement/SKILL.md`) and work in that role in this chat. Tools that do not automatically read `AGENTS.md` need this explicit direction.

The role persists until the user switches or cancels it, including across context summaries in the same chat. A switch replaces the earlier role. New chats start without a role. This lifetime is an instruction convention, not an application permission setting.

Linear access must be configured in the agent's environment. Role assignment does not install a connector or grant additional permissions.
