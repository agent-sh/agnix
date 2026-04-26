---
kind: local
name: broken-agent
description: Shows the invalid auth variant
mcp_servers:
  srv:
    auth:
      type: basic-auth
      client_id: abc
system_prompt: Fails to load because basic-auth isn't a valid auth.type.
---

Body here.
