# Release Triage - 2026-11-08

Sweep of the six automated agnix issues opened 2026-11-06/08: five tool
releases and one spec-drift sentinel alert. All version data comes from
`scripts/check-tool-releases.sh` run live with `UPDATE_BASELINES=true`
(errors=0, new=6) against the sources below.

## Baseline advances

| Tool | Old | New | Source | Notes |
|------|-----|-----|--------|-------|
| Claude Code | v2.1.318 | v2.1.319 | github.com/anthropics/claude-code/releases/tag/v2.1.319 | Auto-mode server checks, MCP tasks moved to the background, `permissions.denyInsteadOfPrompting`, gateway settings. No change to a validated config surface: `denyInsteadOfPrompting` is a boolean permission switch (permission-rule shape unchanged), MCP task behavior is runtime, env vars are not validated fields. |
| Codex CLI | rust-v0.166.0 | rust-v0.167.0 | github.com/openai/codex/releases/tag/rust-v0.167.0 | Command center, backend switching, image previews, `gcp_setup_wizard` experimental, proxy settings. All TUI/runtime; no config-schema or hooks change. |
| Kiro CLI | 3.0.0 | 3.1.0 | kiro.dev/changelog/cli | Compaction hooks (rule below) plus session-credits UI, defaults, recap and notification fixes. |
| OpenCode 2 | v2.2.5 | v2.2.6 | github.com/anomalyco/opencode/releases/tag/v2.2.6 | `pool: true` MCP connection sharing (rule below), `--worktree`/`--archived` run flags, `websearch.use` policy action, `experimental.all_flags`. Policy-action and flag surface: OC-POL covers `websearch.*` actions generically; no other validated field changes. |
| Antigravity CLI | 1.5.0 | 1.5.3 | antigravity.google/docs/cli/commands/changelog | Version marker only (the page is JS-rendered; no notes exposed). Issue #1800 reported 1.5.2; the live marker is 1.5.3, so the baseline advances to the latest. |
| Gemini CLI | v0.66.0 | v0.67.0 | github.com/google-gemini/gemini-cli/releases/tag/v0.67.0 | No issue: CI triage filed it as not agnix-relevant, confirmed here. Fixes, a `models`/`--list-models` discovery command, deprecated-settings migration keeping `${VAR}` placeholders, numeric settings validation. No `.gemini/settings.json` key surface added. |

## New rules

### KR-HK-031 [LOW] Kiro Compaction Hook Requires Kiro CLI 3.1.0

Kiro CLI 3.1.0 (2026-11-05, "Compaction Hooks and Session Credits") added
`PreCompact` and `PostCompact` hooks in V3 sessions: "`PreCompact` fires
before manual/automatic compaction and can refuse it; `PostCompact` fires
afterward and receives the summary." The V3 engine tables in
`schemas/kiro_hook_v3.rs` mirror kiro-cli 2.32.1, whose trigger lookup does
not know the names, and an unrecognized trigger makes the engine skip just
that hook. The rule reports an Info at the `trigger` value instead of the
KR-HK-012 unknown-trigger error: the file is well-formed for current CLI,
but 3.0.x and older silently skip the hook. A matcher on a compaction
trigger has no documented behavior, so KR-HK-020 stays silent for them.
Agent-embedded hooks share the hook schema, so the same info replaces the
KR-HK-005 unknown-trigger warning on the V3 path (object key and array
`trigger`), keeping both surfaces consistent.

### OC-CFG-023 [MEDIUM] MCP Server pool Flag Not a Boolean

OpenCode 2 v2.2.6 added opt-in MCP connection sharing: "Added opt-in MCP
connection sharing through `pool: true`" (release notes, PR #61185). The
tagged schema source confirms the contract:
`packages/schema/src/mcp.ts` at v2.2.6 declares `pool: Schema.Boolean` on
both `LocalConfig` and `RemoteConfig`, annotated "Share one connection to
this server across every directory that configures it identically ...
Only connections that negotiate the 2026-07-28 revision are shared; an
unset 'protocol' becomes 'auto', and 'legacy' never shares. Defaults to
false." The rule errors on a non-boolean `pool` in an `mcp.servers` entry
on the OpenCode 2 line; the OpenCode 1 line stays silent (same gating as
the `protocol` check).

Issue #1808's auto-triage proposed this as MCP-034. Implemented as
OC-CFG-023 instead: the MCP rule family is client-neutral by design
(MCP-025/MCP-026 moved out of it when they were found to encode Claude
Code facts), and the existing OC-CFG-021 already holds the OpenCode
`mcp.servers` entry-field checks (`enabled`, numeric `timeout`, OAuth
field spellings).

## Spec drift (#1809): claude-code-hooks, claude-code-skills

Live docs fetched with the sentinel's exact normalization (curl with
`agnix-release-watch/1.0`, CRLF stripped, sha256); computed hashes match
the alert: hooks `bf5bcc0cb4e8...`, skills `2a0e2b6e35fb...`.

Structural comparison against the schemas found no rule gap:

- hooks.md documents 33 events; `HooksSchema::VALID_EVENTS` covers all 33,
  including `PreCompact`/`PostCompact` (with matcher support) and the
  newer `Setup`, `InstructionsLoaded`, `MessageDisplay`, `PostToolBatch`,
  `StopFailure`, `Elicitation*`, `Worktree*`, `FileChanged` events.
- Handler types Command/HTTP/MCP-tool/Prompt/Agent and the `once` field
  (skill frontmatter hooks) are all present in `schemas/hooks.rs`.
- Per-event matcher lists match `rules/hooks/helpers.rs` tables
  (`COMPACT_MATCHERS = [manual, auto]`, session-end legacy values,
  instructions-loaded load reasons, monitor commands).
- skills.md's frontmatter table lists 20 fields;
  `KNOWN_SKILL_FRONTMATTER_FIELDS` matches it exactly, including
  `hooks`, `effort`, `paths`, `shell`, `metadata`, `license`,
  `compatibility`.

The drift is prose and reorganization (hook-lifecycle ordering, workspace
trust, skill hook limits). Both baseline hashes refreshed in
`.github/spec-baselines.json`; no rule changed.

## Verification

- `cargo test -p agnix-core` kiro_hook_v3, opencode, kiro_v3_support
  fixtures (new rule tests green; v3 fixture inventory 18 -> 19).
- `node scripts/sync-rule-bookkeeping.js` (758 -> 760 rules; mirrors,
  count phrases, website docs regenerated) and `--check` clean.
- Rules parity: rules.json and VALIDATION-RULES.md entries added together.
