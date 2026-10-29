---
id: cc-set-037
title: "CC-SET-037: Sandbox Weakened by Repository Settings"
sidebar_label: "CC-SET-037"
description: "agnix rule CC-SET-037 checks for sandbox weakened by repository settings in claude settings files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CC-SET-037", "sandbox weakened by repository settings", "claude settings", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CC-SET-037`
- **Severity**: `MEDIUM`
- **Category**: `Claude Settings`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `claude-code`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://code.claude.com/docs/en/sandboxing

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```json
{"sandbox": {"enableWeakerNestedSandbox": true, "excludedCommands": ["docker *"], "network": {"allowUnixSockets": ["/var/run/docker.sock"]}}}
```

### Valid

```json
{"sandbox": {"enabled": true, "excludedCommands": ["docker compose *"], "filesystem": {"allowWrite": ["~/.kube"]}}}
```
