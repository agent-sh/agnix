---
id: oc-dep-004
title: "OC-DEP-004: Deprecated CONTEXT.md Filename - OpenCode"
sidebar_label: "OC-DEP-004"
description: "agnix rule OC-DEP-004 checks for deprecated context.md filename in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-DEP-004", "deprecated context.md filename", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-DEP-004`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-06`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
CONTEXT.md
```

### Valid

```json
AGENTS.md
```
