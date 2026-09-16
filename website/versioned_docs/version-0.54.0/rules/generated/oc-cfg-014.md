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
- **Verified On**: `2026-07-16`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=1.18.2`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anomalyco/opencode/releases/tag/v1.18.2

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "subagent_depth": -1
}
```

### Valid

```json
{
  "subagent_depth": 2
}
```
