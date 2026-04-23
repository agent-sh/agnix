---
id: cc-os-002
title: "CC-OS-002: Output Style Invalid keep-coding-instructions Type"
sidebar_label: "CC-OS-002"
description: "agnix rule CC-OS-002 checks for output style invalid keep-coding-instructions type in claude-output-styles files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-OS-002", "output style invalid keep-coding-instructions type", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-002`
- **Severity**: `HIGH`
- **Category**: `claude-output-styles`
- **Normative Level**: `MUST`
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
keep-coding-instructions: "yes"
---
Be brief.
```

### Valid

```text
---
name: Concise
description: Short replies
keep-coding-instructions: true
---
Be brief.
```
