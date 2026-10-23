---
id: cdx-cfg-007
title: "CDX-CFG-007: Full Access Sandbox Posture - Codex CLI"
sidebar_label: "CDX-CFG-007"
description: "agnix rule CDX-CFG-007 checks for full access sandbox posture in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-CFG-007", "full access sandbox posture", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-007`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/sandboxing
- https://learn.chatgpt.com/docs/agent-approvals-security
- https://learn.chatgpt.com/docs/non-interactive-mode
- https://learn.chatgpt.com/docs/config-file/config-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
sandbox_mode = "danger-full-access"
approval_policy = "never"
```

### Valid

```toml
sandbox_mode = "workspace-write"
approval_policy = "on-request"
```
