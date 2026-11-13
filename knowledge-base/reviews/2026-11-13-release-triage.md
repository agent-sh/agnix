# Release compatibility review - 2026-11-13

Issues #1825 (Antigravity CLI) and #1826 (Codex CLI) were reviewed against
the live vendor sources, not just the watcher's quoted markers. The
read-only release poll (`UPDATE_BASELINES=true bash
scripts/check-tool-releases.sh`) reports `new=6 errors=0`: antigravity
1.5.8, claude-code v2.1.323, codex rust-v0.168.0, cursor 3.30.8,
kimi-code 2.6.1 and kiro 3.3.0. Only the two issued tools are advanced
here; the other four markers stay for their own watcher issues and
triage.

| Issue | Result and primary evidence |
| --- | --- |
| #1826 Codex CLI rust-v0.167.2 (was rust-v0.167.1) | The tagged release notes (published 2026-11-11 22:44Z) name exactly one config change: incremental tool updates are now sent by default on Google Cloud Vertex AI, opting out with the flat-bool `incremental_tools_legacy_gcp = false` under `[features]`. The tagged config.schema.json diff rust-v0.167.1...rust-v0.167.2 confirms the flag (boolean, both feature subschemas) and nothing else. Comparing the full 0.167.2 feature set against the CDX-CFG-011 known-flag set also exposed eight flags added by the rust-v0.167.0/0.167.1 schemas that the 2026-11-11 commit-compare triage never saw because no release notes name them: `apps_early_resource_discovery`, `code_mode_host_output_truncation`, `gcp_setup_wizard`, `incremental_exec_tools`, `incremental_tools_legacy`, `interrupt_on_cancel`, `suppress_model_change_announcements` and `tui_extensions_statusline`. All nine are registered in `KNOWN_FEATURE_KEYS` (each segment stays sorted), with a TOML/JSON/YAML regression in `release_triage.rs` and the CDX-CFG-011 evidence repinned to the rust-v0.167.2 tagged schema in both rules.json copies and VALIDATION-RULES.md. Baseline advances to rust-v0.167.2 only. rust-v0.168.0 (published 2026-11-13 03:54Z) adds nine further flags (`tui_workspace`, `image_generation_animation`, `hosted_search_policy`, `agent_message_board_mcp`, `async_compaction_root_only`, `cli_resume_account_check`, `command_sessions_world_state`, `guardian_denial_reconsideration`, `guardian_include_additional_context`), a `GuardianElasticSectionConfigToml` section and `tui.notification_method = "osc777"`, and removes both `incremental_tools_legacy*` names from the schema - a substantive release that gets its own issue and triage rather than being folded silently into this one. |
| #1825 Antigravity CLI 1.5.6 (was 1.5.5) | The installer manifest is the authoritative version extractor and now reports 1.5.8 (issue #1825 quoted 1.5.6; the live marker moved on, as in #1815). The changelog page remains a version marker only. All four published Antigravity sources were re-fetched with the spec-drift workflow's exact recipe (curl `agnix-release-watch/1.0`, the sentinel Accept header, CRLF normalization) and are byte-identical to the 2026-11-11 baselines: `schemas/v1/settings.json` (ab765e72...), `schemas/v1/plugin.json` (0eea5e48...), `docs/rules.md` (da142a29...) and `docs/customization-discovery.md` (cc50d3e5...). No constraint change, no re-vendor; `last_checked` is refreshed to 2026-11-13 on the four spec sources. Baseline-only advance to 1.5.8. |

## Scope left open

claude-code v2.1.323, cursor 3.30.8, kimi-code 2.6.1, kiro 3.3.0 and
codex rust-v0.168.0 are at or above their baselines and were not triaged
here; the next tool-release-watch run files their issues.

## Verification

- `UPDATE_BASELINES=true bash scripts/check-tool-releases.sh`:
  `ok=8 new=6 skipped=1 errors=0` before the advance. After the
  baseline edits `TOOL_FILTER=antigravity` polls clean at 1.5.8
  (`new=0`); `TOOL_FILTER=codex` still reports rust-v0.168.0 as new,
  which is the release deliberately left for its own issue.
- The four Antigravity sentinel hashes reproduce exactly under the
  workflow's fetch recipe (see above).
- `cargo test -p agnix-core --test release_triage codex_0167` passes and
  `KNOWN_FEATURE_KEYS` stays sorted in all three segments; the
  rules.json/VALIDATION-RULES.md parity and generated-docs checks run
  through `node scripts/sync-rule-bookkeeping.js`.
