---
id: cc-ag-021
title: "CC-AG-021: Subagent Descriptions Over Startup Budget"
sidebar_label: "CC-AG-021"
description: "agnix rule CC-AG-021 checks for subagent descriptions over startup budget in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-021", "subagent descriptions over startup budget", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-021`
- **Severity**: `MEDIUM`
- **Category**: `Claude Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sub-agents
- https://code.claude.com/docs/en/errors

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
description: <tens of thousands of characters of guidance across all agents>
---
Review the diff.
```

### Valid

```markdown
---
name: reviewer
description: Reviews pull requests for correctness. Use proactively after edits.
---
Review the diff.
```
