---
id: cc-pl-017
title: "CC-PL-017: Unknown Plugin Manifest Field - Claude Plugins"
sidebar_label: "CC-PL-017"
description: "agnix rule CC-PL-017 checks for unknown plugin manifest field in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-017", "unknown plugin manifest field", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-017`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-08`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins-reference

## Test Coverage Metadata

- Unit tests: `false`
- Fixture tests: `false`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name":"demo","homepag":"https://example.com"}
```

### Valid

```json
{"name":"demo","homepage":"https://example.com"}
```
