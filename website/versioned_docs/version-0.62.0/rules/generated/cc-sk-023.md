---
id: cc-sk-023
title: "CC-SK-023: Argument Placeholder Inside Code - Claude Skills"
sidebar_label: "CC-SK-023"
description: "agnix rule CC-SK-023 checks for argument placeholder inside code in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-023", "argument placeholder inside code", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-023`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
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

````markdown
---
name: top-processes
description: Use when checking which processes use the most memory
---
```bash
ps -eo pid,rss | sort -k2 -n | awk '{print $1}'
```
````

### Valid

````markdown
---
name: top-processes
description: Use when checking which processes use the most memory
---
```bash
ps -eo pid,rss | sort -k2 -n | awk '{print \$1}'
```
````
