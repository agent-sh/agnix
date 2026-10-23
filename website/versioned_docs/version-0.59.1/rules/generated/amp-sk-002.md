---
id: amp-sk-002
title: "AMP-SK-002: Amp Skill Masked by an .agents/skills Copy"
sidebar_label: "AMP-SK-002"
description: "agnix rule AMP-SK-002 checks for amp skill masked by an .agents/skills copy in amp skills files. Severity: LOW. See examples and fix guidance."
keywords: ["AMP-SK-002", "amp skill masked by an .agents/skills copy", "amp skills", "validation", "agnix", "linter"]
---

## Summary

- **Rule ID**: `AMP-SK-002`
- **Severity**: `LOW`
- **Category**: `Amp Skills`
- **Normative Level**: `SHOULD`
- **Auto-Fix**: `No`
- **Verified On**: `2026-10-17`

## Applicability

- **Tool**: `amp`
- **Version Range**: `unspecified`
- **Spec Revision**: `unspecified`

## Evidence Sources

- https://ampcode.com/docs/customize/skills

## Test Coverage Metadata

- Unit tests: `true`
- Fixture tests: `true`
- E2E tests: `false`

## Examples

The following examples demonstrate what triggers this rule and how to fix it.

### Invalid

```markdown
# .claude/skills/deploy/SKILL.md
---
name: deploy
description: Deploys the app. Use when deploying.
---

(.agents/skills/deploy/SKILL.md, with different content, also has name: deploy; Amp loads that one.)
```

### Valid

```markdown
# .claude/skills/deploy/SKILL.md
---
name: deploy-claude
description: Deploys the app. Use when deploying.
---

(.agents/skills/deploy/SKILL.md names its skill 'deploy', or is byte-identical.)
```
