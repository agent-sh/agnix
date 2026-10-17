---
id: kr-hk-009
title: "KR-HK-009: Command Uses Absolute Path - Kiro Hooks"
sidebar_label: "KR-HK-009"
description: "agnix rule KR-HK-009 checks for command uses absolute path in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-009", "command uses absolute path", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-009`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/ide/0x-reference/
- https://kiro.dev/docs/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"when": {"type": "promptSubmit"}, "then": {"type": "runCommand", "command": "/usr/local/bin/npm test"}}
```

### Valid

```json
{"when": {"type": "promptSubmit"}, "then": {"type": "runCommand", "command": "npm test"}}
```
