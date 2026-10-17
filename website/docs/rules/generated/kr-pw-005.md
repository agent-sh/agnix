---
id: kr-pw-005
title: "KR-PW-005: Step Heading Without Content - Kiro Powers"
sidebar_label: "KR-PW-005"
description: "agnix rule KR-PW-005 checks for step heading without content in kiro powers files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-PW-005", "step heading without content", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-005`
- **Severity**: `LOW`
- **Category**: `Kiro Powers`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
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
## Step 1: Install
## Step 2: Configure
Edit the config.
```

### Valid

```text
---
name: test
description: desc
keywords: [test]
---
## Step 1: Install
Run the installer.
```
