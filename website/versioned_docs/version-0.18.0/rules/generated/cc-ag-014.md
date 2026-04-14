---
id: cc-ag-014
title: "CC-AG-014: Invalid Effort Value - Claude Agents"
sidebar_label: "CC-AG-014"
description: "agnix rule CC-AG-014 checks for invalid effort value in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-014", "invalid effort value", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

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
name: my-agent
effort: maximum
---
Agent instructions.
```

### Valid

```markdown
---
name: my-agent
effort: high
---
Agent instructions.
```
