---
id: cdx-hk-000
title: "CDX-HK-000: Codex Hooks File Fails to Load - Codex CLI"
sidebar_label: "CDX-HK-000"
description: "agnix rule CDX-HK-000 checks for codex hooks file fails to load in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-HK-000", "codex hooks file fails to load", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-HK-000`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
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
{"version": 1, "hooks": {"Stop": [{"hooks": [{"type": "command", "command": "./stop.sh", "timeout": "30"}]}]}}
```

### Valid

```json
{"hooks": {"Stop": [{"hooks": [{"type": "command", "command": "./stop.sh"}]}]}}
```
