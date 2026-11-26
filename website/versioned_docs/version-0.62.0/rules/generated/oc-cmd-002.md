---
id: oc-cmd-002
title: "OC-CMD-002: JSON Command Without Template - OpenCode"
sidebar_label: "OC-CMD-002"
description: "agnix rule OC-CMD-002 checks for json command without template in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CMD-002", "json command without template", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CMD-002`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/commands/
- https://opencode.ai/v2/docs/commands/
- https://opencode.ai/config.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"command": {"test": {"description": "Run tests"}}}
```

### Valid

```json
{"command": {"test": {"template": "Run the tests."}}}
```
