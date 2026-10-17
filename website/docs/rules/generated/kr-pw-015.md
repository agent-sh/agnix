---
id: kr-pw-015
title: "KR-PW-015: Power Without Activation Description or Keywords"
sidebar_label: "KR-PW-015"
description: "agnix rule KR-PW-015 checks for power without activation description or keywords in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-015", "power without activation description or keywords", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-015`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Powers`
- **Normative Level**: `SHOULD`
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
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase"}
```

### Valid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase", "version": "1.0.0", "description": "Build apps with Supabase", "author": {"name": "Supabase"}, "keywords": ["database", "postgres"]}
```
