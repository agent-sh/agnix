# Release compatibility review - 2026-11-06

Review the two open release notices against current main and the live sources,
not the older watcher baseline quoted in each issue. The read-only release poll
reports `new=1` for both filtered tools and `errors=0`. No rule, validator or
schema changes; the catalog remains at 758 rule IDs.

| Issue | Result and primary evidence |
| --- | --- |
| #1798 Cursor 3.29.9 | The [stable update endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) returns `{"name":"3.29.9"}`, matching the issue; it publishes a version marker only. The [changelog](https://cursor.com/changelog) gained one entry after 2026-10-20: 2026-11-05 "Build Live Apps", a hosted-app product feature (cloud-agent app building, Postgres provisioning, SSO, publishing guardrails). None of the validated surfaces (`.cursor/rules/**/*.mdc`, `.cursorrules`, `.cursor/hooks.json`, `.cursor/agents/**/*.md`, `.cursor/environment.json` and its schema, `.cursor/mcp.json`, `.cursor/skills/*/SKILL.md`, `.cursor-plugin/`) is mentioned. Baseline-only advance to 3.29.9. |
| #1799 Kimi Code @moonshot-ai/kimi-code@2.5.0 | The issue's marker is already superseded: the [release feed](https://github.com/MoonshotAI/kimi-code/releases) also published [2.5.1](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai%2Fkimi-code%402.5.1) (2026-11-05) and [2.5.2](https://github.com/MoonshotAI/kimi-code/releases/tag/%40moonshot-ai%2Fkimi-code%402.5.2) (2026-11-06). 2.5.0 adds session forking, multi-directory skill merging (primary directory wins on name conflicts), creation-time SSH/Docker/custom-command environments, `--environment`/`--environment-cwd`, `[workspace] require_trust`, `[database] global_search`/`KIMI_CODE_GLOBAL_SEARCH`, and reworks the server WebSocket (`subscribe_v2`, `agent`/`usage`/`goal`/`todo`/`task` frames) while removing REST task/status endpoints and `[secondary_model] force`. 2.5.1 and 2.5.2 are runtime and web fixes (MCP tool declaration dedupe, HTTP 400 system-message fix, web session/image fixes). Kimi Code has no tool-specific validators (`validation_status: pending`), so agnix has no rule surface to accept or reject any of this; release tracking is the only implemented contract. Baseline advances to the live `@moonshot-ai/kimi-code@2.5.2`, which covers the issue's 2.5.0. |

## Baseline verification

Run `UPDATE_BASELINES=true TOOL_FILTER=cursor` and `TOOL_FILTER=kimi-code`
against `scripts/check-tool-releases.sh`; both emit the advances applied in
`.github/tool-release-baselines.json` (`cursor` 3.28.7 to 3.29.9, `kimi-code`
`@moonshot-ai/kimi-code@2.4.1` to `@moonshot-ai/kimi-code@2.5.2`) with
`errors=0`. A new fixture test in `scripts/tool-support-policy.test.py` pins
both extraction paths: the Cursor update-endpoint JSON marker and the Kimi
release-tag regex that must skip non-CLI tags such as `desktop-v9.0.0`.
