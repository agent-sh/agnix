---
id: cc-sk-031
title: "CC-SK-031: XML Tag in Skill Name or Description"
sidebar_label: "CC-SK-031"
description: "agnix rule CC-SK-031 checks for xml tag in skill name or description in claude skills files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-SK-031", "xml tag in skill name or description", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-031`
- **Severity**: `LOW`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://platform.claude.com/docs/en/build-with-claude/skills-guide

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: formatter
description: Formats <output> blocks. Use when formatting.
---
Format.
```

### Valid

```markdown
---
name: formatter
description: Formats output blocks. Use when formatting.
---
Format.
```
