---
id: kr-ag-027
title: "KR-AG-027: Kiro Agent Hook Field Ignored - Kiro Agents"
sidebar_label: "KR-AG-027"
description: "agnix rule KR-AG-027 checks for kiro agent hook field ignored in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-027", "kiro agent hook field ignored", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-027`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/cli/2x-reference/
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"command": "./audit.sh", "toolTypes": ["read"]}]}}
```

### Valid

```json
{"name": "review-agent", "hooks": {"preToolUse": [{"command": "./audit.sh", "matcher": "read"}]}}
```
