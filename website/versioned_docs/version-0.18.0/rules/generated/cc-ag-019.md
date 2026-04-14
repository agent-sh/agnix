---
id: cc-ag-019
title: "CC-AG-019: Unknown Agent Frontmatter Field - Claude Agents"
sidebar_label: "CC-AG-019"
description: "agnix rule CC-AG-019 checks for unknown agent frontmatter field in claude agents files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-AG-019", "unknown agent frontmatter field", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-019`
- **Severity**: `LOW`
- **Category**: `Claude Agents`
- **Normative Level**: `SHOULD`
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
priority: high
---
Agent instructions.
```

### Valid

```markdown
---
name: my-agent
description: A helpful agent
---
Agent instructions.
```
