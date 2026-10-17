---
id: kiro-016
title: "KIRO-016: AGENTS.md Exceeds Kiro Truncation Limit"
sidebar_label: "KIRO-016"
description: "agnix rule KIRO-016 checks for agents.md exceeds kiro truncation limit in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-016", "agents.md exceeds kiro truncation limit", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-016`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `Kiro CLI 2.32.1 V3 engine (@kiro/agent 0.66.48)`

## Evidence Sources

- https://kiro.dev/docs/steering/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
AGENTS.md of 80,000 characters (Kiro CLI V3 keeps the first 50,000)
```

### Valid

```markdown
AGENTS.md under 50,000 characters, with scoped guidance in .kiro/steering/
```
