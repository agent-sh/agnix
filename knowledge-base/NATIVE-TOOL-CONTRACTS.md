# Pi, Antigravity CLI and Grok Code contracts

Verified on 2026-10-29. These three tools add 55 rules: 17 Pi, 18 Antigravity CLI and 20 Grok Code. The catalog contains 757 rules across 46 categories and 66 validators. Tiers remain the owner's maintenance priorities: Pi A, Antigravity CLI B and Grok Code B.

## Qualified sources

| Tool | Qualification | Primary contracts |
|---|---|---|
| Pi | v1.4.0 tagged source | [Configuration](https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/configuration.md), [settings](https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/settings.md), [schemas](https://github.com/earendil-works/pi/tree/v1.4.0/packages/coding-agent/schemas), [native parsers](https://github.com/earendil-works/pi/tree/v1.4.0/packages/coding-agent/src/core) |
| Antigravity CLI (`agy`) | 1.4.9 installer manifest and current documentation | [Settings](https://antigravity.google/docs/settings?tab=cli), [settings schema](https://antigravity.google/schemas/v1/settings.json), [plugin schema](https://antigravity.google/schemas/v1/plugin.json), [rules](https://antigravity.google/docs/rules), [MCP](https://antigravity.google/docs/mcp), [hooks](https://antigravity.google/docs/hooks), [subagents](https://antigravity.google/docs/subagents) |
| Grok Code (Grok Build CLI, `grok`) | Public source snapshot `2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8`, reconciled with released CLI 1.0.70 | [Source](https://github.com/xai-org/grok-build/tree/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8), [bundled guides](https://github.com/xai-org/grok-build/tree/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-pager/docs/user-guide), [settings](https://docs.x.ai/build/settings/reference), [released changelog](https://x.ai/changelog/build) |

The Grok source snapshot predates the latest released CLI. Released additions take precedence over that snapshot: command-hook `args`, `keys.vim`, and retirement of `ui.simple_mode` are reconciled against the changelog. Version-dependent advice uses the optional `tool_versions.grok_code` pin. An omitted pin uses the current qualified contract. Older releases have not been independently qualified by this onboarding.

Pi and Antigravity publish machine-readable schemas. Agnix bundles six normalized constraint files, with source and normalized SHA-256 digests in [`sources.json`](../crates/agnix-core/src/schemas/vendor/sources.json). Grok's static schema is derived by agnix from its public parsers and guides. It is not a vendor-published schema.

## Static validation surfaces

| Tool | Files and resource roots | Rule groups |
|---|---|---|
| Pi | User `~/.pi/agent/` or `$PI_CODING_AGENT_DIR`; project `.pi/`; settings, models, keybindings, MCP, package manifests, JSON themes, Markdown prompts and skills | `PI-CFG`, `PI-MDL`, `PI-KB`, `PI-MCP`, `PI-PKG`, `PI-TH`, `PI-PR`, applicable shared skill/MCP rules |
| Antigravity CLI | `~/.gemini/antigravity-cli/` settings and keybindings; `.agents/`, `.agent/` and `~/.gemini/config/` rules, registries, MCP and hooks; native agents and plugin manifests | `AGY-CFG`, `AGY-KB`, `AGY-MCP`, `AGY-PL`, `AGY-HK`, `AGY-RL`, `AGY-AG`, applicable shared skill/MCP rules |
| Grok Code | `.grok/`, `$GROK_HOME`, system `/etc/grok/requirements.toml`; TOML settings, roles, personas and sandbox profiles; JSON hooks, marketplace indexes and LSP config; plugin manifests, Markdown agents, rules and skills | `GRK-CFG`, `GRK-SBX`, `GRK-MCP`, `GRK-HK`, `GRK-PL`, `GRK-AG`, `GRK-SK`, applicable shared skill/MCP rules |

Rules validate configuration and local metadata. They do not run extensions, launch servers, authenticate, execute command references, infer credentials or contact model providers. Unknown extension fields remain open where the native client accepts them. A valid file does not establish project trust or prove that a referenced executable is installed.

Authentication/session stores, including Pi's manually editable `auth.json`, remain outside these rules. Grok `pager.toml` renderer settings, Antigravity App-only sidecars and SDK program configuration are also outside CLI agent-file validation. This inventory describes implemented coverage, not every file a tool can read.

### Pi

- Settings, keybindings, themes, packages and MCP use strict JSON with an optional BOM. `models.json` follows Pi's distinct parser: line comments and trailing commas are accepted; block comments are rejected.
- Settings, model providers, bindings and themes use the tagged published schemas, plus parser-backed checks for requirements those schemas do not encode.
- Project settings that only apply globally receive an advisory. User MCP login credentials cannot be selected by project files. Partial project MCP overrides do not need to repeat an inherited transport.
- Environment/header values beginning with `!` are runtime command references. Agnix recognizes the reference without executing it or reporting its literal text as a credential.
- Package metadata recognizes both a `pi` resource object and the `pi-package` keyword. Package paths resolve from the package root. Manifest globs discover resources; top-level settings globs filter resources found through plain paths. Exclusions and exact overrides preserve native precedence.
- Declared themes keep their theme subtype even if named `models.json` or `mcp.json`. They do not become model or MCP configurations. Custom prompt and skill paths retain their native metadata rules.
- Pi reads an actual string description. An omitted, empty or non-string skill name falls back to the directory. Claude-only fields ignored by Pi do not make the skill's YAML invalid.
- Theme checks cover missing variables, cycles, ANSI indexes and the native color literal forms. Prompt empty/null headers follow native body fallback.

### Antigravity CLI

- The native CLI is distinct from legacy Gemini CLI. Global MCP is `~/.gemini/config/mcp_config.json`; project MCP is `.agents/mcp_config.json`. MCP and hooks placed inside CLI `settings.json` are ignored by the client and receive an advisory.
- MCP supports stdio and Streamable HTTP. Negative timeouts disable the timeout and zero uses the default. Legacy SSE fields are not accepted as native Streamable HTTP configuration.
- Settings and plugin schemas do not by themselves establish fatal runtime behavior. Malformed permission resources, unknown formatter fallback, ignored logo values and folder-loaded plugin name fallback use advisories where appropriate.
- `~/.gemini/config/config.json` checks the documented `cliRemoteControlHostname` field's type without starting or restarting the daemon.
- Hooks use native event maps, tool matcher groups and direct handlers. A command handler's `type` may be omitted where the native format permits it.
- Modular rules use `always_on`, `model_decision`, `glob` or `manual`. Default rule discovery is flat. Nested or external rules require registry entries, including inherited registries and relative include/exclude lists.
- `@[label](path)` includes resolve from the containing rule; `~/` resolves from the user home. Included frontmatter is stripped before byte counting. Expansion uses the configured filesystem, rejects symlink reads, detects cycles and shares agnix's existing aggregate read limit. An unavailable expansion leaves the expanded size unknown.
- Agent model values remain open strings. Official examples use both aliases and full model IDs; an invented closed model list would reject valid agents.

### Grok Code

- User settings and project settings have different scopes. Project `config.toml` contributes MCP servers, plugins, permissions and `mcp.max_output_bytes`. User settings belong under `$GROK_HOME` or the default user directory.
- Managed ZDR video storage checks required S3-compatible storage fields and credential-object shapes. Diagnostics never include credential values. The undocumented minimum URL lifetime is not invented as a constraint.
- The bypass-permissions lock is effective only in the native system policy file. An unrelated ancestor named `etc/grok` does not establish system scope.
- MCP timeouts and per-tool timeout maps use unsigned integers. Blank commands/URLs and malformed inline agent MCP configurations receive native diagnostics.
- Hooks accept documented event aliases, command and HTTP handlers, and released command arguments. Ignored-event matchers do not receive false regex errors.
- Plugin paths must stay within their plugin root. In-root absolute paths are accepted when the root can be established. Plugin versions are opaque strings rather than an invented SemVer requirement.
- Grok skill names normalize spaces and underscores to hyphens. Omitted names and descriptions use the native fallback behavior. `allowed-tools` is advisory metadata rather than an enforced runtime sandbox.
- Agent metadata preserves the native YAML scalar coercion for typed strings. Recovery objects require the complete native field set. Plugin agents cannot gain privileged MCP, hooks or bypass permission settings through their own metadata.
- Marketplace indexes and LSP maps use their own contracts; they are not parsed as hook files. Static LSP checks cover documented command/argument types, transports, extension maps, aliases and timeout fields.

## Discovery and interfaces

Recognized `$schema` IDs select bundled Pi or Antigravity contracts. Agnix never fetches a user-provided schema. Renamed schema-tagged JSON documents work in explicit-file validation, project scans and editor buffers. Explicit `[files]` exclusions take precedence.

Editor buffer content replaces the saved copy during classification and validation, including package ownership. Adding, changing or removing metadata cannot leave stale disk ownership active in that document.

The CLI accepts `pi`, `antigravity` (`agy`) and `grok-code` (`grok`). The config schema, VS Code selector/settings, language server configuration, WebAssembly tool list and editor path recognition include the new tools. All interfaces use the same core validators.

## Monitoring

Release tracking uses Pi's stable GitHub tags, the Antigravity installer's platform manifest and the official Grok Build RSS feed. Structured extractors ignore unrelated version numbers and fail when source markers are missing. HTTP failures do not produce partial baseline updates.

Pi schemas join A-tier specification checks. Antigravity schemas/rules and Grok's reference join B-tier checks. A/B source checks run weekly on Wednesday; D-tier checks remain monthly. A detected change requires review before advancing the baseline.

Source freshness is separate from coverage. A release marker or a downloaded blog does not establish a new rule. Constraints in the catalog cite the particular schema, parser or documentation page that supports them.
