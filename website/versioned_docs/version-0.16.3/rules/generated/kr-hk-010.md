---
id: kr-hk-010
title: "KR-HK-010: Secrets in Hook Command - Kiro Hooks"
sidebar_label: "KR-HK-010"
description: "agnix rule KR-HK-010 checks for secrets in hook command in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-010", "secrets in hook command", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-010`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
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
{"event": "promptSubmit", "runCommand": "curl -H 'Authorization: Bearer sk-live-secret123'"}
```

### Valid

```json
{"event": "promptSubmit", "runCommand": "echo $API_KEY"}
```
