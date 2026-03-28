---
id: cdx-cfg-024
title: "CDX-CFG-024: Invalid Approvals Reviewer Value - Codex Config"
sidebar_label: "CDX-CFG-024"
description: "agnix rule CDX-CFG-024 checks for invalid approvals reviewer value in codex config files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-024", "invalid approvals reviewer value", "codex config", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-024`
- **Severity**: `MEDIUM`
- **Category**: `Codex Config`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
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

```json
[approvals]
reviewer = 42
```

### Valid

```json
[approvals]
reviewer = "user"
```
