---
id: agy-ag-002
title: "AGY-AG-002: Invalid Antigravity Agent Metadata - antigravity"
sidebar_label: "AGY-AG-002"
description: "agnix rule AGY-AG-002 checks for invalid antigravity agent metadata in antigravity files. Severity: HIGH. See examples and fix guidance."
keywords: ["AGY-AG-002", "invalid antigravity agent metadata", "antigravity", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AGY-AG-002`
- **Severity**: `HIGH`
- **Category**: `antigravity`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `antigravity`
- **Version Range**: `>=1.4.9`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://antigravity.google/docs/subagents
- https://antigravity.google/blog/introducing-custom-agents
- https://antigravity.google/blog/custom-agents-in-google-plugins

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
