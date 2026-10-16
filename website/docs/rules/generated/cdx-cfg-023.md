---
id: cdx-cfg-023
title: "CDX-CFG-023: Invalid Approval Policy Sub-field - Codex CLI"
sidebar_label: "CDX-CFG-023"
description: "agnix rule CDX-CFG-023 checks for invalid approval policy sub-field in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-023", "invalid approval policy sub-field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-023`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-16`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.163.0/codex-rs/core/config.schema.json
- https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[approval_policy.granular]
mcp_elicitations = true
rules = true
sandbox_approval = true
unknown_field = true
```

### Valid

```toml
[approval_policy.granular]
mcp_elicitations = true
rules = true
sandbox_approval = true
```
