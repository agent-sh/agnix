---
id: gm-cmd-002
title: "GM-CMD-002: Gemini Command Description Missing, Multi-Line or Truncated"
sidebar_label: "GM-CMD-002"
description: "agnix rule GM-CMD-002 checks for gemini command description missing, multi-line or truncated in gemini cli files. Severity: LOW. See examples and fix guidance."
keywords: ["GM-CMD-002", "gemini command description missing, multi-line or truncated", "gemini cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-CMD-002`
- **Severity**: `LOW`
- **Category**: `Gemini CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `gemini-cli`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://geminicli.com/docs/cli/custom-commands
- https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/cli/src/services/FileCommandLoader.ts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
prompt = "Refactor the code in the current context into a pure function."
```

### Valid

```markdown
description = "Asks the model to refactor the current context into a pure function."
prompt = "Refactor the code in the current context into a pure function."
```
