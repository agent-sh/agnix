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

## Review findings round (PR #1862)

The baseline sweep above left review findings behind; every claim was
re-verified against a live primary source on 2026-11-20 before writing.

### Grok Code pin reconciliation, 1.0.77 -> 1.0.89

`grok_contract.json` and `NATIVE-TOOL-CONTRACTS.md` still pinned the CLI
changelog reconciliation at 1.0.77. The live
[x.ai/changelog/build RSS](https://x.ai/changelog/build/rss.xml) (fetched
2026-11-20) was triaged item by item for 1.0.78 through 1.0.89:

| Version | Config-surface item | Disposition |
| --- | --- | --- |
| 1.0.78 | `@` chip editing, update-channel/installer fixes, export speed | No static config surface |
| 1.0.79 | `/vim-mode all` "enables full Vim editing inside the agent prompt" | **Contract change**: `keys.vim` enum gains `all` (GRK-CFG-003); `/vim-mode` is the command UI of the `keys.vim` setting reconciled since 1.0.68 |
| 1.0.80 | Rendering/startup fixes only | No static config surface |
| 1.0.81 | UI polish only | No static config surface |
| 1.0.82 | "New `minimal` sandbox profile and `workspace_write` option let custom profiles control launch-folder writes" | **Rule change**: `minimal` joins GRK-SBX-001's reserved built-in names (a custom profile can no longer use it). `workspace_write` is a custom-profile option of undocumented type; the sandbox contract's profile objects stay `additionalProperties: true`, so it validates open-typed until the settings reference documents it |
| 1.0.83 | Welcome-screen and MinGit fixes | No static config surface |
| 1.0.84 | Cosmetics only | No static config surface |
| 1.0.85 | `/skills` behavior, `${ARGUMENTS}` expansion, `grok inspect` counting | Runtime behavior; no static config surface |
| 1.0.86 | Cosmetics only | No static config surface |
| 1.0.87 | UI fixes only | No static config surface |
| 1.0.88 | "MCP servers can now specify a `headers_helper` command to provide dynamic request headers" | **Contract + rule change**: `headers_helper` joins the typed MCP server fields as a string (GRK-MCP-001, grok_contract mcp_servers) |
| 1.0.89 | `/rename`, `/models`, `grok mcp enable/disable` and `grok inspect` behavior | Runtime behavior; no static config surface |

