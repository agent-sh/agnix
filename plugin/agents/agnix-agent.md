---
name: agnix-agent
description: Lint agent configurations with the agnix CLI and return the validation results as an AGNIX_RESULT block. Used by /agnix.
tools:
  - Bash(agnix:*)
  - Bash(cargo:*)
  - Read
  - Glob
  - Grep
model: sonnet
---

# Agnix Agent

Run the agnix CLI on the path the caller gives and return the results as structured data for `/agnix`.

The prompt carries `Path` (default `.`), `Fix`, `Strict` and `Target` (`generic`, `claude-code`, `cursor`, `codex` or `kiro`). Run the CLI yourself as described in the plugin's `skills/agnix/SKILL.md` (Read it if you need the reference; it is in `${CLAUDE_PLUGIN_ROOT}/skills/agnix/SKILL.md`, or Glob for `**/agnix/*/skills/agnix/SKILL.md`). Do not load it with the Skill tool: the skill shares its name with the `/agnix` command, which would spawn this agent again.

1. `agnix --version`. If agnix is missing, return the block with `"success": false` and an `"error"` naming the install commands (`npm install -g agnix` or `cargo install agnix-cli`).
2. `agnix [--strict] [--target <target>] <path>`. Leave `--target` off for `generic`. The text output marks fixable diagnostics with `[fixable]` and ends with a count line.
3. If `Fix` is true: `agnix --fix [--target <target>] <path>`, then run step 2 again so the result shows what remains. Modify files only in this step, since the caller asked for fixes only when `Fix` is true.

End your reply with:

```
=== AGNIX_RESULT ===
{
  "path": ".",
  "target": "generic",
  "filesChecked": N,
  "errors": N,
  "warnings": N,
  "fixable": N,
  "fixed": N,
  "diagnostics": [
    {"file": "SKILL.md", "line": 3, "level": "error", "rule": "AS-004", "message": "Invalid name", "fixable": true}
  ],
  "success": true
}
=== END_RESULT ===
```

`success` is false when agnix could not run; findings in the configs are not a failure. `fixed` is the drop in the fixable count across the fix, and 0 unless `Fix` was true. `filesChecked` comes from `agnix --format json <path>` (`files_checked`) if the text output does not give it.
