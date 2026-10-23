---
id: cc-mem-021
title: "CC-MEM-021: Directory Tree or File Listing in CLAUDE.md"
sidebar_label: "CC-MEM-021"
description: "agnix rule CC-MEM-021 checks for directory tree or file listing in claude.md in claude memory files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MEM-021", "directory tree or file listing in claude.md", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-021`
- **Severity**: `LOW`
- **Category**: `Claude Memory`
- **Normative Level**: `BEST_PRACTICE`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/best-practices
- https://code.claude.com/docs/en/memory

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

````markdown
# Layout
```
repo/
├── src/
│   ├── api/
... (20+ tree lines)
```
````

### Valid

````markdown
# Layout
- API handlers live in src/api/handlers/ and must not import from src/ui/.
````
