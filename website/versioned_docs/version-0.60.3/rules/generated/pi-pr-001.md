---
id: pi-pr-001
title: "PI-PR-001: Invalid Pi Prompt YAML - pi"
sidebar_label: "PI-PR-001"
description: "agnix rule PI-PR-001 checks for invalid pi prompt yaml in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-PR-001", "invalid pi prompt yaml", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-PR-001`
- **Severity**: `HIGH`
- **Category**: `pi`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-29`

## Applicability

- **Tool**: `pi`
- **Version Range**: `>=1.4.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/prompt-templates.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/src/core/prompt-templates.ts

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
