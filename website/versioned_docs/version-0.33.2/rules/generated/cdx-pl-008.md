---
id: cdx-pl-008
title: "CDX-PL-008: Too Many Default Prompts - Codex CLI"
sidebar_label: "CDX-PL-008"
description: "agnix rule CDX-PL-008 checks that Codex CLI plugins define no more than 3 default prompts. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-008", "too many default prompts", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-008`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-06-04`

## Applicability

- **Tool**: `codex`
- **Version Range**: `>=0.117.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/rust-v0.137.0/codex-rs/core-plugins/src/manifest.rs

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "my-plugin", "interface": {"defaultPrompt": ["a", "b", "c", "d"]}}
```

### Valid

```json
{"name": "my-plugin", "interface": {"defaultPrompt": ["Fix the bug", "Add tests"]}}
```
