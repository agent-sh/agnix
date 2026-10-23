---
id: cc-set-034
title: "CC-SET-034: Settings Key Ignored at This Scope"
sidebar_label: "CC-SET-034"
description: "agnix rule CC-SET-034 checks for settings key ignored at this scope in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-034", "settings key ignored at this scope", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-034`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/settings
- https://code.claude.com/docs/en/settings-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
.claude/settings.json:
{"autoMode": {"environment": ["..."]}, "allowedProviders": ["firstParty"]}
```

### Valid

```text
~/.claude/settings.json:
{"autoMode": {"environment": ["Trusted repo: github.com/acme"]}}
```
