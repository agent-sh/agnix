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
| CC-PL | `valid/plugins/`, `invalid/plugins/` | `valid/plugins/` fixtures | `invalid/plugins/` fixtures |
| AGM | `agents_md/` | `agents_md/valid/AGENTS.md` | `agents_md/no-headers/AGENTS.md` |
| COP | `copilot/`, `copilot-invalid/` | `copilot/.github/copilot-instructions.md` | `copilot-invalid/.github/copilot-instructions.md` |
| CUR | `cursor/`, `cursor-invalid/`, `cursor-legacy/` | `cursor/.cursor/rules/valid.mdc` | `cursor-invalid/.cursor/rules/empty.mdc` |
| XP | `cross_platform/` | `cross_platform/valid/AGENTS.md` | `cross_platform/hard-coded/AGENTS.md` |
| MCP | `mcp/` | `mcp/valid-tool.mcp.json` | `mcp/invalid-jsonrpc-version.mcp.json` |
| PE | `prompt/` | `prompt/pe-001-valid.md` | `prompt/pe-001-critical-in-middle.md` |
| REF | `refs/` | `refs/valid-links.md` | `refs/broken-link/CLAUDE.md`, `refs/missing-import.md` |
| XML | `xml/` | `xml/xml-valid.md` | `xml/xml-001-unclosed.md` |
| Real-world | `real-world/` | `real-world/html5-void-elements/CLAUDE.md` | `real-world/absolute-paths/AGENTS.md` |
| Kiro Powers (fixture pack) | `kiro-powers/` | `kiro-powers/valid-power/POWER.md` | `kiro-powers/missing-frontmatter/POWER.md`, `kiro-powers/bad-mcp/mcp.json` |
| Kiro Agents (fixture pack) | `kiro-agents/` | `kiro-agents/.kiro/agents/valid-agent.json` | `kiro-agents/.kiro/agents/invalid-model.json`, `kiro-agents/.kiro/agents/missing-hook-command.json` |
| Kiro Hooks (fixture pack) | `kiro-hooks/` | `kiro-hooks/.kiro/hooks/valid-file-save.kiro.hook` | `kiro-hooks/.kiro/hooks/invalid-event.kiro.hook`, `kiro-hooks/.kiro/hooks/missing-action.kiro.hook` |
| Kiro MCP (fixture pack) | `kiro-mcp/` | `kiro-mcp/.kiro/settings/valid-local-mcp.json` | `kiro-mcp/.kiro/settings/missing-command-url.json`, `kiro-mcp/.kiro/settings/hardcoded-secrets.json` |

## Notes
- AGENTS.md and cross-platform fixtures intentionally overlap; they are validated by different rule families.
- `real-world/` fixtures are regression tests derived from testing against 121 real-world repositories.
- REF-002 only fires on agent config files (CLAUDE.md, AGENTS.md, SKILL.md), so broken-link fixture uses CLAUDE.md.
- Keep fixture paths stable, as tests assert on filenames.
- Kiro powers/agents/hooks/MCP fixture packs are guarded by inventory and CLI smoke-baseline checks in `crates/agnix-cli/tests/kiro_fixture_inventory.rs` (including explicit unsupported-path baselines for `.kiro/*` families).
