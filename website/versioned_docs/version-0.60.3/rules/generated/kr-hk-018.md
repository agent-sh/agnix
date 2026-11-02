---
id: kr-hk-018
title: "KR-HK-018: Invalid Kiro V3 Hook Confirm Block - Kiro Hooks"
sidebar_label: "KR-HK-018"
description: "agnix rule KR-HK-018 checks for invalid kiro v3 hook confirm block in kiro hooks files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-HK-018", "invalid kiro v3 hook confirm block", "kiro hooks", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-HK-018`
- **Severity**: `HIGH`
- **Category**: `Kiro Hooks`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/hooks/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"version":"v1","hooks":[{"name":"submit","trigger":"Stop","action":{"type":"command","command":"./submit.sh"},"confirm":{"question":"Submit?","options":[]}}]}
```

### Valid

```json
{"version":"v1","hooks":[{"name":"submit","trigger":"Stop","action":{"type":"command","command":"./submit.sh"},"confirm":{"question":"Submit?","options":[{"id":"yes","label":"Yes","run":true}]}}]}
```
