---
name: twice
description: Defines MCP servers twice. Use when testing AMP-005.
mcpServers:
  linear:
    url: https://mcp.linear.app/mcp
    includeTools: ["list_issues"]
---

AMP-005: Amp uses mcpServers and ignores the sibling mcp.json.
