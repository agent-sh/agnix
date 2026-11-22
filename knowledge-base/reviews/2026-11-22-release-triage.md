# Release compatibility review - 2026-11-22

Issues #1873 (Claude Code), #1874 (Cursor), #1875 (Kimi Code) and
#1876 (Pi), the spec-drift alert #1878, and three unfiled advances the
read-only release poll surfaced (Antigravity CLI, Cline, OpenCode 2)
were triaged against the live vendor sources, not just the watcher's
quoted markers. `UPDATE_BASELINES=true bash scripts/check-tool-releases.sh`
reported `new=7 errors=0` on 2026-11-22; each marker below was fetched
fresh from its live source before writing.

| Tool | Claimed | Baseline now | Primary live evidence | Verdict |
| --- | --- | --- | --- | --- |
| Claude Code | v2.1.330 | v2.1.330 | registry.npmjs.org/@anthropic-ai/claude-code `dist-tags.latest` (published 2026-11-20); npm tarball 2.1.329 vs 2.1.330 differs only in `package.json` and `sdk-tools.d.ts` (scheduled-task cron expiry 7 -> 30 days doc text, new `maxAgeMs` SDK field); `cli.js` byte-identical | Baseline-only: no config surface change |
| Cursor | 3.31.15 | 3.31.15 | api2.cursor.sh stable update endpoint (`name`, latest-only); cursor.com/changelog's newest entry predates the bump and publishes no per-patch notes; tier D, monthly docs cadence unchanged (spec-baselines cursor sources did not drift) | Baseline-only |
| Kimi Code | 2.7.2 | 2.7.2 | registry.npmjs.org/@moonshot-ai/kimi-code `dist-tags.latest`; dist/main.mjs quoted-key set diff 2.7.1 vs 2.7.2: zero added, zero removed (web assets are rebuilt hashes) | Baseline-only: no validator yet |
| Pi | v1.6.0 | v1.6.0 | registry.npmjs.org/@earendil-works/pi-coding-agent; bundled CHANGELOG (models/providers/extension-API release); schemas at tag v1.6.0 vs v1.5.2: models.schema.json description-only, settings/keybindings/theme byte-identical; first review note below | Baseline-only for rules; vendored constraints refreshed |
| Antigravity CLI | 1.5.14 | 1.5.14 | `agy changelog` (settings.json gains `showThinking` and `orchestrator` keys; Auto mode non-workspace read semantics; Windows UNC path security fix); live settings schema vs 1.5.13-era bundle: keyset identical, added `deprecated` markers and removed `title`; root `additionalProperties: true` so unknown keys stay tolerated | Baseline-only for rules; vendored constraints refreshed |
| Cline | v4.1.27 | v4.1.27 | github.com/cline/cline releases v4.1.27 (LM Studio API key field, MCP read-only badge, provider default-model change; no `.clinerules`/hooks surface change) | Baseline-only |
| OpenCode 2 | v2.3.2 | v2.3.2 | github.com/anomalyco/opencode releases v2.3.1/v2.3.2; tagged schema compare v2.3.0 vs v2.3.2: `config/compaction.ts` adds stable boolean `compaction.background` (default true) and `config/flag.ts` removes the `background_compaction` experimental flag (`experimental.*` keys are not allowlist-validated by agnix, and OC-CFG-009 checks only `keep`/`reserved`/`buffer`/`keep.tokens` types, so no rule change is forced); `all_flags` description semantics narrowed to non-opt-in flags | Baseline-only; `compaction.background` accepted as a known boolean |

## Pi vendored constraints (v1.6.0 pin)

`packages/coding-agent/schemas` at tag v1.6.0: `keybindings.schema.json`
and `theme.schema.json` are byte-identical to v1.5.2, `models.schema.json`
changes two description strings only, and `settings.schema.json` is
byte-identical to v1.5.2. The vendored `pi-settings.json` bundle was
however missing four `deprecated: true` markers that the v1.5.2 source
already carried (`queueMode`, `retry.maxDelayMs`, the object form of
`skills`, `websockets`), so the bundle is re-derived from the live
v1.6.0 source and `sources.json` pins advance to the v1.6.0 tag URLs.
`jsonschema` validation treats `deprecated` as an annotation, so no
diagnostic behavior changes; the markers are now tracked for a future
deprecation rule.

## Antigravity vendored constraints

The published settings schema (fetched 2026-11-22) adds
`deprecated: true` to `enableTelemetry`, `hooks`, the boolean branch of
`statusLine` and `customModelsConfig[].modelId`, and removes the
`title` setting and its `$defs/title` entirely. Root
`additionalProperties: true` means a `title` key in user settings stays
valid (ignored-by-CLI), matching the new schema description "The CLI
keeps keys it doesn't recognize". The bundle is re-derived
(constraint-level diff is exactly those deltas) and `sources.json`
hashes/verified_on refresh; the plugin schema is byte-identical
(`0eea5e48...`).

## Spec drift (#1878)

All twelve changed sources were fetched live under the sentinel
recipe; the five Claude Code hashes reproduce the issue values
exactly. Full current-contract-to-validator comparison (per source):

- claude-code-hooks: all 19 CC-HK claims verified; the 33-event table,
  prompt/agent event matrices and SessionEnd budget text are intact.
- claude-code-memory: every CC-MEM/CC-SET-040 quote present verbatim;
  the page no longer mentions `.claude/agent-memory/` (it lives on the
  sub-agents page, still present there).
- claude-code-plugins: strict-object and reserved-name quotes intact;
  the page no longer marks description/version "recommended", so
  CC-PL-004's requirement text now records that warning as an agnix
  heuristic.
- claude-code-skills: the 20 documented frontmatter fields are
  set-equal to `KNOWN_SKILL_FRONTMATTER_FIELDS`; the 1,536-character
  budget is already AS-008/AS-020.
- claude-code-subagents: every quote verified except one real drift -
  `autoCompactWindow` is now "a token count of `100000` or more" with
  no upper bound, while CC-AG-019 and the validator rejected values
  above 1,000,000. Fixed: the validator accepts any integer >= 100000,
  the rule text, locales and tests follow; unit tests now cover
  1,000,001 and 5,000,000 as accepted.
- mcp-spec (7 pages): revision stays 2026-07-28; the root page was
  slimmed to an overview and three pages restructured (transports
  overview, security page moved to
  `/docs/2026-07-28/tutorials/security/security_best_practices`, new
  `local-server-security` tutorial), optional `title`/`icons` display
  fields were added to Tool/Prompt/Resource definitions, and no
  `*.mcp.json` client-config contract changed. MCP-018's quotes moved
  verbatim to the local-server-security tutorial (its `source_urls`
  already list it), MCP-012 already cites the `deprecated` registry,
  and MCP-021's wildcard-bind SHOULD is still verbatim on the
  streamable-http page it cites. All seven hashes refresh, and the
  mcp-spec-security baseline entry now points at the canonical tutorial
  URL (hash identical via redirect).

`rules.json` `verified_on` advances to 2026-11-22 for CC-AG-019,
MCP-012, MCP-018, MCP-021 and CC-PL-004. MCP spec revision in
`SpecRevisions` is unchanged (2026-07-28 is still latest).
