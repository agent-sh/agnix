---
id: cc-os-006
title: "CC-OS-006: Invalid Output Style Frontmatter Syntax"
sidebar_label: "CC-OS-006"
description: "agnix rule CC-OS-006 checks for invalid output style frontmatter syntax in claude-output-styles files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-OS-006", "invalid output style frontmatter syntax", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-006`
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
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: Concise
description: Short replies
```

### Valid

```text
---
name: Concise
description: Short replies
---
Be brief.
```
