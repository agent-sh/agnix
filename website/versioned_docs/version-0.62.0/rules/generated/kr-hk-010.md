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
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `v1 hook files and IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/mcp/security/
- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/ide/0x-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version": "v1", "hooks": [{"name": "notify", "trigger": "Stop", "action": {"type": "command", "command": "curl -H 'Authorization: Bearer sk-live-4f9a8b7c6d5e4f3a2b1c'"}}]}
```

### Valid

```json
{"version": "v1", "hooks": [{"name": "notify", "trigger": "Stop", "action": {"type": "command", "command": "deploy --token=$DEPLOY_TOKEN"}}]}
```
