---
name: tracker
description: Files issues in the tracker. Use when the user asks to open or update an issue.
mcpServers:
  tracker:
    url: https://tracker.example.com/mcp
    headers:
      Authorization: Bearer abcdefghijklmnop123456
    includeTools: ["create_issue"]
---
Open the issue with the tracker tools.
