# Research Tracking

> Master document for tracking AI tool ecosystem changes, research updates, and community feedback.

**Last Updated**: 2026-08-26
**Review Cadence**: Monthly (1st week of each month)
**Related**: [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) | [VALIDATION-RULES.md](./VALIDATION-RULES.md) | [INDEX.md](./INDEX.md)

---

## Tool Inventory

Tools are organized by support tier (see [../CONTRIBUTING.md#tool-tier-system](../CONTRIBUTING.md#tool-tier-system) for definitions). Each entry tracks the config format, documentation source, monitoring approach, review frequency, and rule coverage.

### S Tier (test always)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Claude Code | `CLAUDE.md`, `.claude/settings.json`, `.claude/settings.local.json`, `.claude/managed-settings.json`, `.claude/agents/*.md`, `.claude/skills/*/SKILL.md`, `.claude/hooks configuration`, `.mcp.json`, `.claude-plugin/plugin.json`, `.claude/rules/**/*.md`, `.claude/output-styles/*.md` | https://code.claude.com/docs/en | Automated (spec-drift.yml + tool-release-watch.yml) | Weekly | 2026-08-25 | CC-SK, CC-HK, CC-MEM, CC-AG, CC-PL, CC-SET, CC-OS, MCP |
| Codex CLI | `AGENTS.md`, `.codex/config.toml` (also `.codex/config.json`/`.yaml`), `.codex/skills/*/SKILL.md`, `.codex-plugin/plugin.json`, Agent Plugins root `plugin.json` | https://developers.openai.com/codex/ | Automated (spec-drift.yml + tool-release-watch.yml) | Weekly | 2026-08-25 | AGM, XP, CDX, CDX-AG, CDX-APP, CDX-CFG, CDX-PL, CDX-REQ, CX-SK |
| OpenCode | `AGENTS.md`, `opencode.json`, `opencode.jsonc`, `.opencode/config.json`, `.opencode/skills/*/SKILL.md` | https://opencode.ai/docs/ | Automated (spec-drift.yml + tool-release-watch.yml) | Weekly | 2026-08-25 | AGM, XP, OC, OC-AG, OC-AGM, OC-CFG, OC-DEP, OC-LSP, OC-PM, OC-TUI, OC-SK |
| Kiro CLI | `.kiro/steering/*.md`, `.kiro/agents/**/*.{json,md}`, `.kiro/hooks/*.kiro.hook`, `.kiro/settings/mcp.json`, `.kiro/settings.json`, `.kiro/powers/*/POWER.md`, `.kiro/skills/*/SKILL.md`, `AGENTS.md` (nested, loaded as steering since 2.18.0) | https://kiro.dev/changelog/cli/ | Automated (tool-release-watch.yml via HTML scrape) | Quarterly | 2026-08-21 | AGM, KIRO, KR-AG, KR-HK, KR-MCP, KR-PW, KR-SK, KR-SET |

### A Tier (test on major changes)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| GitHub Copilot | `.github/copilot-instructions.md`, `.github/instructions/*.instructions.md` | https://docs.github.com/en/copilot/customizing-copilot | Automated (spec-drift.yml + tool-release-watch.yml via microsoft/vscode-copilot-chat) | Weekly | 2026-07-26 | COP |
| Cline | `.clinerules` (file or dir), `.clinerules/workflows/*.md`, `.clinerules/hooks/*`, `.clinerules/skills/*/SKILL.md`, `.cline/skills/*/SKILL.md` | https://docs.cline.bot/features/cline-rules/overview | Automated (spec-drift.yml + tool-release-watch.yml) | Weekly | 2026-08-24 | CLN, CL-SK |
| Cursor | `.cursor/rules/**/*.mdc`, `.cursorrules` (legacy), `.cursor/hooks.json`, `.cursor/agents/**/*.md`, `.cursor/environment.json`, `.cursor/mcp.json`, `.cursor/skills/*/SKILL.md` | https://cursor.com/docs/rules | Automated (spec-drift.yml + tool-release-watch.yml via api2.cursor.sh stable update endpoint) | Weekly | 2026-08-25 | CUR, MCP, CR-SK |

### B Tier (test on significant changes if time permits)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Roo Code | `.roomodes`, `.rooignore`, `.roorules`, `.roo/rules/*.md`, `.roo/rules-{slug}/*.md`, `.roo/mcp.json` | https://github.com/RooCodeInc/Roo-Code | Automated (tool-release-watch.yml) | Quarterly | 2026-07-26 | ROO |
| amp | `.amp/settings.json`, `.agents/checks/*.md`, `AGENTS.md` | https://ampcode.com/news | Automated (tool-release-watch.yml via ampcode.com/news.rss; tracks latest news-post slug as the release marker) | Quarterly | 2026-08-24 | AGM, AMP, AMP-SK |

### C Tier (community reports fixes only)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| gemini cli | `GEMINI.md`, `.gemini/settings.json`, `.geminiignore`, `gemini-extension.json`, `.gemini/agents/*.md` | https://github.com/google-gemini/gemini-cli | Automated (tool-release-watch.yml) | As reported | 2026-08-26 | GM, GM-AG |

### D Tier (no support, nice to have)

| Tool | Config Format | Documentation URL | Monitoring | Frequency | Last Reviewed | Rule Prefix |
|------|---------------|-------------------|------------|-----------|---------------|-------------|
| Windsurf | `.windsurf/rules/*.md`, `.windsurf/workflows/*.md`, `.windsurfrules` | https://windsurf.com/changelog | Automated (tool-release-watch.yml via HTML scrape; Wave-level granularity) | Ad hoc | 2026-07-26 | WS, WS-SK |

### E Tier (no support, community only)

All other AI coding tools - including continue, Antigravity, Tabnine, Codeium, Amazon Q, Aider, SourceGraph Cody, pi, and others. These tools do **not** have validators in `crates/agnix-core/src/rules/` and are not currently supported by agnix. Community contributions welcome via the Tool Support Request issue template.

---

## Documentation Sources

Authoritative sources monitored for changes that may affect validation rules.

### Specifications (Primary)

| Source | URL | Watch Method | Rules Affected |
|--------|-----|-------------|----------------|
| Agent Skills Spec | https://agentskills.io/specification | spec-drift.yml (weekly) + tool-release-watch.yml (commit-path on agentskills/agentskills `docs/specification.mdx`) | AS-001 through AS-019 |
| MCP Spec | https://modelcontextprotocol.io/specification/2026-07-28 | spec-drift.yml (weekly; overview, schema, tools, resources, prompts, transports, security, versioning) | MCP-001 through MCP-024 |
| MCP GitHub Repo | https://github.com/modelcontextprotocol/modelcontextprotocol | mcp-release-watch.yml | MCP-001 through MCP-024 |

### Vendor Documentation (Secondary)

| Source | URL | Watch Method | Rules Affected |
|--------|-----|-------------|----------------|
| Claude Code - Memory | https://code.claude.com/docs/en/memory | spec-drift.yml (weekly) | CC-MEM-001 through CC-MEM-010 |
| Claude Code - Hooks | https://code.claude.com/docs/en/hooks | spec-drift.yml (weekly) | CC-HK-001 through CC-HK-012 |
| Claude Code - Skills | https://code.claude.com/docs/en/skills | spec-drift.yml (weekly) | CC-SK-001 through CC-SK-009 |
| Claude Code - Plugins | https://code.claude.com/docs/en/plugins-reference | spec-drift.yml (weekly) | CC-PL-001 through CC-PL-015 |
| Claude Code - Sub-agents | https://code.claude.com/docs/en/sub-agents | spec-drift.yml (weekly) | CC-AG-001 through CC-AG-007 |
| Codex CLI - AGENTS.md | https://learn.chatgpt.com/docs/agent-configuration/agents-md | spec-drift.yml (weekly) | AGM-001 through AGM-006, XP-001 through XP-006 |
| OpenCode - Rules | https://opencode.ai/docs/rules/ | spec-drift.yml (weekly) | XP-001 through XP-006 |
| Cursor - Rules | https://cursor.com/docs/rules | spec-drift.yml (weekly) | CUR-001 through CUR-009, CUR-020 |
| Cursor - Hooks | https://cursor.com/docs/hooks | spec-drift.yml (weekly) | CUR-010 through CUR-013, CUR-017 through CUR-019 |
| Cursor - Subagents | https://cursor.com/docs/subagents | spec-drift.yml (weekly) | CUR-014, CUR-015 |
| Cursor - Environment | https://cursor.com/docs/cloud-agent/setup | spec-drift.yml (weekly) | CUR-016 |
| Cursor - Environment Schema | https://cursor.com/schemas/environment.schema.json | spec-drift.yml (weekly) | CUR-016 |
| Cursor - MCP | https://cursor.com/docs/mcp | spec-drift.yml (weekly) | MCP-009 through MCP-012, MCP-017 through MCP-024 |
| GitHub Copilot | https://docs.github.com/en/copilot/customizing-copilot | spec-drift.yml (weekly) | COP-001 through COP-006 |
| Cline - Rules | https://docs.cline.bot/features/cline-rules/overview | spec-drift.yml (weekly) | -- |

### Community Sources

| Source | URL | What to Watch |
|--------|-----|---------------|
| agentsys | https://github.com/anthropics/agentsys | Pattern updates, new enhance plugins |
| MCP Servers Registry | https://github.com/modelcontextprotocol/servers | New server patterns, security advisories |
| Stack Overflow AI Survey | https://survey.stackoverflow.co/2025/ai | Developer pain points, tool adoption trends |

---

## Academic Research

Research papers that inform validation rules, particularly prompt engineering and instruction-following rules.

| Paper | Authors | Year | Key Finding | Rules Informed |
|-------|---------|------|-------------|----------------|
| Lost in the Middle: How Language Models Use Long Contexts | Liu et al. | 2023 | Critical content in the middle of long contexts loses recall; position at start or end for best results | PE-001, CC-MEM-008 |
| Anthropic Long Context Research | Anthropic | 2023 | Single prompt change ("here is the most relevant sentence") improved retrieval accuracy from 27% to 98% | PE-001 |
| Positive Framing Studies | Multiple | 2023-2024 | "Do X" instructions outperform "Don't do Y" with measurable improvement in compliance rates | CC-MEM-006 |
| Constraint Strength Research | Instruction-following researchers | 2024 | MUST > imperatives > should > try to; weak language reduces compliance by significant margins | CC-MEM-007, PE-003 |
| Instruction-Following Reliability in LLMs | Multiple | 2024 | LLMs more reliably follow explicit, structured constraints than implicit or conversational ones | PE-003, PE-004 |
| Chain-of-Thought Prompting | Wei et al. | 2022 | CoT improves reasoning on complex tasks but adds overhead on simple tasks | PE-002 |

### Research Watch Areas

- Prompt injection defense mechanisms (currently unsolved, noted in MCP security)
- Multi-agent coordination patterns
- Config format convergence across tools
- Empirical studies on instruction-following in coding contexts

---

## Emerging Tools Watchlist

New developments that may require future rule additions or tool tier changes.

### Agent Protocol Standardization

- **Status**: Active development across multiple vendors
- **Watch**: Whether a universal agent config format emerges
- **Impact**: Could simplify cross-platform rules (XP-*) or require new universal rules
- **Sources**: Vendor announcements, community discussions

### New MCP Patterns

- **Status**: MCP ecosystem rapidly expanding
- **Watch**: New transport types, authentication patterns, tool annotation schemas
- **Impact**: May require updates to MCP-001 through MCP-008, new rules for auth/transport
- **Sources**: https://modelcontextprotocol.io, mcp-release-watch.yml workflow

### AGENTS.md Ecosystem

- **Status**: Adopted by Codex CLI, OpenCode; recognized by Claude Code
- **Watch**: Additional tools adopting AGENTS.md, format extensions
- **Impact**: AGM rules may need updates; XP rules may need to cover more tools
- **Sources**: https://learn.chatgpt.com/docs/agent-configuration/agents-md

### Sub-agent Patterns

- **Status**: Claude Code sub-agents gaining adoption
- **Watch**: How other tools implement sub-agent delegation
- **Impact**: CC-AG rules may need cross-platform equivalents
- **Sources**: https://code.claude.com/docs/en/sub-agents

---

## MCP Ecosystem Tracking

The Model Context Protocol ecosystem is tracked separately due to its rapid evolution and cross-tool impact.

### Current MCP Coverage

- **Protocol version monitored**: 2026-07-28
- **Rules implemented**: MCP-001 through MCP-026; MCP-025 and MCP-026 are Claude Code vendor rules
- **Automated monitoring**: mcp-release-watch.yml plus spec-drift.yml for MCP-001 through MCP-024; Claude Code release tracking covers MCP-025 and MCP-026
- **Baseline hash**: See `.github/spec-baselines.json` for current hash

### MCP Areas to Watch

| Area | Current Status | Potential Rule Impact |
|------|---------------|---------------------|
| Authentication patterns | No standard yet | New MCP-009+ rules for auth validation |
| Remote transport (SSE/WebSocket) | Supported in spec | Transport-specific validation rules |
| Tool annotations | annotations field in spec | MCP-006 may need expansion |
| Resource subscriptions | In spec, limited adoption | New rules for subscription patterns |
| Sampling/completions | Spec-defined, vendor-specific | Cross-vendor compatibility rules |

---

## Community Feedback Log

Tracking community input that influences rule development, tool support decisions, and validation improvements.

### Addressed Items

| Date | Source | Feedback | Action Taken | Issue/PR |
|------|--------|----------|-------------|----------|
| 2026-01-15 | GitHub Issues | Skills invoke at 0% without explicit trigger phrases | Added AS-010 rule for missing trigger phrase detection; sourced from Vercel research | #14 |
| 2026-01-20 | agentsys patterns | Enhance plugins identified 70 production-tested config patterns | Created PATTERNS-CATALOG.md; patterns informed CC-SK-007, CC-HK-009, CC-MEM-005 rules | #28 |
| 2026-02-01 | February 2026 monthly review | Coverage gap: no rules for Aider, Continue, Roo Code, Kiro CLI | Documented in tool inventory; awaiting community contributions via issue templates | #191 |
| 2026-02-01 | README pain points | "Almost-right configs" and "skills don't auto-trigger" as top developer frustrations | Prioritized auto-fix for AS-004 (kebab-case), AS-010 (trigger phrase), CC-HK-001 (event name) | #45, #46 |
| 2026-02-01 | Tool tier decisions | Community adoption data used to assign S/A/B/C/D/E tiers | Tier assignments documented in CLAUDE.md; spec-drift frequency matches tier priority | #107 |

### Pending Items

| Date | Source | Feedback | Status |
|------|--------|----------|--------|
| 2026-02-05 | Tool inventory audit | B/C/D tier tools lack rule coverage | Awaiting community contributions; issue templates now available |
| 2026-02-05 | Cross-tool compatibility | Users mixing Cursor + Claude Code + Copilot report silent failures | XP-004/005/006 rules address some cases; more patterns needed |

---

## Update Process

When this document needs updating:

1. **Tool changes**: Update the Tool Inventory table when tools change tiers, config formats, or documentation URLs
2. **New research**: Add papers to the Academic Research table when findings are actionable for rule development
3. **Spec changes**: spec-drift.yml creates issues automatically; update Documentation Sources after addressing drift
4. **Community feedback**: Log feedback in the Community Feedback Log; link to resulting issues/PRs
5. **Monthly reviews**: See [MONTHLY-REVIEW.md](./MONTHLY-REVIEW.md) for the structured review process
