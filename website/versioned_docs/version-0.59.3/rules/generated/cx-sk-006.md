---
id: cx-sk-006
title: "CX-SK-006: agents/openai.yaml Field Ignored by Codex"
sidebar_label: "CX-SK-006"
description: "agnix rule CX-SK-006 checks for agents/openai.yaml field ignored by codex in codex skills files. Severity: MEDIUM. See examples and fix guidance."
keywords: ["CX-SK-006", "agents/openai.yaml field ignored by codex", "codex skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `CX-SK-006`
- **Severity**: `MEDIUM`
- **Category**: `Codex Skills`
- **Normative Level**: `MUST`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `codex`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://github.com/openai/codex/blob/main/codex-rs/skills/src/interface.rs
- https://github.com/openai/codex/blob/main/codex-rs/skills/src/metadata.rs
- https://github.com/openai/codex/blob/main/codex-rs/skills/src/assets/samples/skill-creator/references/openai_yaml.md

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
interface:
  icon_small: "/abs/icon.svg"
  brand_color: "blue"
```

### Valid

```markdown
interface:
  display_name: "Release Notes"
  icon_small: "./assets/small.svg"
  brand_color: "#3B82F6"
```
