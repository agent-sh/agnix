# Release and spec-drift review - 2026-11-04

Covers the six open bot issues: #1789 Cursor 3.28.7, #1790 Kiro CLI (2.37.1
marker, 3.0.0 live), #1791 Pi v1.5.0, #1792 spec drift, #1793 Antigravity CLI
1.5.0 and #1794 Claude Code v2.1.318. Every version was checked against the
live upstream surface on 2026-11-04, and all 32 tracked spec sources were
re-hashed with the sentinel's own fetch and normalization, not only the four
the issue named. The catalog stays at 757 rule IDs.

| Item | Result and primary evidence |
| --- | --- |
| Claude Code v2.1.316-v2.1.318 (#1794) | The issue skipped 2.1.316 and 2.1.317; all three [release notes](https://github.com/anthropics/claude-code/releases/tag/v2.1.318) were read. No settings key, hook event, frontmatter field, plugin or marketplace schema that agnix validates changed. `claude plugin validate` now rejects a custom-named marketplace file, which agnix never detects (only `.claude-plugin/marketplace.json`). `CLAUDE_CODE_SUBAGENT_EFFORT`, `deniedMcpServers` semantics and Agent SDK `onFailure` are runtime. Baseline-only advance. |
| Claude Code docs drift (found by the re-hash) | memory, hooks, plugins-reference and sub-agents changed after the 2026-11-04 sentinel run. Diffed against Wayback captures from before the 2026-11-02 baseline: `/context all` wording, the `*.md`/`/*.md` glob table (matching semantics, CC-MEM-011 checks syntax only), a hooks note on `updatedInput` in auto mode (runtime), and `CLAUDE_PROJECT_DIR` now exported to plugin MCP stdio and LSP servers. The plugins reference lists three plugin path variables, none in the Bash tool's environment, so CC-PL-020 now also reports unbraced `$CLAUDE_PROJECT_DIR` in plugin skill and agent bodies. |
| Cursor 3.28.7 (#1789) | The stable update endpoint reports `3.28.7`. The [changelog](https://cursor.com/changelog) has no entry after 2026-10-20. Baseline-only advance. |
| Spec Drift Sentinel fix | cursor.com answers 404 to its `.md` URLs when the request asks for `text/markdown`, the sentinel's default since 2026-10-29, so all five Cursor sources failed to fetch on the 2026-11-01 d-tier run (reported as "may be temporary" in #1784). Sources can now set `accept`; with `*/*` four Cursor hashes match the baselines and `cursor-subagents` changed only in billing wording. |
| Kiro CLI 2.37.0, 2.37.1, 3.0.0 (#1790) | The [canonical changelog](https://kiro.dev/changelog/cli/) lists 3.0.0 (2026-11-03): V3 becomes the interactive default. The current [permissions reference](https://kiro.dev/docs/permissions/) lists `evaluate` for user, workspace and agent scope ("Both scopes support all effects"), so KR-PM-003 no longer warns that it is unsupported; it stays fatal in `managed-settings.json`. The [settings reference](https://kiro.dev/docs/reference/settings/) adds `acp.agentEngine` (v1/v2/v3, global) and documents `app.warnOnElevatedPrivileges` as global. Since 2.37 users set `chat.enableAutoAgentUpgrade` themselves, so it no longer marks a file as a copy of the global file. KR-AG-015's message notes that 2.37+ rewrites an unconverted profile at the next V3 launch. A 3.x headless run without a saved harness still runs V2, so headless CI should leave Kiro unpinned. |
| Pi v1.5.0 (#1791) | Tagged [v1.5.0](https://github.com/earendil-works/pi/releases/tag/v1.5.0). `oauth.clientRegistration: "auto"` is the new default (`mcp-servers.ts`), so PI-MCP-005 no longer rejects it. `selfUpdateCommand` "Can only be set in agent-directory settings" (PI-CFG-003). The settings constraints gain `extensionChangelogVersions` and `selfUpdateCommand` (PI-CFG-002). A `pi` object with only metadata keeps directory auto-discovery (`hasManifestResourceFields`). PI-PKG-001 reports a non-string `changelogPath`. PI-PKG-002 covers the full host-provided list, including the `@mariozechner/*` names and `@sinclair/typebox`. The TypeBox change concerns extension tool schemas, which agnix does not execute or validate. |
| Pi schema drift (#1792) | `pi-settings` equals the v1.5.0 tag. `pi-models` moved twice on `main` after the sentinel run (`supportsMessageThreads`, `requiresNonNullAssistantContent`, `promptCacheKeyMode` under `ProviderCompat`, unreleased and accepted by `additionalProperties`). The baseline takes the live `main` hash; the vendored constraints stay on the v1.5.0 tag. |
| Antigravity CLI 1.5.0 (#1793) | The installer manifest reports `1.5.0`; `agy changelog` lists 13 entries, none on a file agnix validates (ACP modes, CLI flags, runtime fixes; the alt-screen rename keeps the saved setting). Baseline advances. |
| Antigravity drift (#1792) | `settings.json`: only the `agentMode` default changed (`""` to `"auto"`); the constraint file is byte-identical, so only the source digest is refreshed. `rules`: the page now points to [Customization discovery](https://antigravity.google/docs/customization-discovery), which defines registry resolution. agnix resolved entries from the `.agents/` folder, registered every file below a folder entry and expanded `~`, so AGY-RL-001..003 ran on the wrong files. Registry paths now start at the folder that contains `.agents/`, then the repository root; personal manifests start at `~/.gemini/config/`; `~` is not expanded; folder entries load direct children only, and `include_only`/`exclude` match names with or without `.md` or relative paths. The `_agents/` and `_agent/` folders are recognized. The rules source now tracks `rules.md` (the HTML hash changed on every deploy) and the discovery page is a new source. |

## Not changed, with reasons

- Kiro "classic only" settings (`chat.diffTool`, `chat.enableTangentMode` and others) have no effect in a 3.0 terminal UI session. Reporting them would be a new rule; it is left for a separate decision.
- Kiro `evaluate` in agent `permissions`: the agent configuration reference lists three effects while the permissions Scopes table lists four. KR-AG-014 is unchanged until the two agree.
- Antigravity `~/.gemini/antigravity-cli/rules/*.md`: dropped from the docs without a changelog entry; agnix still treats those files as rules.

## Verification digests

| Source | SHA-256 (sentinel form) |
| --- | --- |
| code.claude.com/docs/en/memory.md | `ba5f52145910b15d92d9b23a68b2181e8cf648c79644db2a8048bfff559a6833` |
| code.claude.com/docs/en/hooks.md | `62afd408e8fe643615312d9ca8a067f5e5f93b9d70afe7f45ca05b23a1fdb69e` |
| code.claude.com/docs/en/plugins-reference.md | `b402b702d29c7f5953ad370ac3d4069301d125c170d96aa989640e7db4f2c066` |
| code.claude.com/docs/en/sub-agents.md | `62a7d93eed76d8a13b6c71343d93c38d916b60d9be1dbec1de944ddebdee1dac` |
| cursor.com/docs/subagents.md (`Accept: */*`) | `83d76612ae1a4ff90a79064834b72d17312f7a3ea91d1655705b697acbe83160` |
| pi settings.schema.json (main = v1.5.0) | `d46ea8863f3a20b9f651fca95b163ce8d5bb11dea02623212027f06199188dde` |
| pi models.schema.json (main) | `66ade0ce69584a4d709fab536f3015d46e3fc0d2344d28c747b5c5f6fc2a7fe4` |
| antigravity.google/schemas/v1/settings.json | `607ed49919b07cc8d3f2a9ff536b0f1a3c95dd93ca4d98b62e6054c864626e1c` (raw `3a29af2bf640...`) |
| antigravity.google/docs/rules.md | `da142a2920f0b8dae56a6a7fc1a2bcedb46bcfe929ef5e24d0ad3e956880fbb3` |
| antigravity.google/docs/customization-discovery.md | `ad17fd95ed873820f870daff76091d697e02981733a1aa6ddf7ed256387e5021` |

Vendored Pi v1.5.0 constraints: settings `17d7228a42501e24...`, models
`c812e82e5da09f95...`; keybindings and theme are unchanged.

This review concerns static configuration compatibility. Runtime dispatch,
model behavior and vendor runtime fixes are not executed or qualified by agnix.
