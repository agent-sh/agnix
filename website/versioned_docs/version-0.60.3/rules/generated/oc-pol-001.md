---
id: oc-pol-001
title: "OC-POL-001: Invalid Policy Statement - OpenCode"
sidebar_label: "OC-POL-001"
description: "agnix rule OC-POL-001 checks for invalid policy statement in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-POL-001", "invalid policy statement", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-POL-001`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-30`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/policies/
- https://opencode.ai/config.json
- https://opencode.ai/docs/config/
- https://github.com/anomalyco/opencode/blob/v2.2.3/packages/schema/src/config/policy.ts

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"policies": [{"action": "provider.use", "resource": "openai", "effect": "ask"}]}
```

### Valid

```json
{"policies": [{"action": "provider.use", "resource": "openai", "effect": "deny"}]}
```
