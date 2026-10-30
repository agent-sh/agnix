---
id: cc-ag-019
title: "CC-AG-019: Unknown or Ignored Agent Frontmatter Field"
sidebar_label: "CC-AG-019"
description: "agnix rule CC-AG-019 checks for unknown or ignored agent frontmatter field in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-019", "unknown or ignored agent frontmatter field", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-019`
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
description: Review code
autoCompactWindow: 200000
experimental:
  cacheTtl: 1h
---
Review the changed code.
```
