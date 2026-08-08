---
id: cc-set-012
title: "CC-SET-012: Invalid sandbox.credentials Setting"
sidebar_label: "CC-SET-012"
description: "agnix rule CC-SET-012 checks for invalid sandbox.credentials setting in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-012", "invalid sandbox.credentials setting", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-012`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-08-08`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `>=2.1.187`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/anthropics/claude-code/releases/tag/v2.1.187
- https://github.com/anthropics/claude-code/releases/tag/v2.1.199
- https://github.com/anthropics/claude-code/releases/tag/v2.1.221
- https://github.com/anthropics/claude-code/releases/tag/v2.1.224
- https://code.claude.com/docs/en/settings
- https://code.claude.com/docs/en/sandboxing

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `false`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{
  "sandbox": {
    "credentials": {
      "files": [
        {"path": "~/.aws/", "mode": "mask", "extract": "password=\\S+", "onExtractNoMatch": "ignore"}
      ],
      "envVars": [
        {"name": "BAD-NAME", "mode": "allow"}
      ],
      "allowPlaintextInject": "false"
    }
  }
}
```

### Valid

```json
{
  "sandbox": {
    "credentials": {
      "files": [
        {"path": "~/.netrc", "mode": "mask", "extract": "password:\\s*(\\S+)", "onExtractNoMatch": "deny", "injectHosts": ["api.example.com"]}
      ],
      "envVars": [
        {"name": "GITHUB_TOKEN", "mode": "mask", "injectHosts": ["api.github.com"]}
      ],
      "allowPlaintextInject": false
    }
  }
}
```
