---
id: kr-ag-017
title: "KR-AG-017: Invalid Kiro Markdown Agent Frontmatter"
sidebar_label: "KR-AG-017"
description: "agnix rule KR-AG-017 checks for invalid kiro markdown agent frontmatter in kiro agents files. Severity: HIGH. See examples and fix guidance."
keywords: ["KR-AG-017", "invalid kiro markdown agent frontmatter", "kiro agents", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-AG-017`
- **Severity**: `HIGH`
- **Category**: `Kiro Agents`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `>=2.8.0`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/custom-agents/
- https://kiro.dev/docs/cli/v3/agent-config/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```text
# Reviewer

Review the diff.
```

### Valid

```text
---
description: Reviews pull requests
tools: [read]
---
Review the diff.
```
