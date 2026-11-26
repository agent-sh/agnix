---
id: oc-cfg-001
title: "OC-CFG-001: Invalid Model Format - OpenCode"
sidebar_label: "OC-CFG-001"
description: "agnix rule OC-CFG-001 checks for invalid model format in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-001", "invalid model format", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-001`
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

- https://opencode.ai/config.json
- https://opencode.ai/v2/docs/agents/
- https://opencode.ai/docs/agents/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"model": "gpt-5"}
```

### Valid

```json
{"model": "anthropic/claude-sonnet-4-5"}
```
