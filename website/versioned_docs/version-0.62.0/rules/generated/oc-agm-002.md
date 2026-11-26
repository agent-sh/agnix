---
id: oc-agm-002
title: "OC-AGM-002: Secrets in AGENTS.md - OpenCode"
sidebar_label: "OC-AGM-002"
description: "agnix rule OC-AGM-002 checks for secrets in agents.md in opencode files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["OC-AGM-002", "secrets in agents.md", "opencode", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `OC-AGM-002`
- **Severity**: `MEDIUM`
- **Category**: `OpenCode`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `opencode`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://opencode.ai/v2/docs/instructions/
- https://opencode.ai/docs/rules/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
Deploy token: ghp_...
```

### Valid

```json
Use `$GITHUB_TOKEN` from the environment.
```
