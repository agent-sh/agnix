# Tool and spec review - 2026-10-22

Primary sources fetched on 2026-10-22 UTC. Issue bodies, including automated AI
triage, are leads rather than schema evidence. This review adds no rule IDs.

## Release issues

| Issue | Baseline and primary comparison | Result |
| --- | --- | --- |
| #1696 | Claude Code v2.1.299 to v2.1.304: [tagged CHANGELOG](https://github.com/anthropics/claude-code/blob/v2.1.304/CHANGELOG.md), entries 2.1.300 through 2.1.304 | No new per-file schema constraints. Mod `tool.check`, `settings.read`, drawing and notification behavior are runtime APIs, not settings hook event names. `CLAUDE_CODE_AUTO_MODE_MODEL` and watchdog/window variables are environment variables, not settings keys. Existing `idleCompaction` validation remains correct. Advance to v2.1.304. |
| #1697 | OpenCode v2.0.32 to v2.1.0: [tag comparison](https://github.com/anomalyco/opencode/compare/v2.0.32...v2.1.0), recursive Git trees and tagged `packages/schema/src/config*.ts` | The only changed config-schema module is [`config/experimental.ts`](https://github.com/anomalyco/opencode/blob/v2.1.0/packages/schema/src/config/experimental.ts): optional boolean `todo`, default false. Root, agent, command, MCP, permission/policy and other config schemas have identical blob IDs. The open experimental object already accepts the new key. Add a compatibility test, advance to v2.1.0; no new constraints. |
| #1698 | Cursor baseline 3.25.9; issue marker 3.26.13; [stable update endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable) now reports 3.27.6. [Public changelog](https://cursor.com/changelog) and current references below | The endpoint contains only `url` and `name`, not patch notes. Latest public entry is October 16 (cloud canvases, Environments API and larger environments), already before the previous review. No newly documented validation constraints found. Rules and environment-schema sentinel hashes are unchanged. Current hook options, subagent fields, environment setup and MCP configuration remain compatible. Add a hook compatibility test and record the live marker 3.27.6, not a claim of a binary/schema diff for each intervening patch. |
| #1699 | [Kiro CLI changelog](https://kiro.dev/changelog/cli/), 2.31.1 to 2.32.0; [workflow authoring](https://kiro.dev/docs/workflows/authoring.md) | "A step's tool access comes from its agent definition, including its `tools` and `excludedTools` settings." Bundled `wf-planner`, `wf-coder`, `wf-design` can use installed Powers; `wf-design` can use configured MCP servers. Reject the suggested KR-WF-010: no new recipe access field or static server-resolution requirement is documented. Existing agent and workflow validators accept these forms. Test the three agent selections and advance to 2.32.0. |

OpenCode tree identities: v2.0.32 `843a6c514e606b436a7fe713422ace0a93ad3d61`,
v2.1.0 `60c68d145a14127ce633e26aa5fdfb882a7baa6d` (Git tree IDs, not commit IDs).
The compare API returns at most 300 changed files here, so recursive tree/blob
comparison, not that truncated file list, establishes the config-schema delta.
Skill scanning now avoids ignored/hidden directories and limits traversal to six
levels, stopping below an encountered SKILL.md. This is discovery/runtime scope;
this patch does not add a per-file skill-depth rule.

