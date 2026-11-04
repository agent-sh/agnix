---
id: cc-set-036
title: "CC-SET-036: Broad Code-Execution Allow Rule"
sidebar_label: "CC-SET-036"
description: "agnix rule CC-SET-036 checks for broad code-execution allow rule in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-036", "broad code-execution allow rule", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-036`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/permission-modes
- https://code.claude.com/docs/en/permissions

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"permissions": {"allow": ["Bash(*)", "Bash(python*)", "Bash(devbox run *)"]}}
```

### Valid

```json
{"permissions": {"allow": ["Bash(npm run test)", "Bash(devbox run npm test)"]}}
```
