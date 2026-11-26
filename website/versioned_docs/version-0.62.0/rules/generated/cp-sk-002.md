---
id: cp-sk-002
title: "CP-SK-002: Copilot Skill Pre-approves shell or bash"
sidebar_label: "CP-SK-002"
description: "agnix rule CP-SK-002 checks for copilot skill pre-approves shell or bash in copilot skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CP-SK-002", "copilot skill pre-approves shell or bash", "copilot skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CP-SK-002`
- **Severity**: `MEDIUM`
- **Category**: `Copilot Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `github-copilot`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/add-skills
- https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
---
name: image-convert
description: Converts images. Use when asked to convert image formats.
allowed-tools: shell
---
Run scripts/convert.sh.
```

### Valid

```markdown
---
name: image-convert
description: Converts images. Use when asked to convert image formats.
---
Run scripts/convert.sh.
```
