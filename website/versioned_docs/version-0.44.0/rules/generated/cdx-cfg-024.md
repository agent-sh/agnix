---
id: cdx-cfg-024
title: "CDX-CFG-024: Invalid Approvals Reviewer Value - Codex CLI"
sidebar_label: "CDX-CFG-024"
description: "agnix rule CDX-CFG-024 checks for invalid approvals reviewer value in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-024", "invalid approvals reviewer value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-024`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-06-16`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.137.0/codex-rs/core/config.schema.json
- https://github.com/openai/codex/blob/rust-v0.140.0/codex-rs/core/config.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
approvals_reviewer = "approve"

[apps.browser]
approvals_reviewer = 42

[apps._default]
approvals_reviewer = "approve"
```

### Valid

```toml
approvals_reviewer = "auto_review"

[apps.browser]
approvals_reviewer = "user"

[apps._default]
approvals_reviewer = "guardian_subagent"
```
