---
id: cdx-req-002
title: "CDX-REQ-002: Requirements Rule Without Prompt or Forbidden Decision"
sidebar_label: "CDX-REQ-002"
description: "agnix rule CDX-REQ-002 checks for requirements rule without prompt or forbidden decision in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-REQ-002", "requirements rule without prompt or forbidden decision", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-REQ-002`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://learn.chatgpt.com/docs/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[rules]
prefix_rules = [{ pattern = [{ token = "rm" }] }]
```

### Valid

```toml
[rules]
prefix_rules = [{ pattern = [{ token = "rm" }], decision = "forbidden" }]
```
