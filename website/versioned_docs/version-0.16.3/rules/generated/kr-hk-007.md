---
id: kr-hk-007
title: "KR-HK-007: Hook Timeout Out of Range - Kiro Hooks"
sidebar_label: "KR-HK-007"
description: "agnix rule KR-HK-007 checks for hook timeout out of range in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-007", "hook timeout out of range", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-007`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"event": "fileEdited", "patterns": ["**/*.md"], "runCommand": "echo ok", "timeout": 999999}
```

### Valid

```json
{"event": "fileEdited", "patterns": ["**/*.md"], "runCommand": "echo ok"}
```
