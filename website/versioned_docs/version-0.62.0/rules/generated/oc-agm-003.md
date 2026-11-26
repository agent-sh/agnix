---
id: oc-agm-003
title: "OC-AGM-003: CLAUDE.md Ignored by OpenCode 2 - OpenCode"
sidebar_label: "OC-AGM-003"
description: "agnix rule OC-AGM-003 checks for claude.md ignored by opencode 2 in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-AGM-003", "claude.md ignored by opencode 2", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AGM-003`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `>=2.0.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/instructions/
- https://opencode.ai/v2/docs/migrate-v1/
- https://opencode.ai/docs/rules/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
CLAUDE.md only, in a project with .opencode/
```

### Valid

```json
AGENTS.md (CLAUDE.md may import it)
```
