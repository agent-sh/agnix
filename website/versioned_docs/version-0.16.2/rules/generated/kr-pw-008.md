---
id: kr-pw-008
title: "KR-PW-008: Secrets in Power Body - Kiro Powers"
sidebar_label: "KR-PW-008"
description: "agnix rule KR-PW-008 checks for secrets in power body in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-008", "secrets in power body", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-008`
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
API_KEY=sk-live-secret123
```

### Valid

```text
---
name: test
description: desc
keywords: [test]
---
Use ${API_KEY}
```
