---
id: cdx-cfg-007
title: "CDX-CFG-007: Danger Full Access Without Acknowledgment"
sidebar_label: "CDX-CFG-007"
description: "agnix rule CDX-CFG-007 checks for danger full access without acknowledgment in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-007", "danger full access without acknowledgment", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-007`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json
- https://developers.openai.com/codex/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
sandbox_mode = "danger-full-access"
```

### Valid

```toml
sandbox_mode = "danger-full-access"
[notice]
hide_full_access_warning = true
```
