---
id: cdx-cfg-029
title: "CDX-CFG-029: Invalid Agent Concurrency Limit - Codex CLI"
sidebar_label: "CDX-CFG-029"
description: "agnix rule CDX-CFG-029 checks for invalid agent concurrency limit in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-029", "invalid agent concurrency limit", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-029`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.145.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/releases/tag/rust-v0.145.0
- https://github.com/openai/codex/pull/33550
- https://github.com/openai/codex/blob/rust-v0.145.0/codex-rs/config/src/config_toml.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[agents]
max_concurrent_threads_per_session = 0
```

### Valid

```toml
[agents]
max_concurrent_threads_per_session = 4

[features]
multi_agent_v2 = true
```