Cursor references reviewed: [rules](https://cursor.com/docs/rules.md),
[hooks](https://cursor.com/docs/hooks.md),
[subagents](https://cursor.com/docs/subagents.md),
[environment setup](https://cursor.com/docs/cloud-agent/setup.md),
[environment schema](https://cursor.com/schemas/environment.schema.json), and
[MCP](https://cursor.com/docs/mcp.md). In particular, the hook contract still has
integer `version`, string `matcher`, positive `timeout`, nullable `loop_limit`,
and boolean `failClosed`; subagents retain `name`, `description`, `model`,
`readonly`, and `is_background`. The environment JSON schema is unchanged from
the recorded baseline. These are current-contract comparisons, not reconstructed
historical vendor documentation diffs.

## Spec issue #1691

Previous vendor document bodies were not retained in the repository. Accordingly,
this is a current-source-to-validator comparison, not a claimed line-by-line
diff against the old hash or the issue's already superseded hash.

| Source | Contract reviewed against implementation | Action |
| --- | --- | --- |
| [Hooks](https://code.claude.com/docs/en/hooks.md) | Lifecycle event and matcher tables, handler types/fields, `once` scope, default timeouts and `onFailure` applicability | Add `plugin_notification` to CC-HK-025. Both the matcher overview and Notification table document it: "A mod raises a notification with `$.ui.notify`." Existing code covered all other listed Notification values but warned on this exact value. Test the single value, exact alternatives, and a misspelled negative control without autofix. Other reviewed contracts already match existing checks. |
| [Memory](https://code.claude.com/docs/en/memory.md) | Relative/absolute imports, four-hop depth, code-span/fence exclusion, 200-line guidance, MEMORY.md first 200 lines or 25KB, absolute exclusion patterns and memory directory, AGENTS.md loading | Existing rules remain applicable. `cc-plugin-agents-md@builtin` and the legacy ID are already accepted by the open plugin configuration. Refresh only this reviewed source hash. No new static load-order rule. |
| [Plugins](https://code.claude.com/docs/en/plugins-reference.md) | Manifest field table and component forms, path containment, opaque version string, dependencies and strict objects | Current keys (`types`, `workflows`, `experimental`, listing metadata, etc.) already occur in CC-PL-017's allowlist. Existing path/strict-object checks cover their declared shapes. Add a current-field compatibility test; refresh hash, no new fields or rule IDs. |
| [Skills](https://code.claude.com/docs/en/skills.md) | Frontmatter table, model values, free-form metadata, named/positional argument substitution, injected commands, hook lifetime and budgets | Current frontmatter fields already accepted. Preserve the broader skill model syntax rather than importing the narrower subagent alias set. The new hook budgets are source/session-wide, not one skill's schema; do not fabricate a per-file limit rule. Refresh hash. |
| [Subagents](https://code.claude.com/docs/en/sub-agents.md) | Frontmatter table: name, tools/disallowedTools, model, permission mode, maxTurns, autoCompactWindow, skills, mcpServers, hooks, memory, background, omitClaudeMd, effort, isolation, color, initialPrompt and experimental | Model alias correction already shipped in #1692: `sonnet`, `opus`, `haiku`, `fable`, full model IDs and `inherit`, separate from skills. The other documented fields are already recognized and checked. Refresh hash; no duplicate alias fix. |

### Reviewed spec fingerprints

These are the actual fetched bodies, using the sentinel algorithm: strip trailing
newlines as shell command substitution does, normalize only CR at line endings
with `sed 's/\r$//'`, then SHA-256. No whitespace collapsing or issue-body hashes.

| Source ID | Reviewed SHA-256 |
| --- | --- |
| claude-code-hooks | `d8a5f4bcd12107d3045ea7ceeb84dd35704b356063d2322f8b5564d743c524b9` |
| claude-code-memory | `6dde753aa5ee3890d12ef290b4ce5341776a99a4c7f0f682ab28b7e0f9ff03a5` |
| claude-code-plugins | `84d600cf53b13538827c2b162802d7d0bf54c4d874c01ae4e2b12fcfe32943d9` |
| claude-code-skills | `24761a9f1e2c06fcfa92220ea58a10907c20ab2303944b39849203662517f004` |
| claude-code-subagents | `8bf34515042be7bf00009c7234e0c21771e31f659dc5e6c8961812d10738caa2` |

## Verification and scope

`oct22_tool_spec_triage` initially failed on the real CC-HK-025 diagnostic for
`plugin_notification`; OpenCode and Kiro compatibility cases already passed.
The matcher addition fixes that failure while retaining the negative control.
Tests run through `validate_content` with the real file detector and registry.
No autofix is introduced.

Not claimed complete: runtime enforcement of Claude's 1,024-hook / 16,384-matcher-
character budget shared by all skills of one source, 12-at-once prompt/agent hook
scheduling, or 64-name/64-character argument substitution caps. These require
separate scoped designs if lint coverage is desired; a baseline review is not a
claim that agnix validates every upstream feature. No runtime tool execution or
cross-account server availability was tested. Kiro 2.33.0 (already visible on the
live changelog) introduces workspace trust and is outside this 2.32.0 review;
its release baseline is deliberately not consumed here. No issues are closed,
no PR is merged, and no release is made by this change.
