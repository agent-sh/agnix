---
id: cc-ag-005
title: "CC-AG-005: Preloaded Skill Not Found or Not Preloadable"
sidebar_label: "CC-AG-005"
description: "agnix rule CC-AG-005 checks for preloaded skill not found or not preloadable in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-005", "preloaded skill not found or not preloadable", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-005`
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
---
name: code-reviewer
description: Reviews code
skills:
  - nonexistent-skill
---
Review code changes.
```

### Valid

```markdown
---
name: code-reviewer
description: Reviews code
skills:
  - code-review
---
Review code changes.
```
