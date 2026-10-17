---
name: test-skill
description: Use when testing per-client skill rules
mcpServers:
  linear:
    url: https://mcp.linear.app/sse
hooks: some-value
---
# Shared `.agents/skills/` Test Fixture

Codex, Amp, Cline, Devin Desktop, Copilot, OpenCode, Cursor and Gemini all
read `.agents/skills/`. Unscoped, no client's field list applies (only
XP-SK-001). With `tools = ["amp"]`, AMP-SK-001 accepts the Amp field
`mcpServers` and reports `hooks`; with `tools = ["codex"]`, CX-SK-001 reports
both.
