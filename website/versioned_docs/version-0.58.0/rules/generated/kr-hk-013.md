---
id: kr-hk-013
title: "KR-HK-013: Invalid Kiro V3 Hook Action - Kiro Hooks"
sidebar_label: "KR-HK-013"
description: "agnix rule KR-HK-013 checks for invalid kiro v3 hook action in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-013", "invalid kiro v3 hook action", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-013`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-16`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.26.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/hooks/actions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "ctx", "trigger": "UserPromptSubmit", "action": {"type": "http", "url": "https://x.example"}}
```

### Valid

```json
{"name": "ctx", "trigger": "UserPromptSubmit", "action": {"type": "agent", "prompt": "Add repo context"}}
```
