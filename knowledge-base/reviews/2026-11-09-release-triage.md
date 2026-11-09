# Release compatibility review - 2026-11-09

Review the open release notice against current main and the live sources, not
the older watcher baseline quoted in the issue. The read-only release poll
reports `new=1` (cursor) and `errors=0`. No rule, validator or schema change;
no new extraction path. Issue #1811 was created by the poll script under the
shared owner login during the 2026-11-09 sweep run (the tool-release-watch
workflow had polled 3.29.9 at 07:09 UTC, minutes before Cursor published
3.29.15); the issue text itself is the standard automated tracker.

| Issue | Result and primary evidence |
| --- | --- |
| #1811 Cursor 3.29.15 | The [stable update endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) returns `{"name":"3.29.15"}`, matching the issue; it publishes a version marker only. The [changelog](https://cursor.com/changelog) entries published after the already-reviewed 2026-11-05 Build Live Apps release are: voice chat with agents (IDE/iOS calling UI on Grok Voice), the Agent SDK (a separate `@cursor/agent-sdk` npm package with its own folder-of-code project layout), cloud-agent canvases and environment improvements (canvases from cloud agents, a beta Environments API, larger faster environments) and the Origin app marketplace (hosted app integrations). Each is a hosted product feature or an out-of-repo SDK; none of the validated surfaces (`.cursor/rules/**/*.mdc`, `.cursorrules`, `.cursor/hooks.json`, `.cursor/agents/**/*.md`, `.cursor/environment.json` and its schema, `.cursor/mcp.json`, `.cursor/skills/*/SKILL.md`, `.cursor-plugin/`, `.cursorignore`) is mentioned. Baseline-only advance to 3.29.15. |

## Baseline verification

Run `UPDATE_BASELINES=true TOOL_FILTER=cursor` against
`scripts/check-tool-releases.sh`: it reports `new=1` with the advance applied
in `.github/tool-release-baselines.json` (`cursor` 3.29.9 to 3.29.15) and
`errors=0`. The existing fixture test in `scripts/tool-support-policy.test.py`
already pins the update-endpoint extraction path; no new path exists, so no
new test is required.
