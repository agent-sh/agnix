---
id: cc-mem-017
title: "CC-MEM-017: Emphasis Overuse - Claude Memory"
sidebar_label: "CC-MEM-017"
description: "agnix rule CC-MEM-017 checks for emphasis overuse in claude memory files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MEM-017", "emphasis overuse", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-017`
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
- https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Rules
- IMPORTANT: run tests
- You MUST lint
- NEVER skip CI
- ALWAYS use pnpm
- CRITICAL: no force push
- MUST update docs
```

### Valid

```markdown
# Rules
- Run cargo test before committing; CI rejects untested changes.
- IMPORTANT: never edit generated/ files by hand.
```
