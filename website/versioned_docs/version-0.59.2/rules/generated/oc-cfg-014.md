---
id: oc-cfg-014
title: "OC-CFG-014: Invalid subagent_depth Value - OpenCode"
sidebar_label: "OC-CFG-014"
description: "agnix rule OC-CFG-014 checks for invalid subagent_depth value in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-CFG-014", "invalid subagent_depth value", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-014`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config/
- https://opencode.ai/config.json
- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"subagent_depth": -1}
```

### Valid

```json
{"subagent_depth": 2}
```
