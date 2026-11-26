---
id: cc-sk-006
title: "CC-SK-006: Side-Effect Skill Without disable-model-invocation"
sidebar_label: "CC-SK-006"
description: "agnix rule CC-SK-006 checks for side-effect skill without disable-model-invocation in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-006", "side-effect skill without disable-model-invocation", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-006`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `Yes (unsafe)`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: deploy-staging
description: Use when deploying to staging environment
---
Deploy the application to staging.
```

### Valid

```markdown
---
name: deploy-staging
description: Use when deploying to staging environment
disable-model-invocation: true
---
Deploy the application to staging.
```
