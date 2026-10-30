---
id: cdx-req-003
title: "CDX-REQ-003: Managed Hook Script Outside managed_dir"
sidebar_label: "CDX-REQ-003"
description: "agnix rule CDX-REQ-003 checks for managed hook script outside managed_dir in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-REQ-003", "managed hook script outside managed_dir", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-REQ-003`
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
- https://learn.chatgpt.com/docs/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[hooks]
managed_dir = "/enterprise/hooks"

[[hooks.PreToolUse]]
[[hooks.PreToolUse.hooks]]
type = "command"
command = "python3 ./policy.py"
```

### Valid

```toml
[hooks]
managed_dir = "/enterprise/hooks"

[[hooks.PreToolUse]]
[[hooks.PreToolUse.hooks]]
type = "command"
command = "python3 /enterprise/hooks/policy.py"
```
