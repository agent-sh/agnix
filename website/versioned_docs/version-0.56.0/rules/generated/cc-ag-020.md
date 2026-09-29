---
id: cc-ag-020
title: "CC-AG-020: Reserved Colon in Agent Name - Claude Agents"
sidebar_label: "CC-AG-020"
description: "agnix rule CC-AG-020 checks for reserved colon in agent name in claude agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-AG-020", "reserved colon in agent name", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-020`
- **Severity**: `HIGH`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.218`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.218
- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: plugin:reviewer
description: Reviews code
---
Review the change.
```

### Valid

```markdown
---
name: reviewer
description: Reviews code
---
Review the change.
```
