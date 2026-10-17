---
name: test-skill
description: Use when testing per-client skill rules
hooks: some-value
---
# CX-SK-001 Test Fixture

This skill has a field Codex CLI ignores (hooks). The CX-SK-001 rule should
flag it with an unsafe fix, since hooks is a field another client acts on.
