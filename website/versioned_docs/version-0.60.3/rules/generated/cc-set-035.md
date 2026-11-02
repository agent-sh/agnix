---
id: cc-set-035
title: "CC-SET-035: Ineffective Permission Rule Syntax"
sidebar_label: "CC-SET-035"
description: "agnix rule CC-SET-035 checks for ineffective permission rule syntax in claude settings files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SET-035", "ineffective permission rule syntax", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-035`
- **Severity**: `HIGH`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/permissions
- https://code.claude.com/docs/en/errors

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permissions": {"allow": ["Bash(git * main)", "mcp__*", "Bash(ls) x"], "deny": ["Bash(command:rm *)", "Read(/Users/alice/secrets/**)"]}}
```

### Valid

```json
{"permissions": {"allow": ["Bash(git status *)", "mcp__github__*"], "deny": ["Read(//Users/alice/secrets/**)"]}}
```
