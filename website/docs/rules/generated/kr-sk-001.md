---
id: kr-sk-001
title: "KR-SK-001: Kiro Skill Uses Unsupported Field - Kiro Skills"
sidebar_label: "KR-SK-001"
description: "agnix rule KR-SK-001 checks for kiro skill uses unsupported field in kiro skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["KR-SK-001", "kiro skill uses unsupported field", "kiro skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `KR-SK-001`
- **Severity**: `MEDIUM`
- **Category**: `Kiro Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (safe/unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `kiro`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://kiro.dev/docs/skills/
- https://agentskills.io/specification

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: my-skill
description: A useful development skill
version: 1.0.0
model: haiku
---
# My Skill

Skill instructions here.
```

### Valid

```markdown
---
name: my-skill
description: A useful development skill
metadata:
  version: 1.0.0
---
# My Skill

Skill instructions here.
```
