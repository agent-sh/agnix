---
id: kr-ag-002
title: "KR-AG-002: Invalid Kiro Agent Resource Protocol"
sidebar_label: "KR-AG-002"
description: "agnix rule KR-AG-002 checks for invalid kiro agent resource protocol in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-002", "invalid kiro agent resource protocol", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-002`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/custom-agents/creating

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "resources": ["https://example.com/private"]
}
```

### Valid

```json
{
  "resources": ["file://docs/architecture.md", "skill://deploy"]
}
```
