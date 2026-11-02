---
id: pi-kb-001
title: "PI-KB-001: Invalid Pi Keybinding - pi"
sidebar_label: "PI-KB-001"
description: "agnix rule PI-KB-001 checks for invalid pi keybinding in pi files. Severity: HIGH. See examples and fix guidance."
keywords: ["PI-KB-001", "invalid pi keybinding", "pi", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `PI-KB-001`
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

- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/docs/keybindings.md
- https://github.com/earendil-works/pi/blob/v1.4.0/packages/coding-agent/schemas/keybindings.schema.json

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
