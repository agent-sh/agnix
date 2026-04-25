---
kind: local
name: spanner-test-agent
description: An agent to test Spanner MCP with auth
mcp_servers:
  spanner:
    url: https://spanner.googleapis.com/mcp
    type: http
    auth:
      type: google-credentials
      scopes:
        - https://www.googleapis.com/auth/cloud-platform
    timeout: 30000
system_prompt: You are a Spanner test agent.
---

Body here.
