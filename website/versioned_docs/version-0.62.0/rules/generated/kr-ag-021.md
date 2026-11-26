---
id: kr-ag-021
title: "KR-AG-021: Kiro Agent file:// Prompt Cannot Be Loaded"
sidebar_label: "KR-AG-021"
description: "agnix rule KR-AG-021 checks for kiro agent file:// prompt cannot be loaded in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-021", "kiro agent file:// prompt cannot be loaded", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-021`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/configuration-reference/
- https://kiro.dev/docs/cli/v3/agent-config/
- https://kiro.dev/docs/cli/v3/upgrade-agent/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "prompt": "file://~/prompts/review.md", "permissions": {"rules": []}}
```

### Valid

```json
{"name": "review-agent", "prompt": "file://./prompts/review.md"}
```
