---
name: argument-placeholder-in-code
description: Use when listing the processes that use the most memory
---
List the top processes by memory:

```bash
ps -eo pid,rss | sort -k2 -n | tail -5 | awk '{print $1}'
```
