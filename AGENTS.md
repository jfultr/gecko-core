# Working on Gecko

These instructions apply to any agent working in this repository. Follow the user's current task and explicitly assigned role; no role is active by default.

## Project context and sources of truth

- Start with [the documentation index](docs/README.md), then read the relevant model, scenario, and plan. Use [the development cycle](docs/development-cycle.md) for the current stage and [the module catalog](docs/core-modules.md) for module ownership.
- Active models and contracts own accepted rules. Plans track work; histories and archives explain provenance and do not override active rules. Keep each rule in one authoritative place and link to it elsewhere.
- Distinguish proposed decisions, accepted contracts, implemented behavior, and experimentally verified behavior. Report evidence and limitations; writing a document or passing a narrow check does not establish broader completion.
- Preserve the existing downward requirements / upward experiment cycle. Resolve contradictions at the owning contract and update affected consumers and progress records together. Record unresolved design choices in the existing text plans or decision registers.
- Write maintained project documentation and instructions in English. Respond to the user in their preferred language.

## Explicit roles

| Role | Canonical instructions | Native invocation |
|---|---|---|
| Design and documentation | [gecko-design](.agents/skills/gecko-design/SKILL.md) | Codex: `$gecko-design`; Claude Code: `/gecko-design` |
| Implementation | [gecko-implement](.agents/skills/gecko-implement/SKILL.md) | Codex: `$gecko-implement`; Claude Code: `/gecko-implement` |

Activate a role only when the user explicitly invokes its skill or assigns that role in the conversation. Reading, quoting, editing, or discussing a role file is not activation. Mentions of documentation, code, or Linear do not assign a role.

On assignment, read the canonical instructions and briefly acknowledge the role. It remains active in this chat until the user switches or cancels it. A switch replaces the previous role. Preserve an explicit assignment across context summaries, but start new chats without a role unless the user assigns one there. Role lifetime is a project convention, not a permissions mechanism. Roles emphasize a workflow and allow supporting work in other areas.

For agents without native skill support, the user can say: "Read AGENTS.md and .agents/skills/gecko-design/SKILL.md and work in that role in this chat" (or use `gecko-implement`). Agents that do not discover AGENTS.md automatically must be directed to read it. Claude Code's root CLAUDE.md imports this file; its skill entry points load the canonical instructions.

## Planning and Linear

- Keep documentation planning, architectural questions, and decision progress in the existing repository text plans and registers.
- Create Linear issues only for implementation of decisions in code, including the tests and integration needed for that implementation. Documentation-only work stays in text plans.
- An implementation issue should identify the problem, authoritative decision references, scope, dependencies, and observable acceptance criteria. Link decisions instead of copying whole contracts. Identify an experimental implementation as an experiment, without presenting its hypothesis as a validated guarantee.
- Use the available Linear integration and actual workspace/project/status metadata. Search for existing issues before creating new ones. If the destination is ambiguous, ask for the missing detail. If access is unavailable, prepare the issue text and report that it has not been published.
- Role assignment alone does not start an unbounded backlog run. Work within the current user request. Do not infer authority to merge, deploy, or send unrelated messages from a role.

## Changes and verification

- Inspect the working tree before editing and preserve unrelated work. Use the existing checkout unless the task requires otherwise.
- Make changes at the owning module or document; keep dependent references and progress entries consistent. Choose routine implementation details within the agreed contract. Surface material contract changes rather than silently introducing them through code.
- Run checks appropriate to the change. For documentation, run `python3 docs/check_links.py` from the repository root. For implementation, use the selected module's build and test commands and exercise relevant observable behavior.
- Report what changed, what was actually checked, remaining limitations, and any external task updates. Keep code completion, review, merge, and deployment statuses distinct.

## Improve the project's tools through use

During substantive work and before finishing an experiment, consider whether a temporary script, repeated manual action, missing diagnostic, or corrected instruction should become reusable project knowledge.

- Reuse or improve an existing tool first. Save a small, verified tool within the task when it already has demonstrated repeat use or materially improves reproducibility. Propose larger or speculative additions with their purpose, likely reuse, location, and maintenance cost.
- Choose the artifact by need: a script or CLI for deterministic actions, a test or scenario for a repeatable check, a skill for a procedure requiring judgment, or an API change for a missing system capability.
- Validate a retained tool on the real scenario. Document inputs, invocation, expected result, and relevant limitations; link it from the relevant skill or module entry point so another session can discover it.
- Keep durable artifacts in versioned project files. Do not rely on chat history or live interpreter state as the only record. Avoid storing secrets, machine-specific output, or large raw logs in reusable artifacts.
- Briefly report useful artifacts saved or concrete proposals. Do not manufacture a tool, a new skill, or a retrospective entry for every task. Evaluate improvements against task outcomes; do not weaken acceptance checks to make them pass.

## Machine-specific development environment

Apply these defaults only on Windows host DESKTOP-L3QT1UI for user jf, including remote access to that host:

- Keep the checkout at `C:\Home\projects\gecko`, also accessible as `/mnt/c/Home/projects/gecko` in WSL. Do not create or synchronize a second checkout without a user request.
- Run development commands, dependency installation, Git, servers, builds, linters, and tests with Linux tools in WSL distribution `Ubuntu-24.04`. From PowerShell, use `wsl.exe -d Ubuntu-24.04 --cd /mnt/c/Home/projects/gecko --exec bash -lc 'COMMAND'`. Inside that WSL environment, run Linux commands directly.
- Use Linux runtimes and dependency environments consistently. Convert and quote paths correctly. Windows-native file editing, previews, browser interaction, and administration are allowed; use Windows development tools when the project requires them or the user requests them.
- If WSL or a required tool is unavailable, report the concrete blocker. Do not silently move builds or tests to Windows or relocate the project. Use an available equivalent for routine searches when appropriate.

On other hosts, use the actual environment and project requirements; do not assume these Windows paths or WSL defaults.
