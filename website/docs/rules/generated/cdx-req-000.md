---
id: cdx-req-000
title: "CDX-REQ-000: Codex requirements.toml TOML Parse Error"
sidebar_label: "CDX-REQ-000"
description: "agnix rule CDX-REQ-000 checks for codex requirements.toml toml parse error in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-REQ-000", "codex requirements.toml toml parse error", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-REQ-000`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-02`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.133.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.136.0/codex-rs/config/src/config_requirements.rs
- https://github.com/openai/codex/blob/rust-v0.136.0/codex-rs/config/src/loader/mod.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
allowed_sandbox_modes = [unclosed
```

### Valid

```toml
allow_managed_hooks_only = true
```
