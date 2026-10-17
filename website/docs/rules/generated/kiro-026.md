---
id: kiro-026
title: "KIRO-026: Kiro Configuration File Never Loaded"
sidebar_label: "KIRO-026"
description: "agnix rule KIRO-026 checks for kiro configuration file never loaded in kiro steering files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KIRO-026", "kiro configuration file never loaded", "kiro steering", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KIRO-026`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Steering`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/configuration/
- https://kiro.dev/docs/custom-agents/
- https://kiro.dev/docs/hooks/
- https://kiro.dev/docs/steering/
- https://kiro.dev/docs/reference/settings/
- https://kiro.dev/docs/permissions/

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `true`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
.kiro/agents/reviewer.JSON
.kiro/settings/permissions.yml
.kiro/settings.json
```

### Valid

```markdown
.kiro/agents/reviewer.json
.kiro/settings/permissions.yaml
.kiro/settings/cli.json
```
