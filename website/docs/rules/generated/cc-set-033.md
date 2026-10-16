---
id: cc-set-033
title: "CC-SET-033: Idle Compaction Not Boolean - Claude Settings"
sidebar_label: "CC-SET-033"
description: "agnix rule CC-SET-033 checks for idle compaction not boolean in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-033", "idle compaction not boolean", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-033`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-16`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.290`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.299

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"idleCompaction": "false"}
```

### Valid

```json
{"idleCompaction": false}
```
