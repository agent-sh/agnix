---
id: cdx-pl-001
title: "CDX-PL-001: Codex Plugin Manifest Location or Agent Plugins Schema"
sidebar_label: "CDX-PL-001"
description: "agnix rule CDX-PL-001 checks for codex plugin manifest location or agent plugins schema in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-001", "codex plugin manifest location or agent plugins schema", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-001`
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

- https://github.com/openai/codex/blob/rust-v0.146.0/codex-rs/core-plugins/src/agent_plugin_manifest.rs
- https://agent-plugins.org/schemas/1.0.0/plugin.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
root plugin.json with unsupported $schema https://agent-plugins.org/schemas/2.0.0/plugin.schema.json
```

### Valid

```toml
.codex-plugin/plugin.json, or root plugin.json with $schema https://agent-plugins.org/schemas/1.0.0/plugin.schema.json
```
