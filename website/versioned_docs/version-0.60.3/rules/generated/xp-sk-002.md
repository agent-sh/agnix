---
id: xp-sk-002
title: "XP-SK-002: Hardcoded User Directory Path In Skill"
sidebar_label: "XP-SK-002"
description: "agnix rule XP-SK-002 checks for hardcoded user directory path in skill in cross-platform files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["XP-SK-002", "hardcoded user directory path in skill", "cross-platform", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `XP-SK-002`
- **Severity**: `MEDIUM`
- **Category**: `Cross-Platform`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `all`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

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
name: build-helper
description: Build the project
---
Run /home/alice/skills/build-helper/scripts/build.sh to compile.
```

### Valid

```markdown
---
name: build-helper
description: Build the project
---
Run scripts/build.sh from the skill root.
```
