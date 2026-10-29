---
id: grk-sk-001
title: "GRK-SK-001: Grok Skill Tool Policy Advisory - grok"
sidebar_label: "GRK-SK-001"
description: "agnix rule GRK-SK-001 checks for grok skill tool policy advisory in grok files. Severity: LOW. See examples and fix guidance."
keywords: ["GRK-SK-001", "grok skill tool policy advisory", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-SK-001`
- **Severity**: `LOW`
- **Category**: `grok`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `grok-code`
- **Version Range**: `>=1.0.70`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-pager/docs/user-guide/08-skills.md

## Test Coverage Metadata

- Unit tests: `false`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
Configuration omitted required fields for this rule.
```

### Valid

```text
Configuration includes required fields and follows the rule.
```
