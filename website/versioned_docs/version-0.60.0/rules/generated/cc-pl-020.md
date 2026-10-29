---
id: cc-pl-020
title: "CC-PL-020: Unbraced Plugin Variable in Plugin Content"
sidebar_label: "CC-PL-020"
description: "agnix rule CC-PL-020 checks for unbraced plugin variable in plugin content in claude plugins files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-PL-020", "unbraced plugin variable in plugin content", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-020`
- **Severity**: `MEDIUM`
- **Category**: `Claude Plugins`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/manifest-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
---
name: run
description: Runs the bundled tool. Use when asked to run it.
---
Run `bash $CLAUDE_PLUGIN_ROOT/scripts/run.sh`.
```

### Valid

```json
---
name: run
description: Runs the bundled tool. Use when asked to run it.
---
Run `bash "${CLAUDE_PLUGIN_ROOT}"/scripts/run.sh`.
```
