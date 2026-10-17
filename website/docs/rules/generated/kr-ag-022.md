---
id: kr-ag-022
title: "KR-AG-022: Reserved or Duplicate Kiro Agent Name"
sidebar_label: "KR-AG-022"
description: "agnix rule KR-AG-022 checks for reserved or duplicate kiro agent name in kiro agents files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-AG-022", "reserved or duplicate kiro agent name", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-022`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Agents`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/
- https://kiro.dev/docs/custom-agents/creating/
- https://kiro.dev/docs/custom-agents/built-in/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
---
name: plan
description: Plans releases
---
Plan the release.
```

### Valid

```text
---
name: release-planner
description: Plans releases
---
Plan the release.
```
