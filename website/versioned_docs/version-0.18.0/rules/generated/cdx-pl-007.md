---
id: cdx-pl-007
title: "CDX-PL-007: Component Path Empty Relative - Codex CLI"
sidebar_label: "CDX-PL-007"
description: "agnix rule CDX-PL-007 checks for component path empty relative in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-PL-007", "component path empty relative", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-007`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-01`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.117.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/main/codex-rs/core/src/plugins/manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"components": [{"path": "./"}]}
```

### Valid

```json
{"components": [{"path": "./src/index.js"}]}
```
