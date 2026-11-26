---
id: kr-set-004
title: "KR-SET-004: Unparseable Kiro CLI Settings File"
sidebar_label: "KR-SET-004"
description: "agnix rule KR-SET-004 checks for unparseable kiro cli settings file in kiro settings files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-SET-004", "unparseable kiro cli settings file", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-004`
- **Severity**: `HIGH`
- **Category**: `Kiro Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  // default agent
  "chat.defaultAgent": "my-agent",
}
```

### Valid

```json
{"chat.defaultAgent": "my-agent"}
```
