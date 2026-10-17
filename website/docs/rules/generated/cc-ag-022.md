---
id: cc-ag-022
title: "CC-AG-022: Subagent Inherits All Tools - Claude Agents"
sidebar_label: "CC-AG-022"
description: "agnix rule CC-AG-022 checks for subagent inherits all tools in claude agents files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-AG-022", "subagent inherits all tools", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-022`
- **Severity**: `LOW`
- **Category**: `Claude Agents`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: reviewer
description: Reviews code
---
Review the diff.
```

### Valid

```markdown
---
name: reviewer
description: Reviews code
tools: Read, Grep, Glob
---
Review the diff.
```
