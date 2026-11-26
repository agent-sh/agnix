---
id: oc-sk-002
title: "OC-SK-002: Duplicate OpenCode Skill ID - OpenCode"
sidebar_label: "OC-SK-002"
description: "agnix rule OC-SK-002 checks for duplicate opencode skill id in opencode files. Severity: LOW. See examples and fix guidance."
keywords: ["OC-SK-002", "duplicate opencode skill id", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-SK-002`
- **Severity**: `LOW`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/skills/
- https://opencode.ai/docs/skills/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
.claude/skills/release/SKILL.md and .opencode/skills/release/SKILL.md
```

### Valid

```json
.opencode/skills/release/SKILL.md only
```
