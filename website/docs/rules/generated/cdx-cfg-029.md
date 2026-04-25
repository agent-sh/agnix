---
id: cdx-cfg-029
title: "CDX-CFG-029: Incompatible agents.max_threads with multi_agent_v2"
sidebar_label: "CDX-CFG-029"
description: "agnix rule CDX-CFG-029 checks for incompatible agents.max_threads with multi_agent_v2 in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-029", "incompatible agents.max_threads with multi_agent_v2", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-029`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-26`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/pull/19129

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[agents]
max_threads = 4

[features]
multi_agent_v2 = true
```

### Valid

```toml
[agents]
max_threads = 4

[features]
# multi_agent_v2 omitted or false
```
