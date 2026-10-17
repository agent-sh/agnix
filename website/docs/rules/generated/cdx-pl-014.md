---
id: cdx-pl-014
title: "CDX-PL-014: Missing Required Codex-Format Package Field"
sidebar_label: "CDX-PL-014"
description: "agnix rule CDX-PL-014 checks for missing required codex-format package field in codex cli files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CDX-PL-014", "missing required codex-format package field", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-014`
- **Severity**: `MEDIUM`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/plugins/deploy/submission
- https://developers.openai.com/plugins/build/plugins

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "my-plugin"}
```

### Valid

```json
{"name": "my-plugin", "version": "1.0.0", "description": "Bundle skills.", "author": {"name": "Team"}, "interface": {"composerIcon": "./assets/icon.png", "logo": "./assets/logo.png"}}
```
