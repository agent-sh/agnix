# Tool and spec review - 2026-10-28

Primary sources were fetched on 2026-10-28. This review addresses #1733, #1734,
#1736, #1737 and #1738. Issue bodies guided the review; the vendor sources below
establish the contracts. No rule IDs are added.

## Reported skill defects (#1738)

- AS-015: the nested `node_modules` reproduction exceeds 30 MB even though the
  project walk skips the payload. Count in-scope files only. Respect parent and
  nested `.gitignore`/`.ignore` patterns, ancestor `.ignore` files outside the
  repository, Git global excludes, configured exclusions, symlinks and
  hidden/dependency directories. Like project traversal, do not apply
  `.git/info/exclude`. Regression cases include a visible 32 MB payload as a
  negative control, rooted nested ignores, child negation and isolated global
  Git configuration.
- AS-024: [Using scripts](https://agentskills.io/skill-creation/using-scripts)
  shows both major-only examples and an exact-version reproducibility tip.
  Keep the informational reproducibility advice, but explicitly ask for an exact version rather
  than calling a major-only selector unpinned. Do not silently change the
  package-runner policy shared with MCP checks.
- CC-SK-027: the current [command-name table](https://code.claude.com/docs/en/skills#how-a-skill-gets-its-command-name)
  documents `/frontend:component` for a nested legacy command. The proposed
  flat skill changes it to `/component`; the migration suggestion must say so.
- CC-SK-028: the [error reference](https://code.claude.com/docs/en/errors#no-shell-command-in-a-skill-or-command-was-run)
  scopes refused characters to a command. Inspect recognized inline commands
  and executable fences, not prose, frontmatter or ordinary code examples.
  Use the shared Markdown scanner so indented examples and multiline inline
  code cannot become executable fences. Explicit PowerShell commands also
  refuse U+FEFF and U+180E, as the refreshed error reference documents.
  The original four pipeline regressions failed before the fixes and passed
  afterward; additional cases cover the independent review findings.

## Release reviews

| Issue | Primary evidence | Result |
| --- | --- | --- |
| #1736 | [Claude Code v2.1.311 tagged CHANGELOG](https://github.com/anthropics/claude-code/blob/v2.1.311/CHANGELOG.md), entries v2.1.310 and v2.1.311 | Monitor availability expands to more providers, and runners gain command-line flags. Monitor is already recognized. The entries add no new settings, hook-event or frontmatter schema constraints. Advance the reviewed baseline from v2.1.309 to v2.1.311. |
| #1737 | Complete recursive Git trees at v2.1.4 and v2.2.0, then changed `packages/schema/src/config/experimental.ts` and `config/provider.ts`, plus their `provider.ts` and `model.ts` imports | Only those two config modules have changed blob IDs. New `experimental.media_blob`, open-ended model `type`, and provider/model `settings.cache.ttl` already pass the validator. Add tagged-schema compatibility cases for JSON and JSONC, including an unknown future model type. Advance to v2.2.0. |
| #1734 | [Cursor stable update endpoint](https://api2.cursor.sh/updates/api/update/darwin/cursor/0.0.1/stable), [public changelog](https://cursor.com/changelog), six current configuration references including the environment schema | Live endpoint reports 3.27.22, later than the issue's 3.27.14. It contains `name` and download `url`, not patch-specific notes. The public Agent SDK announcement does not introduce a constraint on files agnix currently validates. Current rules, hooks, subagent, environment and MCP contracts below remain compatible. Record the live marker without claiming a patch-by-patch binary diff. |

The OpenCode trees were not truncated. This matters because the compare API's
file list is capped and the range contains 661 commits. Exact schema links:
[experimental](https://github.com/anomalyco/opencode/blob/v2.2.0/packages/schema/src/config/experimental.ts),
[provider config](https://github.com/anomalyco/opencode/blob/v2.2.0/packages/schema/src/config/provider.ts),
[provider settings](https://github.com/anomalyco/opencode/blob/v2.2.0/packages/schema/src/provider.ts),
[model type/settings](https://github.com/anomalyco/opencode/blob/v2.2.0/packages/schema/src/model.ts).

## Spec drift (#1733)

Earlier vendor document bodies were not retained. This is a current-contract
comparison to validators, not a historical line diff reconstructed from hashes.
The sources moved again after the issue was opened; use the fetched hashes below.

| Sources | Contract reviewed against current code | Action |
| --- | --- | --- |
| Claude hooks | The lifecycle table's 33 events, handler kinds, matcher alternatives and ignored matchers, once scope and handler field shapes match hooks/agent/skill validators. `plugin_notification` is already accepted. | Refresh reviewed hash. No additional hook rule needed. |
| Claude memory | Import syntax/depth, code exclusions, 200-line/25 KB memory guidance, exclusions and memory directory, AGENTS.md fallback and combined mode. | Existing static checks remain applicable; refresh hash. No runtime loading claim. |
| Claude plugins | Manifest field table, component path forms, free-form metadata, dependencies/settings and experimental fields match current allowlists. | Refresh hash. No new manifest fields needed. |
| Claude skills | Frontmatter fields and model syntax, command names, argument substitution, injection syntax and error scope. | Correct CC-SK-027/028 as above, refresh hash. |
| Claude subagents | Required name/description, tool fields, model aliases, modes, autoCompactWindow 100000-1000000, hooks, memory, effort, experimental cache TTL and other table fields. Existing code already covers these bounds and aliases. | Refresh hash. No duplicate range/model rule. |
| MCP index and versioning | Modern per-request metadata, server/discover negotiation and legacy initialization compatibility; existing MCP version checks distinguish both eras. | Refresh hashes; no new version assumption. |
| MCP tools | Input/output schemas, tool names, duplicate names and x-mcp-header constraints already covered by MCP rules. | Refresh hash. No new static constraint. |
| MCP resources and prompts | URIs, resource content and prompt argument/message forms match static validators. Runtime permissions, list notifications and error responses are not established by file linting. | Refresh hashes. |
| MCP transports and security | Client transport/config rules and shared URL/header/secret checks remain applicable. HTTP request metadata, OAuth consent, redirects and authenticated state are runtime contracts outside this review's static changes. | Refresh hashes. No claim to enforce every runtime security obligation. |
| Cursor rules | MDC frontmatter `alwaysApply`, `description`, `globs`, attachment modes and ignore guidance remain compatible with CUR rules. | Refresh hash. |
| Cursor hooks | Positive integer version, command/prompt kinds, string matcher, timeout, nullable loop_limit, failClosed and documented events match current checks. | Refresh hash. |
| Cursor subagents | `name`, `description`, `model`, `readonly`, `is_background` and model parameter syntax remain compatible. | Refresh hash. |
| Cursor environment | Current JSON schema's common/container property union matches CUR-016, including egress, Chrome, testing, image and snapshot fields. Setup guidance is not a new required environment key. | Refresh setup hash. Environment schema checked separately. |
| Cursor MCP | stdio and remote config forms, command/args/env/envFile, URL/header interpolation and type values remain compatible. Existing transport inference remains deliberate; no runtime authorization claim. | Refresh hash. |
| GitHub Copilot repository instructions | Repository-wide and path-specific files, applyTo, code-review/cloud-agent excludeAgent and nearest AGENTS.md semantics remain compatible. | Refresh hash. |

## Reviewed fingerprints

Hashes follow the sentinel: strip trailing newlines as shell substitution does,
normalize CRLF, then SHA-256. Only the 18 reviewed source entries are refreshed.

| Source | SHA-256 |
| --- | --- |
| [mcp-spec](https://modelcontextprotocol.io/specification/2026-07-28/index.md) | `10a982e47c02684d10c24c6a7d9ab19f424eab767ac31e7c1595cc469cfda0b5` |
| [mcp-spec-tools](https://modelcontextprotocol.io/specification/2026-07-28/server/tools.md) | `2e0808ce801b61502f1a227f2bf965e5a2ebbadd9b62fcbfa78a3f0abd362856` |
| [mcp-spec-resources](https://modelcontextprotocol.io/specification/2026-07-28/server/resources.md) | `d910e577dad5f6955c01ef36f4b4b1a47c264002b7668b0e878a4f870254195e` |
| [mcp-spec-prompts](https://modelcontextprotocol.io/specification/2026-07-28/server/prompts.md) | `c7084a0df8d45571c246770ba0be4d5738ee2faa981e6a8c8e3180805827e1fb` |
| [mcp-spec-transports](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports.md) | `e7782ecc90710a0f544a4521dbec9534d9b787da358aef63d98650c14331a1e7` |
| [mcp-spec-security](https://modelcontextprotocol.io/specification/2026-07-28/basic/security_best_practices.md) | `9c9917dcde72102aff5e6cc0354f9433cade906e752aaae81d46e8ad6db584bd` |
| [mcp-spec-versioning](https://modelcontextprotocol.io/specification/2026-07-28/basic/versioning.md) | `a9ae76726d1fe3fe479759ed228f495f016ce621ccd7d16c76845ce2851c9d62` |
| [claude-code-memory](https://code.claude.com/docs/en/memory.md) | `4b5b5e9cb544f0959e0e8f41abdb2b8d75139eb5e31513b2e8b01ea2990e552e` |
| [claude-code-hooks](https://code.claude.com/docs/en/hooks.md) | `4dd52664edb08c9d73e04daa59c0e25a10db17a4a8d9e52525bd1dd5c65980d8` |
| [claude-code-skills](https://code.claude.com/docs/en/skills.md) | `942a10cce0b454dc75b1953db4d431732d8bf3cb76a494abb6b53e7b28b63648` |
| [claude-code-plugins](https://code.claude.com/docs/en/plugins-reference.md) | `429d7f0315da20fade015644ad9756de8ef335c8a7fdb825d55d23d3371f1d9c` |
| [claude-code-subagents](https://code.claude.com/docs/en/sub-agents.md) | `7ee4221bb9e94cb2394ad23bc55180f0bd2ef123dcbf68097d8f6a5d73d3a62b` |
| [cursor-rules](https://cursor.com/docs/rules.md) | `dda30726ed0e9f714e42b4660973d64a87ba4d1f70269f175ab53e6f9cb2f7a4` |
| [cursor-hooks](https://cursor.com/docs/hooks.md) | `d2532ce50b144ec80f8c301654420362ada749784e7eb4869bcacaf10e2cfb1d` |
| [cursor-subagents](https://cursor.com/docs/subagents.md) | `d3ddbe27ff9d9fa35412d648b424450a2ec8229eaa0b8109e861e2622786f84c` |
| [cursor-environment](https://cursor.com/docs/cloud-agent/setup.md) | `fea35174b16030d35dce458cb78e194ecb22275480f8679ed792883db7946103` |
| [cursor-mcp](https://cursor.com/docs/mcp.md) | `de104ff30abcf9ce98fa88336aa374658880a7d1e4e8d5a3ed81106fcd5dd91d` |
| [github-copilot](https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions.md) | `a7f25f9ffa4c37166add506ada0a4925dc31ebf2fcf7752e211f115ee799e0ee` |

## Scope

No vendor runtime was executed. This review establishes static validator
compatibility for the referenced fields and fixes the reproduced diagnostics.
It does not claim exhaustive coverage of vendor runtime semantics, session-wide
budgets, remote server behavior, or transport enforcement.
