# Release compatibility review - 2026-11-23

Issue #1881 (Antigravity CLI 1.5.15) was triaged against the live
vendor sources, not just the watcher's quoted marker. The Monday
full-sweep release poll (`UPDATE_BASELINES=true bash
scripts/check-tool-releases.sh`) reported `new=1 errors=0` on
2026-11-23; every other tracked tool matched its baseline, and the
1.5.15 marker below was fetched fresh from its live source before
writing.

| Issue | Tool | Claimed | Baseline now | Primary live evidence |
| --- | --- | --- | --- | --- |
| #1881 | Antigravity CLI | 1.5.15 | 1.5.15 | linux_amd64 auto-updater manifest (`version: 1.5.15`), plus the full 1.5.15 release notes fetched live with `agy changelog` (the notes channel the `/changelog` command reads; the local binary at 1.5.14 still fetches the latest published notes) |

## Triage

Baseline-only advance; no rule, locale or vendored-constraint change:

- No settings.json key changes: the release notes state the
  Orchestrator thinking-`Effort` default change leaves saved
  `"orchestrator"` choices in settings.json unchanged, and no other
  key is added, renamed or retyped. `showThinking` and `orchestrator`
  were already covered by the 1.5.14 review.
- `AGY_CLI_AUTO_COMPACT_OVERRIDE` gains token-count, `K`/`M` shorthand
  and percentage forms - an environment variable, not a file agnix
  reads; `/context` capacity display is runtime UI.
- The `agy acp` items (session/load replay, session/prompt turn
  lifetime, `session/cancel` JSON-RPC codes) are editor-protocol
  behavior, not the hooks.json/mcp_config.json/plugin.json file
  contracts agnix validates.
- Local conversation storage no longer prunes beyond the 500 most
  recent - a runtime default, not a settings key.
- The published settings and plugin schemas are byte-identical to the
  vendored bundles (both sha256 values match sources.json exactly:
  settings `0d88365f...`, plugin `0eea5e48...`), so the constraints
  are unchanged; both entries' verified_on refresh to 2026-11-23 for
  the re-verification.
- The remainder is Remote Control browser-view UX, color-scheme
  contrast and rendering fixes - explicitly irrelevant per the
  baseline's changes_of_interest.

## Verification

- `cargo check -p agnix-rules -p agnix-core`: pass
- `cargo test -p agnix-core --test release_triage --test spec_drift_integration`: pass
- `cargo test -p agnix-cli --test rule_parity` (reads the baselines and RESEARCH-TRACKING): pass
- `node scripts/sync-rule-bookkeeping.js --check`: clean
