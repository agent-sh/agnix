# Release compatibility review - 2026-11-18

Issues #1848 (Antigravity CLI), #1849 (Claude Code), #1850 (Kiro CLI) and
#1851 (spec drift) were reviewed against the live vendor sources, not just
the watcher's quoted markers. The read-only release poll
(`UPDATE_BASELINES=true TOOL_FILTER=<tool> bash scripts/check-tool-releases.sh`)
reports `new=1` for kiro (3.4.0), claude-code (v2.1.327, covering the
issue's v2.1.326 plus the newer v2.1.327) and antigravity (1.5.11, covering
the issue's 1.5.10), so the baselines advance to the live markers, which
cover and exceed the versions the issues quote.

A fetch caveat worth recording: CDN-cached copies of kiro.dev served a
stale changelog (top entry 3.3.0) and a stale hooks/types page (no
`PermissionRequest` section) on first retrieval; fresh curl fetches with
`Cache-Control: no-cache` and the `.md` doc endpoints return the 3.4.0
changelog entry and the full Permission Request / structured-decision
sections quoted below. All quotes come from the fresh fetches.

| Issue | Result and primary evidence |
| --- | --- |
| #1850 Kiro CLI 3.3.0 -> 3.4.0 | The live changelog entry "Hooks That Rewrite Tool Inputs and Answer Permission Prompts" (Nov 16, 2026) and the live hooks types doc back both rule candidates, so both are implemented. KR-HK-032 (info, LOW): "In Kiro CLI 3.4.0 or later, the `PermissionRequest` trigger runs in a V3 session after Kiro has evaluated its permission policy, immediately before a tool call would prompt you"; "The camelCase alias `permissionRequest` is also accepted"; "Only command actions run on this trigger. The `matcher` field works as it does for Pre Tool Use"; CLI 2.x "has no equivalent". The mirrored engine table is kiro-cli 2.32.1, which knows neither spelling, so older CLI (3.3.x and earlier) skips the hook. Implementation mirrors KR-HK-031: standalone files report at the `trigger` value, agent-embedded hooks (object key and array form) get the info instead of the KR-HK-005 unknown-trigger warning, and `matcher_subject` maps the trigger to tool semantics so KR-HK-020 stays silent and KR-HK-027/028/029 apply. Unlike KR-HK-031, the standalone-file finding is gated on the resolved CLI version (silent when pinned to 3.4.0+, per the issue's rule candidate). KR-HK-033 (info, LOW): "In Kiro CLI 3.4.0 or later, a V3 Hook that exits 0 can also print a JSON decision on STDOUT. Before 3.4.0, the exit code was the only decision a CLI Hook could make"; "`updatedInput` replaces the whole `tool_input` object. It is not merged, and later Pre Tool Use Hooks receive the replacement." Detection scans a PreToolUse command action's command text for `updatedInput` (the response shape lives in the command's STDOUT, not in a hook-file field) and is silent when the CLI is pinned to 3.4.0+. The docs also state only command actions run on `PermissionRequest`; an agent-action never-runs branch for KR-HK-024 was considered and left out of this batch (scope: the two issue candidates). Fixtures `kr-hk-032-permission-request.json` and `kr-hk-033-updated-input.json` join the v3/invalid corpus (kiro-hooks inventory 35 -> 37 files, 32 -> 34 diagnostics). |
| #1849 Claude Code v2.1.325 -> v2.1.326 -> v2.1.327 | The issue's expected "mostly UI/UX" triage holds with one rule-bearing exception, re-verified against the v2.1.326 tagged release notes and the live manifest reference: v2.1.326 "Changed enabled plugins that were refused for a malformed `plugin.json` display field: they now load, with their hooks and MCP servers" and "Added a note in `/plugin` and `claude plugin list` when a malformed `plugin.json` field such as `author` or `homepage` is ignored". CC-PL-010's load-bearing quote ("Must parse as a URL, or the plugin fails to load") is therefore no longer the behavior on current CLI (the manifest reference table still carries the older sentence), so CC-PL-010 drops from HIGH/error to MEDIUM/warning, matching CC-PL-009's display-field class, and its message names both behaviors; CC-PL-009's detection text records the same change. No other v2.1.326 item touches a validated surface: the `--allowedTools`/`--disallowedTools` startup warning is CLI-flag UX (agnix validates the rule shapes in files), the 64-level settings depth cap has no depth rule, HTTP hooks' `Content-Length`/`Transfer-Encoding` ignoring has no header rule, and the PreToolUse `updatedInput`-keeping improvement concerns Claude Code's own long-standing field. v2.1.327 (published 2026-11-17, after the issue was filed) was triaged from the tagged release notes: no validated-field change (`CwdChanged`/`FileChanged` in the notes are already among the 33 known hook events; `claude mcp add-json -`, the gateway checksum header and `attributionProject` are CLI features with no schema rule). Baselines advance to v2.1.327. |
| #1848 Antigravity CLI 1.5.9 -> 1.5.10 -> 1.5.11 | Tier B. The changelog source remains a version marker only; the live poll reports 1.5.11, so the baseline advances to the live marker covering both releases. The two Antigravity spec sources the drift sentinel flagged (see #1851) were constraint-diffed as part of this batch and changed nothing agnix validates; no validator change. |
| #1851 Spec drift (opencode-rules, pi-models, antigravity-plugin, antigravity-settings) | All four live sources were re-fetched with the spec-drift workflow's exact recipe (curl `agnix-release-watch/1.0`, the sentinel Accept header, CRLF normalization) and the computed hashes reproduce the issue's "current hash" values byte for byte: opencode-rules 9ec707ba349c..., pi-models e132feaec22f..., antigravity-plugin c54359a306b6..., antigravity-settings b50f0aceb4e8.... None changes a rule. opencode-rules: the live rules.mdx still carries every load-bearing claim behind OC-AGM-003 and XP-001..006 ("`CLAUDE.md` in your project directory (used if no `AGENTS.md` exists)" for the OpenCode 1 fallback the rule quotes, the precedence order including the Claude Code fallback, "While opencode doesn't automatically parse file references in `AGENTS.md`" for XP-001, the `instructions` array for XP-004/005/006); the file's git history at the tracked path shows no commit since the 2026-11-10 docs move that the 2026-11-11 baseline already recorded as byte-identical, so the delta is serving-side prose churn (frontmatter/description work in the site repo), not a semantic change. pi-models: the only upstream commit since the vendored v1.5.2 constraints (49fe0a2374, 2026-11-11) changes one word of the `supportsMessageThreads` description ("provider anthropic with a baseUrl on api.anthropic.com" -> "a baseUrl on api.anthropic.com"); a full recursive constraint-level diff (descriptions, titles, defaults, examples, deprecated markers, $comment stripped) against the vendored copy is identical, so PI-MDL-001/002 are unchanged. antigravity-plugin/settings: constraint-level diffs against the vendored copies are identical; the visible deltas are the annotation set the schema host alternates (descriptions/examples/defaults present on one serving, absent on the other - the same flip the 2026-11-13 and 2026-11-15 reviews recorded), so AGY-PL-001/002 and AGY-CFG-001..005/AGY-KB-001 are unchanged and no re-vendor is needed. All four hashes and `last_checked` refresh to 2026-11-18. |

## Verification

- Supply-chain note: two brand-new advisories broke the security jobs for
  every PR in the repo (main's last Security run, 2026-11-16, predates
  them): RUSTSEC-2026-0543 (`chrono` unmaintained, no vulnerability, no
  patched version; required non-optionally by rmcp 3.9.0 and latest
  3.11.0) and GHSA-fc5q-39xc-vjpf (picomatch ReDoS "through 4.0.7", no
  vendor fix), the latter reddening the npm audit of the website (26
  high) and the VS Code extension (6 high) through chokidar/micromatch
  and @vscode/vsce/tinyglobby dev toolchains. The cargo side follows the
  repo's documented remediation for unmaintained crates (ignore in
  security.yml + deny.toml + docs/RUSTSEC-ADVISORIES.md, cf. the
  RUSTSEC-2025-0141/0173 precedents); verified locally with
  `cargo audit --deny warnings` and `cargo deny check advisories`, both
  clean. The npm audit was NOT modified: no fixed version exists to bump
  to and the project has no npm-ignore mechanism, so that check stays
  red until the advisory is revised upstream - recorded here and in the
  PR body as the one open CI blocker.
