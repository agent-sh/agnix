---
id: cdx-cfg-031
title: "CDX-CFG-031: Invalid skills.max_context_tokens - Codex CLI"
sidebar_label: "CDX-CFG-031"
description: "agnix rule CDX-CFG-031 checks for invalid skills.max_context_tokens in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-031", "invalid skills.max_context_tokens", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-031`
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

- https://learn.chatgpt.com/docs/config-file/config-reference
- https://learn.chatgpt.com/docs/build-skills
- https://learn.chatgpt.com/docs/config-schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[skills]
max_context_tokens = 20000
```

### Valid

```toml
[skills]
max_context_tokens = 4000
```
