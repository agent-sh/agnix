---
id: kr-set-012
title: "KR-SET-012: Kiro CLI Settings in Legacy .kiro/settings.json"
sidebar_label: "KR-SET-012"
description: "agnix rule KR-SET-012 checks for kiro cli settings in legacy .kiro/settings.json in kiro settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SET-012", "kiro cli settings in legacy .kiro/settings.json", "kiro settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SET-012`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/changelog/cli/1-23/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
// .kiro/settings.json
{"chat.defaultModel": "claude-sonnet-4.6"}
```

### Valid

```text
// .kiro/settings/cli.json
{"chat.defaultModel": "claude-sonnet-4.6"}
```
