# Release compatibility review - 2026-11-20

Issues #1852 (Grok Code), #1853 (Kimi Code), #1855 (Antigravity CLI),
#1856 (Claude Code), #1857 (Codex CLI), #1858 (Cursor), #1859 (Gemini
CLI), #1860 (Kiro CLI) and #1861 (OpenCode 1) were triaged against the
live vendor sources, not just the watcher's quoted markers. The
read-only release poll (`UPDATE_BASELINES=true bash
scripts/check-tool-releases.sh`) reported `new=1` for every tool above,
and each marker below was fetched fresh from its live source on
2026-11-20 before writing.

The issue titles lagged the live feeds for eight of the nine tools, so
each baseline advances to the live latest marker, which covers and
exceeds the claimed version. Baseline-only sweep: no rule pins these
versions (the Kiro rules gate on `>= 3.4.0` minimums), so `rules.json`,
`VALIDATION-RULES.md` and the `crates/agnix-rules` mirror are unchanged
and `sync-rule-bookkeeping.js --check` is clean.

| Issue | Tool | Claimed | Baseline now | Primary live evidence |
| --- | --- | --- | --- | --- |
| #1856 | Claude Code | v2.1.328 | v2.1.329 | registry.npmjs.org/@anthropic-ai/claude-code `dist-tags.latest`; github.com/anthropics/claude-code/releases lists both |
| #1857 | Codex CLI | rust-v0.170.0 | rust-v0.171.0 | github.com/openai/codex/releases (latest stable rust- tag; rust-v0.173.0-alpha.1 is a prerelease) |
| #1858 | Cursor | 3.31.10 | 3.31.11 | api2.cursor.sh stable update endpoint (`name` field; latest-only, so 3.31.10 itself is not independently confirmable and is superseded) |
| #1859 | Gemini CLI | v0.69.0 | v0.70.0 | registry.npmjs.org/@google/gemini-cli `dist-tags.latest`; github.com/google-gemini/gemini-cli/tags |
| #1860 | Kiro CLI | 3.5.0 | 3.5.0 | kiro.dev/changelog/cli/ (highest published CLI release; matches the claim) |
| #1855 | Antigravity CLI | 1.5.12 | 1.5.13 | linux_amd64 auto-updater manifest plus its /releases history (1.5.12 present) |
| #1852 | Grok Code | 1.0.88 | 1.0.89 | x.ai/changelog/build RSS (1.0.88 present; feed lists 1.0.89 newest) |
| #1853 | Kimi Code | 2.6.3 | 2.7.1 | registry.npmjs.org/@moonshot-ai/kimi-code `dist-tags.latest`; MoonshotAI/kimi-code releases (no validator yet, watchlist) |
| #1861 | OpenCode 1 | v1.19.6 | v1.19.6 | registry.npmjs.org/opencode-ai (1.19.6 published); check-tool-releases.sh `new=1` (matches the claim) |

The poll also reported a new marker for OpenCode 2 (v2.3.2). No open
issue covers it in this batch, so its baseline stays at v2.3.0 until the
watcher files it.

## Verification

- `cargo check -p agnix-rules -p agnix-core`: pass
- `cargo test -p agnix-core --test release_triage --test spec_drift_integration`: pass
- `cargo test -p agnix-cli --test rule_parity` (reads the baselines and RESEARCH-TRACKING): pass
- `node scripts/sync-rule-bookkeeping.js --check`: clean
- `bash scripts/preflight.sh --quick`: pass; CLAUDE.md and AGENTS.md byte-identical to main
