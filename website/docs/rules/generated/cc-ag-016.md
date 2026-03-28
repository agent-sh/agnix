---
id: cc-ag-016
title: "CC-AG-016: Invalid Background Type - Claude Agents"
sidebar_label: "CC-AG-016"
description: "agnix rule CC-AG-016 checks for invalid background type in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-016", "invalid background type", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-016`
- **Severity**: `MEDIUM`
- **Category**: `Claude Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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

```yaml
---
name: my-agent
background: "yes"
---
Agent instructions.
```

### Valid

```yaml
---
name: my-agent
background: true
---
Agent instructions.
```
