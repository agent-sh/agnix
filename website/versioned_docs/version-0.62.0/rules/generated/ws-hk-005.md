---
id: ws-hk-005
title: "WS-HK-005: Secret in Workspace Hook Command - windsurf"
sidebar_label: "WS-HK-005"
description: "agnix rule WS-HK-005 checks for secret in workspace hook command in windsurf files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["WS-HK-005", "secret in workspace hook command", "windsurf", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-HK-005`
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
{"hooks": {"post_cascade_response": [{"command": "/usr/local/bin/notify --token hk_9f8e7d6c5b4a3f2e1d0c"}]}}
```

### Valid

```json
{"hooks": {"post_cascade_response": [{"command": "/usr/local/bin/notify --token $NOTIFY_TOKEN"}]}}
```
