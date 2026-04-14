---
id: kr-pw-005
title: "KR-PW-005: Step Missing Description - Kiro Powers"
sidebar_label: "KR-PW-005"
description: "agnix rule KR-PW-005 checks for step missing description in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-005", "step missing description", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-005`
- **Severity**: `HIGH`
- **Category**: `Kiro Powers`
- **Normative Level**: `MUST`
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
name: test
description: desc
keywords: [test]
---
## Step 1
```

### Valid

```text
---
name: test
description: desc
keywords: [test]
---
## Step 1
Do something.
```
