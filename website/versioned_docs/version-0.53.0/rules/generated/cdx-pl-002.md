---
id: cdx-pl-002
title: "CDX-PL-002: Invalid Plugin Manifest - Codex CLI"
sidebar_label: "CDX-PL-002"
description: "agnix rule CDX-PL-002 checks for invalid plugin manifest in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-002", "invalid plugin manifest", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-002`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-30`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.117.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core-plugins/src/manifest.rs
- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core-plugins/src/agent_plugin_manifest.rs
- https://agent-plugins.org/schemas/1.0.0/plugin.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"my-plugin","homepage":42}
```

### Valid

```json
{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"my-plugin","keywords":["tools"]}
```
