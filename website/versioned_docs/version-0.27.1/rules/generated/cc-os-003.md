---
id: cc-os-003
title: "CC-OS-003: Output Style Unknown Frontmatter Key"
sidebar_label: "CC-OS-003"
description: "agnix rule CC-OS-003 checks for output style unknown frontmatter key in claude-output-styles files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-OS-003", "output style unknown frontmatter key", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-003`
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
alwaysApply: true
---
Be brief.
```

### Valid

```text
---
name: Concise
description: Short replies
keep-coding-instructions: false
---
Be brief.
```
