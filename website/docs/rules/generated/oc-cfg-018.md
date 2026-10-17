---
id: oc-cfg-018
title: "OC-CFG-018: Reference Without Description - OpenCode"
sidebar_label: "OC-CFG-018"
description: "agnix rule OC-CFG-018 checks for reference without description in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-CFG-018", "reference without description", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-018`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/references/
- https://opencode.ai/v2/docs/references/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"references": {"docs": "../docs"}}
```

### Valid

```json
{"references": {"docs": {"path": "../docs", "description": "Product behavior"}}}
```
