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
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[approvals]
reviewer = 42
```

### Valid

```toml
[approvals]
reviewer = "user"
```
