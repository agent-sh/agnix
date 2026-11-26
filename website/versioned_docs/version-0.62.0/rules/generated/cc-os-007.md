---
id: cc-os-007
title: "CC-OS-007: outputStyle Case Mismatch - claude-output-styles"
sidebar_label: "CC-OS-007"
description: "agnix rule CC-OS-007 checks for outputstyle case mismatch in claude-output-styles files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-OS-007", "outputstyle case mismatch", "claude-output-styles", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-OS-007`
- **Severity**: `MEDIUM`
- **Category**: `claude-output-styles`
- **Normative Level**: `MUST`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/output-styles

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"outputStyle": "explanatory"}
```

### Valid

```json
{"outputStyle": "Explanatory"}
```
