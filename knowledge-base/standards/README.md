# AI Coding Assistant Standards Documentation

**Last Updated:** 2026-01-31

## Start Here

1. `RESEARCH-SUMMARY.md` - What was validated and which sources were used (S/A tiers first)
2. `multi-platform-HARD-RULES.md` - Cross-tool filename and precedence constraints (S/A tiers)
3. `multi-platform-OPINIONS.md` - Cross-tool best practices and recommendations
4. Pick a standard:
   - `claude-code-HARD-RULES.md` / `claude-code-OPINIONS.md`
   - `agent-skills-HARD-RULES.md` / `agent-skills-OPINIONS.md`
   - `mcp-HARD-RULES.md` / `mcp-OPINIONS.md`
   - `prompt-engineering-HARD-RULES.md` / `prompt-engineering-OPINIONS.md`

---

## Support Tiers (Ordering Rule)

Current priority order (see [coverage status](../RESEARCH-TRACKING.md#tool-inventory) before claiming validation support):

### S Tier (test always)
- Claude Code
- Codex CLI

### A Tier (test on major changes)
- Kiro CLI
- OpenCode
- Pi (tool-specific validator pending)

### B Tier (test on significant changes)
- Antigravity CLI (validator pending)
- Grok Code (validator pending)

### C Tier (community-reported maintenance)
- Muse Code (validator pending)
- Kimi Code (validator pending)

### D Tier (nice-to-have maintenance)
- GitHub Copilot
- Cline
- Cursor
- Legacy Gemini CLI

Continue, Aider, Amazon Q, Tabnine, Codeium, Devin/Windsurf and Amp are dropped from active support. The dated research documents retain their historical scopes and are not a current coverage claim.

---

## HARD RULES vs OPINIONS

- HARD RULES: only include non-negotiable, sourced facts (what is loaded / ignored / breaks)
- OPINIONS: recommendations and patterns (include rationale; note confidence when uncertain)

---

## Historical File Naming Cheat Sheet (2026-01-31)

- Claude Code: `CLAUDE.md`, `.claude/rules/*.md`
- Codex CLI: `AGENTS.md` (+ `AGENTS.local.md`, `AGENTS.override.md`)
- OpenCode: `AGENTS.md` (+ `AGENTS.local.md`, global `~/.config/opencode/AGENTS.md`, `Claude.md` fallback)
- Cursor: `.cursor/rules/*.mdc` (preferred), `.cursorrules` (legacy), plus `AGENTS.md`/`CLAUDE.md` supported at root
- Cline: `.clinerules` or `.clinerules/`, plus `AGENTS.md`
