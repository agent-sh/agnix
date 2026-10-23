---
id: cdx-pl-018
title: "CDX-PL-018: Plugin Exceeds Public Submission Limits"
sidebar_label: "CDX-PL-018"
description: "agnix rule CDX-PL-018 checks for plugin exceeds public submission limits in codex cli files. Severity: LOW. See examples and fix guidance."
keywords: ["CDX-PL-018", "plugin exceeds public submission limits", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-PL-018`
- **Severity**: `LOW`
- **Category**: `Codex CLI`
- **Normative Level**: `SHOULD`
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
{"name": "Notes_Plugin", "interface": {"displayName": "A display name over thirty characters", "developerName": "Acme"}}
```

### Valid

```json
{"name": "notes", "interface": {"displayName": "Notes", "developerName": "Acme"}}
```
