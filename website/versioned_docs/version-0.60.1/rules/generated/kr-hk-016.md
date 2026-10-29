---
id: kr-hk-016
title: "KR-HK-016: V3 Hook File Saved as .kiro.hook - Kiro Hooks"
sidebar_label: "KR-HK-016"
description: "agnix rule KR-HK-016 checks for v3 hook file saved as .kiro.hook in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-016", "v3 hook file saved as .kiro.hook", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-016`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook / IDE 1.0`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/ide/whats-new-v1/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"Lint on save","trigger":"PostFileSave","matcher":"\\.ts$","action":{"type":"command","command":"npx eslint --fix"}}]}
```

### Valid

```json
{"name": "Lint", "version": "1", "when": {"type": "fileEdited", "patterns": ["**/*.ts"]}, "then": {"type": "askAgent", "prompt": "Lint the file"}}
```
