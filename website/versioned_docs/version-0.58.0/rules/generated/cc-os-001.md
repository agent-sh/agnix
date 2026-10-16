---
id: cc-os-001
title: "CC-OS-001: Output Style Missing Description"
sidebar_label: "CC-OS-001"
description: "agnix rule CC-OS-001 checks for output style missing description in claude-output-styles files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-OS-001", "output style missing description", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-001`
- **Severity**: `LOW`
- **Category**: `claude-output-styles`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-22`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/output-styles

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: Concise
---
Be brief.
```

### Valid

```text
---
name: Concise
description: Short, direct replies
---
Be brief.
```
