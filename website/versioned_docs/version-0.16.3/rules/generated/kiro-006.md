---
id: kiro-006
title: "KIRO-006: Secrets Detected in Steering File - Kiro Steering"
sidebar_label: "KIRO-006"
description: "agnix rule KIRO-006 checks for secrets detected in steering file in kiro steering files. Severity: HIGH. See examples and fix guidance."
keywords: ["KIRO-006", "secrets detected in steering file", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-006`
- **Severity**: `HIGH`
- **Category**: `Kiro Steering`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/steering/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
inclusion: always
---
API_KEY=hardcoded-secret-123
```

### Valid

```markdown
---
inclusion: always
---
Use ${API_KEY} from the environment at runtime.
```
