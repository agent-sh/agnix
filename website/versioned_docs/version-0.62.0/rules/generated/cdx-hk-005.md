---
id: cdx-hk-005
title: "CDX-HK-005: Unknown Codex Hook Event - Codex CLI"
sidebar_label: "CDX-HK-005"
description: "agnix rule CDX-HK-005 checks for unknown codex hook event in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-HK-005", "unknown codex hook event", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-HK-005`
- **Severity**: `MEDIUM`
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
{"hooks": {"Notification": []}}
```

### Valid

```json
{"hooks": {"PreToolUse": []}}
```
