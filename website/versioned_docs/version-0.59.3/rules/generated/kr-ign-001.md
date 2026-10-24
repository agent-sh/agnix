---
id: kr-ign-001
title: "KR-IGN-001: Ineffective Kiro Ignore Negation - Kiro Ignore"
sidebar_label: "KR-IGN-001"
description: "agnix rule KR-IGN-001 checks for ineffective kiro ignore negation in kiro ignore files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-IGN-001", "ineffective kiro ignore negation", "kiro ignore", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-IGN-001`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Ignore`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/kiroignore/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
secrets/
!secrets/public.txt
```

### Valid

```text
secrets/*
!secrets/public.txt
```
