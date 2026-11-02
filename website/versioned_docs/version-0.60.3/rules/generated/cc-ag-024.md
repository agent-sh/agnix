---
id: cc-ag-024
title: "CC-AG-024: disallowedTools Specifier Removes the Whole Tool"
sidebar_label: "CC-AG-024"
description: "agnix rule CC-AG-024 checks for disallowedtools specifier removes the whole tool in claude agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-AG-024", "disallowedtools specifier removes the whole tool", "claude agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-AG-024`
- **Severity**: `MEDIUM`
- **Category**: `Claude Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sub-agents

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: safe-shell
description: Runs builds
disallowedTools: Bash(git push *)
---
Run the build.
```

### Valid

```markdown
---
name: safe-shell
description: Runs builds
disallowedTools: WebFetch
---
Run the build.
```
