---
id: grk-sbx-002
title: "GRK-SBX-002: Invalid Grok Sandbox Profile - grok"
sidebar_label: "GRK-SBX-002"
description: "agnix rule GRK-SBX-002 checks for invalid grok sandbox profile in grok files. Severity: HIGH. See examples and fix guidance."
keywords: ["GRK-SBX-002", "invalid grok sandbox profile", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-SBX-002`
- **Severity**: `HIGH`
- **Category**: `grok`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `grok-code`
- **Version Range**: `>=1.0.70`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.x.ai/build/features/sandbox
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-config/src/sandbox_mode.rs

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
