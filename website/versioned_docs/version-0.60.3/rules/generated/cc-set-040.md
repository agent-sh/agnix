---
id: cc-set-040
title: "CC-SET-040: Memory Path Setting Cannot Match"
sidebar_label: "CC-SET-040"
description: "agnix rule CC-SET-040 checks for memory path setting cannot match in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-040", "memory path setting cannot match", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-040`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory
- https://code.claude.com/docs/en/settings-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"claudeMdExcludes": ["packages/web/CLAUDE.md"], "autoMemoryDirectory": "memory"}
```

### Valid

```json
{"claudeMdExcludes": ["**/packages/web/CLAUDE.md"], "autoMemoryDirectory": "~/memory"}
```
