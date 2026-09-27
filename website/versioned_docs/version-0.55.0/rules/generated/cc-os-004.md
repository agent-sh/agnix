---
id: cc-os-004
title: "CC-OS-004: Output Style Empty Body - claude-output-styles"
sidebar_label: "CC-OS-004"
description: "agnix rule CC-OS-004 checks for output style empty body in claude-output-styles files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-OS-004", "output style empty body", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-004`
- **Severity**: `MEDIUM`
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
description: Short replies
---
```

### Valid

```text
---
name: Concise
description: Short replies
---
Be brief and direct.
```
