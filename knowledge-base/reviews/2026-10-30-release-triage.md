# Release compatibility review - 2026-10-30

Review all nine open release notices against current main, not the older watcher
baseline in each issue. The read-only release poll also found OpenCode 2.2.3 and
Kimi Code 2.4.1. The catalog remains at 757 rule IDs.

| Issue | Result and primary evidence |
| --- | --- |
| #1752 Antigravity 1.4.11 | Read `agy changelog`, documented at the [official CLI changelog command](https://antigravity.google/docs/cli/commands/changelog). `terminationBehavior` and injected `errorMessage` are hook stdout fields, not hook configuration. `finished_reason` is CLI output. Billing selection and Claude plugin import fixes do not change the static configuration contract. Settings and plugin schemas remain byte-identical to the pinned sources. Advance the baseline. |
| #1753 Claude Code 2.1.314 | [Release notes](https://github.com/anthropics/claude-code/releases/tag/v2.1.314) add `permissions.blockWritesOutsideWorkingDirectories`. Existing validation accepts this nested setting. Add compatibility coverage alongside `Monitor(tail *)` hook conditions. Monitor/Bash matching changes concern runtime dispatch; agnix does not execute conditions. Gateway `groups` and `guardrail_trace` belong to the separate apps gateway. Advance the baseline. |
| #1754 Codex 0.166.0 | Compare the complete tagged [0.165 schema](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/core/config.schema.json) and [0.166 schema](https://github.com/openai/codex/blob/rust-v0.166.0/codex-rs/core/config.schema.json). Eight root/profile feature names need allowlist updates. Other additions are below accepted tables: GCP provider credentials/location/project, memory consolidation service tier, rollout budget enforcement, transcript keymap and proxy credential actions. Gateway OAuth delivery becomes optional. Agnix does not enforce those nested contracts. Advance the baseline. |
| #1755 Amp macOS satellites | [Announcement](https://ampcode.com/news/macos-satellites) describes remote Mac execution, not a new configuration file contract. Amp was removed from active support and tracking in #1743, released in 0.60.0. Close as no longer tracked; preserve shipped legacy validators. |
| #1756 Cline 4.1.26 | Already reviewed and baselined in #1741, released in 0.60.0. [Tagged release](https://github.com/cline/cline/releases/tag/v4.1.26): disabled rules persist an already accepted `disabled` field. MCP `_meta` is a tool-result field. Existing integration coverage remains. |
| #1757 Cursor 3.27.22 | Already reviewed in #1739/#1741 and baselined. The [stable endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) still reports 3.27.22; it does not publish per-patch notes. The [changelog](https://cursor.com/changelog) describes Agent SDK deployment, distinct from the supported editor configuration. No unsupported claim of SDK validation. |
| #1758 Gemini 0.66.0 | Already reviewed and fixed in #1741, released in 0.60.0. [Tagged skill loader](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/skills/skillLoader.ts): delimiter whitespace and Gemini ownership are covered by integration regressions. Nested `oauth.allowPrivateNetwork` was already accepted. |
| #1759 Kiro 2.34.0 | Issue marker is older than the reviewed 2.34.2 patch. [Canonical release and patch links](https://kiro.dev/changelog/cli/2-34/) still resolve to 2.34.2. #1741 added documented `chat.workflowLaunch` choices with type/enum checks and fixed patch detection. Keep the newer baseline. |
| #1760 OpenCode 1.19.5 | Already reviewed in #1741 and baselined. [Release](https://github.com/anomalyco/opencode/releases/tag/v1.19.5) fixes a TUI worker response. Complete tagged source-tree comparison found no configuration schema change, as recorded in the October 29 all-tool review. Keep V1 tracking separate from V2. |

## Additional releases found by the live poll

[OpenCode 2.2.3](https://github.com/anomalyco/opencode/releases/tag/v2.2.3)
adds `integration.connect`, `mcp.use`, `skill.use` and `plugin.use` policy actions.
The [tagged policy schema](https://github.com/anomalyco/opencode/blob/v2.2.3/packages/schema/src/config/policy.ts)
accepts bare integration/account resources and normalizes legacy `mcp:`, `skill:`
and `plugin:` resources to the separate actions. The 2.2.2 tagged schema still
has the old three-action contract. Update OC-POL-001, preserve older pins,
and compare normalized legacy rules for shadowing. The `policies` and deprecated
`experimental.policies` arrays share validation. The complete 2.2.1/2.2.3 trees
contain 10,098/10,246 entries and are not truncated; the ordinary compare endpoint
caps at 300 files. Other changed configuration schemas only add optional
`experimental.location_idle_timeout` or provider settings already accepted as
open objects. Provider SDK and schedule/runtime changes add no per-file checks.

[Kimi Code 2.4.1](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai%2Fkimi-code%402.4.1)
changes file-read limits and adds `[read].max_chars`, with prompt/web fixes.
Advance release tracking only. Tool-specific Kimi validation remains pending;
this review does not claim support for its native configuration.

The remaining live markers match their baselines: Agent Skills specification,
OpenCode 1, Kiro, Cline, Cursor, Gemini, Copilot, Pi and Grok Code. Muse remains
untracked with validation pending. Do not reintroduce retired Amp tracking.
The final read-only poll reports `ok=14 new=0 skipped=1 errors=0`.

The independent review also found seven feature keys present in both the 0.165
and 0.166 root/profile schemas but absent from agnix: `app_server_websocket_compression`,
`executor_capability_discovery_v2`, `mcp_tool_list_changed`, `multi_env_copy_file`,
`runtime_agent_instances`, `subagent_thread_environments` and `wake_on_agent_messages`.
Accept these alongside the eight new keys. Cover the structured
`runtime_agent_instances = { enabled = true }` form. This removes unknown-key
false positives; it does not add validation of nested feature values. The
existing CDX-APP-003 warning for legacy inline profiles is preserved.

## Antigravity source digests

| Source | SHA-256 |
| --- | --- |
| [Settings](https://antigravity.google/schemas/v1/settings.json) | `6d4ab641fff729c0ec416afc8c96ccb414d9f438fe3f28fec294086a8f824877` |
| [Plugin](https://antigravity.google/schemas/v1/plugin.json) | `0eea5e483af6f03fa099f32b065d2924a323f1a616ed0323cfa0b7a74ff9543c` |

This review concerns static configuration compatibility. Hook stdout, model
behavior, remote execution, authentication and vendor runtime fixes are not
executed or qualified by agnix.
