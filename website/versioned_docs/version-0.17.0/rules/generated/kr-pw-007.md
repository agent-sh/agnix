---
id: kr-pw-007
title: "KR-PW-007: Name Invalid Characters - Kiro Powers"
sidebar_label: "KR-PW-007"
description: "agnix rule KR-PW-007 checks for name invalid characters in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-007", "name invalid characters", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-007`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Powers`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: My Power!
description: desc
keywords: [test]
---
# Body
```

### Valid

```text
---
name: my-power
description: desc
keywords: [test]
---
# Body
```
