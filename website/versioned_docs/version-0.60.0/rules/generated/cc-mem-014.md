---
id: cc-mem-014
title: "CC-MEM-014: Instruction File Exceeds 200 Lines"
sidebar_label: "CC-MEM-014"
description: "agnix rule CC-MEM-014 checks for instruction file exceeds 200 lines in claude memory files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-MEM-014", "instruction file exceeds 200 lines", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-014`
- **Severity**: `MEDIUM`
- **Category**: `Claude Memory`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/memory

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# Project

(250 lines of instructions, including guidance for one subdirectory)
```

### Valid

```markdown
# Project

Build: cargo build
Test: cargo test
(under 200 lines; part-of-codebase rules live in .claude/rules/ with paths)
```
