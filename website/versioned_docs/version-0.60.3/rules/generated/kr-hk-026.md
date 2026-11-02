---
id: kr-hk-026
title: "KR-HK-026: Invalid Kiro IDE Hook File - Kiro Hooks"
sidebar_label: "KR-HK-026"
description: "agnix rule KR-HK-026 checks for invalid kiro ide hook file in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-026", "invalid kiro ide hook file", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-026`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `IDE 0.x .kiro.hook`

## Evidence Sources

- https://kiro.dev/docs/ide/0x-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "Lint docs", "when": {"type": "fileEdited", "patterns": "docs/**/*.md"}, "then": {"type": "askAgent", "prompt": "Check links"}}
```

### Valid

```json
{"name": "Lint docs", "when": {"type": "fileEdited", "patterns": ["docs/**/*.md"]}, "then": {"type": "askAgent", "prompt": "Check links"}}
```
