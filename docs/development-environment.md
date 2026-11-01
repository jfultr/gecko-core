# Machine-specific development environment

These defaults apply only on a Windows development host with WSL, including remote access. On other hosts, use the actual environment and project requirements.

All development commands (Git, dependency installation, servers, builds, linters, tests) run with Linux tools in the project's WSL distribution. Windows-native file editing, previews, browser interaction, and administration are allowed; use Windows development tools only when the project requires them or the user requests them. Documents and scripts refer to the repository root rather than an absolute path, so they work in either layout below.

## Layouts

The user selects one layout per session. Detect it from the session's working directory; if it is unclear, ask.

| | A: Windows checkout | B: WSL checkout |
|---|---|---|
| Checkout | `C:\Home\projects\gecko`, in WSL `/mnt/c/Home/projects/gecko` | `~/gecko/gecko-core` inside the WSL distribution (Windows view: `\\wsl$\<distro>\home\<user>\gecko\gecko-core`) |
| Suited to | Agents and tools that run on Windows, such as Codex | Working directly in WSL / Linux, including Claude Code in WSL |
| Commands from PowerShell | `wsl.exe -d <distro> --cd /mnt/c/Home/projects/gecko --exec bash -lc 'COMMAND'` | `wsl.exe -d <distro> --cd '~/gecko/gecko-core' --exec bash -lc 'COMMAND'` |
| Commands inside WSL | Run directly in the checkout | Run directly in the checkout |
| Trade-off | Windows tools see the files natively; file I/O through `/mnt/c` is slower, which affects builds | Native Linux file I/O and permissions; Windows tools reach files through `\\wsl$` |

## Rules for both layouts

- Do not create or synchronize an additional checkout without a user request. The two layouts are alternatives, not a pair to keep in step: if both clones exist, treat only the one chosen for the session as active.
- Keep work in the active clone committed and pushed before switching layouts. Do not leave uncommitted changes in the inactive clone.
- Use Linux runtimes and dependency environments consistently. Convert and quote paths correctly.
- If WSL or a required tool is unavailable, report the concrete blocker. Do not silently move builds or tests to Windows or relocate the project.
