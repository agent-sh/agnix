---
id: agy-pl-002
title: "AGY-PL-002: Ignored Antigravity Plugin Metadata"
sidebar_label: "AGY-PL-002"
description: "agnix rule AGY-PL-002 checks for ignored antigravity plugin metadata in antigravity files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["AGY-PL-002", "ignored antigravity plugin metadata", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-PL-002`
- **Severity**: `MEDIUM`
- **Category**: `antigravity`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `antigravity`
- **Version Range**: `>=1.4.9`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://antigravity.google/docs/plugins
- https://antigravity.google/schemas/v1/plugin.json

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
