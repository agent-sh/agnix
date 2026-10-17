---
description: Reviews backend changes before they are merged
model: claude-opus-4.8
tools: [read, shell]
permissions:
  rules:
    - capability: shell
      effect: allow
      match: ["git diff *", "npm test *"]
hooks:
  - name: format-on-save
    trigger: PostFileSave
    matcher: "\\.rs$"
    action:
      type: command
      command: cargo fmt
    timeout: 30
---
Review the change, run the tests, and summarize any risks.
