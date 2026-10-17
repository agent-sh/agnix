---
id: kr-pw-011
title: "KR-PW-011: Missing or Invalid plugin.json Name - Kiro Powers"
sidebar_label: "KR-PW-011"
description: "agnix rule KR-PW-011 checks for missing or invalid plugin.json name in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-011", "missing or invalid plugin.json name", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-011`
- **Severity**: `HIGH`
- **Category**: `Kiro Powers`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.16.2`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/powers/create/
- https://agent-plugins.org/schemas/1.0.0/plugin.schema.json

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "Supabase_Power"}
```

### Valid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase", "version": "1.0.0", "description": "Build apps with Supabase", "author": {"name": "Supabase"}, "keywords": ["database", "postgres"]}
```
