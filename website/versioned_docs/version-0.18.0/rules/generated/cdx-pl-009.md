---
id: cdx-pl-009
title: "CDX-PL-009: Default Prompt Too Long - Codex CLI"
sidebar_label: "CDX-PL-009"
description: "agnix rule CDX-PL-009 checks for default prompt too long in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-009", "default prompt too long", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-009`
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
{"default_prompts": ["<500+ character prompt string>"]}
```

### Valid

```json
{"default_prompts": ["Fix the login bug in auth module"]}
```
