---
id: cc-sk-021
title: "CC-SK-021: Hardcoded User Directory Path - Claude Skills"
sidebar_label: "CC-SK-021"
description: "agnix rule CC-SK-021 checks for hardcoded user directory path in claude skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SK-021", "hardcoded user directory path", "claude skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SK-021`
- **Severity**: `MEDIUM`
- **Category**: `Claude Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-04-26`

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
name: build-helper
description: Build the project
---
Run the script at /Users/alice/projects/myrepo/build.sh to compile.
```

### Valid

```markdown
---
name: build-helper
description: Build the project
---
Run `cargo build` from the repo root, or set `$BUILD_DIR` to override.
```
