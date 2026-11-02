---
id: oc-cmd-003
title: "OC-CMD-003: template in Markdown Command Frontmatter"
sidebar_label: "OC-CMD-003"
description: "agnix rule OC-CMD-003 checks for template in markdown command frontmatter in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-CMD-003", "template in markdown command frontmatter", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CMD-003`
- **Severity**: `LOW`
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

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
---
template: Review $ARGUMENTS.
---
```

### Valid

```json
---
description: Review
---
Review $ARGUMENTS.
```
