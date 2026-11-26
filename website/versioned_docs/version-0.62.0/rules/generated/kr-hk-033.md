---
id: kr-hk-033
title: "KR-HK-033: Kiro PreToolUse updatedInput Decision Requires Kiro CLI 3.4.0"
sidebar_label: "KR-HK-033"
description: "agnix rule KR-HK-033 checks for kiro pretooluse updatedinput decision requires kiro cli 3.4.0 in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-033", "kiro pretooluse updatedinput decision requires kiro cli 3.4.0", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-033`
- **Severity**: `LOW`
- **Category**: `Kiro Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-18`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/changelog/cli/
- https://kiro.dev/docs/hooks/types/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
The same hook on Kiro CLI 3.3.x or older: the JSON is plain output, the exit code is the only decision, and the tool call keeps its original inputs
```

### Valid

```text
A PreToolUse command hook whose STDOUT prints { "hookSpecificOutput": { "permissionDecision": "allow", "updatedInput": { "path": "docs/README.md" } } } (on Kiro CLI >= 3.4.0)
```
