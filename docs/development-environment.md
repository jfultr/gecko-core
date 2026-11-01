# Machine-specific development environment

These defaults apply only on Windows host DESKTOP-L3QT1UI for user jf, including remote access. On other hosts, use the actual environment and project requirements.

- Keep the checkout at `C:\Home\projects\gecko`, also accessible as `/mnt/c/Home/projects/gecko` in WSL. Do not create or synchronize a second checkout without a user request.
- Run development commands, dependency installation, Git, servers, builds, linters, and tests with Linux tools in WSL distribution `Ubuntu-24.04`. From PowerShell, use `wsl.exe -d Ubuntu-24.04 --cd /mnt/c/Home/projects/gecko --exec bash -lc 'COMMAND'`. Inside that WSL environment, run Linux commands directly.
- Use Linux runtimes and dependency environments consistently. Convert and quote paths correctly. Windows-native file editing, previews, browser interaction, and administration are allowed; use Windows development tools when the project requires them or the user requests them.
- If WSL or a required tool is unavailable, report the concrete blocker. Do not silently move builds or tests to Windows or relocate the project. Use an available equivalent for routine searches when appropriate.
