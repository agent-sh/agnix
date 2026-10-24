---
id: cdx-app-003
title: "CDX-APP-003: Removed Profile Selector or Profiles Table"
sidebar_label: "CDX-APP-003"
description: "agnix rule CDX-APP-003 checks for removed profile selector or profiles table in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-APP-003", "removed profile selector or profiles table", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-APP-003`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/config-file/config-advanced

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
profile = "deep-review"

[profiles.deep-review]
model = "gpt-5"
```

### Valid

```toml
# ~/.codex/deep-review.config.toml
model = "gpt-5"
```
