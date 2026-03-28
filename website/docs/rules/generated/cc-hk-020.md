---
id: cc-hk-020
title: "CC-HK-020: HTTP Hook Missing URL - Claude Hooks"
sidebar_label: "CC-HK-020"
description: "agnix rule CC-HK-020 checks for http hook missing url in claude hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-HK-020", "http hook missing url", "claude hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-HK-020`
- **Severity**: `HIGH`
- **Category**: `Claude Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-28`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/hooks

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{ "type": "http", "timeout": 30 }
```

### Valid

```json
{ "type": "http", "url": "https://example.com/hook", "timeout": 30 }
```
