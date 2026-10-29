---
id: grk-cfg-003
title: "GRK-CFG-003: Invalid Grok Setting Value - grok"
sidebar_label: "GRK-CFG-003"
description: "agnix rule GRK-CFG-003 checks for invalid grok setting value in grok files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["GRK-CFG-003", "invalid grok setting value", "grok", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `GRK-CFG-003`
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

- https://docs.x.ai/build/settings/reference
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-agent/src/config.rs
- https://x.ai/changelog/build
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-tools/src/implementations/lsp/config.rs
- https://docs.x.ai/build/settings/zdr-video-storage
- https://github.com/xai-org/grok-build/blob/2bdd1d6a6369de0e8c68132ea4539e9abd9e14a8/crates/codegen/xai-grok-tools/src/implementations/grok_build/video_gen/mod.rs

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
