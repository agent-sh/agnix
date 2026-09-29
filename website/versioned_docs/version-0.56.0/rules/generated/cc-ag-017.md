---
id: cc-ag-017
title: "CC-AG-017: Invalid MaxTurns Value - Claude Agents"
sidebar_label: "CC-AG-017"
description: "agnix rule CC-AG-017 checks for invalid maxturns value in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-017", "invalid maxturns value", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-017`
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

```markdown
---
name: my-agent
max-turns: "ten"
---
Agent instructions.
```

### Valid

```markdown
---
name: my-agent
max-turns: 10
---
Agent instructions.
```
