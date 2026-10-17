---
name: researcher
description: Researches library APIs before code changes
tools:
  - read_file
mcp_servers:
  docs:
    command: docs-mcp
    args: ["--stdio"]
    env:
      DOCS_API_TOKEN: literal-token-value-123
---
Look up the API in the docs server before answering.
