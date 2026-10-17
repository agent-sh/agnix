---
name: status
description: Shows branch status. Use when asked for status.
---
Branch: !`git branch --show-current`
BRANCH=!`git rev-parse HEAD`
