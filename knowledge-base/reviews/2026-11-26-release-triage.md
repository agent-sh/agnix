# Release and spec follow-up - 2026-11-26

The read-only release poll found seven newer markers, seven matches and one manual source (Muse). It reported no errors. The table covers the ten open issues; Cline advanced during the same poll without an open release issue. The [November 25 review](2026-11-25-release-triage.md) records the earlier validator changes.

| Issue | Reviewed marker | Finding |
| --- | --- | --- |
| #1885 | Antigravity CLI 1.5.18 | [`agy changelog`](https://antigravity.google/docs/cli/commands/changelog) lists runtime fixes, including Windows cross-drive plugin loading and `file:///` permission resolution. Published settings and plugin schemas still match the vendored source digests. |
| #1886 | Claude Code v2.1.332 | Poll unchanged. The previous review covers `PreSkillShellCommand` and plugin `optionalDependencies`. |
| #1887 | Codex CLI rust-v0.172.0 | Poll unchanged. The previous review covers per-tool `approvals_reviewer` and two feature flags. |
| #1888 | Kimi Code 2.8.1 | The [2.8.0 CLI release](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai/kimi-code%402.8.0) skips legacy `[experimental]` config on migration and describes `ask` permission overrides. [2.8.1](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai/kimi-code%402.8.1) fixes cancellation, token usage and session tools. Kimi-specific validation remains unimplemented. |
| #1890 | Four reported spec sources | See the source-by-source findings below. The two Pi sentinel hashes track `main`, not the released contract. |
| #1891 | Cursor 3.32.10 | The [stable endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) advanced. The [changelog](https://cursor.com/changelog) has no patch-specific notes; the rules, hooks, subagents, environment guide and schema, and MCP sentinels all match. No change identified beyond the earlier `clone` finding for CUR-016. |
| #1892 | Grok Code 1.0.92 | Poll unchanged. The previous review covers GRK-SBX-002 and the native rejection of unknown sandbox keys. |
| #1893 | Kiro CLI 3.6.2 | The [3.6 patches](https://kiro.dev/changelog/cli/3-6#patch-3-6-2) fix OAuth callback handling (3.6.1) and change permission prompts, tool trust and headless agent selection (3.6.2). An unresolved `chat.defaultAgent` now warns and continues in V3 headless runs; the key remains a string. No new static field beyond the 3.6.0 review. |
| #1894 | OpenCode 2 v2.4.2 | [Release notes](https://github.com/anomalyco/opencode/releases/tag/v2.4.2) change the scope of saved web approvals and what happens after an automation fails. The tagged `packages/schema/src/config.ts` and `packages/cli/src/config/{config,schema}.ts` blobs are identical to v2.4.1. No new OC-004 key. |
| #1895 | Pi v1.6.1 | The [release](https://github.com/earendil-works/pi/releases/tag/v1.6.1) ships typed notice settings. See the tag versus `main` comparison below. |
| No issue | Cline v4.1.28 | [Release notes](https://github.com/cline/cline/releases/tag/v4.1.28) cover the model catalog, providers, license notices and telemetry. The Cline rules sentinel still matches. |

## #1890: reported sources

- **Pi models:** v1.6.0 to v1.6.1 changes only the two descriptions of the thinking-level mapping. The tagged model constraints do not change. The release notes also mention Gemini `supportsStrictMode`; the tagged schema diff does not introduce that key. See the separate `main` changes below.
- **Antigravity settings:** the [published schema](https://antigravity.google/schemas/v1/settings.json) still matches the raw SHA-256 in `sources.json`. The sentinel uses normalized bytes and therefore records a different digest. No new AGY-CFG or AGY-KB constraint.
- **Antigravity rules:** the [rules reference](https://antigravity.google/docs/rules) still requires a frontmatter trigger for modular rules, a description for `model_decision` and globs for `glob`. The per-file limit is 24,000 bytes after includes. The 20,000-token budget for all active global and `always_on` rules depends on the runtime context; AGY-RL-003 is a per-file check, not a prediction of which rules will be demoted.
- **Antigravity discovery:** the [discovery reference](https://antigravity.google/docs/customization-discovery) distinguishes immediate children from explicit registry entries and workspace ancestors from descendants. AGY-RL-004 checks registry shape; it does not reconstruct a running session's search roots. Both documentation hashes still match the November 25 reproduction.

## Pi: release versus moving source

In the [tagged settings schema](https://github.com/earendil-works/pi/blob/v1.6.1/packages/coding-agent/schemas/settings.schema.json), `notices` is an object. Its optional `cacheMisses`, `cacheWarming`, `compactionCosts` and `reasoningDrops` fields are booleans. The old `showCacheMissNotices` boolean remains valid but deprecated; Pi migrates an existing value to the four settings. PI-CFG-002 checks types without rewriting user settings. The tagged keybinding and theme schemas are unchanged.

Compared with the release, the [reviewed `main` snapshot](https://github.com/earendil-works/pi/tree/ef17a41ad66a3d3bfae42e46fa472d4d63a77e89/packages/coding-agent/schemas) removes `enableInstallTelemetry` from settings, removes `ant-ling` from the model thinking-format choices and adds the boolean `supportsReasoningReplay`. These observations update the sentinels only. They do not remove or add constraints in the v1.6.1 bundle.

The full sentinel reproduction also found drift on six MCP and five Claude Code sources outside #1890. Their baselines are unchanged; this review does not establish whether those pages require rule changes.

## Verification

| Read-only check | Before | After |
| --- | --- | --- |
| Release watcher (`UPDATE_BASELINES=true`) | 7 matches, 7 new, 1 skipped, 0 errors | 14 matches, 0 new, 1 skipped, 0 errors |
| Spec sentinel | 13 changed, 0 fetch failures | 11 changed, all outside #1890; 0 fetch failures |

`node scripts/sync-rule-bookkeeping.js --check` passed after regeneration. The rule catalog and its crate mirror are byte-identical.
