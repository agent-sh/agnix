---
id: gm-cmd-003
title: "GM-CMD-003: Unbalanced Braces in Gemini Command Injection"
sidebar_label: "GM-CMD-003"
description: "agnix rule GM-CMD-003 checks for unbalanced braces in gemini command injection in gemini cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["GM-CMD-003", "unbalanced braces in gemini command injection", "gemini cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GM-CMD-003`
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
- https://raw.githubusercontent.com/google-gemini/gemini-cli/main/packages/cli/src/services/prompt-processors/injectionParser.ts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
description = "First column"
prompt = """
Summarize:
!{awk '{print $1' data.txt}
"""
```

### Valid

```markdown
description = "Commit message"
prompt = """
Write a commit message for:
!{git diff --staged}
"""
```
