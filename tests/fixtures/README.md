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
| CC-PL-001 on other tools' manifests, inline `mcpServers` (MCP, CC-MCP) | `plugin-manifests/` | `plugin-manifests/agent-plugins/plugin.json`, `plugin-manifests/dual-manifest/plugin.json` (no CC-PL) | `plugin-manifests/copilot-legacy/plugin.json` (CC-PL-001 warning), `plugin-manifests/claude-inline-mcp/.claude-plugin/plugin.json`, `plugin-manifests/copilot-inline-mcp/`, `plugin-manifests/cursor-inline-mcp/`; exercised by `crates/agnix-cli/tests/fixture_family_cli.rs` |
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

## Notes
- AGENTS.md and cross-platform fixtures intentionally overlap; they are validated by different rule families.
- `real-world/` fixtures are regression tests derived from testing against 121 real-world repositories.
- REF-002 only fires on agent config files (CLAUDE.md, AGENTS.md, SKILL.md), so broken-link fixture uses CLAUDE.md.
- Keep fixture paths stable, as tests assert on filenames.
- Kiro powers/agents/hooks/MCP/permissions/detection fixture packs are guarded by inventory and CLI smoke-baseline checks in `crates/agnix-cli/tests/kiro_fixture_inventory.rs`, including detection baselines and representative file-type assertions.
- `crates/agnix-cli/tests/kiro_rule_reachability.rs` runs the CLI over every Kiro fixture family and requires each `KIRO-*` and `KR-*` rule in `knowledge-base/rules.json` to fire at least once. A new Kiro rule that no family triggers needs a minimal project at a real Kiro path under `kiro-reach/<rule-id-lowercase>/`, with an `.agnix.toml` holding only `[tool_versions] kiro = "..."` when the rule needs a pinned version; each such project must produce its own rule.
- Kiro MCP file-type detection currently only matches `.kiro/settings/mcp.json`; other JSON files in `kiro-mcp/.kiro/settings/` are fixture artifacts and are not detected as `KiroMcp`.
