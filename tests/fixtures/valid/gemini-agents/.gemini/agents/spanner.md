---
kind: local
name: spanner-test-agent
description: An agent to test Spanner MCP with auth
tools:
  - read_file
mcp_servers:
  spanner:
    url: https://spanner.googleapis.com/mcp
    type: http
    auth:
      type: google-credentials
      scopes:
        - https://www.googleapis.com/auth/cloud-platform
    timeout: 30000
---

You are a Spanner test agent. The Markdown body is the system prompt;
`system_prompt` is not a frontmatter key in agentLoader.ts.
