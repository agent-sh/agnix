---
id: kr-hk-011
title: "KR-HK-011: Invalid Kiro V3 Hook Document Version"
sidebar_label: "KR-HK-011"
description: "agnix rule KR-HK-011 checks for invalid kiro v3 hook document version in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-011", "invalid kiro v3 hook document version", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-011`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"hooks": [{"name": "lint", "trigger": "PostFileSave", "action": {"type": "command", "command": "npx eslint --fix"}}]}
```

### Valid

```json
{"version": "v1", "hooks": [{"name": "lint", "trigger": "PostFileSave", "action": {"type": "command", "command": "npx eslint --fix"}}]}
```
