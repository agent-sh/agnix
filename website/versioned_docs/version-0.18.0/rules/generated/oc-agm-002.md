---
id: oc-agm-002
title: "OC-AGM-002: Secrets in AGENTS.md - OpenCode"
sidebar_label: "OC-AGM-002"
description: "agnix rule OC-AGM-002 checks for secrets in agents.md in opencode files. Severity: HIGH. See examples and fix guidance."
keywords: ["OC-AGM-002", "secrets in agents.md", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AGM-002`
- **Severity**: `HIGH`
- **Category**: `OpenCode`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-02`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/docs/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
# AGENTS

API_KEY=sk-123456
```

### Valid

```json
# AGENTS

Use environment variables.
```
