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
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (safe)`
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
[approval_policy]
mode = "yolo"
```

### Valid

```toml
[approval_policy]
mode = "suggest"
```
