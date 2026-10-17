---
- kind: remote
  name: remote-1
  agent_card_url: https://example.com/1
- kind: remote
  name: remote-2
  agent_card_url: https://example.com/2
  auth:
    type: apiKey
    key: $REMOTE_2_API_KEY
---
