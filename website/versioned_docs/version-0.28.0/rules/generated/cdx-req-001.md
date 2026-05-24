---
id: cdx-req-001
title: "CDX-REQ-001: Unknown Codex requirements.toml Key - Codex CLI"
sidebar_label: "CDX-REQ-001"
description: "agnix rule CDX-REQ-001 checks for unknown codex requirements.toml key in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-REQ-001", "unknown codex requirements.toml key", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-REQ-001`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-05-24`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.133.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.133.0/codex-rs/config/src/config_requirements.rs
- https://github.com/openai/codex/blob/rust-v0.133.0/docs/config.md

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
allowed_sandbox_mode = ["read-only"]
```

### Valid

```toml
allowed_sandbox_modes = ["read-only"]
```
