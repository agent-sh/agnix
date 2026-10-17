---
id: kr-pw-013
title: "KR-PW-013: plugin.json Value Kiro Ignores - Kiro Powers"
sidebar_label: "KR-PW-013"
description: "agnix rule KR-PW-013 checks for plugin.json value kiro ignores in kiro powers files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-PW-013", "plugin.json value kiro ignores", "kiro powers", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-PW-013`
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
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase", "author": "Supabase", "keywords": "database"}
```

### Valid

```json
{"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json", "name": "supabase", "version": "1.0.0", "description": "Build apps with Supabase", "author": {"name": "Supabase"}, "keywords": ["database", "postgres"]}
```
