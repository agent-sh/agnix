---
id: cc-pl-022
title: "CC-PL-022: Monitor Command References user_config"
sidebar_label: "CC-PL-022"
description: "agnix rule CC-PL-022 checks for monitor command references user_config in claude plugins files. Severity: HIGH. See examples and fix guidance."
keywords: ["CC-PL-022", "monitor command references user_config", "claude plugins", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-PL-022`
- **Severity**: `HIGH`
- **Category**: `Claude Plugins`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/plugins/components
- https://code.claude.com/docs/en/plugins/manifest-reference

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "experimental": {
    "monitors": [{ "name": "deploys", "command": "poll ${user_config.url}", "description": "Deploy status" }]
  }
}
```

### Valid

```json
{
  "experimental": {
    "monitors": [{ "name": "deploys", "command": "\"${CLAUDE_PLUGIN_ROOT}\"/scripts/poll.sh", "description": "Deploy status" }]
  }
}
```
