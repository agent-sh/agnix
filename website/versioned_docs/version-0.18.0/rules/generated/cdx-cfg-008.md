---
id: cdx-cfg-008
title: "CDX-CFG-008: Invalid shell_environment_policy.inherit Value"
sidebar_label: "CDX-CFG-008"
description: "agnix rule CDX-CFG-008 checks for invalid shell_environment_policy.inherit value in codex cli files. Severity: HIGH. See examples and fix guidance."
keywords: ["CDX-CFG-008", "invalid shell_environment_policy.inherit value", "codex cli", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CDX-CFG-008`
- **Severity**: `HIGH`
- **Category**: `Codex CLI`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-03-03`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://developers.openai.com/codex/config-reference
- https://developers.openai.com/codex/config-schema.json
- https://developers.openai.com/codex/enterprise/managed-configuration

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```toml
[shell_environment_policy]
inherit = "system"
```

### Valid

```toml
[shell_environment_policy]
inherit = "core"
```
