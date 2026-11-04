---
id: oc-pm-002
title: "OC-PM-002: Possibly Misspelled Permission Key - OpenCode"
sidebar_label: "OC-PM-002"
description: "agnix rule OC-PM-002 checks for possibly misspelled permission key in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-PM-002", "possibly misspelled permission key", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-PM-002`
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

- https://opencode.ai/docs/agents/
- https://opencode.ai/docs/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permission": {"raed": "allow"}}
```

### Valid

```json
{"permission": {"mymcp_*": "deny"}}
```
