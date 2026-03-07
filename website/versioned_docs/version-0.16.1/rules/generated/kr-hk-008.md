---
id: kr-hk-008
title: "KR-HK-008: Duplicate Event Handlers - Kiro Hooks"
sidebar_label: "KR-HK-008"
description: "agnix rule KR-HK-008 checks for duplicate event handlers in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-008", "duplicate event handlers", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-008`
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
# Two hooks with identical event+patterns
```

### Valid

```json
{"event": "fileEdited", "patterns": ["**/*.md"], "runCommand": "echo ok"}
```
