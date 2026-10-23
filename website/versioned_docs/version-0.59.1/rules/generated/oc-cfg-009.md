---
id: oc-cfg-009
title: "OC-CFG-009: Invalid Compaction Token Budget - OpenCode"
sidebar_label: "OC-CFG-009"
description: "agnix rule OC-CFG-009 checks for invalid compaction token budget in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-CFG-009", "invalid compaction token budget", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-CFG-009`
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
- https://opencode.ai/v2/docs/migrate-v1/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"compaction": {"reserved": -1}}
```

### Valid

```json
{"compaction": {"keep": {"tokens": 8000}, "buffer": 20000}}
```
