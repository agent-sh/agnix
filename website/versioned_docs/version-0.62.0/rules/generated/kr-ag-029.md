---
id: kr-ag-029
title: "KR-AG-029: Overly Broad Kiro Agent Resource Glob"
sidebar_label: "KR-AG-029"
description: "agnix rule KR-AG-029 checks for overly broad kiro agent resource glob in kiro agents files. Severity: LOW. See examples and fix guidance."
keywords: ["KR-AG-029", "overly broad kiro agent resource glob", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-029`
- **Severity**: `LOW`
- **Category**: `Kiro Agents`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/cli/chat/context/
- https://kiro.dev/docs/custom-agents/configuration-reference/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"name": "review-agent", "resources": ["file://**"]}
```

### Valid

```json
{"name": "review-agent", "resources": ["file://docs/**/*.md", "file://README.md"]}
```
