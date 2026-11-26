---
id: grk-hk-002
title: "GRK-HK-002: Unknown Grok Hook Event - grok"
sidebar_label: "GRK-HK-002"
description: "agnix rule GRK-HK-002 checks for unknown grok hook event in grok files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["GRK-HK-002", "unknown grok hook event", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-HK-002`
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

- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-hooks/src/config.rs
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-hooks/src/event.rs
- https://x.ai/changelog/build

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
