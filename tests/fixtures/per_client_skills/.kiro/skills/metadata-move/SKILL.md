---
name: metadata-move
description: Use when testing how KR-SK-001 moves extra Kiro skill fields under metadata
version: 1.0.0
tags:
- skill
- testing
argument-hint: "[path] [--fix]"
metadata:
  author: agnix
---
# KR-SK-001 Fix Fixture

Kiro reads only name, description, license, compatibility and metadata.
`version` moves under metadata (safe), `argument-hint` moves only in unsafe
mode (another client's field) and the `tags` list is deleted only in unsafe mode.
