---
id: ws-hk-002
title: "WS-HK-002: Unknown Cascade Hook Event - windsurf"
sidebar_label: "WS-HK-002"
description: "agnix rule WS-HK-002 checks for unknown cascade hook event in windsurf files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["WS-HK-002", "unknown cascade hook event", "windsurf", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-HK-002`
- **Severity**: `MEDIUM`
- **Category**: `windsurf`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `windsurf`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.devin.ai/desktop/cascade/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"hooks": {"pre_read_file": [{"command": "python3 /path/to/your/script.py"}]}}
```

### Valid

```json
{"hooks": {"pre_read_code": [{"command": "python3 /path/to/your/script.py"}]}}
```
