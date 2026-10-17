# Fixtures

This directory contains fixture files used by unit and CLI integration tests.
Keep fixtures minimal, deterministic, and focused on one rule family when possible.

## Conventions
- Use `tests/fixtures/valid` and `tests/fixtures/invalid` for general file-type fixtures.
- Use family-specific directories for rule-family coverage and cross-platform cases.
- Prefer filenames that hint at the rule or scenario (`xml-001-unclosed.md`, `pe-002-cot-on-simple.md`).
- Valid fixtures should avoid triggering diagnostics for their own family.

## Rule Family Coverage
| Family | Directory | Valid example | Invalid example(s) |
| --- | --- | --- | --- |
| AS, CC-SK | `valid/skills/`, `invalid/skills/` | `valid/skills/code-review/SKILL.md` | `invalid/skills/unknown-tool/SKILL.md` |
| CC-HK | `valid/hooks/`, `invalid/hooks/` | `valid/hooks/valid-hooks.json` | `invalid/hooks/` fixtures |
| CC-AG | `valid/agents/`, `invalid/agents/` | `valid/agents/valid-agent.md` | `invalid/agents/missing-name.md` |
| CC-MEM | `valid/memory/`, `invalid/memory/` | `valid/memory/CLAUDE.md` | `invalid/memory/CLAUDE.md` |
| CC-MEM, CC-SET, CC-OS-007 (fixture pack) | `claude-mem/` | `claude-mem/valid/CLAUDE.md`, `claude-mem/settings-valid/.claude/settings.json` | `claude-mem/emphasis/` (CC-MEM-017), `claude-mem/long-file/` (CC-MEM-014), `claude-mem/combined/` (CC-MEM-015), `claude-mem/agent-memory/` (CC-MEM-016), `claude-mem/settings-scope/` (CC-SET-034), `claude-mem/settings-rules/` (CC-SET-035, CC-SET-036); exercised by `crates/agnix-core/tests/claude_memory_rules.rs` |
| CC-PL | `valid/plugins/`, `invalid/plugins/` | `valid/plugins/` fixtures | `invalid/plugins/` fixtures (`invalid/plugins/misplaced-manifest/` is a Claude Code manifest outside `.claude-plugin/`, CC-PL-001) |
| CC-PL-001 on other tools' manifests, inline `mcpServers` (MCP, CC-MCP), COP-019/COP-020/COP-039 on the root and `.claude-plugin/` manifests | `plugin-manifests/` | `plugin-manifests/agent-plugins/plugin.json`, `plugin-manifests/dual-manifest/plugin.json` (no CC-PL), `plugin-manifests/claude-copilot-limits/.claude-plugin/plugin.json` (clean unscoped; COP-039 with `tools = ["copilot"]`) | `plugin-manifests/copilot-legacy/plugin.json` (CC-PL-001 warning), `plugin-manifests/copilot-legacy-invalid/` (legacy root, COP-020, COP-039), `plugin-manifests/agent-plugins-invalid/` (Agent Plugins root, COP-039), `plugin-manifests/agent-plugins-no-schema/` (COP-019 warning), `plugin-manifests/claude-inline-mcp/.claude-plugin/plugin.json`, `plugin-manifests/copilot-inline-mcp/`, `plugin-manifests/cursor-inline-mcp/`; exercised by `crates/agnix-cli/tests/fixture_family_cli.rs` |
| AGM | `agents_md/` | `agents_md/valid/AGENTS.md` | `agents_md/no-headers/AGENTS.md` |
| COP | `copilot/`, `copilot-invalid/` | `copilot/.github/copilot-instructions.md` | `copilot-invalid/.github/copilot-instructions.md` |
| CUR | `cursor/`, `cursor-invalid/`, `cursor-legacy/` | `cursor/.cursor/rules/valid.mdc` | `cursor-invalid/.cursor/rules/empty.mdc` |
| XP | `cross_platform/` | `cross_platform/valid/AGENTS.md` | `cross_platform/hard-coded/AGENTS.md` |
| MCP | `mcp/` | `mcp/valid-tool.mcp.json` | `mcp/invalid-jsonrpc-version.mcp.json` |
| PE | `prompt/` | `prompt/pe-004-valid.md` | `prompt/pe-002-cot-on-simple.md`, `prompt/pe-004-ambiguous.md` (the `pe-001-*` and `pe-003-*` files assert that the retired rules stay silent) |
| REF | `refs/` | `refs/valid-links.md` | `refs/broken-link/CLAUDE.md`, `refs/missing-import.md` |
| XML | `xml/` | `xml/xml-valid.md` | `xml/xml-001-unclosed.md` |
| Real-world | `real-world/` | `real-world/html5-void-elements/CLAUDE.md` | `real-world/absolute-paths/AGENTS.md` |
| Kiro Powers (fixture pack) | `kiro-powers/` | `kiro-powers/.kiro/powers/valid-power/POWER.md` | `kiro-powers/.kiro/powers/missing-frontmatter/POWER.md`, `kiro-powers/.kiro/powers/bad-mcp/mcp.json` |
| Kiro Agents (fixture pack) | `kiro-agents/` | `kiro-agents/.kiro/agents/valid-agent.json` | `kiro-agents/.kiro/agents/invalid-model.json`, `kiro-agents/.kiro/agents/missing-hook-command.json` |
| Kiro Hooks (fixture pack) | `kiro-hooks/` | `kiro-hooks/.kiro/hooks/valid-file-save.kiro.hook` | `kiro-hooks/.kiro/hooks/invalid-event.kiro.hook`, `kiro-hooks/.kiro/hooks/missing-action.kiro.hook` |
| Kiro MCP (fixture pack) | `kiro-mcp/` | `kiro-mcp/.kiro/settings/mcp.json` | `kiro-mcp/.kiro/settings/missing-command-url.json`, `kiro-mcp/.kiro/settings/hardcoded-secrets.json` |
| Kiro detection, workflows, ignore files (fixture pack) | `kiro-detect/` | `kiro-detect/.kiro/settings/cli.json`, `kiro-detect/power/POWER.md`, `kiro-detect/.kiro/workflows/plan-and-implement.workflow.json` | `kiro-detect/.kiro/agents/reviewer.JSON` (KIRO-026), `kiro-detect/.kiro/workflows/broken.workflow.yaml` (KR-WF), `kiro-detect/.kiroignore` (KR-IGN-001) |
| KR-SET | `valid/kiro-settings/`, `invalid/kiro-settings/` | `valid/kiro-settings/.kiro/settings/cli.json` | `invalid/kiro-settings/.kiro/settings/cli.json` |
| Kiro rule reachability (one project per rule) | `kiro-reach/<rule-id>/` | - | `kiro-reach/kr-ag-030/.kiro/agents/review-agent.json` (KR-AG-030, with its orchestrator `lead.json`), `kiro-reach/kr-hk-025/.agnix.toml` (Kiro 2.x pin) |
| GM (Gemini CLI settings, extensions, hooks) | `gemini_settings/`, `gemini_extension/`, `gemini_ignore/`, `gemini_md/`, `gemini_md-invalid/` | `gemini_settings/valid/.gemini/settings.json` | `gemini_settings/invalid/.gemini/settings.json`, `gemini_extension/invalid/gemini-extension.json` |
| Gemini commands, policies, MCP settings and agents (fixture pack) | `gemini_surfaces/` | `gemini_surfaces/valid/` (documented commands, settings, local and remote agents, a skipped `_draft.md`) | `gemini_surfaces/invalid/` (GM-CMD-001..003, GM-011, GM-013..017, GM-020, GM-024, GM-025, GM-027, GM-AG-002..005), `gemini_surfaces/extension/` (extension root: GM-011, GM-014, GM-015, GM-021..023, GM-CMD-002, GM-AG-006) |
| Gemini hooks and extension routing (fixture pack) | `gemini_hooks/` | `gemini_hooks/remember-layout/` (extension in `.gemini/`, as in the remember plugin), `gemini_hooks/extension-root/` (repository that is an extension) | `gemini_hooks/invalid/` (GM-004, GM-008, GM-010, GM-018, GM-019, GM-026, GM-028), `gemini_hooks/installed/.gemini/extensions/audit/` (GM-018, GM-026) |
| Amp checks, settings and AGENTS.md mentions (fixture pack) | `amp-checks/` | `amp-checks/.agents/checks/valid.md`, `amp-checks/valid/` (documented `.amp/settings.jsonc` with comments, AGENTS.md mentioning a file with valid `globs`) | `amp-checks/.agents/checks/` (AMP-001, AMP-002), `amp-checks/.amp/settings.json` (AMP-004, AMP-006, AMP-007), `amp-checks/docs/typescript.md` (AMP-003, mentioned by `amp-checks/AGENTS.md`) |
| Amp skills (fixture pack) | `amp-skills/` | `amp-skills/valid/` (the vendor `mcp.json` examples, `mcpServers` frontmatter, byte-identical `.agents`/`.claude` copies, the docs depth example `something/deep`) | `amp-skills/invalid/` (AMP-005, AMP-006, AMP-008, AMP-010, AMP-SK-002, AMP-SK-003) |
| Windsurf, legacy `.windsurf/` layout (fixture pack) | `windsurf/`, `windsurf-legacy/` | `windsurf/.windsurf/rules/valid-rule.md` | `windsurf/.windsurf/rules/` (WS-001, WS-005, WS-006, WS-007), `windsurf/.windsurf/workflows/empty-workflow.md` (WS-003, WS-007), `windsurf-legacy/.windsurfrules` (WS-004) |
| Devin Desktop `.devin/` layout, global rules, AGENTS.md and Cascade hooks (fixture pack) | `devin/` | `devin/valid/` (the vendor examples, including `globs: **/*.test.ts` and the documented `hooks.json`; Devin CLI's `.devin/config.json` and `.devin/hooks.v1.json`, which are not Cascade files) | `devin/invalid/` (WS-005, WS-006, WS-007, WS-008, WS-009, WS-010, WS-011, WS-HK-001..005) |

## Notes
- AGENTS.md and cross-platform fixtures intentionally overlap; they are validated by different rule families.
- `real-world/` fixtures are regression tests derived from testing against 121 real-world repositories.
- REF-002 only fires on agent config files (CLAUDE.md, AGENTS.md, SKILL.md), so broken-link fixture uses CLAUDE.md.
- Keep fixture paths stable, as tests assert on filenames.
- Kiro powers/agents/hooks/MCP/permissions/detection fixture packs are guarded by inventory and CLI smoke-baseline checks in `crates/agnix-cli/tests/kiro_fixture_inventory.rs`, including detection baselines and representative file-type assertions.
- `crates/agnix-cli/tests/kiro_rule_reachability.rs` runs the CLI over every Kiro fixture family and requires each `KIRO-*` and `KR-*` rule in `knowledge-base/rules.json` to fire at least once. A new Kiro rule that no family triggers needs a minimal project at a real Kiro path under `kiro-reach/<rule-id-lowercase>/`, with an `.agnix.toml` holding only `[tool_versions] kiro = "..."` when the rule needs a pinned version; each such project must produce its own rule.
- `gemini_surfaces/` fixtures are asserted per file through `validate_project` in `crates/agnix-core/tests/gemini_surfaces.rs`. `extension/commands`, `extension/policies` and `extension/agents` are Gemini extension files only next to `extension/gemini-extension.json`.
- `amp-checks/` and `amp-skills/` fixtures are asserted per file (AMP rules only) through `validate_project` in `crates/agnix-core/tests/amp_support.rs`. AMP-003 reports on `amp-checks/docs/typescript.md`, which is validated only because `amp-checks/AGENTS.md` @-mentions it.
- `windsurf/`, `windsurf-legacy/` and `devin/` fixtures are asserted per file (WS rules only) through `validate_project` in `crates/agnix-core/tests/windsurf_support.rs`. Every workflow gets WS-009 (info) by design, so `windsurf/.windsurf/workflows/valid-workflow.md` reports it; WS-007 lands on the first file of each legacy `.windsurf/` directory. `devin/invalid/AGENTS.md` (over 12,000 characters) and `devin/invalid/.codeium/windsurf/memories/global_rules.md` (over 6,000) are long on purpose.
- `gemini_hooks/` fixtures are asserted per file through `validate_project` in `crates/agnix-core/tests/gemini_support.rs`. A `hooks/hooks.json` or `agents/*.md` is a Gemini extension file only next to a `gemini-extension.json` (or under `.gemini/extensions/<name>/`); keep the manifests in place.
- Kiro MCP file-type detection currently only matches `.kiro/settings/mcp.json`; other JSON files in `kiro-mcp/.kiro/settings/` are fixture artifacts and are not detected as `KiroMcp`.
