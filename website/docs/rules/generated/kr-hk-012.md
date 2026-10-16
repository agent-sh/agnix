---
id: kr-hk-012
title: "KR-HK-012: Invalid Kiro V3 Hook Entry - Kiro Hooks"
sidebar_label: "KR-HK-012"
description: "agnix rule KR-HK-012 checks for invalid kiro v3 hook entry in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-012", "invalid kiro v3 hook entry", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-012`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-16`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.26.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/hooks/types/
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "audit", "trigger": "onFileChange", "action": {"type": "command", "command": "echo"}}
```

### Valid

```json
{"name": "audit-shell", "trigger": "PreToolUse", "matcher": "shell", "action": {"type": "command", "command": "echo audit"}}
```
