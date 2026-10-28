# All supported tool release review - 2026-10-29

This review covers all twelve entries in `.github/tool-release-baselines.json`:
ten supported tools, the second OpenCode release line, and the Agent Skills
specification. Issue #1740 tracks the work. Roo Code remains retired; current
main removed its rules and tracking in 0.59.0 after the upstream shutdown.

## Reviewed markers

| Entry | Previous marker | Reviewed marker | Primary comparison and result |
| --- | --- | --- | --- |
| Claude Code | v2.1.311 | v2.1.312 | [Tagged CHANGELOG](https://github.com/anthropics/claude-code/blob/v2.1.312/CHANGELOG.md). `oidc.user_id_claim` belongs to the apps gateway, not settings.json. Hook concurrency and subagent compaction changes are session/runtime behavior. No new per-file validation constraints. |
| Codex CLI | rust-v0.164.0 | rust-v0.165.0 | Compare the complete [old](https://github.com/openai/codex/blob/rust-v0.164.0/codex-rs/core/config.schema.json) and [new](https://github.com/openai/codex/blob/rust-v0.165.0/codex-rs/core/config.schema.json) JSON schemas. Add four feature names below. `auto_review.timeout_ms` is a nested field under an already accepted table. `model_provider = "gcp"` is already accepted as a string; the project-scope warning remains valid. |
| OpenCode 1 | v1.19.4 | v1.19.5 | [Release](https://github.com/anomalyco/opencode/releases/tag/v1.19.5) and complete recursive Git trees. Config and skill-discovery changes remove unused imports/exports, not field schemas or discovery behavior. The shared V1 schema tree has no changed blobs. No new validation constraints. |
| OpenCode 2 | v2.2.0 | v2.2.0 | [Release](https://github.com/anomalyco/opencode/releases/tag/v2.2.0) unchanged. Tagged config/model/provider schema review and compatibility regressions were merged in #1739. |
| Kiro CLI | 2.33.1 | 2.34.2 | [2.34 release and patches](https://kiro.dev/changelog/cli/2-34/), [settings reference](https://kiro.dev/docs/reference/settings/) and workflow authoring. Add documented `chat.workflowLaunch` string choices. MCP elicitation, plan revision, memory deletion and patch fixes introduce no additional recipe fields. |
| GitHub Copilot | v0.43.0 | v0.43.0 | [Latest stable GitHub release](https://github.com/microsoft/vscode-copilot-chat/releases/tag/v0.43.0) unchanged. This entry deliberately tracks stable releases, not timestamped CI tags. Current repository-instruction contracts were reviewed in #1739; no claim of nightly build parity. |
| Cline | v4.1.25 | v4.1.26 | [Release](https://github.com/cline/cline/releases/tag/v4.1.26), complete recursive trees, and tagged rule/skill frontmatter, toggles and MCP changes. UI toggles now persist `disabled` into the document, which CLN rules already accept. The MCP `_meta` addition is a runtime tool-result field, not a server config field. Add compatibility coverage; no new validation constraints. |
| Cursor | 3.27.22 | 3.27.22 | [Stable endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) unchanged. The endpoint supplies a marker and URL, not patch notes. Current config references were reviewed in #1739. |
| Amp | self-hosted-orbs | personal-self-hosted-orbs | [News feed](https://ampcode.com/news.rss) includes Android and personal self-hosted orbs since the baseline. Both concern app/account behavior. The current [settings schema](https://ampcode.com/cli-settings.schema.json) still has the same 21 `amp.` properties and `$schema`, all represented by `amp_settings.rs`; no new validation constraints. |
| Gemini CLI | v0.65.0 | v0.66.0 | Compare tagged settings schemas and [skillLoader.ts](https://github.com/google-gemini/gemini-cli/blob/v0.66.0/packages/core/src/skills/skillLoader.ts). The only settings schema addition is nested `oauth.allowPrivateNetwork`, already accepted. Fix BOM plus trailing delimiter whitespace in Gemini skills, and identify Gemini ownership before applying skill rules. |
| Windsurf / Devin Desktop | Wave 14 | v3.11.228 | [Canonical changelog](https://docs.devin.ai/desktop/changelog.md), current rules, workflow, hook and CLI-rule references. `Wave 14` points to January 30's v1.9544.24 and no longer identifies the current editor. Existing WS checks already cover preferred `.devin/` locations, legacy fallback, activation modes, 12,000/6,000-character limits and Cascade-only workflows. No new validation constraints. |
| Agent Skills spec | 217be548739f | 217be548739f | [Latest commit touching specification.mdx](https://github.com/agentskills/agentskills/commit/217be548739f21d6008915c29aefe320ea1a90af) unchanged. No new spec revision. |

## Validator changes

- Codex 0.165 adds `guardian_recorded_items`, `guardianv3`,
  `prediction_preserve_parent_tools` and `tui_multi_backend` to both root and
  profile feature schemas. CDX-CFG-011 rejected the names. Accept them through
  the shared allowlist used by TOML and JSON/YAML. Regression cases cover all
  three file formats and the structured Guardian enabled form.
- Kiro's current settings reference documents `chat.workflowLaunch` as a
  string with `when-asked`, `large-tasks` and `delegate-all`. KR-SET-005
  incorrectly reported it as ignored. Add it to the setting table and let
  KR-SET-008/009 retain type and enum checks. Do not invent a global-only
  restriction: the reference does not state one. The pre-existing 140-key
  runtime inventory now has this additional documented setting.
- Gemini 0.66's loader accepts a BOM and spaces/tabs following both skill
  delimiters. The old splitter kept the opening tab in the YAML text and
  emitted AS-016. Add a Gemini-specific delimiter parser with original byte
  offsets. Generic/other-client parsing stays unchanged. Resolve `.gemini`
  skills and explicit `tools = ["gemini-cli"]` shared skills as Gemini, so
  Claude-only checks no longer apply to them. This does not reproduce every
  permissive fallback in Gemini's loader or introduce non-UTF-8 input support.

## Release detection and failure handling

Kiro's rendered page lists 2.34.0 before its newer 2.34.2 patch. Select versions
from same-site CLI release and patch links, comparing numeric version tuples.
Ignore IDE links, unrelated prose/framework versions and links to other hosts.

Devin's current changelog uses stable `vX.Y.Z` headings. Read its canonical
Markdown body while keeping the human-facing URL in issue metadata. Select
release headings, not marketing waves or version numbers in prose.

Source fetch/extraction errors now make the watcher fail. An incomplete poll
does not emit a partial baseline-update snippet, including when HTTP failure
leaves a body containing an otherwise valid marker. Dry-run fixtures forbid
GitHub writes. These regressions run in full local preflight and Linux CI.

After review, the live read-only watcher reported
`ok=12 new=0 skipped=0 errors=0`. No LLM triage or GitHub issue mutation was used
for that verification.

## Source completeness and limits

The recursive Git trees were not truncated. Comparing blob IDs matters because
the Cline tag range contains 486 changed files, beyond the compare API's cap.
The reviewed OpenCode source wrappers changed, while the shared V1 schema
blobs did not. The Gemini settings schema comparison includes both ordinary
and required MCP server definitions.

Tree IDs, not commit IDs:

| Range | Old tree | New tree |
| --- | --- | --- |
| OpenCode 1 | `148842ef7cb1bc34192b3fff57d0f9721ab9c801` | `810c0eba1a7348036f8bdf61e6632d0c36c6876c` |
| Cline | `01b8a9724da905423e636396d7fb4acdf47796bb` | `a459f1d04dea030833b558be645011ab739c463b` |
| Gemini | `6006c14a4decfe4b891c6263ad6ad5992e404b5b` | `970413207732f5d4d8c50540c7fc359c44fdaaf2` |

Earlier proprietary doc bodies were not retained. Kiro, Amp and Devin checks
are current-contract comparisons, not reconstructed historical schema diffs.
This review does not claim every vendor runtime behavior is statically
enforced. Gateway OIDC configuration, hook scheduling, authentication,
private-network OAuth enforcement and account-level orb management remain
outside these per-file changes. No vendor runtime was installed or executed.
