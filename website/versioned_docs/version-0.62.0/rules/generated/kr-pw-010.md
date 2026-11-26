---
id: kr-pw-010
title: "KR-PW-010: Missing or Unsupported plugin.json $schema"
sidebar_label: "KR-PW-010"
description: "agnix rule KR-PW-010 checks for missing or unsupported plugin.json $schema in kiro powers files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-PW-010", "missing or unsupported plugin.json $schema", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-010`
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
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "supabase", "description": "Build apps with Supabase", "keywords": ["database"]}
```

### Valid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase", "version": "1.0.0", "description": "Build apps with Supabase", "author": {"name": "Supabase"}, "keywords": ["database", "postgres"]}
```
