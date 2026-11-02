---
id: grk-ag-004
title: "GRK-AG-004: Restricted Grok Plugin Agent Field - grok"
sidebar_label: "GRK-AG-004"
description: "agnix rule GRK-AG-004 checks for restricted grok plugin agent field in grok files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["GRK-AG-004", "restricted grok plugin agent field", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-AG-004`
- **Severity**: `MEDIUM`
- **Category**: `grok`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `grok-code`
- **Version Range**: `>=1.0.70`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-agent/src/config.rs
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-pager/docs/user-guide/16-subagents.md

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
