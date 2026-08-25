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

## WSL Execution Mode (Windows)

If your project lives inside a WSL distribution (`\\wsl$\Ubuntu\home\you\project`
or `\\wsl.localhost\Ubuntu\...`) and `agnix-lsp` is installed inside that
distribution rather than on Windows, enable WSL execution mode so the server runs
where the files are. This does not require JetBrains Remote Development.

`Settings > Tools > agnix`:

1. Check **Run agnix-lsp inside WSL (Windows only)**.
2. **WSL distribution** - the name as printed by `wsl --list --quiet`, for
   example `Ubuntu`. Leave it empty to reuse the distribution the project itself
   is opened from (derived from its `\\wsl$\` / `\\wsl.localhost\` path).
3. **agnix-lsp path in WSL** - the absolute *Linux* path inside the distribution,
   for example `/home/you/.cargo/bin/agnix-lsp`. Windows paths and relative paths
   are rejected.

Behavior while WSL mode is on:

- The server is started as an argument list through
  `wsl.exe --distribution <name> --exec <path>`. No shell is involved, so spaces
  in the path need no quoting and nothing is interpreted by `bash`.
- Document and workspace URIs are translated at the LSP boundary: UNC paths under
  the selected distribution become plain Linux paths, `C:\src` becomes
  `/mnt/c/src`, and diagnostics coming back from the server are mapped to the
  matching Windows path. If your distribution moves the automount root away from
  `/mnt` (`[automount] root=` in `/etc/wsl.conf`), drive-letter paths outside WSL
  will not map.
- No working directory is passed to `wsl.exe`; `agnix-lsp` takes the workspace
  root from the translated `rootUri`.
- Host-local install and download of `agnix-lsp.exe` are skipped, so the Windows
  binary is not fetched.
- Files opened from a *different* distribution than the configured one are not
  mapped, and the server is not asked to validate them.

Misconfiguration (blank or shell-unsafe distribution name, non-Linux path) is
rejected when you apply the settings, and a launch-time error notification with a
**Settings** action appears if the stored configuration cannot produce a command.

Turning the checkbox off restores the normal host-local launch path exactly as
before.

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

On Windows with WSL installed, additionally verify:

1. A project opened from `\\wsl.localhost\<distro>\...` with WSL mode enabled
   starts the server and reports diagnostics at the right lines.
2. Quick fixes applied from the IDE land in the correct file inside the distribution.
3. Disabling WSL mode falls back to the host-local binary with no leftover state.

## Troubleshooting

- If `agnix-lsp` is not detected, set `LSP binary path` explicitly.
- For download issues, verify internet access to GitHub release asset domains.
- Enable trace logging with `Trace level = Messages` or `Verbose`.
- In WSL mode, confirm the path resolves inside the distribution:
  `wsl --distribution <name> --exec <path> --version`. If that fails, the plugin
  cannot start it either.

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
2. In the IDE: **Tools → agnix → Settings → LSP binary path** → point it at the
   full absolute path to that executable, e.g. `C:\Program Files\agnix\agnix-lsp.exe`
   (not just `agnix-lsp.exe`), then restart the language server (`Tools → agnix →
   Restart Language Server`). The configured path takes priority over the blocked
   auto-downloaded copy.
3. Alternatively, ask IT to allowlist the binary or the `%AppData%` plugin path.
