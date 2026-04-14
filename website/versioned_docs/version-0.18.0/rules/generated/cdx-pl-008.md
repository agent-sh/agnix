---
id: cdx-pl-008
title: "CDX-PL-008: Too Many Default Prompts - Codex CLI"
sidebar_label: "CDX-PL-008"
description: "agnix rule CDX-PL-008 checks for too many default prompts in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-008", "too many default prompts", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-008`
- **Severity**: `MEDIUM`
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
{"default_prompts": ["a","b","c","d","e","f","g","h","i","j","k","l","m","n","o","p","q","r","s","t","u"]}
```

### Valid

```json
{"default_prompts": ["Fix the bug", "Add tests"]}
```
