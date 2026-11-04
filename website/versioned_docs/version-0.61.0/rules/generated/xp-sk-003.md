---
id: xp-sk-003
title: "XP-SK-003: Hardcoded Secret in Skill Files - Cross-Platform"
sidebar_label: "XP-SK-003"
description: "agnix rule XP-SK-003 checks for hardcoded secret in skill files in cross-platform files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["XP-SK-003", "hardcoded secret in skill files", "cross-platform", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `XP-SK-003`
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

- https://geminicli.com/docs/cli/skills-best-practices
- https://docs.cline.bot/getting-started/config

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
TOKEN = "ghp_..."  # a literal token
```

### Valid

```markdown
TOKEN = os.environ["GITHUB_TOKEN"]
```