- `UPDATE_BASELINES=true TOOL_FILTER=<tool> bash scripts/check-tool-releases.sh`
  for kiro, claude-code and antigravity: `new=1` before the baseline edits
  (3.4.0, v2.1.327, 1.5.11); after the edits each per-tool poll reports
  `ok=1 new=0`. A full unfilitered poll was not used as evidence because the
  sandbox had intermittent api.github.com timeouts (kimi-code, opencode,
  opencode-v2, pi errored) and it also reports an out-of-scope grok-code
  1.0.88 marker that no issue in this batch covers (left for its own issue).
- The four sentinel hashes reproduce exactly under the workflow's fetch
  recipe (see above).
- `cargo test -p agnix-core --lib` (4,531 tests including the new
  KR-HK-032/033 unit tests and the kiro_agent coverage),
  `cargo test -p agnix-core --test claude_ext_fixtures --test
  spec_drift_integration --test release_triage` (8 + 24 + 5 tests),
  `cargo test -p agnix-cli --test rule_parity --test kiro_fixture_inventory
  --test fixture_family_cli` (24 + 7 + 21 tests, kiro-hooks inventory at the
  new 37/34 baseline), `cargo check --workspace`, `cargo fmt --check`,
  `cargo clippy --workspace --all-targets`,
  `node scripts/sync-rule-bookkeeping.js` and the quick preflight (locale
  sync, rule counts, canonical agent instructions) all pass; see the PR body
  for exact counts.
