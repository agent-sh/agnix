---
id: cc-sk-014
title: "CC-SK-014: Invalid disable-model-invocation Type"
sidebar_label: "CC-SK-014"
description: "agnix rule CC-SK-014 checks for invalid disable-model-invocation type in claude skills files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-SK-014", "invalid disable-model-invocation type", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-014`
- **Severity**: `HIGH`
- **Category**: `Claude Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-07-26`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.218`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/skills
- https://github.com/anthropics/claude-code/releases/tag/v2.1.218

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: quiet-skill
description: Use when running without model invocation
disable-model-invocation: "maybe"
---
Run silently.
```

### Valid

```markdown
---
name: quiet-skill
description: Use when running without model invocation
disable-model-invocation: "YES"
---
Run silently.
```