Both pins now read 1.0.89. GRK-CFG-003, GRK-SBX-001 and GRK-MCP-001
evidence repinned to 2026-11-20 (the changelog URL was already in each
rule's sources). The live settings reference (last updated October 13)
and sandbox page (July 21) predate these releases, so the changelog is
the newer primary evidence for all three changes.

### Codex CLI schema reconciliation, rust-v0.169.0 -> rust-v0.171.0

The tagged `codex-rs/core/config.schema.json` was fetched at
[rust-v0.170.0](https://github.com/openai/codex/blob/rust-v0.170.0/codex-rs/core/config.schema.json)
and
[rust-v0.171.0](https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/core/config.schema.json)
and diffed against the 0.169.0 set pinned on 2026-11-15. rust-v0.170.0
adds one `[features]` flag (`multi_environment_mcp`); rust-v0.171.0 adds
eight more (`catalog_apps_instructions`, `file_backed_tool_audio`,
`guardian_nested_approval_context`, `mcp_shared_repl_cwd`,
`per_environment_context`, `restored_history_prewarm`,
`safe_image_decoder`, `utc_environment_date`) and the
`tui.status_line_provider` preference. Nothing is removed and no other
typed surface moved. None of the additions are named in the release
notes (the 2026-11-13 triage pattern). All nine flags join
`KNOWN_FEATURE_KEYS` (main segment stays sorted) and
`status_line_provider` joins `KNOWN_TUI_KEYS`; CDX-CFG-011/CDX-CFG-006
evidence repinned to 2026-11-20 with a TOML/JSON/YAML regression.

### Kiro CLI 3.5.0 (Bugbot claim)

Verified live against [kiro.dev/changelog/cli](https://kiro.dev/changelog/cli/)
and the live MCP configuration and sign-in governance references:

- `auth.portalUrl` **is real** (CLI changelog 3.5.0): "Set
  `auth.portalUrl` and `kiro-cli login` opens that portal in your
  browser instead of the default one." It joins the KR-SET-005
  known-settings list as a string (the settings reference table has not
  caught up yet). `auth_portal_url` in `managed-settings.json` and the
  `AuthPortalUrl` policy are documented in the same sentence; agnix's
  KR-PM validator reads only `rules`/`policies` and Kiro "ignores" extra
  keys in the managed `settings` object by design, so no rule change.
- MCP `oauth.mode` **is real** (MCP configuration reference, "OAuth
  properties"): "How Kiro IDE (1.4.236 or later) or a Kiro CLI V3
  session (3.5.0 or later) gets the server's token: `browser`
  (default), `enterprise`, or `auto`. ... Kiro CLI V2 sessions ignore
  it." Joins KR-MCP-007's typed `oauth.*` fields as a string with a
  V3-only failure. The value set is documented but no source states
  what an unrecognized value does, so no value check.
- The top-level `identityProvider` block (issuer, clientId, name,
  authorizationEndpoint, tokenEndpoint) is read only from the user-level
  `~/.kiro/settings/mcp.json` (a workspace copy is ignored); the
  settings-file mcp.json variant has no unknown-top-level-key rule, so
  no change.

### Claude Code v2.1.328 (Bugbot claim)

Verified against the tagged
[release notes](https://github.com/anthropics/claude-code/releases/tag/v2.1.328)
(published 2026-11-19): "Added `\"onFailure\": \"block\"` for `mcp_tool`
hooks, as for command and HTTP hooks: a hook whose call fails blocks the
action." The live hooks reference still documents `onFailure` only under
"Command hook fields" and "HTTP hook fields" (the "MCP tool hook fields"
table has not caught up yet), the same docs-lag pattern accepted for
CC-PL-017 at v2.1.323. CC-HK-029 now treats `mcp_tool` as a valid
`onFailure` hook type; a pinned Claude Code version below 2.1.328 keeps
the no-effect note with a reason naming the mcp_tool minimum. The
non-blocking-event set (Stop, SubagentStop, TaskCompleted, TeammateIdle)
still applies.

### Verification (review round)

- `cargo check -p agnix-core`: pass
- `cargo test -p agnix-core` scoped: `rules::hooks` (CC-HK-029),
  `rules::kiro_settings`, `rules::kiro_mcp`, `rules::codex`,
  `rules::grok` plus `--test release_triage --test spec_drift_integration`:
  pass
- `cargo test -p agnix-cli --test rule_parity`: pass
- `node scripts/sync-rule-bookkeeping.js` then `--check`: clean
- `bash scripts/preflight.sh --quick`: pass

## Review findings round 2 (PR #1862)

The round-1 sweep above landed three findings whose new checks were not
gated on the pinned CLI version, plus one missing tagged-schema
comparison. Each item was re-verified on 2026-11-20.

### OpenCode 1 v1.19.5...v1.19.6 tagged schema-tree comparison

Mirroring the 2026-10-29 OpenCode comparison, the complete Git trees at
[v1.19.5](https://github.com/anomalyco/opencode/tree/v1.19.5/packages/schema/src)
and
[v1.19.6](https://github.com/anomalyco/opencode/tree/v1.19.6/packages/schema/src)
were fetched live (6,627 and 6,651 blobs, not truncated) and the
`packages/schema/src` subtrees compared blob by blob: the same 64 files,
byte-identical (every blob SHA matches; nothing added or removed). The
remaining tree delta is release infrastructure (`.github/release/**`,
publish workflows), desktop-app updater and i18n bundles, package.json
version bumps, `packages/llm` provider-error internals and
`packages/opencode/src/session/message-v2.ts` (runtime session code, not
a validated config surface). No schema source the validators depend on
moved, so the v1.19.6 advance recorded in the baseline table is
confirmed baseline-only: no rule, contract or docs change.

### Version gates for the round-1 Grok and Kiro additions

- GRK-SBX-001 `minimal` (1.0.82): the reserved name now applies only
  when the pinned grok-code version is >= 1.0.82 (unpinned files are
  checked against the latest release). A config pinned to 1.0.81 with a
  custom `profiles.minimal` no longer errors. Boundary tests on both
  sides (1.0.81/1.0.77 vs 1.0.82/1.0.89, plus the always-reserved
  pre-1.0.82 builtins).
- GRK-MCP-001 `headers_helper` (1.0.88): gated identically. The rule's
  own shape treats unknown server fields as valid (`_ => true`), so a
  wrong-typed `headers_helper` on a pin below 1.0.88 - a CLI that does
  not read the field - was a false positive relative to that semantic.
  A pin below 1.0.88 keeps the unknown-field treatment; >= 1.0.88 and
  unpinned require the documented string. Boundary tests on both sides.
- GRK-CFG-003 `keys.vim` enum value `all` (1.0.79): **no gate needed**.
  Adding an enum member only widens the accepted set, so no pinned-older
  config can newly fail: a 1.0.78-pinned file using `all` now passes
  lint where the old CLI would reject it, which is the lenient direction
  (a missed finding, not a false positive), consistent with how schema
  drift is otherwise handled. Enforcing the narrower pre-1.0.79 enum
  would require version-parameterized contract enums, which the static
  `grok_contract.json` does not support, and no existing rule encodes
  per-version enum sets. Unknown-enum semantics stay
  version-independent.
- KR-MCP-007 `oauth.mode` (3.5.0): the typed check is skipped when the
  Kiro version is pinned below 3.5.0, because the pre-3.5.0 V3 contract
  strips the unknown field (a pinned 3.4.x session never fails on its
  type). Unpinned files and >= 3.5.0 pins keep the check. New
  pinned-older test (3.4.0, 3.4.2, 3.2.0, 2.32.1 pass; 3.5.0 still
  reports the wrong type).

VALIDATION-RULES.md detection text updated for GRK-SBX-001,
GRK-MCP-001 and KR-MCP-007; no rules.json metadata change (the rule
applicability floors are unchanged and `verified_on` is already
2026-11-20 for all three).

### Verification (review round 2)

- `cargo check -p agnix-core`: pass
- `cargo test -p agnix-core --test release_triage`: pass (28 tests,
  including the two new grok boundary tests)
- `cargo test -p agnix-core --lib kiro_mcp` (62) `kiro_settings` (39):
  pass
- `cargo test -p agnix-cli --test rule_parity`: pass
- `node scripts/sync-rule-bookkeeping.js` then `--check`: clean
