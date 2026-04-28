---
id: cc-os-005
title: "CC-OS-005: Output Style Name Exceeds Length"
sidebar_label: "CC-OS-005"
description: "agnix rule CC-OS-005 checks for output style name exceeds length in claude-output-styles files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-OS-005", "output style name exceeds length", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-005`
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
name: A very very very very very very very very very very long output style name
description: Short
---
Be brief.
```

### Valid

```text
---
name: Concise
description: Short replies
---
Be brief.
```
