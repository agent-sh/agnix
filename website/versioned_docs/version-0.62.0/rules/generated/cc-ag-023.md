---
id: cc-ag-023
title: "CC-AG-023: Duplicate Subagent Name - Claude Agents"
sidebar_label: "CC-AG-023"
description: "agnix rule CC-AG-023 checks for duplicate subagent name in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-023", "duplicate subagent name", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-023`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# .claude/agents/reviewer.md and .claude/agents/team/reviewer.md
---
name: reviewer
description: Reviews code
---
```

### Valid

```markdown
# .claude/agents/reviewer.md
---
name: reviewer
description: Reviews code
---
```
