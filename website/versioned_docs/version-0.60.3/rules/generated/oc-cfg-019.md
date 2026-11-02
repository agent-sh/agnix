---
id: oc-cfg-019
title: "OC-CFG-019: Bare Reference Shorthand Read as Git Repository"
sidebar_label: "OC-CFG-019"
description: "agnix rule OC-CFG-019 checks for bare reference shorthand read as git repository in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-CFG-019", "bare reference shorthand read as git repository", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-019`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/references/
- https://opencode.ai/docs/references/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"references": {"docs": "docs"}}
```

### Valid

```json
{"references": {"docs": "./docs"}}
```
