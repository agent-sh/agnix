---
id: ws-hk-003
title: "WS-HK-003: Cascade Hook Skipped or Option Ignored - windsurf"
sidebar_label: "WS-HK-003"
description: "agnix rule WS-HK-003 checks for cascade hook skipped or option ignored in windsurf files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["WS-HK-003", "cascade hook skipped or option ignored", "windsurf", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `WS-HK-003`
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
{"hooks": {"pre_run_command": [{"powershell": "python3 C:\\hooks\\check.py", "working_directory": "~/project"}]}}
```

### Valid

```json
{"hooks": {"pre_run_command": [{"command": "python3 /opt/hooks/check.py", "powershell": "python3 C:\\hooks\\check.py"}]}}
```
