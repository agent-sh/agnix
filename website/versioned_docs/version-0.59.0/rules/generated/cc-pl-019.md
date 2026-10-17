---
id: cc-pl-019
title: "CC-PL-019: Invalid Dependency Version Range - Claude Plugins"
sidebar_label: "CC-PL-019"
description: "agnix rule CC-PL-019 checks for invalid dependency version range in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-019", "invalid dependency version range", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-019`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/dependencies
- https://code.claude.com/docs/en/plugins/manifest-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "name": "p",
  "dependencies": [{ "name": "secrets-vault", "version": "latest" }]
}
```

### Valid

```json
{
  "name": "p",
  "dependencies": [{ "name": "secrets-vault", "version": "~2.1.0" }]
}
```
