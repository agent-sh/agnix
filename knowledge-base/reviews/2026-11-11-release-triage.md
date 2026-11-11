# Release compatibility review - 2026-11-11

Review the open notices against current main and the live sources, not the
older watcher baselines quoted in the issues. The read-only release poll
(`UPDATE_BASELINES=true bash scripts/check-tool-releases.sh`) reports `new=1`
for antigravity (1.5.5), grok-code (1.0.77) and pi (v1.5.2) with `errors=0`;
every other tracked tool polls at or below its baseline except the six
no-issue advances triaged below. The spec-drift sentinel hashes were
reproduced locally with the workflow's exact fetch (curl with
`agnix-release-watch/1.0`, the sentinel Accept header and CRLF
normalization) before any baseline was rewritten.

| Issue | Result and primary evidence |
| --- | --- |
| #1817 Pi v1.5.1/v1.5.2 | v1.5.1 (published 2026-11-09 23:47Z) adds `pi auth login/logout` credential commands, idle prompt-cache warming, Anthropic message threads, Mermaid pie charts, Copilot classifiers and package update summaries; its breaking changes are extension API (`GrepOperations.readFile` no longer called for context lines) and wire behavior (OpenAI-compatible `tools: []` omitted, matching vLLM 0.20+). v1.5.2 (2026-11-10 12:25Z) is session fork/import and OAuth fixes only. Schema diff v1.5.0 -> v1.5.2: `models.schema.json` adds the `compat` booleans `requiresNonNullAssistantContent` and `requiresToolsWithToolHistory`, the `promptCacheKeyMode` enum (`auto`/`enabled`/`disabled`, default `auto`) and `supportsMessageThreads` (boolean); `settings.schema.json` changes only the `cacheWarming` default `streaming` -> `idle`. `cacheWarming` is global-only and already on the PI-CFG-003 project-ignore list. The new model fields are constraint-only: `check_schema(Schema::PiModels)` reads the vendored schema, and `model_runtime` (PI-MDL-002) checks provider usability, not model member fields. Re-vendor both constraints from the v1.5.2 tag (`raw main` is byte-identical to the tag for both schemas, verified with `cmp`); the pi-settings constraint is byte-identical to the v1.5.0 one because its only upstream change is a `default`. Repin the 17 Pi rule evidence URLs to v1.5.2. No rule semantics change. |
| #1815 Antigravity 1.5.4/1.5.5 | The installer manifest reports 1.5.5 live (issue #1815 quoted 1.5.4 from the 07:07Z poll); the changelog page is a version marker only. The sentinel's antigravity-settings drift was reproduced and diffed constraint-to-constraint: the live `schemas/v1/settings.json` differs from the 2026-11-04 source only in `description`, `title` and `examples` annotations - the stripped constraint is byte-identical, so AGY-CFG-001..005 and AGY-KB-001 are unaffected and no re-vendor is needed. The customization-discovery page still documents the manifest semantics AGY-RL-004 validates (`entries`/`inherits` objects with required `path` and `include_only`/`exclude` string lists; rules.json entries keep their frontmatter trigger). Baseline-only advance to 1.5.5. |
| #1816 Grok Code 1.0.71-1.0.77 | Changelog RSS triaged item by item. Config-relevant candidates: 1.0.71 adds the hook handler flag `"failClosed": true` and `[ui] single_ctrl_c_quit`; 1.0.73 adds `[ui] multiline_default`; 1.0.75 warns instead of silently skipping permission rules that fail to load; 1.0.77 adds `[ui] kitty_keyboard = "query"`. Handler validation in `rules/grok.rs` checks known fields (type/command/url/timeout/args/env) and does not reject unknown members, and the settings contract is open-world (`additionalProperties: true`, "Unknown settings remain open"), so all of these validate today; the 1.0.75 change is runtime warning behavior, not a config format. Contract `$comment` reconciled through 1.0.77. Baseline-only advance. |
| #1818 spec drift | All four changed sources reviewed above; hashes refreshed. The opencode-rules fetch failure is not temporary: commit `c1aa4e27` (2026-11-10 13:58Z) moved the docs tree from `packages/web/src/content/docs/` to `services/site/src/docs/v1/content/`, and the old URL now returns 404. The file moved byte-identical (6758 bytes, `diff` clean against the dev-branch copy), so the baseline is repointed to the new path with the new content hash; OC-AGM-003 and the shared XP rules are unchanged. |

## No-issue advances (workflow triage, confirmed here)

The 07:07Z tool-release-watch run opened no issue for these, meaning its
triage found no agnix-relevant change; each was spot-checked against primary
sources:

- Claude Code v2.1.319 -> v2.1.322: v2.1.321 adds the `claude-fable-5-2`
  model id, `--settings <file>` on `claude remote-control` (a CLI flag, not a
  settings schema change) and an MCP OAuth secret fix; v2.1.322 adds Vertex
  fast mode. No settings.json key, hook event or frontmatter change.
- Codex CLI rust-v0.167.0 -> rust-v0.167.1 (compare
  `rust-v0.167.0...rust-v0.167.1`): request-catalog plumbing (incremental
  tools for regular Responses, Bedrock restriction). Runtime only.
- Gemini CLI v0.67.0 -> v0.68.0, Kimi Code 2.5.2 -> 2.5.3, OpenCode 2
  v2.2.6 -> v2.2.7: patch releases, no validator surface named by triage.
- Kiro CLI 3.1.0 -> 3.2.0 (2026-11-10 changelog entry): full-screen Mermaid
  viewer (`chat.keybindings.viewDiagram`), sign-in to more MCP servers,
  workflow iteration extension, XDG_CONFIG_HOME/HOME confirmation prompt.
  Runtime/UI; no settings.json field, hook trigger or agent format change.

## Baseline verification

Run `UPDATE_BASELINES=true bash scripts/check-tool-releases.sh` after the
advance: `ok=14 new=0 errors=0` (agentskills-spec polled at its baseline
commit marker). The four sentinel hashes in `.github/spec-baselines.json`
reproduce exactly under the workflow's fetch recipe; the opencode-rules
source fetches again at the new path.

## Review round 2 (revuto findings and Cursor Bugbot)

- Kiro 3.2.0 `chat.keybindings.viewDiagram` (Bugbot): added to the settings
  table (Str, following the other `chat.keybindings.*` entries) and the
  not-loaded list; count tests 143 -> 144; a type test covers a non-string
  value (KR-SET-008). The 3.2.0 changelog quote is recorded in the
  KR-SET-005 refresh notes.
- The five no-issue tools' inventory rows (Claude Code, Codex CLI, Kiro CLI,
  OpenCode, Gemini CLI) now carry the 2026-11-11 review date and advanced
  versions.
- The v1.5.2 qualification is propagated everywhere: sources.json
  keybindings/theme entries repointed to v1.5.2 (both schemas byte-identical
  between v1.5.0 and v1.5.2, hashes unchanged, verified by sha256 of both
  tags), the pi.rs header pin, and the NATIVE-TOOL-CONTRACTS.md Pi row.
- sources.json antigravity-settings: `source_sha256` now records the live
  verbatim fetch (ab765e72...); the constraint is unchanged and the 2026-11-04
  value described a superseded copy of the same annotations-only source.
- spec-baselines.json: `last_checked` advanced to 2026-11-11 on the four
  re-hashed entries and the repointed opencode-rules source.
