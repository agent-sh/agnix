---
id: gm-cmd-001
title: "GM-CMD-001: Invalid Gemini Custom Command File - Gemini CLI"
sidebar_label: "GM-CMD-001"
description: "agnix rule GM-CMD-001 checks for invalid gemini custom command file in gemini cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-CMD-001", "invalid gemini custom command file", "gemini cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-CMD-001`
- **Severity**: `HIGH`
- **Category**: `Gemini CLI`
- **Normative Level**: `MUST`
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
description = "Generates a fix for a given issue."
text = "Please provide a code fix for the issue described here: {{args}}."
```

### Valid

```markdown
description = "Generates a fix for a given issue."
prompt = "Please provide a code fix for the issue described here: {{args}}."
```
