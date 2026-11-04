---
id: oc-cmd-001
title: "OC-CMD-001: Command Argument Inside Shell Block - OpenCode"
sidebar_label: "OC-CMD-001"
description: "agnix rule OC-CMD-001 checks for command argument inside shell block in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CMD-001", "command argument inside shell block", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CMD-001`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/commands/
- https://opencode.ai/docs/commands/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
Summarize these commits:
!`git log --oneline -$1`
```

### Valid

```json
Summarize these commits:
!`git log --oneline -10`
```
