---
id: kr-hk-031
title: "KR-HK-031: Kiro Compaction Hook Requires Kiro CLI 3.1.0"
sidebar_label: "KR-HK-031"
description: "agnix rule KR-HK-031 checks for kiro compaction hook requires kiro cli 3.1.0 in kiro hooks files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-HK-031", "kiro compaction hook requires kiro cli 3.1.0", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-031`
- **Severity**: `LOW`
- **Category**: `Kiro Hooks`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-11-08`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/changelog/cli/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
A PreCompact hook in a repository whose CI or teammates still run Kiro CLI 3.0.x
```

### Valid

```json
{ "trigger": "PreCompact", "action": { "type": "command", "command": "cp history.json archive/" } } (on Kiro CLI >= 3.1.0)
```
