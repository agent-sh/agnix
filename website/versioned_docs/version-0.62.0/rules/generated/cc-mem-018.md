---
id: cc-mem-018
title: "CC-MEM-018: AGENTS.md Not Imported - Claude Memory"
sidebar_label: "CC-MEM-018"
description: "agnix rule CC-MEM-018 checks for agents.md not imported in claude memory files. Severity: LOW. See examples and fix guidance."
keywords: ["CC-MEM-018", "agents.md not imported", "claude memory", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-MEM-018`
- **Severity**: `LOW`
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
# Claude
Read AGENTS.md before making changes.
```

### Valid

```markdown
@AGENTS.md

## Claude Code
Use plan mode for changes under src/billing/.
```
