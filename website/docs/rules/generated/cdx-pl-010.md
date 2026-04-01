---
id: cdx-pl-010
title: "CDX-PL-010: Empty Default Prompt Entry - Codex CLI"
sidebar_label: "CDX-PL-010"
description: "agnix rule CDX-PL-010 checks for empty default prompt entry in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-010", "empty default prompt entry", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-010`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
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
{"default_prompts": ["Fix the bug", "", "Add tests"]}
```

### Valid

```json
{"default_prompts": ["Fix the bug", "Add tests"]}
```
