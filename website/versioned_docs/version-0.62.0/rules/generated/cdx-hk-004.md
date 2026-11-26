---
id: cdx-hk-004
title: "CDX-HK-004: additionalContextLimit Ineffective or Unbounded"
sidebar_label: "CDX-HK-004"
description: "agnix rule CDX-HK-004 checks for additionalcontextlimit ineffective or unbounded in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-HK-004", "additionalcontextlimit ineffective or unbounded", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-HK-004`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/hooks
- https://github.com/openai/codex/blob/main/codex-rs/config/src/hook_config.rs
- https://github.com/openai/codex/blob/main/codex-rs/hooks/src/engine/discovery.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "./ctx.sh", "additionalContextLimit": 0}]}]}}
```

### Valid

```json
{"hooks": {"SessionStart": [{"hooks": [{"type": "command", "command": "./ctx.sh", "additionalContextLimit": 5000}]}]}}
```
