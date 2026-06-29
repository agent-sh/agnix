# agnix JetBrains Plugin

JetBrains IDE integration for agnix using LSP4IJ.

<!-- Plugin description -->
Real-time validation for AI agent configuration files in JetBrains IDEs.
Install from the [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/30087-agnix).

Features:

- Validation for `SKILL.md`, `CLAUDE.md`, `AGENTS.md`, `.claude/settings.json`, `*.mcp.json`, `.cursor/rules/*.mdc`, and related files
- Diagnostics with quick fixes and hover docs through `agnix-lsp`
- Automatic `agnix-lsp` install/update via LSP4IJ server installer flow
- Actions to restart the server, validate current file, and open settings

![agnix diagnostics in JetBrains](https://raw.githubusercontent.com/agent-sh/agnix/main/editors/jetbrains/assets/jetbrains-validation.png)
<!-- Plugin description end -->

## Requirements

- IntelliJ Platform 2023.3+
- Java 17

## Build From Source

```bash
cd editors/jetbrains
./gradlew test
./gradlew buildPlugin
```

Built plugin zip:

```text
editors/jetbrains/build/distributions/agnix-<version>.zip
```

## Run In Sandbox IDE

```bash
cd editors/jetbrains
./gradlew runIde
```

After the sandbox IDE launches:

1. Open a project with agnix config files.
2. Open `Tools > agnix > Settings`.
3. Confirm diagnostics appear for invalid files and clear after fixes.
4. Use `Tools > agnix > Restart Language Server` and verify reconnect.

## Real IDE Test Matrix

Run these checks against real installs (not only sandbox):

1. IntelliJ IDEA Community 2023.3+
2. WebStorm 2023.3+
3. PyCharm Community 2023.3+

For each IDE, verify:

1. Plugin installs from zip without startup errors.
2. `agnix-lsp` auto-download works (with auto-download enabled).
3. Manual path override works (`Settings > Tools > agnix > LSP binary path`).
4. Diagnostics and hover work on supported files.
5. Unrelated `settings.json` files (for example `.vscode/settings.json`) do not activate agnix diagnostics.

## Troubleshooting

- If `agnix-lsp` is not detected, set `LSP binary path` explicitly.
- For download issues, verify internet access to GitHub release asset domains.
- Enable trace logging with `Trace level = Messages` or `Verbose`.

### "agnix-lsp could not be started (access denied)" on locked-down Windows

On managed/corporate machines, the OS or a security policy
(AppLocker, Windows Defender Application Control, EDR, or antivirus) can
**block execution** of the auto-downloaded `agnix-lsp.exe` because it lives in a
user-writable location (`%AppData%\...\plugins\agnix\bin\`). This surfaces as a
process-launch failure:

```text
Cannot run program "...\plugins\agnix\bin\agnix-lsp.exe": CreateProcess error=5, Access is denied
```

`error=5` is the Win32 `ERROR_ACCESS_DENIED` code (the trailing text is localized,
e.g. Danish "Adgang nægtet"). Note this is *access denied*, not *file not found*
(`error=2`) — the binary downloaded fine; the OS is refusing to run it.
**Upgrading the IDE does not fix this** — it is an OS/security-policy decision.

Resolve it by running `agnix-lsp` from an allowed location:

1. Get `agnix-lsp` into an execution-allowed directory — download
   `agnix-lsp-x86_64-pc-windows-msvc.zip` from the
   [GitHub releases](https://github.com/agent-sh/agnix/releases) and extract it to
   a whitelisted path (e.g. `C:\Program Files\agnix\`), or use `%USERPROFILE%\.cargo\bin`.
   (`cargo install agnix-cli` installs the CLI, not the LSP server, so use the
   release asset.)
2. In the IDE: **Tools → agnix → Settings → LSP binary path** → point it at that
   `agnix-lsp.exe`, then restart the language server (`Tools → agnix → Restart
   Language Server`). The configured path takes priority over the blocked
   auto-downloaded copy.
3. Alternatively, ask IT to allowlist the binary or the `%AppData%` plugin path.
