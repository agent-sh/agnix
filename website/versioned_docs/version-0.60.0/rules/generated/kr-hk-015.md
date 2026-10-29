---
id: kr-hk-015
title: "KR-HK-015: Legacy IDE Hook Saved as a V3 Hook File"
sidebar_label: "KR-HK-015"
description: "agnix rule KR-HK-015 checks for legacy ide hook saved as a v3 hook file in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-015", "legacy ide hook saved as a v3 hook file", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-015`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/ide/whats-new-v1/hooks/
- https://kiro.dev/docs/ide/0x-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "Lint on save", "version": "1.0.0", "when": {"type": "fileEdited", "patterns": ["**/*.ts"]}, "then": {"type": "askAgent", "prompt": "Lint the file"}}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"Lint on save","trigger":"PostFileSave","matcher":"\\.ts$","action":{"type":"command","command":"npx eslint --fix"}}]}
```
