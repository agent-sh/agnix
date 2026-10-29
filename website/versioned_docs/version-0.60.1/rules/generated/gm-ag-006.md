---
id: gm-ag-006
title: "GM-AG-006: Gemini Local Agent Inherits Every Tool"
sidebar_label: "GM-AG-006"
description: "agnix rule GM-AG-006 checks for gemini local agent inherits every tool in gemini agents files. Severity: LOW. See examples and fix guidance."
keywords: ["GM-AG-006", "gemini local agent inherits every tool", "gemini agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-AG-006`
- **Severity**: `LOW`
- **Category**: `Gemini Agents`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `gemini-cli`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://geminicli.com/docs/core/subagents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: reviewer
description: Reviews pull requests
---
```

### Valid

```text
---
name: reviewer
description: Reviews pull requests
tools:
  - read_file
  - grep_search
---
```
