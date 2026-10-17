---
id: kr-hk-021
title: "KR-HK-021: Kiro V3 Tool Matcher Uses Non-V3 Tool Names"
sidebar_label: "KR-HK-021"
description: "agnix rule KR-HK-021 checks for kiro v3 tool matcher uses non-v3 tool names in kiro hooks files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-HK-021", "kiro v3 tool matcher uses non-v3 tool names", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-021`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Hooks`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/types/
- https://kiro.dev/docs/cli/v3/hooks-migration/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"guard","trigger":"PreToolUse","matcher":"^Bash$","action":{"type":"command","command":"./guard.sh"}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"guard","trigger":"PreToolUse","matcher":"shell","action":{"type":"command","command":"./guard.sh"}}]}
```
