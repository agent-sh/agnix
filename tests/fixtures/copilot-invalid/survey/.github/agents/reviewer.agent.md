---
description: Reviews pull requests
mcp-servers:
  tracker:
    type: local
    command: tracker-mcp
    env:
      TOKEN: ${{ secrets.TRACKER_TOKEN }}
  docs:
    command: docs-mcp
    tools: ["*"]
---
Review the change for correctness and missing tests.
