# agnix Validation Rules - Master Reference

> Consolidated from 320KB knowledge base, 75+ sources, 5 research agents

**Last Updated**: 2026-02-27
**Coverage**: Agent Skills • MCP • Claude Code • Cursor • Multi-Platform • Prompt Engineering

---

## Rule Format

```
[RULE-ID] [CERTAINTY] Rule description
  ├─ Detection: How to detect
  ├─ Fix: Auto-fix if available
  └─ Source: Citation
```

**Certainty Levels**:
- **HIGH**: >95% true positive, always report, auto-fix safe
- **MEDIUM**: 75-95% true positive, report in default mode
- **LOW**: <75% true positive, verbose mode only

---

## Evidence Metadata Schema

Each rule in `knowledge-base/rules.json` includes an `evidence` object that documents the authoritative source, applicability, and test coverage. This metadata enables:

- **Traceability**: Link rules to their source specifications or research
- **Filtering**: Apply rules only to relevant tools/versions
- **Quality assurance**: Track test coverage for each rule

### Evidence Fields

| Field | Type | Description |
|-------|------|-------------|
| `source_type` | enum | Classification: `spec`, `vendor_docs`, `vendor_code`, `paper`, `community` |
| `source_urls` | string[] | URLs to authoritative documentation or specifications |
| `verified_on` | string | ISO 8601 date when the source was last verified (YYYY-MM-DD) |
| `applies_to` | object | Tool/version/spec constraints for when the rule applies |
| `normative_level` | enum | RFC 2119 level: `MUST`, `SHOULD`, `BEST_PRACTICE` |
| `tests` | object | Test coverage: `{ unit: bool, fixtures: bool, e2e: bool }` |

### Source Types

| Type | Description | Examples |
|------|-------------|----------|
| `spec` | Official specification | agentskills.io/specification, modelcontextprotocol.io/specification |
| `vendor_docs` | Vendor documentation | code.claude.com/docs, docs.github.com/copilot, docs.cursor.com |
| `vendor_code` | Vendor source code | Reference implementations |
| `paper` | Academic research | Liu et al. (2023) TACL, Wei et al. (2022) |
| `community` | Community research | agentsys, multi-platform patterns |

### Applicability Constraints

The `applies_to` object specifies when a rule is relevant:

```json
{
  "applies_to": {
    "tool": "claude-code",       // Optional: specific tool
    "version_range": ">=1.0.0", // Optional: semver range
    "spec_revision": "2025-11-25" // Optional: spec version
  }
}
```

Rules with an empty `applies_to` object (`{}`) apply universally.

### Security Metadata

Security-relevant rules may include a `security` object in `knowledge-base/rules.json`. These fields feed SARIF rule properties and run taxonomies:

| Field | Type | Description |
|-------|------|-------------|
| `cwe` | string[] | CWE IDs such as CWE ID 798 |
| `owasp` | string[] | OWASP Top 10 IDs such as `A07:2021` |
| `vulnerability_class` | string | Short class such as `hardcoded-secret` or `command-injection` |
| `subcategory` | enum | `vuln`, `audit`, or `secure-default` |
| `confidence` | enum | `HIGH`, `MEDIUM`, or `LOW` |
| `likelihood` | enum | `HIGH`, `MEDIUM`, or `LOW` |
| `impact` | enum | `HIGH`, `MEDIUM`, or `LOW` |

### Lifecycle Metadata

Rules are active by default. Deprecated rules should include `status`, `deprecated_since`, `replaced_by`, and `reason` fields in `rules.json`. Removed rule IDs are tracked in `knowledge-base/removed-rules.json` so `.agnix.toml` validation can warn on stale suppressions and point users at replacement rules.

`CC-SK-009`, `CC-SK-012`, and `CC-HK-004` were retired in 0.57.0. There is no documented three-injection cap, arguments are appended when no placeholder receives them, and CC-HK-018 covers ignored matchers.

`CC-SK-011` was retired in 0.56.5. A user-typed inline `/skill-name` mention can authorize Claude to invoke a skill with both `user-invocable: false` and `disable-model-invocation: true`. The flags still restrict direct slash invocation and automatic model invocation, respectively. No reachability diagnostic or permission-changing auto-fix is emitted. See [Claude Code skill invocation](https://code.claude.com/docs/en/skills#where-you-write-the-skills-name).

### Example Evidence Block

```json
{
  "id": "MCP-001",
  "name": "Invalid JSON-RPC Version",
  "severity": "HIGH",
  "category": "mcp",
  "evidence": {
    "source_type": "spec",
    "source_urls": ["https://modelcontextprotocol.io/specification"],
    "verified_on": "2026-02-13",
    "applies_to": { "spec_revision": "2025-11-25" },
    "normative_level": "MUST",
    "tests": { "unit": true, "fixtures": true, "e2e": false }
  }
}
```

---

## AGENT SKILLS RULES

<a id="as-001"></a>
### AS-001 [HIGH] Missing Frontmatter
**Requirement**: SKILL.md MUST have YAML frontmatter between `---` delimiters
**Detection**: `!content.starts_with("---")` or no closing `---`
**Fix**: [AUTO-FIX] Add template frontmatter
**Source**: agentskills.io/specification

<a id="as-002"></a>
### AS-002 [HIGH] Missing Required Field: name
**Requirement**: `name` field REQUIRED in Agent Skills frontmatter; Claude Code skills may omit it and derive the name from the skill location
**Detection**: For non-Claude skills, parse YAML and check for `name` key
**Fix**: [AUTO-FIX] Add `name: directory-name`
**Source**: agentskills.io/specification

<a id="as-003"></a>
### AS-003 [HIGH] Missing Required Field: description
**Requirement**: `description` field REQUIRED in Agent Skills frontmatter; Claude Code skills may omit it and use the first body paragraph
**Detection**: For non-Claude skills, parse YAML and check for `description` key
**Fix**: [AUTO-FIX] Add `description: "Use when..."`
**Source**: agentskills.io/specification

<a id="as-004"></a>
### AS-004 [HIGH] Invalid Name Format
**Requirement**: name MUST be either a bare skill name or `<plugin>:<skill-name>`; each segment MUST be 1-64 chars, lowercase letters/numbers/hyphens only
**Regex**: `^[a-z0-9]+(-[a-z0-9]+)*(?::[a-z0-9]+(-[a-z0-9]+)*)?$`
**Detection**:
```rust
!is_valid_skill_name(name)
```
**Fix**: [AUTO-FIX] Convert each name segment to kebab-case (lowercase, replace `_` with `-`, remove invalid chars, collapse consecutive hyphens, truncate to 64 chars)
**Source**: agentskills.io/specification

<a id="as-005"></a>
### AS-005 [HIGH] Name Starts/Ends with Hyphen
**Requirement**: name MUST NOT start or end with `-`
**Detection**: `name.starts_with('-') || name.ends_with('-')`
**Fix**: Remove leading/trailing hyphens
**Source**: agentskills.io/specification

<a id="as-006"></a>
### AS-006 [HIGH] Consecutive Hyphens in Name
**Requirement**: name MUST NOT contain `--`
**Detection**: `name.contains("--")`
**Fix**: Replace `--` with `-`
**Source**: agentskills.io/specification

<a id="as-008"></a>
### AS-008 [HIGH] Description Too Short
**Requirement**: description MUST be 1-1024 characters (agentskills.io baseline, matched by Codex/OpenCode/Kiro). **Claude Code skills**: 1-1536 (Claude truncates at 1536), resolved per owning client.
**Detection**: `description.len() < 1 || description.len() > max` where `max` is 1536 for Claude Code skills, 1024 otherwise
**Fix**: Add minimal description or truncate
**Source**: agentskills.io/specification (Claude exception: code.claude.com/docs/en/skills)

<a id="as-009"></a>
### AS-009 [HIGH] Description Contains Angle Brackets (Codex)
**Requirement**: Codex skill descriptions MUST NOT contain `<` or `>`. **Codex-only** - agentskills.io and Claude Code impose no such restriction, so this fires only for Codex skills.
**Detection**: skill client is Codex AND `description` contains any `<` or `>` (matches Codex's `if "<" in description or ">" in description`)
**Fix**: [AUTO-FIX] Strip all `<` and `>` characters
**Source**: openai/codex codex-rs/skills/.../quick_validate.py

<a id="as-011"></a>
### AS-011 [HIGH] Invalid Compatibility Length
**Requirement**: compatibility field MUST be 1-500 chars if present
**Detection**: `compatibility.len() == 0 || compatibility.len() > 500`
**Fix**: Add compatibility text or truncate to 500 chars
**Source**: agentskills.io/specification

<a id="as-012"></a>
### AS-012 [MEDIUM] Content Exceeds 500 Lines
**Requirement**: SKILL.md SHOULD be under 500 lines. **Generic agentskills.io rule** ("Keep your main `SKILL.md` under 500 lines") - applies to all clients, not Claude-specific.
**Detection**: `body.lines().count() > 500`
**Fix**: Suggest moving to references/
**Source**: agentskills.io/specification (progressive disclosure)

<a id="as-013"></a>
### AS-013 [MEDIUM] File Reference Too Deep
**Requirement**: File references SHOULD be one level deep ("Keep file references one level deep from `SKILL.md`") - a SHOULD-level agentskills.io guideline, emitted as a warning. Covers all three documented resource directories: `references/`, `scripts/`, and `assets/`.
**Detection**: Check references like `references/guide.md` vs `refs/deep/nested/file.md`, `scripts/deep/nested/x.py`, `assets/deep/nested/y.png`. Git ref paths (`refs/heads`, `refs/remotes`, `refs/tags`) are excluded.
**Fix**: Flatten directory structure
**Source**: agentskills.io/specification

<a id="as-015"></a>
### AS-015 [MEDIUM] Claude API Upload Size Exceeds 30MB
**Requirement**: Claude API skill uploads MUST be under 30 MB uncompressed. Local Claude Code skill directories have no documented size cap; this rule emits an upload-readiness warning.
**Detection**: `directory_size > 30 * 1024 * 1024`; honor top-level `exclude` and `[files].exclude`.
**Fix**: Remove large assets before uploading to the Claude API.
**Source**: platform.claude.com/docs/en/build-with-claude/skills-guide

<a id="as-016"></a>
### AS-016 [HIGH] Skill Parse Error
**Requirement**: SKILL.md frontmatter MUST be valid YAML and match the Agent Skills field types. In particular, `metadata` is a map from string keys to string values.
**Detection**: YAML syntax or typed-frontmatter parse error, including non-string `metadata` values
**Fix**: Fix YAML syntax errors and use string keys and values in `metadata`
**Source**: agentskills.io/specification

<a id="as-017"></a>
### AS-017 [HIGH] Name Must Match Parent Directory
**Requirement**: Skill name MUST match parent directory name. **agentskills.io baseline only** - Claude Code decouples the two ("`name` sets only the display label shown in skill listings, and the command still comes from the directory or file name"), and a plugin skill's `name` deliberately replaces the last command segment, so the rule is scoped out for Claude Code skills the same way AS-008's length cap is.
**Detection**: owning client is not Claude Code AND name field does not match directory containing SKILL.md
**Fix**: Manual fix required - rename directory or update name field
**Source**: agentskills.io/specification

## CLAUDE CODE RULES (SKILLS)

> **Scope:** the `CC-SK-*` rules encode Claude Code's skill schema (Claude-only frontmatter fields, tool vocabulary, model values, hooks, fork/argument semantics). They run for Claude Code skills (`.claude/skills/` or a `claude-code` target) and for unscoped skills with no identifiable client, but are **suppressed for skills owned by another known tool** (Codex `.agents/skills/`, OpenCode, Cursor, …), which are covered by the generic `AS-*` rules and the per-client skill validator.

<a id="cc-sk-001"></a>
### CC-SK-001 [HIGH] Invalid Model Value
**Requirement**: model MUST be a documented Claude Code model alias, a full `claude-*` model ID, or `inherit`. The skill reference accepts the same values as `/model`: `default`, `best`, `fable`, `sonnet`, `opus`, `haiku`, `opusplan`, `sonnet[1m]`, `opus[1m]`.
**Detection**: `!VALID_MODEL_ALIASES.contains(model) && !model.starts_with("claude-")`; shared with CC-AG-003
**Fix**: Replace with closest valid option
**Source**: code.claude.com/docs/en/skills, code.claude.com/docs/en/model-config

<a id="cc-sk-002"></a>
### CC-SK-002 [HIGH] Invalid Context Value
**Requirement**: context MUST be "fork" or omitted
**Detection**: `context.is_some() && context != "fork"`
**Fix**: Change to "fork" or remove
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-004"></a>
### CC-SK-004 [HIGH] Agent Without Context
**Requirement**: `agent` field REQUIRES `context: fork`
**Detection**: `agent.is_some() && context != Some("fork")`
**Fix**: Add `context: fork`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-005"></a>
### CC-SK-005 [HIGH] Invalid Agent Type
**Requirement**: agent MUST be: Explore, Plan, general-purpose, or custom kebab-case name (1-64 chars, pattern: `^[a-z0-9]+(-[a-z0-9]+)*$`)
**Detection**: Check against built-in agents or validate kebab-case format
**Fix**: Auto-fix (unsafe) - replace invalid agent with 'general-purpose'
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-sk-006"></a>
### CC-SK-006 [MEDIUM] Side-Effect Skill Without disable-model-invocation
**Requirement**: Skills with side effects SHOULD set `disable-model-invocation: true`. Vendor text: "Use this for workflows with side effects or that you want to control timing, like `/commit`, `/deploy`, or `/send-slack-message`. You don't want Claude deciding to deploy because your code looks ready." (skills, "Control who invokes a skill")
**Detection**: a segment of the skill name (split on non-alphanumeric characters) is `deploy`, `ship`, `publish`, `delete`, `release`, `push`, `commit` or `send`, no segment is a read-only qualifier (`style`, `guide`, `guidelines`, `conventions`, `format`, `triage`, `analysis`, `analyzer`, `summary`, `report`, `review`, `status`, `check`, `docs`, `notes`, `history`, `lookup`, `search`), and `disable-model-invocation` is not true. `commit` and `send` are the vendor's own examples; the other words and the qualifier list are agnix heuristics.
**Fix**: [AUTO-FIX] (unsafe) Add `disable-model-invocation: true`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-007"></a>
### CC-SK-007 [MEDIUM] Unrestricted Bash
**Requirement**: Bash in allowed-tools SHOULD be scoped. Vendor text: "Workspace trust doesn't gate this field. Claude Code applies a project skill's `allowed-tools` even in a `-p` run in a folder you've never trusted. A skill can grant itself broad tool access, so review the `allowed-tools` of skills checked into a repository before you run Claude Code there." (skills, "Pre-approve tools for a skill")
**Detection**: `allowed_tools.contains("Bash") && !allowed_tools.contains("Bash(")`
**Fix**: [AUTO-FIX] (unsafe) Replace plain Bash with a scoped `Bash(git:*)`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-008"></a>
### CC-SK-008 [HIGH] Unknown Tool Name
**Requirement**: Tool names MUST match Claude Code tools. **Claude Code skills only** - other clients (Codex/OpenCode/…) have their own tool vocabularies, so this is scoped to the owning client.
**Known Tools**: the current built-in set (code.claude.com/docs/en/tools, verified 2026-07-31) incl. `PowerShell`, `Agent`, `Workflow`, `Artifact`, `ReportFindings`, `SendUserFile`, `EndConversation`, `Cron*`, `Team*`, `EnterWorktree`/`ExitWorktree`, `ScheduleWakeup`, `ListMcpResourcesTool`/`ReadMcpResourceTool`, `WaitForMcpServers`, plus legacy names kept for tolerance
**Detection**: skill client is Claude Code AND a tool in `allowed-tools` or `disallowed-tools` is not in the set; MCP tools are accepted in both documented forms - server-only `mcp__<server>` and fully qualified `mcp__<server>__<tool>` (case-sensitive lowercase prefix). The tool segment may glob (`mcp__github__get_*`); the server segment may not, since a rule must name one configured server
**Fix**: Suggest closest match
**Source**: code.claude.com/docs/en/tools

<a id="cc-sk-010"></a>
### CC-SK-010 [HIGH] Invalid Hooks in Skill Frontmatter
**Requirement**: `hooks` field in skill frontmatter MUST follow the same schema as settings.json hooks (valid events, handler types, required fields)
**Detection**: Parse hooks YAML value and validate against HooksSchema rules
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-013"></a>
### CC-SK-013 [MEDIUM] Fork Context Without Actionable Instructions
**Requirement**: Skills with `context: fork` SHOULD contain imperative instructions for the forked agent
**Detection**: Check body for imperative verbs when context is fork
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-014"></a>
### CC-SK-014 [HIGH] Invalid disable-model-invocation Type
**Requirement**: `disable-model-invocation` MUST use a Claude Code boolean value: boolean or case-insensitive `true`/`false`, `yes`/`no`, `on`/`off`, or `1`/`0`
**Detection**: Parse the raw YAML value and flag values outside the accepted boolean aliases
**Fix**: Manual - replace the invalid value with an accepted alias
**Source**: code.claude.com/docs/en/skills, github.com/anthropics/claude-code/releases/tag/v2.1.218

<a id="cc-sk-015"></a>
### CC-SK-015 [HIGH] Invalid user-invocable Type
**Requirement**: `user-invocable` MUST use a Claude Code boolean value: boolean or case-insensitive `true`/`false`, `yes`/`no`, `on`/`off`, or `1`/`0`
**Detection**: Parse the raw YAML value and flag values outside the accepted boolean aliases
**Fix**: Manual - replace the invalid value with an accepted alias
**Source**: code.claude.com/docs/en/skills, github.com/anthropics/claude-code/releases/tag/v2.1.218

<a id="cc-sk-016"></a>
### CC-SK-016 [LOW] Indexed $ARGUMENTS Without argument-hint
**Requirement**: A skill body that uses indexed arguments MAY declare an `argument-hint`. This is an agnix suggestion: the skills reference defines the field only as an autocomplete hint ("Hint shown during autocomplete to indicate expected arguments.") and does not require it.
**Detection**: prose `$ARGUMENTS[n]` or `$N` without `argument-hint` (info)
**Fix**: Manual - add an `argument-hint`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-017"></a>
### CC-SK-017 [MEDIUM] Unknown Frontmatter Field
**Requirement**: Skill frontmatter SHOULD only use recognized Claude Code fields. **Claude Code skills only** - other clients' field support is checked by the per-client skill validator (CL-SK/CX-SK/OC-SK/WS-SK). Known fields include the documented `when_to_use`, `arguments`, and `disallowed-tools`.
**Detection**: skill client is Claude Code AND frontmatter contains a field not in the Claude Code skill schema
**Fix**: Manual fix required - remove unknown field or correct typo
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-018"></a>
### CC-SK-018 [MEDIUM] Invalid Effort Value
**Requirement**: effort SHOULD be low, medium, high, xhigh, or max
**Detection**: Check effort field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-019"></a>
### CC-SK-019 [LOW] Invalid Paths Format
**Requirement**: paths field MAY be present and must not be empty
**Detection**: Check paths field is non-empty
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-020"></a>
### CC-SK-020 [MEDIUM] Invalid Shell Value
**Requirement**: shell field SHOULD be "bash" or "powershell"
**Detection**: Check shell field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-021"></a>
### CC-SK-021 [MEDIUM] Hardcoded User Directory Path
**Requirement**: Skill content SHOULD NOT contain hardcoded user-home paths (`/Users/<name>/`, `/home/<name>/`, `C:\Users\<name>\`). They leak the author's identity, are non-portable across machines, and rarely resolve for the agent at runtime. Applies to `SKILL.md`, any `.md` file bundled in the same skill directory subtree, and bundled scripts (`.sh`/`.bash`/`.zsh`/`.fish`/`.py`/`.rb`/`.pl`/`.lua`/`.js`/`.ts`/`.mjs`, or any extensionless file with a `#!` shebang).
**Detection**: For each `SKILL.md`, scan its body and the body of every in-scope file under its containing directory (recursively). For `.md` files skip frontmatter; for scripts scan the whole file including shebangs and comments. Match `(/Users/|/home/)[a-zA-Z0-9._-]+/` and `[Cc]:[\\/]Users[\\/][a-zA-Z0-9._-]+[\\/]`. Skip matches where the name segment is a placeholder: generic words (`user`, `username`, `name`, `you`, `yourname`, `me`, `myname`, `someone`, `example`, `johndoe`, `foo`, `bar`), or any segment wrapped in `<...>`, `${...}`, or `{{...}}` (these never match the name character class).
**Fix**: Manual - replace with `~/`, `$HOME/`, a project-relative path, or an env var like `$PROJECT_ROOT`. For shebangs, prefer `#!/usr/bin/env <interpreter>`.
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-022"></a>
### CC-SK-022 [HIGH] Reserved Claude Skill Name
**Requirement**: In Claude Code 2.1.282+, a local skill under `.claude/skills/` MUST NOT use the folder name `synced` or the reserved `anthropic-skills` namespace in its folder or frontmatter `name`. Claude Code skips these skills at load time. The `claude-ai` namespace is not reserved in 2.1.283.
**Detection**: For local Claude skill files, compare the folder and effective frontmatter name against `synced` and `anthropic-skills[:...]`; exempt a skill folder with its own `.claude-plugin/plugin.json`, which loads as a plugin; honor a pinned older Claude Code version.
**Fix**: Rename the local skill folder or frontmatter name.
**Source**: code.claude.com/docs/en/skills, github.com/anthropics/claude-code/releases/tag/v2.1.283

<a id="cc-sk-023"></a>
### CC-SK-023 [MEDIUM] Argument Placeholder Inside Code
**Requirement**: A Claude Code skill that declares no `argument-hint` or `arguments` SHOULD NOT contain an unescaped `$N` (`$0` through `$9`) inside a fenced or inline code span. Claude Code substitutes `$N` everywhere in the skill body, code included, so shell or awk syntax such as `awk '{print $1}'` receives the second skill argument whenever the skill is invoked with two or more arguments. The skills docs say to escape a literal `$` before a digit with a single backslash (`\$1`).
**Detection**: Find `$N` shorthand matches in the body that fall inside code and are not preceded by a single backslash; report the first one. Skills with `argument-hint` or `arguments` are skipped, since they likely mean the placeholder. Prose `$N` is covered by CC-SK-016.
**Fix**: Manual - escape the dollar sign (`\$1`) when it is code syntax, or add an `argument-hint` when it should receive an argument.
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-024"></a>
### CC-SK-024 [LOW] Skill Without Description
**Requirement**: A Claude Code skill SHOULD have a `description` (or `when_to_use`). Vendor text: "All fields are optional. Only `description` is recommended so Claude knows when to use the skill." and `description` "If omitted, uses the first non-empty line of the markdown content." (skills, "Frontmatter reference")
**Detection**: neither `description` nor `when_to_use` is set, and `disable-model-invocation` is not true (info)
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-025"></a>
### CC-SK-025 [LOW] Skill Listing Over Budget
**Requirement**: The project's skill names and descriptions SHOULD fit the skill listing budget. Vendor text: "if you have many skills, Claude Code drops some descriptions to fit the listing's character budget, which removes the keywords Claude needs to match your request. The budget scales at 1% of the model's context window." (skills, "Skill descriptions are cut short")
**Detection**: over the `.claude/skills/*/SKILL.md` files without `disable-model-invocation: true`, sum each name plus its `description` and `when_to_use` (each entry capped at the documented 1,536 characters); report when the total is above 8,000 characters. The threshold is an agnix estimate: the token-to-character conversion is not documented, so agnix assumes a 200,000-token window and 4 characters per token. User and plugin skills are not visible to a project lint. The first skill (sorted) reports it once.
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills, code.claude.com/docs/en/settings-reference

<a id="cc-sk-027"></a>
### CC-SK-027 [LOW] Legacy Custom Command File
**Requirement**: New commands SHOULD be skills. Vendor text: "a Markdown file in `.claude/commands/` is the older format and still works. It supports the same frontmatter except `name` and `paths`. To find the name you type to invoke it, see How a skill gets its command name. Prefer a skill for new work, since skills also support supporting files." (skills, "Choose where skills load")
**Detection**: every `.claude/commands/**/*.md` file (a new file type) gets one migration note (info); when it sets `name` or `paths`, the note says those fields have no effect. Command files get no other skill rules: they are a legacy form the current release still loads.
**Fix**: Manual - move the file to `.claude/skills/<name>/SKILL.md`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-028"></a>
### CC-SK-028 [MEDIUM] Injected Command Not Run
**Requirement**: `!` command injection SHOULD use a form Claude Code runs. Vendor text: "The inline form is only recognized when `!` appears at the start of a line or immediately after whitespace. If `!` follows another character, as in `` KEY=!`cmd` ``, the placeholder is left as literal text and the command does not run." and "`No shell command in <name> was run` means one of the file's `!` commands contains a raw control character other than tab or newline, so none of them runs." (skills, "Inject dynamic context")
**Detection**: in prose (outside code spans and fences), `` !` `` preceded by a non-whitespace character on the same line; and, in a file with a recognized `!` command or a ```` ```! ```` block, any control character other than tab, newline or carriage return
**Fix**: Manual
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-030"></a>
### CC-SK-030 [LOW] Skill Name Uses a Reserved Word
**Requirement**: A skill that may be uploaded SHOULD avoid the reserved words. Vendor text: "`name`: Maximum 64 characters, lowercase letters/numbers/hyphens only, no XML tags, no reserved words ("anthropic", "claude")" (Claude API skills guide, "Limits and constraints")
**Detection**: the skill name (frontmatter `name`, or the directory name) contains `claude` or `anthropic` (info). Claude Code itself loads these skills, so this is a note for skills that may be uploaded to claude.ai or the Claude API.
**Fix**: Manual
**Source**: platform.claude.com/docs/en/build-with-claude/skills-guide, platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

<a id="cc-sk-031"></a>
### CC-SK-031 [LOW] XML Tag in Skill Name or Description
**Requirement**: A skill that may be uploaded SHOULD keep XML tags out of its name and description. Vendor text: "`description`: Maximum 1024 characters, non-empty, no XML tags" (Claude API skills guide, "Limits and constraints")
**Detection**: a tag-shaped `<name>`, `<name attr="...">` or `</name>` in `name` or `description` (info). `a < b` comparisons and prose placeholders such as `<the problem, stated in full>` are not tags.
**Fix**: Manual
**Source**: platform.claude.com/docs/en/build-with-claude/skills-guide

<a id="cc-sk-032"></a>
### CC-SK-032 [LOW] Skill Description Not in Third Person
**Requirement**: Skill descriptions SHOULD be written in the third person. Vendor text: "Always write in third person. The description is injected into the system prompt, and inconsistent point-of-view can cause discovery problems." (agent skills best practices, "Writing effective descriptions")
**Detection**: the description starts with `I `, `I'm `, `I'll `, `I can `, `I will `, `I help `, `You can `, `You should `, `This skill helps you` or `Lets you ` (info). English-only heuristic.
**Fix**: Manual
**Source**: platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

<a id="cc-sk-033"></a>
### CC-SK-033 [LOW] Vague Skill Name
**Requirement**: Skill names SHOULD say what the skill does. Vendor text: "Vague names: `helper`, `utils`, `tools`" and "Overly generic: `documents`, `data`, `files`" (agent skills best practices, "Naming conventions")
**Detection**: the skill name (last segment) is one of helper, helpers, util, utils, tool, tools, document, documents, data, file, files (info)
**Fix**: Manual
**Source**: platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

<a id="cc-sk-034"></a>
### CC-SK-034 [LOW] Long Reference File Without Table of Contents
**Requirement**: Long bundled reference files SHOULD start with a table of contents. Vendor text: "For large reference files (>300 lines), include a table of contents" (skill-creator) and "For reference files longer than 100 lines, include a table of contents at the top." (agent skills best practices). agnix uses the skill-creator's 300 lines to keep the check quiet.
**Detection**: a `.md` file inside the skill directory that SKILL.md links to (Markdown link or `references/...` mention), longer than 300 lines, with no "Contents" or "Table of contents" heading and fewer than three `](#` anchor links in its first 40 lines. Reported on the reference file (info).
**Fix**: Manual
**Source**: github.com/anthropics/skills/blob/main/skills/skill-creator/SKILL.md, platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

<a id="cc-set-001"></a>
### CC-SET-001 [MEDIUM] Invalid prUrlTemplate Setting
**Requirement**: `prUrlTemplate` in `.claude/settings.json` (and `.local.json`/`managed-settings.json`) SHOULD be a non-empty string containing at least one of the documented placeholders: `{host}`, `{owner}`, `{repo}`, `{number}`, `{url}`. A template with no placeholder will render the same static URL for every PR badge.
**Detection**: Parse settings.json; look up top-level `prUrlTemplate`; flag (error) non-string types and empty strings, flag (warning) strings that substring-match none of the documented placeholders.
**Fix**: Manual - set `prUrlTemplate` to a URL template like `https://reviews.example.com/{owner}/{repo}/pull/{number}`
**Source**: code.claude.com/docs/en/settings (added in Claude Code v2.1.119)

<a id="cc-set-002"></a>
### CC-SET-002 [MEDIUM] Non-boolean channelsEnabled Setting
**Requirement**: `channelsEnabled` MUST be a boolean when present (Claude Code 2.1.128+). Its Scope is `Managed`. Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead. A quoted `"true"` or numeric value leaves Channels silently disabled - same footgun shape as CC-MCP-004 `alwaysLoad`.
**Detection**: Parse settings.json; look up top-level `channelsEnabled`; flag (warning) string / number / array / object values. Explicit `false` and `null` are not flagged.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.128 (introduced `--channels` support for console API-key auth; console orgs with managed settings must opt in via `channelsEnabled: true`), code.claude.com/docs/en/settings-reference

<a id="cc-set-003"></a>
### CC-SET-003 [MEDIUM] Invalid worktree.baseRef Value
**Requirement**: `worktree.baseRef` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be one of the strings `"fresh"` or `"head"` when present (Claude Code 2.1.133+). `"fresh"` branches from `origin/<default>` (the v2.1.133 default); `"head"` branches from local `HEAD` (the pre-v2.1.133 `EnterWorktree` behavior). Any other value silently falls back to the default.
**Detection**: Parse settings.json; walk `worktree.baseRef`; flag (warning) non-string types and strings that don't match the `{fresh, head}` enum. Case-sensitive. `null` values and absent keys are not flagged.
**Fix**: Manual - set `worktree.baseRef` to `"fresh"` or `"head"`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.133 (added `worktree.baseRef` setting; default changed back to `fresh` from the `head` behavior that shipped in v2.1.128)

<a id="cc-set-004"></a>
### CC-SET-004 [MEDIUM] Invalid Sandbox Path Setting
**Requirement**: `sandbox.bwrapPath` and `sandbox.socatPath` MUST be non-empty strings when present (Claude Code 2.1.133+, Linux/WSL only). Their Scope is `Managed`: "Claude Code reads it only from managed settings so that a user, project, or local file can't point the sandbox at a different binary." Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead. These override the default bubblewrap / socat binary lookup; an empty string or non-string value means Claude Code cannot locate the sandbox helper.
**Detection**: Parse settings.json; walk `sandbox.bwrapPath` and `sandbox.socatPath`; flag (warning) empty strings and non-string types independently (both fields fire their own diagnostic when wrong). `null` values and absent keys are not flagged. Path existence is NOT checked (agnix validates files, not filesystem state).
**Fix**: Manual - set the field to an absolute path string, or remove it to use Claude Code's default lookup.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.133 (added `sandbox.bwrapPath` and `sandbox.socatPath` managed settings for Linux/WSL), code.claude.com/docs/en/settings-reference

<a id="cc-set-005"></a>
### CC-SET-005 [MEDIUM] Invalid parentSettingsBehavior Value
**Requirement**: `parentSettingsBehavior` (Scope `Managed`; in other files CC-SET-034 reports it) MUST be one of the strings `"first-wins"` or `"merge"` when present (Claude Code 2.1.133+). `"first-wins"` preserves existing behavior; `"merge"` opts SDK `managedSettings` (the parent tier) into the policy merge.
**Detection**: Parse settings.json; look up top-level `parentSettingsBehavior`; flag (warning) non-string types and strings that don't match the `{first-wins, merge}` enum. Case-sensitive. `null` values and absent keys are not flagged.
**Fix**: Manual - set `parentSettingsBehavior` to `"first-wins"` or `"merge"`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.133 (added `parentSettingsBehavior` admin-tier key), code.claude.com/docs/en/settings-reference

<a id="cc-set-006"></a>
### CC-SET-006 [MEDIUM] Non-boolean disableBundledSkills Setting
**Requirement**: `disableBundledSkills` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be a boolean when present (Claude Code 2.1.169+). `true` hides the bundled skills, workflows, and built-in slash commands from the model; equivalent to setting the `CLAUDE_CODE_DISABLE_BUNDLED_SKILLS` environment variable to `1`. Only strict `true`/`false` is documented - a quoted `"true"` or a truthy number is not a documented opt-in (same footgun shape as CC-SET-002).
**Detection**: Parse settings.json; look up top-level `disableBundledSkills`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.169 (added `disableBundledSkills`), code.claude.com/docs/en/settings

<a id="cc-set-007"></a>
### CC-SET-007 [MEDIUM] Non-boolean enforceAvailableModels Setting
**Requirement**: `enforceAvailableModels` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be a boolean when present (Claude Code 2.1.175+). `true` enforces the managed `availableModels` allowlist for users; only strict `true`/`false` is documented.
**Detection**: Parse settings.json; look up top-level `enforceAvailableModels`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.175 (added `enforceAvailableModels`), code.claude.com/docs/en/settings

<a id="cc-set-008"></a>
### CC-SET-008 [MEDIUM] Non-boolean sandbox.allowAppleEvents Setting
**Requirement**: `sandbox.allowAppleEvents` MUST be a boolean when present (Claude Code 2.1.181+). Its Scope is `User or managed` ("Project settings cannot enable it"). Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead. `true` opts macOS sandboxed commands into sending Apple Events; only strict `true`/`false` is documented.
**Detection**: Parse settings.json; walk `sandbox.allowAppleEvents`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged. Non-object `sandbox` values are ignored by this rule to avoid conflating shape errors with the specific boolean opt-in.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.181 (added `sandbox.allowAppleEvents`), code.claude.com/docs/en/settings, code.claude.com/docs/en/settings-reference

<a id="cc-set-009"></a>
### CC-SET-009 [MEDIUM] Non-boolean attribution.sessionUrl Setting
**Requirement**: `attribution.sessionUrl` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be a boolean when present (Claude Code 2.1.183+). `true` keeps the default claude.ai session link in commits and PR descriptions created from web or Remote Control sessions; `false` omits it. Only strict `true`/`false` is documented.
**Detection**: Parse settings.json; walk `attribution.sessionUrl`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged. Non-object `attribution` values are ignored by this rule to avoid conflating container shape errors with the specific boolean toggle.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.183 (added `attribution.sessionUrl`), code.claude.com/docs/en/settings

<a id="cc-set-010"></a>
### CC-SET-010 [MEDIUM] Invalid teammateMode Setting
**Requirement**: `teammateMode` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be one of `"in-process"`, `"auto"`, `"tmux"`, or `"iterm2"` when present (Claude Code 2.1.186+). Invalid strings silently fall back to default behavior in practice, so typos can launch teammates in an unexpected display mode.
**Detection**: Parse settings.json; look up top-level `teammateMode`; flag (warning) non-string types and strings outside the documented enum. Case-sensitive. `null` values and absent keys are not flagged.
**Fix**: Manual - set `teammateMode` to `"in-process"`, `"auto"`, `"tmux"`, or `"iterm2"`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.186 (added `iterm2` teammate mode), code.claude.com/docs/en/settings, code.claude.com/docs/en/agent-teams

<a id="cc-set-011"></a>
### CC-SET-011 [MEDIUM] Non-boolean respondToBashCommands Setting
**Requirement**: `respondToBashCommands` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be a boolean when present (Claude Code 2.1.186+). `false` keeps `!` bash command output context-only; only strict `true`/`false` is documented.
**Detection**: Parse settings.json; look up top-level `respondToBashCommands`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.186 (added `respondToBashCommands`)

<a id="cc-set-012"></a>
### CC-SET-012 [MEDIUM] Invalid sandbox.credentials Setting
**Requirement**: `sandbox.credentials` MUST be an object with optional `files` and `envVars` arrays when present (Claude Code 2.1.187+). Each entry MUST use `mode: "deny"` or `mode: "mask"`; environment-variable masking was added in 2.1.199 and file masking in 2.1.221. Claude Code 2.1.224 added structured environment extraction, JWT-aware `decode: "jwt"` with non-empty `maskClaims`, `awsPairs`, and `sigv4` policies. File entries require a non-empty string `path`; environment entries require a shell-compatible variable `name`. Masked `extract` patterns require a valid ECMAScript regex with a credential capture group. Environment entries cannot combine `extract` and `decode`, and decoded environment variables only accept `onExtractNoMatch: "warn"`. `awsPairs` entries require non-empty `accessKeyIdVar` and `secretAccessKeyVar` strings with an optional non-empty `sessionTokenVar`; every variable name MUST be unique across every credential slot and pair. `sigv4` accepts only `streaming`, `presigned`, and `sigv4a` keys with `deny` or `passthrough` values. When `awsPairs` or `sigv4` is configured, `sandbox.network.tlsTerminate` MUST be an object so Claude Code can inspect and re-sign AWS requests. AWS re-signing options MUST be placed in user settings, managed settings, or a `--settings` file because Claude Code ignores them in project-level `.claude/settings.json` and `.claude/settings.local.json`. Claude Code accepts but ignores mask-only fields on `deny` entries. Top-level `allowPlaintextInject`, when present, MUST be boolean.
**Detection**: Parse settings.json; walk `sandbox.credentials`; validate the container and arrays, required target fields, supported modes, environment-variable names, mask-only constraints, optional field types and enums, ECMAScript regex syntax and capture-group presence, JWT decode and claim-mask combinations, AWS credential-pair fields and variable-name uniqueness, SigV4 policies, the AWS re-signing TLS-termination prerequisite, injection host arrays, and `allowPlaintextInject`. Skip mask-only optional-field validation for `deny` entries to match Claude Code preprocessing. Treat repository `.claude/settings.json` and `.claude/settings.local.json` as project scope; exempt an absolute home-directory `~/.claude/settings.json` and dispatch managed settings from `.claude/managed-settings.json`, the documented macOS, Linux/WSL, and Windows system `managed-settings.json` paths, and their non-hidden `managed-settings.d/*.json` drop-ins.
**Fix**: Manual - use a valid `deny` or `mask` entry shape for the credential target, or remove the invalid entry or option. `allowPlaintextInject`, `awsPairs` and `sigv4` have Scope `User or managed`; in project and local files CC-SET-034 reports them and their shape is not checked.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.187 (added `sandbox.credentials`), github.com/anthropics/claude-code/releases/tag/v2.1.199 (added environment-variable masking), github.com/anthropics/claude-code/releases/tag/v2.1.221 (added file masking), github.com/anthropics/claude-code/releases/tag/v2.1.224 (added structured environment, JWT, and AWS masking), code.claude.com/docs/en/settings, code.claude.com/docs/en/sandboxing, code.claude.com/docs/en/settings-reference

<a id="cc-set-013"></a>
### CC-SET-013 [MEDIUM] Non-boolean autoMode.classifyAllShell Setting
**Requirement**: `autoMode.classifyAllShell` MUST be a boolean when present (Claude Code 2.1.193+). `autoMode` has Scope `User or managed`: "The classifier doesn't read `autoMode` from project settings in `.claude/settings.json` or `.claude/settings.local.json`." Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead. `true` routes all Bash/PowerShell commands through the auto-mode classifier; only strict `true`/`false` is documented.
**Detection**: Parse settings.json; walk `autoMode.classifyAllShell`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged. Non-object `autoMode` values are ignored by this rule to avoid conflating container shape errors with the specific boolean toggle.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.193 (added `autoMode.classifyAllShell`), code.claude.com/docs/en/settings-reference

<a id="cc-set-016"></a>
### CC-SET-016 [MEDIUM] File Path Rule Never Consulted
**Requirement**: File path rules MUST use `Edit(path)` or `Read(path)`: "Claude Code checks file permissions against `Edit(path)` and `Read(path)` rules only. If you write a path rule for `Write`, `NotebookEdit`, `Glob`, or the legacy `MultiEdit` tool instead, Claude Code accepts the rule but never consults it". A deny or ask rule of that form protects nothing, so it is reported as an error; an allow rule as a warning. Bare tool names (`"Write"`) are fine: "Claude Code doesn't warn about a tool-name rule with no path".
**Detection**: Walk `permissions.allow`, `permissions.deny` and `permissions.ask` in Claude settings files; flag `Write(...)`, `NotebookEdit(...)`, `MultiEdit(...)` and `Glob(...)` with a non-empty path. The diagnostic points at the array entry.
**Fix**: [AUTO-FIX, unsafe] Replace the entry with `Edit(path)` (Write, NotebookEdit, MultiEdit) or `Read(path)` (Glob). The rule starts being enforced, so the permissions change.
**Source**: code.claude.com/docs/en/permissions ("Read and Edit"), code.claude.com/docs/en/errors ("Is not matched by file permission checks")

<a id="cc-set-017"></a>
### CC-SET-017 [MEDIUM] Non-boolean sandbox.filesystem.disabled Setting
**Requirement**: `sandbox.filesystem.disabled` MUST be a boolean when present (Claude Code 2.1.216+). Its Scope is `User or managed`. Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead. `true` disables filesystem isolation while preserving network isolation; only strict `true` / `false` is accepted.
**Detection**: Parse settings.json; walk `sandbox.filesystem.disabled`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged. Non-object `sandbox` or `filesystem` values are ignored by this rule to avoid conflating container shape errors with the boolean toggle.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.216, code.claude.com/docs/en/sandboxing, code.claude.com/docs/en/settings-reference

<a id="cc-set-018"></a>
### CC-SET-018 [MEDIUM] Non-boolean emojiCompletionEnabled Setting
**Requirement**: `emojiCompletionEnabled` in `.claude/settings.json` / `.local.json` / `managed-settings.json` MUST be a boolean when present (Claude Code 2.1.217+). The setting controls named emoji completion and accepts only strict `true` / `false` values.
**Detection**: Parse settings.json; look up top-level `emojiCompletionEnabled`; flag (warning) non-boolean types (string, number, array, object). `null` values and absent keys are not flagged.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.217, code.claude.com/docs/en/settings

<a id="cc-set-019"></a>
### CC-SET-019 [MEDIUM] Non-boolean sandbox.network.strictAllowlist Setting
**Requirement**: `sandbox.network.strictAllowlist` MUST be a boolean when present in Claude Code 2.1.219+. Its Scope is `User or managed` ("A repository can't turn it on or off"). Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead.
**Detection**: Parse settings JSON and flag non-boolean, non-null `sandbox.network.strictAllowlist` values
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.219, code.claude.com/docs/en/sandboxing, code.claude.com/docs/en/settings-reference

<a id="cc-set-020"></a>
### CC-SET-020 [MEDIUM] Invalid workflowSizeGuideline Setting
**Requirement**: `workflowSizeGuideline` MUST be `unrestricted`, `small`, `medium`, or `large` when present in Claude Code 2.1.219+
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum
**Fix**: Manual - choose one of the four documented values
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.219, code.claude.com/docs/en/settings

<a id="cc-set-022"></a>
### CC-SET-022 [MEDIUM] Invalid crossSessionInbound Setting
**Requirement**: `crossSessionInbound` MUST be `accept`, `hold`, or `refuse` when present in Claude Code 2.1.224+
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum
**Fix**: Manual - choose one of the three documented values
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.224, code.claude.com/docs/en/settings, code.claude.com/docs/en/cross-session-messaging

<a id="cc-set-023"></a>
### CC-SET-023 [MEDIUM] Invalid dialogExpiry Setting
**Requirement**: `dialogExpiry` MUST be `60s`, `5m`, `10m`, or `never` when present in Claude Code 2.1.224+. Its Scope is `User or managed`. Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead.
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum
**Fix**: Manual - choose one of the four documented values
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.224, code.claude.com/docs/en/settings, code.claude.com/docs/en/settings-reference

<a id="cc-set-025"></a>
### CC-SET-025 [MEDIUM] Invalid modelPicker Setting
**Requirement**: `modelPicker` (Scope `User or managed`: "ignores it in project and local settings"; CC-SET-034 reports it there) MUST be an object with an `options` array. Each option MUST be an object with a non-empty string `model`; optional `label` and `description` fields MUST be strings, and optional `replaceBuiltInOptions` MUST be boolean.
**Detection**: In user and managed settings, parse settings JSON and flag non-object values, a missing or non-array `options`, malformed option rows, or a non-boolean `replaceBuiltInOptions`.
**Fix**: Manual - use the documented object and option-row shape.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.243, code.claude.com/docs/en/settings-reference#modelpicker

<a id="cc-set-026"></a>
### CC-SET-026 [MEDIUM] Invalid promptCacheTtl Setting
**Requirement**: `promptCacheTtl` MUST be `5m` or `1h` when present in Claude Code 2.1.242+.
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum.
**Fix**: Manual - choose `5m` or `1h`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.243, code.claude.com/docs/en/settings-reference#promptcachettl

<a id="cc-set-027"></a>
### CC-SET-027 [MEDIUM] Invalid subagentPromptCacheTtl Setting
**Requirement**: `subagentPromptCacheTtl` MUST be `5m` or `1h` when present in Claude Code 2.1.242+.
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum.
**Fix**: Manual - choose `5m` or `1h`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.243, code.claude.com/docs/en/settings-reference#subagentpromptcachettl

<a id="cc-set-028"></a>

### CC-SET-028 [MEDIUM] Invalid feedbackDrafts Setting
**Requirement**: `feedbackDrafts` MUST be the string `notify`, `quiet`, or `off` when present in Claude Code 2.1.247+. It is a string enum, not a boolean; `off` removes the SendFeedback tool. Its Scope is `User or managed`. Checked only where Claude Code reads the key; in other files CC-SET-034 reports it instead.
**Detection**: Parse settings JSON and flag non-string values or strings outside the documented enum.
**Fix**: Manual - choose `notify`, `quiet`, or `off`.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.247, code.claude.com/docs/en/settings-reference#feedbackdrafts

<a id="cc-set-029"></a>

### CC-SET-029 [MEDIUM] Invalid spinnerTipsOverride Shape
**Requirement**: `spinnerTipsOverride` MUST be an object limited to `tips`, `tipsFile`, `label`, and `excludeDefault`. Each `tips` entry is a plain string or an object with a required `id` (up to 64 letters, digits, `.`, `_`, `-`) and `text` (up to 500 characters), plus optional `cooldownSessions` (0-1000) and `priority` (-10-10). A plain string is read as a tip whose text is the string itself, so it MUST satisfy the same non-empty, 500-character limit. `tipsFile` MUST be absolute or `~/`-rooted, and `label` at most 40 characters.
**Detection**: Parse settings JSON, then check the object's fields and each tip entry against the documented shape and limits.
**Fix**: Manual - Claude Code drops an invalid tip with a debug warning instead of reporting it, so the tip is simply never shown.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.247, code.claude.com/docs/en/settings-reference#spinnertipsoverride

<a id="cc-set-030"></a>

### CC-SET-030 [HIGH] Local Command in managedMcpServers
**Requirement**: Every `managedMcpServers` entry MUST describe a remote HTTP or SSE server. Claude Code 2.1.259+ skips entries that name a local command, including entries with `command` or `type: "stdio"`. The key has Scope `Managed` ("Claude Code drops the key with a warning in user, project, and local settings"); CC-SET-034 reports it there.
**Detection**: Parse settings JSON, require `managedMcpServers` and its named entries to be objects, and flag command-based or explicitly stdio entries.
**Fix**: Manual - provide the server through HTTP or SSE with a URL. Use another supported deployment mechanism for local stdio servers.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.259, code.claude.com/docs/en/settings-reference

<a id="cc-set-031"></a>

### CC-SET-031 [MEDIUM] Invalid maxEffortLevel Setting
**Requirement**: `maxEffortLevel`, both at the top level and inside a `modelSettings` entry, MUST be one of `low`, `medium`, `high`, `xhigh`, or `max` in Claude Code 2.1.267+ when non-null. A `null` value is treated as unset. The `max` value explicitly removes the cap for that settings source.
**Detection**: Parse settings JSON and flag non-string caps or strings outside the documented enum at either supported location.
**Fix**: Manual - choose `low`, `medium`, `high`, `xhigh`, or `max`.
**Source**: code.claude.com/docs/en/settings-reference#maxeffortlevel, github.com/anthropics/claude-code/releases/tag/v2.1.267

<a id="cc-set-033"></a>
### CC-SET-033 [MEDIUM] Idle Compaction Not Boolean
**Requirement**: `idleCompaction` MUST be a strict boolean when present. Claude Code 2.1.290 added idle compaction with the `"idleCompaction": false` settings.json opt-out (v2.1.299 release notes); a quoted `"false"` or truthy number silently keeps idle compaction enabled.
**Detection**: Parse settings JSON and flag a non-boolean, non-null `idleCompaction` value.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.299

<a id="cc-set-034"></a>
### CC-SET-034 [MEDIUM] Settings Key Ignored at This Scope
**Requirement**: A settings key MUST be set in a file Claude Code reads it from: "**Claude Code ignores the key in a repository file.** Look for `User, local, or managed`, `User or managed`, `User`, `Managed`, or `Global config` in the Scope column of the settings index." Value-level exceptions from the Scope lines: "`auto` and `bypassPermissions` don't take effect from project or local settings"; for `remoteControlAtStartup` "a project or local `true` is ignored"; for `bashEditDiffEnabled` "a `true` in a repository's `.claude/settings.json` or `.claude/settings.local.json` can't turn the recording on". For `env`: "Project and local settings can't set variables that a checked-out repository shouldn't control ... Claude Code drops each one, apart from a few values that turn telemetry off", and some variables are "ignored from every file".
**Detection**: A table built from the settings-reference index (2026-10-14). `Managed` keys are reported outside managed settings; `User or managed` keys in `.claude/settings.json` and `.claude/settings.local.json`; `User, local, or managed` keys in `.claude/settings.json`; `User` keys outside `~/.claude/settings.json`; `Global config` keys in every settings file. A key covers its children (one finding for `autoMode`). `autoContinueAtUsageLimit` is not reported, since a project value "turns the feature off rather than being ignored". The documented `env` variables are reported in project and local files (the `OTEL_*_EXPORTER=none` and `OTEL_LOG_*=0` off values excepted), and the launch-only variables in every file. Partial scopes in project and local files: `sandbox.credentials` `mask` entries ("Claude Code honors `mask` entries, `allowPlaintextInject`, `awsPairs`, and `sigv4` only from user settings, managed settings, and the `--settings` flag"), `spinnerTipsOverride` tip objects, `tipsFile`, `label` and `excludeDefault` ("from project and local settings it reads plain string tips only"), and a self-hosted `ccpool_` `remote.defaultEnvironmentId`. Type checks (CC-SET-002, 004, 005, 008, 012, 013, 017, 019, 023, 025, 028, 030) skip the keys this rule reports.
**Fix**: Manual - move the key to a file that reads it, or remove it
**Source**: code.claude.com/docs/en/settings ("A committed key doesn't reach teammates"), code.claude.com/docs/en/settings-reference ("Settings index", "Variables Claude Code ignores in `env`")

<a id="cc-set-035"></a>
### CC-SET-035 [HIGH] Ineffective Permission Rule Syntax
**Requirement**: Permission rules MUST use a form Claude Code applies. Skipped or ignored (error): a rule that is not `Tool` or `Tool(content)` ("Claude Code skips the rule"); "it skips any `mcp__` rule that has parentheses"; a parameter rule on a primary field, such as `Bash(command:rm *)` ("Claude Code ignores it and emits a startup warning"); an unanchored allow glob ("An unanchored allow glob such as `"*"`, `"B*"`, or `"mcp__*"` is skipped with a warning and doesn't auto-approve anything"). Read differently (warning): "The `:*` form is only recognized at the end of a pattern"; a `*` before the subcommand of a Bash allow rule ("Put the `*` after the subcommand"); "A pattern like `/Users/alice/file` isn't an absolute path"; "A deny or ask rule whose tool name matches no known tool produces a startup warning".
**Detection**: Each entry of `permissions.allow`, `permissions.deny` and `permissions.ask`. The subcommand check covers programs with subcommands (git, npm, docker, kubectl and similar). The single-slash check covers Read and Edit paths whose first segment is an absolute root (`Users`, `home`, `etc`, `tmp`, a drive letter). The tool-name check exempts names with `_` or `*` and removed tools such as `TaskOutput`, as Claude Code does.
**Fix**: Manual - rewrite the rule as the message describes
**Source**: code.claude.com/docs/en/permissions ("Permission rule syntax", "Wildcard patterns", "Tool name wildcards", "Read and Edit"), code.claude.com/docs/en/errors ("Malformed Tool(content) rule", "Has a wildcard before the rest of the command")

<a id="cc-set-036"></a>
### CC-SET-036 [MEDIUM] Broad Code-Execution Allow Rule
**Requirement**: Allow rules SHOULD NOT approve arbitrary code execution: "On entering auto mode, broad allow rules that grant arbitrary code execution are dropped: Blanket `Bash(*)` or `PowerShell(*)`; Wildcarded interpreters like `Bash(python*)`; Package-manager run commands; `Agent` allow rules; `Monitor` allow rules". For environment runners: "a rule like `Bash(devbox run *)` matches whatever comes after `run`, including `devbox run rm -rf .`".
**Detection**: In `permissions.allow`: bare `Bash`/`PowerShell` or `(*)`; a `*` in the program position; interpreters (python, node, bash, sh, ruby, perl and similar) with a wildcard; runners (devbox run, docker exec, mise exec, direnv exec, npx, uvx, pnpm dlx and similar) or bare package managers followed by `*` (warning). Package-manager run commands such as `Bash(npm run *)`, and bare `Agent` or `Monitor`, are info: the permissions page itself uses `Bash(npm run *)` as an example, and auto mode drops them. Findings in `~/.claude/settings.json` are info.
**Fix**: Manual - allow the specific command, such as `Bash(npm run test)` or `Bash(devbox run npm test)`
**Source**: code.claude.com/docs/en/permission-modes ("How auto mode evaluates actions"), code.claude.com/docs/en/permissions ("Wrappers")

<a id="cc-set-037"></a>
### CC-SET-037 [MEDIUM] Sandbox Weakened by Repository Settings
**Requirement**: A repository's shared settings SHOULD NOT weaken the sandbox: "A repository's `.claude/settings.json` can too, unless the sandbox is admin-required. Each one weakens the sandbox" (`enableWeakerNestedSandbox`, `enableWeakerNetworkIsolation`, `network.allowAllUnixSockets`, `network.allowLocalBinding`). "allowing access to `/var/run/docker.sock` effectively grants access to the host system through the Docker socket." "An excluded command runs with your full access. A broad entry such as `docker *` covers everything that tool can do." "Allowing writes to directories containing executables in `$PATH`, system configuration directories, or user shell configuration files such as `.bashrc` or `.zshrc` can lead to code execution".
**Detection**: In `.claude/settings.json`: the four flags set to `true`; `network.allowUnixSockets` entries naming `docker.sock`; `excludedCommands` entries that are a whole interpreter or container tool (`docker *`, `python *`, `bash`); `filesystem.allowWrite` entries for shell startup files, `$PATH` directories, `/etc`, `/` or `~`
**Fix**: Manual - remove the setting from the shared file, or narrow it
**Source**: code.claude.com/docs/en/sandboxing ("Keep developers from widening the policy", "Run commands outside the sandbox with `excludedCommands`", "Security limitations")


<a id="cc-set-038"></a>
### CC-SET-038 [MEDIUM] .claudeignore Has No Effect
**Requirement**: Protect files with Read deny rules, not `.claudeignore`: "If your project has a `.claudeignore` file, it has no effect, so move its entries into `Read` deny rules."
**Detection**: Any `.claudeignore` file
**Fix**: Manual - move the entries into `permissions.deny` as `Read(...)` rules
**Source**: code.claude.com/docs/en/permissions ("Read and Edit")

<a id="cc-set-039"></a>
### CC-SET-039 [LOW] Secret Files Not Denied
**Requirement**: Best practice: deny reads of files that hold secrets. The `permissions.deny` entry says: "Use it for files that hold API keys, secrets, or environment values: Claude Code excludes matching files from file discovery and search results, denies reads of them, and blocks the Edit and Write tools on the matching paths."
**Detection**: For `.claude/settings.json`: the project root holds `.env`, `.env.*` (not `.example`, `.sample`, `.template`, `.dist`, `.defaults`), `*.pem`, `*.key` or a `secrets/` directory, and no `Read` deny rule (or `sandbox.filesystem.denyRead` entry) in `.claude/settings.json` or `.claude/settings.local.json` covers it
**Fix**: Manual - add `Read(./.env)`, `Read(./.env.*)` or `Read(./secrets/**)` deny rules
**Source**: code.claude.com/docs/en/settings-reference (`permissions.deny`), code.claude.com/docs/en/permissions

<a id="cc-set-040"></a>
### CC-SET-040 [MEDIUM] Memory Path Setting Cannot Match
**Requirement**: `claudeMdExcludes` patterns MUST be able to match an absolute path: "Patterns are matched against absolute file paths using glob syntax." `autoMemoryDirectory`: "The value must be an absolute path or start with `~/`."
**Detection**: `claudeMdExcludes` entries that are relative and start with a literal segment (not `/`, a drive letter, `~` or `*`); an `autoMemoryDirectory` that is neither absolute nor `~/`-prefixed
**Fix**: Manual - prefix the pattern with `**/` or write an absolute path
**Source**: code.claude.com/docs/en/memory ("Exclude specific CLAUDE.md files", "Storage location"), code.claude.com/docs/en/settings-reference

---

## PER-CLIENT SKILL RULES

<a id="cr-sk-001"></a>
### CR-SK-001 [MEDIUM] Cursor Skill Uses Unsupported Field
**Requirement**: Skills in `.cursor/skills/` SHOULD NOT use frontmatter fields Cursor does not read. Cursor reads `name`, `description`, `paths`, `disable-model-invocation`, `icon`, `color` and `metadata` ("Frontmatter fields"), and still accepts the legacy `globs` ("still accepted as a fallback for older skills"; CUR-023 notes it).
**Detection**: SKILL.md path contains `.cursor/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: cursor.com/docs/skills ("Frontmatter fields", "Scoping a skill to specific files")

<a id="cl-sk-001"></a>
### CL-SK-001 [MEDIUM] Cline Skill Uses Unsupported Field
**Requirement**: Skills in `.cline/skills/` SHOULD NOT use frontmatter fields unsupported by Cline
**Detection**: SKILL.md path contains `.cline/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.cline.bot/customization/skills ("Skill Structure"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseSkillConfigFromMarkdown`)

<a id="cp-sk-001"></a>
### CP-SK-001 [MEDIUM] Copilot Skill Uses Unsupported Field
**Requirement**: Skills in `.github/skills/` SHOULD NOT use frontmatter fields GitHub Copilot does not read. Copilot reads the Agent Skills fields (`name`, `description`, `license`, `compatibility`, `metadata`), `allowed-tools` ("In your `SKILL.md` frontmatter, you can use the `allowed-tools` field") and, in VS Code, `argument-hint`, `user-invocable`, `disable-model-invocation` and `context`.
**Detection**: SKILL.md path contains `.github/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/add-skills, docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills, code.visualstudio.com/docs/agent-customization/agent-skills ("Header (required)")

<a id="cp-sk-002"></a>
### CP-SK-002 [MEDIUM] Copilot Skill Pre-approves shell or bash
**Requirement**: Copilot skills SHOULD NOT pre-approve terminal tools unless reviewed: "Only pre-approve the `shell` or `bash` tools if you have reviewed this skill and any referenced scripts, and you fully trust their source. ... When in doubt, omit `shell` and `bash` from `allowed-tools` so that Copilot must ask for your explicit confirmation before running terminal commands."
**Detection**: `allowed-tools` (string or list) names `shell` or `bash` in a skill Copilot loads: `.github/skills/` and `~/.copilot/skills/`, and `.agents/skills/` or `.claude/skills/` when the run targets Copilot (other tools read those folders too). Reported at the `allowed-tools` line.
**Fix**: Manual - remove the tool after reviewing the skill, or keep it for a trusted skill
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/add-skills ("Enabling a skill to run a script"), docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills

<a id="cx-sk-001"></a>
### CX-SK-001 [MEDIUM] Codex Skill Uses Unsupported Field
**Requirement**: Skills in `.agents/skills/` and `.codex/skills/` SHOULD NOT use frontmatter fields unsupported by Codex CLI. Codex's own validator allows `allowed_properties = {"name", "description", "license", "allowed-tools", "metadata"}`; the runtime parser ignores other keys.
**Detection**: SKILL.md path contains `.agents/skills/` or `.codex/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: learn.chatgpt.com/docs/build-skills, github.com/openai/codex (skill-creator scripts/quick_validate.py)

<a id="cx-sk-004"></a>
### CX-SK-004 [LOW] Skill Catalog Exceeds Codex Fallback Budget
**Requirement**: Repository skill metadata SHOULD fit Codex's initial skill list: "In Codex, the initial list also includes each skill's file path. By default, this list uses at most 2% of the model's context window, or 8,000 characters when the context window is unknown. If many skills are installed, Codex shortens skill descriptions first. For large skill sets, Codex may omit some skills from the initial list and show a warning."
**Detection**: Info on the first skill of a `.agents/skills` or `.codex/skills` directory when the names, descriptions and relative paths of its skills (up to six levels deep) exceed 8,000 characters. Low, not medium: for a model with a known context window the budget is 2% of that window, usually larger; user, system and other-directory skills are not counted, and the exact per-entry format is not documented.
**Fix**: No auto-fix (shorten descriptions, or raise skills.max_context_tokens)
**Source**: learn.chatgpt.com/docs/build-skills, learn.chatgpt.com/docs/config-file/config-reference

<a id="cx-sk-006"></a>
### CX-SK-006 [MEDIUM] agents/openai.yaml Field Ignored by Codex
**Requirement**: Skill interface metadata MUST be in the form Codex reads. Codex drops a whole `agents/openai.yaml` it cannot deserialize ("ignoring {metadata_path}: invalid {label}"), and individual fields it cannot use: empty values, `display_name` over 64 characters and `short_description` or `default_prompt` over 1024 ("ignoring {field}: exceeds maximum length of {max_len} characters"), `brand_color` not `#RRGGBB`, icons that are absolute or outside the skill's `assets/` ("ignoring {field}: icon must be a relative assets path"; `..` only to reach a plugin's shared `assets/`), and `dependencies.tools` entries without `type` or `value`.
**Detection**: Warning at the key in `<skills>/<name>/agents/openai.yaml`. The skill-creator authoring advice (short_description 25-64 characters, `$skill-name` in default_prompt, quoted strings) is not checked: OpenAI's own bundled skills do not follow it.
**Fix**: No auto-fix
**Source**: openai/codex codex-rs/skills/src/interface.rs, codex-rs/skills/src/metadata.rs, skill-creator references/openai_yaml.md

<a id="cx-sk-007"></a>
### CX-SK-007 [LOW] Auxiliary Documentation in Codex Skill Folder
**Requirement**: "Avoid adding a `README.md`, installation guide, changelog, duplicated quick reference, or other auxiliary documentation unless a specific task or packaging requirement calls for it." (skill-creator, "What Not to Include")
**Detection**: Info on a Codex SKILL.md whose folder contains README, README.md, README.txt, CHANGELOG.md, CHANGES.md, INSTALL.md or INSTALLATION.md (case-insensitive)
**Fix**: No auto-fix
**Source**: openai/codex skill-creator SKILL.md

<a id="oc-sk-001"></a>
### OC-SK-001 [MEDIUM] OpenCode Skill Uses Unsupported Field
**Requirement**: Skills in `.opencode/skills/` SHOULD use the Agent Skills fields: "Use these Agent Skills fields: `name` (required), `description` (required), `license` (optional), `compatibility` (optional), `metadata` (optional, string-to-string map)" and "Put OpenCode-specific behavior in `metadata` instead of adding custom top-level fields." (OpenCode 1). OpenCode 2 also documents `disable-model-invocation` ("Set to true for the same effect as opencode/autoinvoke: false").
**Detection**: SKILL.md path contains `.opencode/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: opencode.ai/docs/skills, opencode.ai/v2/docs/skills

<a id="ws-sk-001"></a>
### WS-SK-001 [MEDIUM] Windsurf Skill Uses Unsupported Field
**Requirement**: Skills in `.windsurf/skills/` SHOULD NOT use frontmatter fields unsupported by Windsurf
**Detection**: SKILL.md path contains `.windsurf/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.windsurf.com/windsurf/memories

<a id="kr-sk-001"></a>
### KR-SK-001 [MEDIUM] Kiro Skill Uses Unsupported Field
**Requirement**: Skills in `.kiro/skills/` SHOULD use only the frontmatter fields Kiro reads: `name`, `description`, `license`, `compatibility` and `metadata`. Kiro keeps any other top-level key but never reads it.
**Detection**: Top-level keys of the parsed frontmatter (nested keys and continuation lines are never mistaken for fields; unparseable frontmatter is left to AS-016). Fields other skill clients act on (`allowed-tools`, `disallowed-tools`, `argument-hint`, `model`, `context`, `agent`, `hooks`, `user-invocable`, `disable-model-invocation` and the other Claude Code skill fields) are warnings, because a skill relying on them behaves differently in Kiro; plain extra data (`version`, `tags`, `author`, ...) is info. Replaces XP-SK-001 on Kiro skills (XP-SK-001 still runs when KR-SK-001 is disabled).
**Fix**: [AUTO-FIX, safe/unsafe] Move a single-line scalar field under `metadata` (into an existing block mapping, or by creating the block in place), quoting numbers and booleans so `metadata` stays a string map; safe for plain data, unsafe for another client's field. Other values (lists, maps, multi-line scalars) and moves that cannot be applied are an unsafe deletion of the whole entry. Every fix is applied to a copy and re-parsed and is offered only when the frontmatter parses to the intended mapping. One run creates at most one `metadata` block; run `--fix` again to move the remaining fields.
**Source**: kiro.dev/docs/skills, agentskills.io/specification

<a id="kr-sk-002"></a>
### KR-SK-002 [LOW] Vague Kiro Skill Description
**Requirement**: A Kiro skill `description` SHOULD say what the skill does and when to use it, with specific keywords; Kiro activates skills by matching requests against it ("Write precise descriptions")
**Detection**: The description has fewer than four words (fewer than 12 characters for text written without spaces) or starts with a generic "Helps with" / "Assists with" phrase, the docs' own "Vague" example. Kiro skills only; info level.
**Fix**: No auto-fix (describe the task and add "Use when ...")
**Source**: kiro.dev/docs/skills/

Kiro agent rules check `.kiro/agents/**/*.json` and `*.md` against the engines a file targets. Kiro CLI 2.x ships two agent engines: V2, the 2.x default, loads the top-level `*.json` files with a strict loader; V3, opt-in since CLI 2.8.0 and the default from CLI 3.0, loads JSON (comments and trailing commas allowed) and Markdown agents recursively. `[tool_versions] kiro = "2.x"` checks V2 only and `"3.x"` V3 only. Unpinned, a JSON file with only CLI 2.x fields (`allowedTools`, `toolsSettings`, `toolAliases`, `keyboardShortcut`, `useLegacyMcpJson`, object-form `hooks`) targets V2, one with only V3 fields (non-null `permissions`, `excludedTools`, `includePowers`, `effortLevel`, `dispatchKind`, array `hooks`) targets V3, and other files target both; Markdown and nested JSON agents always target V3. A condition that makes every targeted engine skip the agent is an error, one that makes only one targeted engine skip it is a warning naming that engine, and dropped or ignored values are warnings or info. For an unpinned CLI 2.x-only file, conditions that will make V3 skip the agent are migration warnings. Evidence: the kiro.dev docs below, the Kiro CLI 2.32.1 loader (`kiro-cli agent validate`) and the V3 engine it bundles. Agent-embedded hooks that target V3 also get the hook rules KR-HK-021 (CLI 2.x or Claude Code tool names in a matcher), KR-HK-027 (tool hook without a specific matcher) and KR-HK-029 (unanchored regex matcher); the standalone hook file rules are documented with the Kiro hooks rules.

<a id="kr-ag-001"></a>
### KR-AG-001 [MEDIUM] Unknown or Ignored Kiro Agent Field
**Requirement**: Agent JSON and Markdown frontmatter SHOULD use only fields that a targeted engine reads
**Detection**: A top-level key that no engine knows (warning; the message says "JSON" or "frontmatter"). CLI 2.x fields in a Markdown agent, which V3 strips (warning), or in a JSON file that targets only V3 (info). In JSON files that also target V3 (universal profiles, or unpinned CLI 2.x profiles as a migration note), `toolAliases` and `keyboardShortcut` are info because V3 does not apply them, and `useLegacyMcpJson: true` without `includeMcpJson` is a warning when V3 is targeted (info as a migration note), since V3 then loads only the mcp.json servers that `tools` references. V3 fields in a file pinned to 2.x (info). A frontmatter `prompt`, which V3 ignores in favor of the body (warning). `useLegacyMcpJson`, `$schema`, `kiroUpgrade`, `effortLevel` and `dispatchKind` are known fields
**Fix**: Remove the field, or rename it to a documented field
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config, kiro.dev/docs/cli/v3/migration-guide

<a id="kr-ag-002"></a>
### KR-AG-002 [HIGH] Invalid Kiro Agent Resource Entry
**Requirement**: Agent `resources` entries MUST be `file://` or `skill://` strings (case-sensitive) or `knowledgeBase` objects
**Detection**: Another string, an object with another or no `type`, or a non-string non-object entry: Kiro CLI 2.x skips the agent and V3 drops the entry (error when only V2 is targeted, otherwise a warning). For V3, a `knowledgeBase` entry without a `file://` source, with an `indexType` other than `best` or `fast`, a non-boolean `autoUpdate`, or non-string `name`/`description` is dropped (warning)
**Fix**: Use a valid resource URI or `knowledgeBase` object
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-003"></a>
### KR-AG-003 [MEDIUM] allowedTools Trusts a Tool Not in tools
**Requirement**: Each `allowedTools` entry SHOULD name a tool that `tools` makes available (CLI 2.x)
**Detection**: Only when Kiro CLI 2.x is targeted and `tools` is present. An entry is covered by `*`, by `@builtin` for built-in tools, by any name or alias of the same tool (`read`/`fs_read`/`fsRead`, `shell`/`execute_bash`, ...), by the V3 tags `read`, `write` and `web` for the tools they group, by `@server` for `@server/tool`, by a tools glob, or through a `toolAliases` alias. Glob entries and names that match no tool (KR-AG-024) are not reported
**Fix**: Add the tool to `tools` or remove it from `allowedTools`
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/reference/built-in-tools

<a id="kr-ag-004"></a>
### KR-AG-004 [MEDIUM] Kiro Agent Model Is Not a Kiro Model ID
**Requirement**: Agent `model` SHOULD be a Kiro model ID string or null
**Detection**: Kiro serves model IDs dynamically, so there is no allowlist. Warn on Claude Code aliases (`inherit`, `sonnet`, `opus`, `haiku`, `opusplan`, `default`, optionally with `[1m]`): Kiro CLI 2.x sends them unchanged and V3 accepts one only if it matches a single served ID. Warn on values with whitespace (display names). A non-string value skips the agent in both engines (error), and `model: null` in Markdown frontmatter skips it in V3 (JSON accepts null)
**Fix**: Use a Kiro model ID such as `claude-sonnet-4.6` (see `/model`), or remove the field
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config, kiro.dev/docs/models

<a id="kr-ag-005"></a>
### KR-AG-005 [MEDIUM] Referenced MCP Server Not Loaded by Kiro CLI 2.x
**Requirement**: An agent whose `tools` or `allowedTools` reference `@server` SHOULD make that server available in Kiro CLI 2.x
**Detection**: Kiro CLI 2.x is targeted, `includeMcpJson` (or its alias `useLegacyMcpJson`) is explicitly false, and a referenced server is not in the agent's `mcpServers`. V3 includes mcp.json servers that `tools` references, so the message notes V3 when it is targeted
**Fix**: Set `includeMcpJson` to true or define the server inline
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/mcp/configuration

<a id="kr-ag-006"></a>
### KR-AG-006 [MEDIUM] Kiro Agent Sub-agent Access Cannot Work
**Requirement**: An agent that configures sub-agents SHOULD be able to spawn them, and its agent lists SHOULD name agents that exist
**Detection**: Kiro CLI 2.x is targeted and `toolsSettings.subagent` (or an alias key) is set, but `tools` has no `subagent`, `use_subagent`, `agent_crew`, `@builtin`, `*` or `subagent/...` entry: the docs say "Without it, the agent can't delegate" (warning). `toolsSettings.subagent.availableAgents` / `trustedAgents` entries (globs allowed) and V3 `tools` entries `subagent/<agent>` that match no agent file in the same `.kiro/agents/` directory, the agent itself, or a built-in agent (info, since global agents in `~/.kiro/agents/` are not visible). Prompt `@name` mentions are not checked: Kiro selects sub-agents by description or an explicit request, not an `@` syntax
**Fix**: Add `subagent` to `tools`; fix the agent name
**Source**: kiro.dev/docs/custom-agents/subagents, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-ag-008"></a>
### KR-AG-008 [HIGH] Invalid Explicit Agent Name
**Requirement**: An explicit agent `name` MUST be a non-empty string
**Detection**: A non-string or null `name` skips the agent in both engines (error). An empty string skips it in V3 (error or engine warning; Kiro CLI 2.x accepts it, so a 2.x-only file gets a warning). A whitespace-only name loads but is a warning
**Fix**: Set a non-empty name, or remove the field so V3 derives it from the path relative to `.kiro/agents/`
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-009"></a>
### KR-AG-009 [HIGH] Agent Prompt Empty or Invalid
**Requirement**: A JSON `prompt` MUST be a string or null; a prompt SHOULD not be empty
**Detection**: A non-string, non-null `prompt` skips the agent in both engines (error). `"prompt": null` is valid. A blank JSON prompt or an empty Markdown body is a warning
**Fix**: Set the prompt to inline text or a `file://` URI, or remove it
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-010"></a>
### KR-AG-010 [MEDIUM] Duplicate Tool Entries
**Requirement**: Kiro agent `tools` SHOULD NOT contain duplicate entries
**Detection**: Case-insensitive duplicates in `tools`, reported at the repeated entry
**Fix**: Remove the duplicate
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-011"></a>
### KR-AG-011 [LOW] Empty Tools Array
**Requirement**: Kiro agent `tools` SHOULD contain at least one entry if present
**Detection**: `tools` is an empty array (the agent gets no tools in either engine)
**Fix**: Add tools, or remove the field if that is not intended
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-012"></a>
### KR-AG-012 [MEDIUM] toolAliases Renames a Tool Not in tools
**Requirement**: Each `toolAliases` key (the original tool name, `@server/tool` for MCP tools) SHOULD be a tool that `tools` makes available; the value is the new name
**Detection**: Kiro CLI 2.x is targeted, `tools` is present, and the key is not covered by `tools` with the KR-AG-003 selector semantics
**Fix**: Add the original tool to `tools` or remove the alias
**Source**: kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-ag-013"></a>
### KR-AG-013 [HIGH] Secrets in Agent Prompt
**Requirement**: Kiro agent prompts MUST NOT contain hardcoded credentials
**Detection**: Scan the JSON `prompt` and the Markdown body for secret patterns (API keys, tokens, passwords); `${VAR}` references are ignored
**Fix**: No auto-fix (use environment variables)
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-014"></a>
### KR-AG-014 [HIGH] Invalid Kiro Agent Permissions Block
**Requirement**: An agent `permissions` block MUST be an object with a `rules` array of `{capability, effect, match?, exclude?}` rules (V3)
**Detection**: For V3, any shape error skips the whole agent: `permissions` null or not an object, `rules` missing or not an array, a rule that is not an object, a missing or non-string `capability`, an `effect` other than `allow`, `ask`, `deny` or `evaluate`, non-string-array `match`/`exclude`, or non-string-array `policies` (error, or a warning naming V3 when 2.x is also targeted). Values V3 skips or strips are warnings: an unknown capability or `effect: evaluate` (rule skipped), an unknown policy preset, and unknown keys (a typo such as `matches` leaves a rule that applies to every resource). Kiro CLI 2.x ignores `permissions`
**Fix**: Manual - correct the permissions rule
**Source**: kiro.dev/docs/cli/v3/agent-config, kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions

<a id="kr-ag-015"></a>
### KR-AG-015 [MEDIUM] Unconverted CLI 2.x Trust Configuration
**Requirement**: A JSON agent with an `allowedTools` or `toolsSettings` key MUST also have a non-null `permissions` block to load in Kiro CLI V3
**Detection**: Key presence counts, even for null, `[]` or `{}`. Markdown agents are not reported (V3 strips the keys there; see KR-AG-001). Warning when unpinned, error when pinned to 3.x, nothing when pinned to 2.x
**Fix**: [AUTO-FIX, unsafe] When every such key is empty or null, remove them (`kiro-cli agent upgrade` leaves such profiles unconverted); this makes V3 load the agent. Otherwise run `/upgrade-agent` (or `kiro-cli agent upgrade`) or add a `permissions` block
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/migration-guide, kiro.dev/docs/cli/v3/upgrade-agent

<a id="kr-ag-016"></a>
### KR-AG-016 [HIGH] Invalid Kiro Agent JSON
**Requirement**: Agent JSON MUST parse as a JSON object in every targeted engine
**Detection**: JSON that does not parse even after removing comments and trailing commas, or whose top level is not an object, skips the agent in both engines (error at the syntax error). Comments or trailing commas parse only in V3 (error for a 2.x-only file, a warning naming 2.x when both engines are targeted, nothing for a V3-only file)
**Fix**: Fix the syntax; remove comments and trailing commas for Kiro CLI 2.x
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-017"></a>
### KR-AG-017 [HIGH] Invalid Kiro Markdown Agent Frontmatter
**Requirement**: A Markdown agent MUST start with a non-empty YAML frontmatter mapping (V3)
**Detection**: V3 loads every `.md` file under `.kiro/agents/` as an agent. A file without frontmatter makes V3 report "No front matter found" (warning, since such files are usually notes or prompt files); empty frontmatter, invalid YAML or a non-mapping skips the agent (error)
**Fix**: Add valid frontmatter, or move non-agent Markdown out of `.kiro/agents/`
**Source**: kiro.dev/docs/custom-agents, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-018"></a>
### KR-AG-018 [HIGH] Kiro Agent Field Has the Wrong Type
**Requirement**: Known agent fields MUST have the type their engine expects
**Detection**: Wrong types skip the agent in each engine that reads the field: `description`, `welcomeMessage`, `includeMcpJson` and `resources` in both; `allowedTools`, `toolAliases` (and its values), `toolsSettings`, `useLegacyMcpJson`, `keyboardShortcut` and `$schema` in 2.x; `excludedTools`, `includePowers`, `effortLevel` and `dispatchKind` in V3. `tools` must be an array of strings in 2.x, `"*"` or an array in V3 JSON, and a string or array in Markdown. V3 rejects null for most fields (JSON accepts null for `description`, `prompt`, `model`, `welcomeMessage`). A known field repeated in one JSON object, or both `includeMcpJson` and its alias `useLegacyMcpJson`, skips the agent in 2.x. A non-object `toolsSettings` entry only makes 2.x warn (warning)
**Fix**: Use the documented type
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-019"></a>
### KR-AG-019 [HIGH] Kiro Agent Missing Name for CLI 2.x
**Requirement**: A JSON agent loaded by Kiro CLI 2.x MUST have a `name` field
**Detection**: A top-level JSON agent without `name` that targets 2.x ("missing field `name`"; V3 derives the name from the path, so the finding is a warning when both engines are targeted)
**Fix**: Add a `name` field
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/2x-reference, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-020"></a>
### KR-AG-020 [MEDIUM] Non-Agent File in Kiro Agents Directory
**Requirement**: JSON files in `.kiro/agents/` SHOULD be agent profiles
**Detection**: A JSON object with no known agent field (for example generated `*.attribution.json` metadata): Kiro CLI 2.x rejects it (missing name) and V3 lists it as an empty agent. Error when only 2.x is targeted, otherwise a warning. Field-level rules are not run on such files
**Fix**: Move non-agent files out of `.kiro/agents/`
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/custom-agents

<a id="kr-ag-021"></a>
### KR-AG-021 [HIGH] Kiro Agent file:// Prompt Cannot Be Loaded
**Requirement**: A `file://` prompt MUST point at an existing file that every targeted engine accepts
**Detection**: Relative paths resolve against the agent file's directory. A missing target or an empty path skips the agent in both engines (error). An absolute or `~` path, or a relative path that leaves the workspace root (the directory holding `.kiro/agents`), skips it in V3 only (error for V3-only files, otherwise a warning, including a migration warning for 2.x-only files)
**Fix**: Fix the path or create the file; use a relative path inside the workspace for V3
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/cli/v3/agent-config, kiro.dev/docs/cli/v3/upgrade-agent

<a id="kr-ag-022"></a>
### KR-AG-022 [MEDIUM] Reserved or Duplicate Kiro Agent Name
**Requirement**: An agent id SHOULD be unique in its `.kiro/agents/` directory and not reserved by a V3 built-in agent
**Detection**: V3 silently drops file agents whose id (the `name`, else the path) is `vibe`, `spec`, `quick-spec`, `bug-fix`, `plan`, `autonomous` or `default-v2`. Another file with the same identity is a warning: Kiro CLI 2.x compares the `name` of top-level JSON files and keeps the first it reads; V3 compares ids across JSON and Markdown agents (ignoring JSON profiles it skips) and keeps the last
**Fix**: Rename the agent
**Source**: kiro.dev/docs/custom-agents, kiro.dev/docs/custom-agents/creating, kiro.dev/docs/custom-agents/built-in

<a id="kr-ag-023"></a>
### KR-AG-023 [HIGH] Invalid Inline MCP Server in Kiro Agent
**Requirement**: Inline `mcpServers` MUST be an object of correctly typed server objects
**Detection**: A non-object `mcpServers`, a non-object server, or a wrongly typed field (`command`, `args`, `env`, `headers`, `url`, `type`, `timeout`, `disabled`, `oauth`, `oauthScopes`) skips the agent in Kiro CLI 2.x; V3 skips just that server. For V3 (warning): a server with neither `command` nor `url` (unless disabled or of type `registry`), a URL that does not parse or is not https outside localhost, a `timeout` below 1, an invalid `versionNegotiation`, or wrongly typed V3-only fields (`cwd`, `disabledTools`, `autoApprove`, `waitForReady`)
**Fix**: Correct the server declaration
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/mcp/configuration

<a id="kr-ag-024"></a>
### KR-AG-024 [MEDIUM] Kiro Agent Tool Name Matches No Tool
**Requirement**: `tools` and `allowedTools` entries SHOULD name tools the targeted engines have
**Detection**: An `allowedTools` entry `"*"`, which the docs say is not supported (warning). Claude Code tool names (`Read`, `Bash`, ...), Claude Code MCP names (`mcp__server__tool`) and names differing from a Kiro name only in case, which match nothing in either engine. `allowedTools` entries (2.x) that are not 2.x tool names, `@` references or globs, which trust nothing (the upgrader's unmapped-allowed-tool). For V3: CLI 2.x names V3 does not alias (`fs_read`, `grep`, `glob`, `fsWrite`, `execute_cmd`, `web_search`, `use_subagent`, ...) when no entry selects the V3 equivalent, and `use_aws`/`aws`, which V3 has no tool for (the upgrader's deprecated-aws-tool). For 2.x: V3 tool ids such as `read_file` without a 2.x equivalent in the list. For an unpinned 2.x-only file, the V3 cases the upgrader does not rewrite are info
**Fix**: Use Kiro tool names, `@builtin`, or `@server` / `@server/tool`
**Source**: kiro.dev/docs/custom-agents/configuration-reference, kiro.dev/docs/reference/built-in-tools, kiro.dev/docs/tools, kiro.dev/docs/cli/v3/migration-guide, kiro.dev/docs/cli/v3/upgrade-agent

<a id="kr-ag-025"></a>
### KR-AG-025 [MEDIUM] Agent Hook Matcher Never Fires in Kiro CLI 2.x
**Requirement**: A tool-hook matcher in an agent that Kiro CLI 2.x loads SHOULD name tools CLI 2.x has
**Detection**: Kiro CLI 2.x is targeted and a `preToolUse`/`PreToolUse` or `postToolUse`/`PostToolUse` matcher (or a `|` alternative of one) is a V3 tool id with a CLI 2.x equivalent, such as `read_file`, `file_search`, `grep_search`, `str_replace`, `execute_pwsh` or `remote_web_search`: CLI 2.x matchers are tool names, aliases, globs and `@server` selectors, so the hook never fires there. The V3 direction (CLI 2.x or Claude Code names that match no V3 tool) is KR-HK-021, which also runs on agent-embedded hooks
**Fix**: Use the CLI 2.x tool name (`read`, `glob`, `grep`, `write`, `shell`, ...)
**Source**: kiro.dev/docs/cli/2x-reference, kiro.dev/docs/cli/v3/hooks-migration, kiro.dev/docs/reference/built-in-tools

<a id="kr-ag-026"></a>
### KR-AG-026 [MEDIUM] Kiro Agent Hook Timeout Field or Unit Mismatch
**Requirement**: Object-form hooks SHOULD set `timeout_ms` (milliseconds); array-form hooks SHOULD set `timeout` (seconds)
**Detection**: `timeout` in an object-form entry (ignored by 2.x and dropped by V3), `timeout_ms` in an array-form entry (ignored), a `timeout_ms` below 100 (likely seconds), and an array `timeout` that is a multiple of 1000 (likely milliseconds)
**Fix**: Use the field and unit of the hook form
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/actions, kiro.dev/docs/cli/2x-reference

<a id="kr-ag-027"></a>
### KR-AG-027 [MEDIUM] Kiro Agent Hook Field Ignored
**Requirement**: Agent hook entries SHOULD use only fields the targeted engines read
**Detection**: Unknown keys in object-form entries (for example the IDE's `toolTypes`; `description` and `name` are accepted) or array-form entries (warning). For V3-only files, the CLI 2.x fields `timeout_ms`, `max_output_size` and `cache_ttl_seconds`, which V3 drops (info). Agent-type actions, which Kiro CLI 2.x skips when loading (info)
**Fix**: Remove the field or use the documented field for the hook form
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/cli/2x-reference, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-ag-028"></a>
### KR-AG-028 [MEDIUM] Agent File Not Loaded by a Targeted Kiro Engine
**Requirement**: An agent file SHOULD sit where the engines it is meant for read it
**Detection**: Kiro CLI 2.x reads only top-level `.json` files of `.kiro/agents/` (or of the `.amazonq/cli-agents/` fallback); V3 reads `.json` and `.md` at any depth under `.kiro/agents/` and never reads `.amazonq/cli-agents/`. Pinned to 2.x, a Markdown agent with frontmatter or a nested JSON agent is reported; pinned to 3.x, or unpinned (V3 becomes the default in CLI 3.0), an agent in `.amazonq/cli-agents/` is reported. When the pinned engine does not read the file no other rule runs on it, and findings never name an engine that does not read the file
**Fix**: Move the agent to `.kiro/agents/` as a top-level JSON file, or pin the engine you use
**Source**: kiro.dev/docs/cli/2x-reference, kiro.dev/docs/custom-agents, kiro.dev/docs/cli/v3/agent-config

<a id="kr-ag-029"></a>
### KR-AG-029 [LOW] Overly Broad Kiro Agent Resource Glob
**Requirement**: Agent `resources` SHOULD name the files the agent needs rather than the whole workspace
**Detection**: A `file://` resource of `**`, `*`, `**/*`, `**/**` or `**/*.*` (optionally after `./`), which loads every workspace file into context on each request (info)
**Fix**: List specific files or globs; use `skill://` resources or a knowledge base for large material
**Source**: kiro.dev/docs/cli/chat/context, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-ag-030"></a>
### KR-AG-030 [LOW] Sub-agent Description Missing or Too Short
**Requirement**: An agent that another agent uses as a sub-agent SHOULD have a description that says what it does and when to use it
**Detection**: Another agent file in the same `.kiro/agents/` directory lists this agent (by name, V3 id or glob) in `toolsSettings.subagent.availableAgents` / `trustedAgents` or as a `subagent/<agent>` tool, and this agent has no `description` or one with fewer than four words (info). Kiro picks sub-agents by their description; agents nobody lists are not reported, since the configuration reference calls the field primarily for human readability
**Fix**: Describe the agent's purpose and the requests it handles
**Source**: kiro.dev/docs/custom-agents/subagents, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-hk-005"></a>
### KR-HK-005 [HIGH] Invalid Kiro Agent Hook Trigger
**Requirement**: Agent hook triggers MUST be names the targeted engines accept
**Detection**: An object-form `hooks` key other than `agentSpawn`, `userPromptSubmit`, `preToolUse`, `postToolUse` or `stop` skips the agent in Kiro CLI 2.x; V3 converts the object form and runs the hook if the key is a V3 trigger or exact-case alias (`PreToolUse`, `sessionStart`, ...), otherwise skips the hook. An array-form `trigger` that is not a V3 trigger or alias (case-sensitive; `AgentSpawn` is not an alias) makes V3 skip that hook (warning)
**Fix**: [AUTO-FIX, unsafe] Rename a case variant or V3 name of a CLI 2.x trigger key to its camelCase form (when that key is not already present), or an array trigger to its canonical V3 name; the hook then runs
**Source**: kiro.dev/docs/cli/2x-reference, kiro.dev/docs/hooks/types, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-hk-006"></a>
### KR-HK-006 [HIGH] Invalid Kiro Agent Hook Entry
**Requirement**: Each agent hook entry MUST be valid for the targeted engines
**Detection**: Object form: each trigger value must be an array of objects with a string `command` (both engines), a string `matcher` and integer `timeout_ms`, `max_output_size` and `cache_ttl_seconds` (2.x); an empty command fails V3 and a blank one is a warning. Array form: each entry needs a non-empty `name` (V3) and `trigger`, an `action` of type `command` with a non-empty `command` or `agent` with a non-empty `prompt`, a string `matcher`, a whole non-negative `timeout`, a boolean `enabled` and a valid Stop `confirm` object. A single invalid inline hook skips the whole agent in V3
**Fix**: Correct the hook entry
**Source**: kiro.dev/docs/cli/2x-reference, kiro.dev/docs/hooks, kiro.dev/docs/custom-agents/configuration-reference

<a id="amp-sk-001"></a>
### AMP-SK-001 [MEDIUM] Amp Skill Uses Unsupported Field
**Requirement**: Skills in `.agents/skills/` SHOULD NOT use frontmatter fields unsupported by Amp
**Detection**: SKILL.md path contains `.agents/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.amp.dev/setup/customization

<a id="amp-001"></a>
### AMP-001 [HIGH] Invalid Amp Check Frontmatter
**Requirement**: `.agents/checks/*.md` files MUST include valid YAML frontmatter with required `name` and known optional fields
**Detection**: Missing frontmatter OR invalid YAML OR missing `name` OR unknown key outside `name`, `description`, `severity-default`, `tools`
**Fix**: [AUTO-FIX] Add valid frontmatter with required fields and remove unknown keys
**Source**: ampcode.com/manual#code-review-checks

<a id="amp-002"></a>
### AMP-002 [MEDIUM] Invalid Amp severity-default
**Requirement**: `severity-default` SHOULD be one of `low`, `medium`, `high`, `critical`
**Detection**: Frontmatter `severity-default` value is missing, non-string, or outside allowed values
**Fix**: [AUTO-FIX] Set `severity-default` to a valid value
**Source**: ampcode.com/manual#code-review-checks

<a id="amp-003"></a>
### AMP-003 [MEDIUM] Invalid AGENTS.md globs Frontmatter for Amp
**Requirement**: AGENTS frontmatter `globs` SHOULD contain syntactically valid glob patterns for Amp
**Detection**: `globs` is invalid type OR contains a pattern that fails glob parsing (after Amp implicit `**/` behavior)
**Fix**: Correct glob syntax in `globs` frontmatter
**Source**: ampcode.com/manual#settings

<a id="amp-004"></a>
### AMP-004 [HIGH] Invalid Amp Settings Configuration
**Requirement**: `.amp/settings.json` MUST be valid JSON and use known top-level keys, including documented dotted keys such as `amp.runner.autoUpdate.enabled`, `amp.remoteThreadCreation.enabled`, and `amp.updates.mode`
**Detection**: JSON parse error OR unknown top-level key in `.amp/settings.json` / `.amp/settings.local.json`
**Fix**: [AUTO-FIX] Fix JSON syntax and remove unknown keys
**Source**: ampcode.com/docs/cli/settings, ampcode.com/docs/cli/runners

<a id="rc-sk-001"></a>
### RC-SK-001 [MEDIUM] Roo Code Skill Uses Unsupported Field
**Requirement**: Skills in `.roo/skills/` SHOULD NOT use frontmatter fields unsupported by Roo Code
**Detection**: SKILL.md path contains `.roo/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.roocode.com/features/custom-instructions

---

## CLAUDE CODE RULES (HOOKS)

<a id="cc-hk-001"></a>
### CC-HK-001 [HIGH] Invalid Hook Event
**Requirement**: Event MUST be one of 33 valid names (case-sensitive)
**Valid**: PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, Notification, MessageDisplay, UserPromptSubmit, UserPromptExpansion, Stop, SubagentStart, SubagentStop, TeammateIdle, TaskCompleted, TaskCreated, PreCompact, PostCompact, PreModelSwitch, PostModelSwitch, Setup, SessionStart, SessionEnd, InstructionsLoaded, ConfigChange, CwdChanged, DirectoryAdded, FileChanged, WorktreeCreate, WorktreeRemove, Elicitation, ElicitationResult, PermissionDenied, PostToolBatch, StopFailure
**Detection**: `!VALID_EVENTS.contains(event)`
**Fix**: [AUTO-FIX] Replace with closest matching valid event name
**Source**: code.claude.com/docs/en/hooks, github.com/anthropics/claude-code/releases/tag/v2.1.219, github.com/anthropics/claude-code/releases/tag/v2.1.251

<a id="cc-hk-002"></a>
### CC-HK-002 [HIGH] Prompt or Agent Hook on Wrong Event
**Requirement**: `type: "prompt"` and `type: "agent"` MUST only appear on events supported by that hook type, and `type: "http"` MUST NOT appear on SessionStart or Setup. `PermissionRequest` permits prompt hooks but skips agent hooks.
**Prompt-supported**: PreToolUse, PostToolUse, PostToolUseFailure, PostToolBatch, PermissionRequest, PermissionDenied, UserPromptSubmit, UserPromptExpansion, Stop, SubagentStop, TaskCreated, TaskCompleted, TeammateIdle
**Agent-supported**: PreToolUse, PostToolUse, PostToolUseFailure, PostToolBatch, PermissionDenied, UserPromptSubmit, UserPromptExpansion, Stop, SubagentStop, TaskCreated, TaskCompleted, TeammateIdle
**Vendor text**: "`SessionStart` and `Setup` support `command` and `mcp_tool` hooks ... They don't support `http`, `prompt`, or `agent` hooks." (hooks, "Prompt-based hooks")
**Detection**: `hook.type == "prompt" && !PROMPT_EVENTS.contains(event)` OR `hook.type == "agent" && !AGENT_EVENTS.contains(event)` OR `hook.type == "http" && event in [SessionStart, Setup]`
**Fix**: Change to `type: "command"` for unsupported events
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-003"></a>
### CC-HK-003 [LOW] Matcher Hint for Tool Events
**Requirement**: Tool events support an optional matcher field; omitting it matches all tools
**Detection**: `["PreToolUse", "PermissionDenied", "PostToolUse", "PostToolUseFailure"].contains(event) && matcher.is_none()`. A PermissionRequest group without a matcher is reported by CC-HK-035 instead.
**Fix**: Consider adding `"matcher": "Bash"` or `"*"` to target specific tools
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-005"></a>
### CC-HK-005 [HIGH] Missing Type Field
**Requirement**: Hook MUST declare `type`: `command`, `prompt`, `agent`, `http`, or `mcp_tool`
**Detection**: `hook.type.is_none()`
**Fix**: [AUTO-FIX] Add `"type": "command"`
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-006"></a>
### CC-HK-006 [HIGH] Missing Command Field
**Requirement**: `type: "command"` REQUIRES `command` field
**Detection**: `hook.type == "command" && hook.command.is_none()`
**Fix**: Add command field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-007"></a>
### CC-HK-007 [HIGH] Missing Prompt Field
**Requirement**: `type: "prompt"` and `type: "agent"` REQUIRE `prompt` field
**Detection**: `matches!(hook.type, "prompt" | "agent") && hook.prompt.is_none()`
**Fix**: Add prompt field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-008"></a>
### CC-HK-008 [HIGH] Script File Not Found
**Requirement**: Hook command script MUST exist on filesystem. Script paths in the exec-form `args` vector are checked too, since that is where the script sits when `command` names an interpreter.
**Detection**: Check if script paths in `command` and in each `args` element exist (resolve $CLAUDE_PROJECT_DIR)
**Fix**: Show error with correct path
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-009"></a>
### CC-HK-009 [MEDIUM] Dangerous Command Pattern
**Requirement**: Hook commands SHOULD be reviewed for destructive or remote-execution patterns. Vendor text: "Command hooks execute shell commands with your full user permissions. They can modify, delete, or access any files your user account can access. Review and test all hook commands before adding them to your configuration." (hooks, "Security considerations")
**Patterns**: `rm -rf /`, `rm -rf *`, `rm -rf ..`, `git reset --hard`, `git clean -fd`, `git push --force`, `drop database`, `drop table`, `truncate table`, `curl ... | sh`, `wget ... | sh`, `chmod 777`, writes to block devices, `mkfs.`, `dd ... of=/dev/`. The list is an agnix heuristic; the vendor names no patterns. `|| true` and `2>/dev/null` are no longer reported: they are not destructive, and Anthropic's own plugins use `|| true` on PostToolUse hooks.
**Detection**: Regex match against dangerous patterns
**Fix**: Warn, suggest safer alternative
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-010"></a>
### CC-HK-010 [MEDIUM] SessionEnd Hook Timeout Above Budget
**Requirement**: A SessionEnd hook timeout SHOULD stay within the SessionEnd budget. Vendor text: "`SessionEnd` hooks share a 1.5-second budget; if your settings set a longer per-hook `timeout`, Claude Code raises the budget to match, up to 60 seconds" (hooks, "Common fields") and "Timeouts set on plugin-provided hooks don't raise the budget." (hooks, "SessionEnd")
**Detection**:
  - Settings files and frontmatter hooks: SessionEnd `timeout > 60` (warning)
  - Plugin `hooks/hooks.json`: SessionEnd `timeout > 1` (info), since a plugin timeout never raises the 1.5-second budget
**Not reported**: a missing timeout, or a timeout above the event default. The hooks reference documents only defaults ("Defaults: 600 for `command`, `http`, and `mcp_tool`; 30 for `prompt`; 60 for `agent`.") and its own agent-hook example sets `"timeout": 120`.
**Fix**: Manual; `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` sets the budget explicitly
**Source**: code.claude.com/docs/en/hooks, code.claude.com/docs/en/hooks-guide

<a id="cc-hk-011"></a>
### CC-HK-011 [HIGH] Invalid Timeout Value
**Requirement**: timeout MUST be positive integer
**Detection**: `timeout <= 0`
**Fix**: Set to 30
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-012"></a>
### CC-HK-012 [HIGH] Hooks Parse Error
**Requirement**: Hooks configuration MUST be valid JSON
**Detection**: JSON parse error on settings.json. `.vscode/settings.json` (VS Code editor settings, which allow comments and trailing commas) and the `hooks/hooks.json` of a Gemini CLI extension or Codex plugin are not Claude Code files and get no CC-HK rules.
**Fix**: Fix JSON syntax errors in hooks configuration
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-013"></a>
### CC-HK-013 [HIGH] Async on Non-Command Hook
**Requirement**: `async: true` MUST only appear on `type: "command"` hooks
**Detection**: Check for `async` field on prompt or agent hook types
**Fix**: Auto-fix (safe) - remove the `async` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-014"></a>
### CC-HK-014 [MEDIUM] Once Outside Skill Frontmatter
**Requirement**: `once` field SHOULD only appear in skill frontmatter hooks; Claude Code ignores it in settings files and agent frontmatter
**Detection**: Check for `once` in settings and agent hooks
**Fix**: Manually remove the ignored field. No automatic edit is applied.
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-015"></a>
### CC-HK-015 [MEDIUM] Model on Command Hook
**Requirement**: `model` field MUST only appear on prompt or agent hooks
**Detection**: Check for `model` field on command hook types
**Fix**: Auto-fix (safe) - remove the `model` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-016"></a>
### CC-HK-016 [HIGH] Unknown Hook Type
**Requirement**: Hook `type` MUST be one of the five handler types. Vendor text: "There are five types" - `command`, `http`, `mcp_tool`, `prompt`, `agent` (hooks, "Hook handler fields")
**Detection**: `type` is not one of the five strings, or is not a string
**Fix**: Auto-fix (unsafe) - replace unknown hook type with the closest valid type
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-018"></a>
### CC-HK-018 [LOW] Matcher on Ignored Event
**Requirement**: Matchers on no-matcher events are silently ignored
**Events**: UserPromptSubmit, PostToolBatch, Stop, TeammateIdle, TaskCreated, TaskCompleted, WorktreeCreate, WorktreeRemove, MessageDisplay, CwdChanged
**Detection**: Check for matcher field on events listed in `NO_MATCHER_EVENTS`
**Fix**: Auto-fix (safe) - remove the `matcher` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-020"></a>
### CC-HK-020 [HIGH] HTTP Hook Missing URL
**Requirement**: HTTP hooks (type: "http") MUST have a url field
**Detection**: Check for type:"http" entries missing url key
**Fix**: Manual - add url field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-021"></a>
### CC-HK-021 [HIGH] Invalid If Field
**Requirement**: `if` MUST only appear on tool events and MUST hold one permission rule. Vendor text: "Only evaluated on tool events: `PreToolUse`, `PostToolUse`, `PostToolUseFailure`, `PermissionRequest`, and `PermissionDenied`. On other events, a hook with `if` set never runs" and "The `if` field holds exactly one permission rule. There is no `&&`, `||`, or list syntax for combining rules; to apply multiple conditions, define a separate hook handler for each." (hooks, "Common fields")
**Tool events**: PreToolUse, PermissionRequest, PermissionDenied, PostToolUse, PostToolUseFailure
**Detection**:
  - `if` on any other event: error at the `if` key (the hook never runs)
  - `&&`, `||` or a `,` list outside the rule's parentheses: warning (how Claude Code treats a combined string is not documented). Text inside `Tool(...)`, such as a Bash pattern with `&&`, is the rule's own argument and is not reported.
  - empty or non-string `if`: warning
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-022"></a>
### CC-HK-022 [MEDIUM] Invalid Shell Value
**Requirement**: shell field SHOULD be "bash" or "powershell"
**Detection**: Check shell field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-023"></a>
### CC-HK-023 [LOW] Once Field Not Boolean
**Requirement**: once field MAY be present and must be boolean
**Detection**: Check once field type
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-024"></a>
### CC-HK-024 [MEDIUM] Headers Missing AllowedEnvVars
**Requirement**: HTTP hook headers with $VAR interpolation SHOULD have allowedEnvVars
**Detection**: Check for $ patterns in headers without allowedEnvVars
**Fix**: Manual - add allowedEnvVars array
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-025"></a>
### CC-HK-025 [MEDIUM] Invalid Matcher Value
**Requirement**: Matcher values SHOULD name a value the event can match. Lifecycle events have documented value sets, and tool-event matchers are case-sensitive: "Check that the matcher pattern matches the tool name exactly. Matchers are case-sensitive" (hooks-guide, "Hook not firing")
**Detection**: Check exact matcher tokens against event-specific allowlists. Preserve match-all and JavaScript-regex paths; exact alternatives use the documented separators. StopFailure uses its narrower exact alphabet and pipe separator. Explicit older versions (before v2.1.234, 2026-08-17) retain the removed SessionEnd value `bypass_permissions_disabled`. On tool events, an exact alternative that equals a built-in tool name only when case is ignored (`bash`, `edit`) is reported.
**Notification values**: permission_prompt, idle_prompt, auth_success, elicitation_dialog, elicitation_url_dialog, elicitation_complete, elicitation_response, agent_needs_input, agent_completed, quota_auto_resume_fired, quota_auto_resume_stale, quota_auto_resume_disabled
**DirectoryAdded values**: slash_command, register_repo_root; this event supports matcher filtering
**StopFailure values**: rate_limit, overloaded, authentication_failed, oauth_org_not_allowed, account_on_hold, billing_error, invalid_request, model_not_found, server_error, max_output_tokens, cloud_credential_error, unknown
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks, code.claude.com/docs/en/hooks-guide

<a id="cc-hk-026"></a>
### CC-HK-026 [HIGH] MCP Tool Hook Missing Server
**Requirement**: A hook with `type: "mcp_tool"` MUST include a `server` field that is a non-empty string naming an already-connected MCP server
**Detection**: Walk `hooks.*[*].hooks[*]`; for entries with `type == "mcp_tool"`, flag when `server` is absent, not a string, or the empty string
**Fix**: Manual (add `"server": "<configured-server-name>"`)
**Source**: code.claude.com/docs/en/hooks#mcp-tool-hook-fields (Claude Code v2.1.118+)

<a id="cc-hk-027"></a>
### CC-HK-027 [HIGH] MCP Tool Hook Missing Tool
**Requirement**: A hook with `type: "mcp_tool"` MUST include a `tool` field that is a non-empty string naming the tool to invoke on the server
**Detection**: Walk `hooks.*[*].hooks[*]`; for entries with `type == "mcp_tool"`, flag when `tool` is absent, not a string, or the empty string
**Fix**: Manual (add `"tool": "<tool-name>"`)
**Source**: code.claude.com/docs/en/hooks#mcp-tool-hook-fields (Claude Code v2.1.118+)

<a id="cc-hk-028"></a>
### CC-HK-028 [HIGH] Rejected user_config Interpolation in Shell-Form Command
**Requirement**: A **shell-form** command hook's `command` string MUST NOT contain `${user_config.*}` interpolation. Claude Code v2.1.207 rejects it at load time as a shell-injection fix; values must be read inside the script via `$CLAUDE_PLUGIN_OPTION_<KEY>` (plugin hooks) or passed through the environment. Exec form is explicitly permitted - "Plugin hooks additionally substitute `${user_config.*}` values, in exec form only" - and exec form is what the presence of `args` selects.
**Detection**: Walk command-type hook entries; flag (error) when `args` is absent AND the `command` string contains the substring `${user_config.`.
**Fix**: Manual - replace `${user_config.<key>}` with `$CLAUDE_PLUGIN_OPTION_<KEY>` read inside the script, or restructure to pass the value via the environment.
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.207 (shell-injection fix rejecting `${user_config.*}` in shell-form commands)

<a id="cc-hk-029"></a>
### CC-HK-029 [MEDIUM] Invalid or Ineffective onFailure
**Requirement**: `onFailure` MUST be `"continue"` or `"block"`, and SHOULD only be set where it has an effect. Vendor text: "`onFailure` | no | Whether the action that triggered the hook goes ahead when the hook fails: `"continue"` or `"block"`" and "The field has no effect on these hooks: ... `Stop`, `SubagentStop`, `TaskCompleted`, and `TeammateIdle` hooks ... Background command hooks: command hooks that set `async` or `asyncRewake`" (hooks, "Command hook fields", "Block the action when a hook fails"). "Requires Claude Code v2.1.295 or later."
**Detection**:
  - value other than `continue` or `block`: warning at the value
  - info when it has no effect: on a hook type other than `command` or `http`, on Stop, SubagentStop, TaskCompleted or TeammateIdle, on an `async` or `asyncRewake` command hook, or when the pinned Claude Code version is older than 2.1.295 (released 2026-10-08)
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-030"></a>
### CC-HK-030 [MEDIUM] Unquoted Path Placeholder in Shell-Form Hook
**Requirement**: A shell-form command hook SHOULD quote each path placeholder. Vendor text: "Prefer exec form for any hook that references a path placeholder. In shell form, wrap each placeholder in double quotes." (hooks, "Reference scripts by path") and "If you leave one of these variables outside quotes in a shell-form command in a hooks file, `claude plugin validate` warns about it unless the hook sets `shell` to `"powershell"`." (plugins manifest reference, "Quoting and path separators")
**Detection**:
  - command hook without `args`, `shell` not `powershell`, whose `command` has `${CLAUDE_PROJECT_DIR}`, `$CLAUDE_PROJECT_DIR`, `${CLAUDE_PLUGIN_ROOT}` (or the `$` spelling) or `${CLAUDE_PLUGIN_DATA}` outside double quotes: warning at the command. Single-quoted text counts as quoted.
  - LOW branch (info): a script run by a relative path (`./x.sh`, `.claude/hooks/x.py`) in a settings file or plugin hooks file. Hooks-guide: "If you see "command not found", use absolute paths or `${CLAUDE_PROJECT_DIR}` to reference scripts." Skill and subagent frontmatter hooks are exempt, because the hooks reference's own frontmatter example runs `./scripts/security-check.sh`.
**Fix**: Manual - quote the placeholder or switch to exec form with `args`
**Source**: code.claude.com/docs/en/hooks, code.claude.com/docs/en/plugins/manifest-reference, code.claude.com/docs/en/hooks-guide

<a id="cc-hk-031"></a>
### CC-HK-031 [HIGH] Exec-Form Command With Embedded Arguments
**Requirement**: In exec form, `command` MUST be the executable only. Vendor text: "If `command` is a bare name with no path separator and contains whitespace alongside `args`, Claude Code logs a warning because the spawn will fail: there is no executable named `node script.js`. Move the extra tokens into `args`." and `shell` is "Ignored when `args` is set" (hooks, "Exec form and shell form", "Command hook fields")
**Detection**: command hook with `args` whose `command` contains whitespace and no `/` or `\` (error). `shell` set together with `args` is an info note.
**Fix**: Manual - move the extra words into `args`
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-032"></a>
### CC-HK-032 [HIGH] PowerShell Hook Placeholder Not Expanded
**Requirement**: A `shell: "powershell"` shell-form hook MUST reference the placeholders in a form PowerShell expands. Vendor text: "Don't write the bare `$CLAUDE_PROJECT_DIR` spelling in a PowerShell hook. PowerShell parses it as an undefined local variable and resolves it to `$null`, which leaves the script path without its project-root prefix." and "the placeholder works inside double-quoted strings but not inside single-quoted strings" (hooks, "Windows PowerShell tool")
**Detection**: shell-form hook with `shell: "powershell"` whose `command` has `$CLAUDE_PROJECT_DIR`, `$CLAUDE_PLUGIN_ROOT` or `$CLAUDE_PLUGIN_DATA` without braces (not `$env:`), or a `${CLAUDE_...}` placeholder inside a single-quoted string
**Fix**: Manual - write `${CLAUDE_PROJECT_DIR}` or `$env:CLAUDE_PROJECT_DIR` inside double quotes
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-033"></a>
### CC-HK-033 [HIGH] MCP Matcher Without Wildcard
**Requirement**: A tool-event matcher for an MCP server's tools MUST include the tool segment. Vendor text: "To match every tool from a server, append `.*` to the server prefix. The `.*` is required: a matcher like `mcp__memory` or `mcp__brave-search` contains only exact-match characters, so it is compared as an exact string and matches no tool." (hooks, "Match MCP tools")
**Detection**: on PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest and PermissionDenied, an exact-path matcher (only letters, digits, `_`, `-`, spaces, `,` and `|`) with an alternative `mcp__<server>` that has no second `__`
**Fix**: Auto-fix (unsafe) for a single-alternative matcher: change it to `mcp__<server>__.*`. The hook starts firing on that server's tools, so the fix changes behavior and requires opt-in.
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-034"></a>
### CC-HK-034 [HIGH] Plugin Hook Names Own MCP Server by Bare Key
**Requirement**: A plugin's hooks MUST name the plugin's own MCP servers by their scoped names. Vendor text: "Tools from a plugin-bundled MCP server use a scoped server segment that includes the plugin name: `mcp__plugin_<plugin-name>_<server-name>__<tool>`. A matcher written against the bare server key never fires for these tools." (hooks, "Match MCP tools") and `server` "For a plugin-bundled server, this is the scoped name `plugin:<plugin-name>:<server-name>`, such as `plugin:my-plugin:db`, not the bare server key" (hooks, "MCP tool hook fields")
**Detection**: in a plugin's `hooks/hooks.json` (the plugin root has `.claude-plugin/`), collect the server keys from `.mcp.json` and inline `plugin.json` `mcpServers`; flag a matcher or `if` rule that names `mcp__<key>` and an `mcp_tool` hook whose `server` is a bare key
**Fix**: Manual - use `mcp__plugin_<plugin>_<server>__...` and `plugin:<plugin>:<server>`
**Source**: code.claude.com/docs/en/hooks, code.claude.com/docs/en/plugins/components

<a id="cc-hk-035"></a>
### CC-HK-035 [MEDIUM] Broad PermissionRequest Matcher
**Requirement**: A PermissionRequest hook SHOULD have a narrow matcher. Vendor text: "Keep the matcher as narrow as possible. Matching on `.*` or leaving the matcher empty would auto-approve every tool permission prompt, including file writes and shell commands." (hooks-guide, "Auto-approve specific permission prompts")
**Detection**: PermissionRequest group with no matcher, `""`, `*` or `.*`, unless every hook in the group is `async`/`asyncRewake` (background hooks can't decide)
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks-guide

---

## CLAUDE CODE RULES (SUBAGENTS)

> **Scope note (Claude Code v2.1.116 / v2.1.117)**: agent-frontmatter `hooks` and `mcpServers` fields are loaded both for subagent spawning and for main-thread sessions launched via `claude --agent <name>`. The validators below check the structure of those fields regardless of which execution mode loads them.

<a id="cc-ag-001"></a>
### CC-AG-001 [HIGH] Missing Name Field
**Requirement**: Local agent frontmatter REQUIRES a `name` field. Vendor text: "No `name`: Claude Code treats the file as documentation kept beside your agents." (sub-agents, "Subagent files Claude Code skips"). A file with agent frontmatter but no `name` is therefore not loaded as a subagent.
**Detection**: Parse frontmatter of a `.claude/agents/` file, check for `name`
**Fix**: [AUTO-FIX] Add `name: agent-name`
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-002"></a>
### CC-AG-002 [HIGH] Missing Description Field
**Requirement**: Agent frontmatter REQUIRES `description` field
**Detection**: Parse frontmatter, check for `description`
**Fix**: [AUTO-FIX] Add description
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-003"></a>
### CC-AG-003 [HIGH] Invalid Model Value
**Requirement**: model MUST be a documented alias, a full `claude-*` model ID, or `inherit`. The sub-agents reference says `model` "Accepts the same values as the `--model` flag", i.e. the model-config alias table: `default`, `best`, `fable`, `sonnet`, `opus`, `haiku`, `opusplan`, `sonnet[1m]`, `opus[1m]`.
**Detection**: `!VALID_MODEL_ALIASES.contains(model) && !model.starts_with("claude-")`. The alias list is shared with CC-SK-001 so the two cannot drift.
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents, code.claude.com/docs/en/model-config

<a id="cc-ag-004"></a>
### CC-AG-004 [HIGH] Invalid Permission Mode
**Requirement**: permissionMode MUST be one of the six documented modes - `default`, `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions` - or `manual`, which the CLI accepts as an alias for `default` (requires Claude Code v2.1.200 or later). `delegate` is not a documented mode and is rejected.
**Detection**: `!VALID_PERMISSION_MODES.contains(permission_mode)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents, code.claude.com/docs/en/permission-modes

<a id="cc-ag-005"></a>
### CC-AG-005 [MEDIUM] Preloaded Skill Not Found or Not Preloadable
**Requirement**: Skills listed in `skills` SHOULD be ones Claude Code preloads. Claude Code skips missing or disabled skills with a debug warning, and the sub-agents reference says: "The full content of each listed skill is injected into the subagent's context at startup, up to the first 32 distinct names in the list." and "You can't preload skills that set `disable-model-invocation: true`" (sub-agents, "Preload skills into subagents")
**Detection**: Check project and user `.claude/skills/{name}/SKILL.md`, and for a plugin agent the plugin's own `skills/{name}/SKILL.md`. Reported at the `skills` key. Plugin-qualified references are left to plugin resolution; unresolved unqualified names get a warning because an enabled plugin may supply them. Also warn when the list has more than 32 distinct names, and when a project skill it names sets `disable-model-invocation: true`.
**Fix**: Check enabled plugins or add the skill to project or user skills; trim the list or drop `disable-model-invocation`.
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-006"></a>
### CC-AG-006 [MEDIUM] Tool/Disallowed Overlap
**Requirement**: Overlapping entries SHOULD be intentional. Claude Code applies disallowedTools first and removes tools listed in both fields.
**Detection**: Warn when `tools` intersects `disallowedTools`.
**Fix**: Remove redundant entries if unintended.
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-007"></a>
### CC-AG-007 [HIGH] Agent Parse Error
**Requirement**: Local agent frontmatter MUST begin on the first line and be valid YAML. Vendor text: "YAML that doesn't parse: Claude Code reads no fields from the file, skips it" and "An opening `---` that isn't the file's first line: Claude Code reads the file as having no frontmatter and treats it as documentation." (sub-agents, "Subagent files Claude Code skips"). Plugin agents with missing or malformed frontmatter load under their filename. `tools`/`disallowedTools` accept a comma/space-separated **string** (the canonical sub-agent form `tools: Read, Glob, Grep`) **or** a YAML list - both parse without error.
**Detection**: YAML parse error on agent frontmatter (error). A local agents-directory file with no frontmatter on its first line, such as `agents/README.md`, gets an info note that Claude Code treats it as documentation.
**Fix**: Fix YAML syntax errors in agent frontmatter
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-008"></a>
### CC-AG-008 [HIGH] Invalid Memory Scope
**Requirement**: `memory` field MUST be `user`, `project`, or `local`
**Detection**: Check `memory` value against allowed list
**Fix**: Auto-fix (unsafe) - replace with closest valid memory scope
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-009"></a>
### CC-AG-009 [HIGH] Invalid Tool Name in Tools List
**Requirement**: Tool names in `tools` MUST match known Claude Code tools
**Detection**: Check each tool name against known tools list; MCP tools are accepted in both documented forms - server-only `mcp__<server>` and fully qualified `mcp__<server>__<tool>` (case-sensitive lowercase prefix), with globs allowed in the tool segment only
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-010"></a>
### CC-AG-010 [HIGH] Invalid Tool Name in DisallowedTools
**Requirement**: Tool names in `disallowedTools` MUST match known Claude Code tools
**Detection**: Check each disallowed tool name against known tools list; MCP tools are accepted in both documented forms - server-only `mcp__<server>` and fully qualified `mcp__<server>__<tool>` (case-sensitive lowercase prefix), with globs allowed in the tool segment only
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-011"></a>
### CC-AG-011 [HIGH] Invalid Hooks in Agent Frontmatter
**Requirement**: `hooks` object MUST follow the same schema as settings.json hooks
**Detection**: Validate hooks object structure (event names, hook types, required fields)
**Fix**: Ensure hooks follow the settings.json hooks schema
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-012"></a>
### CC-AG-012 [MEDIUM] Bypass Permissions Warning
**Requirement**: Review `permissionMode: bypassPermissions`; since Claude Code 2.1.267 it takes effect only when the parent session already bypasses permissions and cannot escalate the parent mode
**Detection**: Check if permissionMode equals `bypassPermissions`
**Fix**: Review manually; no automatic rewrite of intent or permission behavior.
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-014"></a>
### CC-AG-014 [MEDIUM] Invalid Effort Value
**Requirement**: effort SHOULD be low, medium, high, xhigh, or max
**Detection**: Check effort field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-015"></a>
### CC-AG-015 [MEDIUM] Invalid Isolation Value
**Requirement**: isolation SHOULD be "worktree"
**Detection**: Check isolation field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-017"></a>
### CC-AG-017 [MEDIUM] Invalid MaxTurns Value
**Requirement**: maxTurns SHOULD be a positive integer
**Detection**: Check maxTurns is > 0
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-019"></a>
### CC-AG-019 [MEDIUM] Unknown or Ignored Agent Frontmatter Field
**Requirement**: Agent frontmatter fields SHOULD be fields Claude Code reads, with values it accepts. Vendor text: "Multi-word field names use camelCase, such as `maxTurns` and `disallowedTools`, and must match the table exactly: Claude Code ignores a field it doesn't recognize without reporting an error." and, for `autoCompactWindow`, "set `autoCompactWindow` in its frontmatter to a token count from `100000` to `1000000`" and "Write the value as a plain integer such as `200000`, because in frontmatter Claude Code reads `200k` as 200, which is outside the allowed range. The subagent still loads and compacts as if the field were absent." (sub-agents, "Supported frontmatter fields", "Set the auto-compact window for one subagent")
**Detection**: keys not in the known agent fields (with a specific hint for a top-level `cacheTtl`, which belongs inside `experimental`), and an `autoCompactWindow` that is not an integer from 100000 to 1000000 (a string such as `200k` is reported as ignored, not as a parse error). `experimental.cacheTtl` recognizes `5m` and `1h`; Claude Code ignores other values, so those are accepted; a non-map `experimental` value is a CC-AG-007 parse error.
**Fix**: Manual - remove or fix typo; write `autoCompactWindow` as a plain integer
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-020"></a>
### CC-AG-020 [HIGH] Invalid Local Agent Name
**Requirement**: Local Claude Code agent names MUST NOT start with `-`, contain `:`, or exceed 256 characters.
**Detection**: For local agents, flag a leading hyphen, reserved colon, or name longer than 256 characters. Plugin agents may use filename fallback.
**Fix**: Manual - use a local agent name of at most 256 characters without a leading hyphen or colon
**Source**: github.com/anthropics/claude-code/releases/tag/v2.1.218, code.claude.com/docs/en/sub-agents

<a id="cc-ag-021"></a>
### CC-AG-021 [MEDIUM] Subagent Descriptions Over Startup Budget
**Requirement**: The project's subagent descriptions SHOULD stay under the combined budget. Vendor text: "When the combined descriptions of your subagents, except the built-in ones, exceed 15,000 tokens, Claude Code shows a warning at startup with the total token count. Trim the `description` fields of your subagents, and move detail into each subagent's system prompt, which only loads when that subagent runs." (sub-agents) and "Each agent counts its name plus its `description` frontmatter." (errors)
**Detection**: sum of `name` plus `description` over the `.md` files in the `.claude/agents/` tree, estimated as characters / 4 (agnix's estimate; Claude Code's own count can differ, and user or plugin agents are not visible to a project lint). The first agent file in the tree reports the total once.
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents, code.claude.com/docs/en/errors

<a id="cc-ag-022"></a>
### CC-AG-022 [LOW] Subagent Inherits All Tools
**Requirement**: A subagent SHOULD get only the tools it needs. Vendor text: "Limit tool access: grant only necessary permissions for security and focus" (sub-agents, "Example subagents") and `tools` "Inherits every tool available to subagents if omitted."
**Detection**: agent with a `name` and neither `tools` nor `disallowedTools` (info)
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-023"></a>
### CC-AG-023 [MEDIUM] Duplicate Subagent Name
**Requirement**: Subagent names SHOULD be unique in a `.claude/agents/` tree. Vendor text: "Keep `name` values unique across the whole tree: if two files under the same `.claude/agents/` directory, including its subfolders, declare the same name, Claude Code loads only one of them, chosen by filesystem read order rather than a documented precedence." (sub-agents, "Choose the subagent scope")
**Detection**: another `.md` file in the same tree (with frontmatter on its first line) declares the same top-level `name`; each duplicate is reported and names the others
**Fix**: Manual - rename all but one
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-024"></a>
### CC-AG-024 [MEDIUM] disallowedTools Specifier Removes the Whole Tool
**Requirement**: `disallowedTools` SHOULD list whole tools. Vendor text: "A `disallowedTools` entry with a specifier, such as `Bash(git push *)`, still removes the whole tool from the subagent, not only the matching commands. To keep Bash and block specific commands, add a Bash deny rule such as `Bash(git push *)` to `permissions.deny` in your settings." and "The `Agent(agent_type)` allowlist syntax applies only to an agent running as the main thread with `claude --agent`. In a subagent definition, listing `Agent` in `tools` lets that subagent spawn subagents of its own while the depth limit allows it, but any type list inside the parentheses is ignored." (sub-agents, "Restrict which tools a subagent can use", "Restrict which subagents can be spawned")
**Detection**: a `disallowedTools` entry of the form `Tool(...)` (warning); an `Agent(...)` or `Task(...)` entry with a type list in `tools` (info, since agnix can't tell whether the file runs as the main thread), unless the agent sets `initialPrompt`, which only applies to a main session agent
**Fix**: Manual - move the pattern to `permissions.deny`
**Source**: code.claude.com/docs/en/sub-agents

---

## CLAUDE CODE RULES (OUTPUT STYLES)

Output-style files (`.claude/output-styles/*.md` or `~/.claude/output-styles/*.md`) customise Claude's response tone/format. The `keep-coding-instructions` frontmatter field was added in Claude Code v2.1.94. The output-styles page lists the fields `name`, `description`, `keep-coding-instructions`, and `force-for-plugin` ("Plugin output styles only"), and says: "All fields are optional, and field names use lowercase words separated by hyphens. A misspelled field is ignored without an error. If the YAML doesn't parse, the style still loads under its file name with no fields set".

<a id="cc-os-001"></a>
### CC-OS-001 [LOW] Output Style Missing Description
**Requirement**: Best practice: give the style a `description`. The field is optional ("All fields are optional"), but it is the text "shown in the `/config` picker".
**Detection**: Frontmatter parse - flag if `description` absent or whitespace-only
**Fix**: Manual - add a one-sentence summary
**Source**: code.claude.com/docs/en/output-styles

<a id="cc-os-002"></a>
### CC-OS-002 [HIGH] Output Style Invalid keep-coding-instructions Type
**Requirement**: `keep-coding-instructions` MUST be a YAML boolean (`true` or `false`): "Set to `true` to keep Claude Code's section of built-in software engineering instructions". Any other value does not keep them.
**Detection**: Frontmatter parse - reject string `"yes"`, number `1`, `null`, etc.
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: code.claude.com/docs/en/output-styles

<a id="cc-os-003"></a>
### CC-OS-003 [MEDIUM] Output Style Unknown Frontmatter Key
**Requirement**: Top-level frontmatter keys SHOULD be one of `name`, `description`, `keep-coding-instructions`: "A misspelled field is ignored without an error." `force-for-plugin` is reported with its own message, because it applies to "Plugin output styles only" and this rule sees only `.claude/output-styles/`.
**Detection**: Line scan - flag any other top-level key
**Fix**: Manual - remove or rename
**Source**: code.claude.com/docs/en/output-styles

<a id="cc-os-004"></a>
### CC-OS-004 [MEDIUM] Output Style Empty Body
**Requirement**: Output styles SHOULD have instructions after the closing `---` ("frontmatter for metadata, then the instructions for Claude"). An empty custom style adds nothing, and without `keep-coding-instructions: true` it also leaves out "Claude Code's section of built-in software engineering instructions".
**Detection**: Body lines after frontmatter are all whitespace
**Fix**: Manual - add the style's instructions
**Source**: code.claude.com/docs/en/output-styles

<a id="cc-os-006"></a>
### CC-OS-006 [HIGH] Invalid Output Style Frontmatter Syntax
**Requirement**: Output-style frontmatter MUST be valid YAML between two `---` delimiters: "If the YAML doesn't parse, the style still loads under its file name with no fields set".
**Detection**: YAML parse error or unclosed frontmatter
**Fix**: Manual - fix the YAML syntax (close frontmatter, escape special chars, etc.)
**Source**: code.claude.com/docs/en/output-styles

<a id="cc-os-007"></a>
### CC-OS-007 [MEDIUM] outputStyle Case Mismatch
**Requirement**: The settings key `outputStyle` MUST match a style name exactly: "The value is case-sensitive, so write the built-in names as `Proactive`, `Concise`, `Explanatory`, and `Learning`. A value that doesn't match a style name exactly, such as `explanatory`, gives you the Default style."
**Detection**: In Claude settings files, flag a string `outputStyle` that equals a built-in style name (or `default`) case-insensitively but not exactly, or the name or file stem of an output style in the project's `.claude/output-styles/` with different case. Unknown names are not reported, because user-level and plugin styles are not visible.
**Fix**: [AUTO-FIX, unsafe] Replace the value with the exact style name. It selects that style instead of Default, so it changes behavior.
**Source**: code.claude.com/docs/en/output-styles

---

## CLAUDE CODE RULES (MEMORY)

<a id="cc-mem-001"></a>
### CC-MEM-001 [HIGH] Invalid Import Path
**Requirement**: @import paths MUST exist on filesystem. Both relative and absolute paths are allowed, including home-directory imports such as `@~/.claude/my-project-instructions.md` (the documented way to share personal instructions across worktrees); Claude Code gates external imports behind a one-time approval dialog rather than rejecting them. Path shape is only rejected for non-memory files (REF-001), where no spec sanctions escaping the project root. A backslash-escaped space is part of the path: "To import a file whose path contains spaces, put a backslash before each space."
**Detection**: Extract `@path` references (code spans and fences skipped, `\ ` read as a space), check existence
**Fix**: Show error with resolved path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-002"></a>
### CC-MEM-002 [HIGH] Circular Import
**Requirement**: @imports MUST NOT create circular references
**Detection**: Build import graph, detect cycles
**Fix**: Show cycle path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-003"></a>
### CC-MEM-003 [HIGH] Import Depth Exceeds 4
**Requirement**: @import chain MUST NOT exceed 4 hops - "Imported files can recursively import other files, with a maximum depth of four hops."
**Detection**: Track import depth during resolution
**Fix**: Flatten import hierarchy
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-004"></a>
### CC-MEM-004 [MEDIUM] Invalid Command Reference
**Requirement**: npm scripts referenced in CLAUDE.md or CLAUDE.local.md SHOULD exist in package.json. The memory page's prompt audit looks for "references to files or commands that don't exist".
**Detection**: Extract `npm run <script>`, check the package.json of the project the memory file belongs to (the parent of `.claude/` for `.claude/CLAUDE.md`)
**Fix**: Show available scripts
**Source**: code.claude.com/docs/en/memory ("Audit your instruction files")

<a id="cc-mem-010"></a>
### CC-MEM-010 [MEDIUM] README Duplication
**Requirement**: Best practice: CLAUDE.md SHOULD NOT repeat the README. The best-practices Exclude column lists "Anything Claude can figure out by reading code", and the memory page says `/doctor` "cuts content Claude can derive from the codebase". The 40 percent word-overlap threshold is an agnix heuristic.
**Detection**: Word-set Jaccard similarity between CLAUDE.md (or CLAUDE.local.md) and the project's README.md above 40 percent
**Fix**: Remove duplicated sections
**Source**: code.claude.com/docs/en/best-practices, code.claude.com/docs/en/memory

<a id="cc-mem-011"></a>
### CC-MEM-011 [HIGH] Invalid Paths Glob in Rules
**Requirement**: Glob patterns in `.claude/rules/**/*.md` frontmatter `paths` MUST be valid. `paths` "Accepts a YAML list or a comma-separated string". "A pattern with a `[` that can't be read as a bracket expression, such as `photos [2024/**`, is invalid: it matches nothing". Frontmatter that does not parse is reported too: "If the YAML between the markers doesn't parse, Claude Code ignores the frontmatter and loads the rule as if it had no `paths`."
**Detection**: Parse YAML frontmatter; split a scalar `paths` string on commas outside brace groups; validate each pattern. Backslash escapes such as `photos \[2024/**` are valid.
**Fix**: Manual - fix glob syntax
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-012"></a>
### CC-MEM-012 [MEDIUM] Rules File Unknown Frontmatter Key
**Requirement**: `.claude/rules/*.md` frontmatter SHOULD only contain known keys: "`paths` is the only field Claude Code reads from a rule; any other field is ignored without an error."
**Detection**: Parse YAML frontmatter, flag keys not in known set
**Fix**: Auto-fix (unsafe) - remove unknown key line (may miss multi-line values)
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-014"></a>
### CC-MEM-014 [MEDIUM] Instruction File Exceeds 200 Lines
**Requirement**: Each instruction file SHOULD stay under 200 lines: "**Size**: target under 200 lines per CLAUDE.md file. Longer files consume more context and reduce adherence." The page counts more than CLAUDE.md: "Each CLAUDE.md, rules file, and `@path` import counts as a separate file." Splitting does not help on its own: "[Imports] help you organize a long file but don't reduce its context cost, because imported files also load at launch."
**Detection**: Count all lines (blank lines included) of CLAUDE.md, `.claude/CLAUDE.md`, CLAUDE.local.md and `.claude/rules/**/*.md`, and of files a memory file imports with `@path`. Block-level HTML comments in CLAUDE.md ("stripped before the content is injected") and rule frontmatter ("Claude Code removes the frontmatter before loading the rule") are not counted. An oversized import is reported at its `@path` line.
**Fix**: Manual - move part-of-codebase guidance into path-scoped `.claude/rules/` files and multi-step procedures into skills
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-015"></a>
### CC-MEM-015 [LOW] Always-Loaded Instructions Exceed Combined Size
**Requirement**: Instruction files loaded at launch SHOULD stay within a combined size: "You also see a warning when files that are each within that length add up past a combined limit at session start." Claude Code does not document the combined limit (its changelog says the warning threshold scales with the context window); the 400-line threshold is an agnix heuristic.
**Detection**: For each project directory, sum the lines of CLAUDE.md, `.claude/CLAUDE.md`, CLAUDE.local.md, every `.claude/rules/**/*.md` without `paths` ("Rules without a `paths` field are loaded unconditionally"), and the files they import (four hops, each file once). Report on the directory's first memory file when at least two files total more than 400 lines.
**Fix**: Manual - give part-of-codebase rules a `paths` field, move procedures into skills, trim content Claude can derive from the code
**Source**: code.claude.com/docs/en/memory ("My CLAUDE.md is too large")

<a id="cc-mem-016"></a>
### CC-MEM-016 [MEDIUM] MEMORY.md Exceeds Load Limit
**Requirement**: A MEMORY.md index MUST fit the load limit: "The first 200 lines of `MEMORY.md`, or the first 25KB, whichever comes first, are loaded at the start of every conversation. Content beyond that threshold is not loaded at session start." Subagent memory loads the same way: "The subagent's system prompt also includes the first 200 lines or 25KB of `MEMORY.md`". The docs do not say whether 25KB is 25,000 or 25,600 bytes; agnix uses 25,600.
**Detection**: `MEMORY.md` under `.claude/agent-memory/<agent>/`, `.claude/agent-memory-local/<agent>/` or `.claude/projects/<project>/memory/` with more than 200 lines or 25,600 bytes. Reported at the first line that is not loaded.
**Fix**: Manual - keep one line per entry, move detail into topic files, merge or drop stale entries
**Source**: code.claude.com/docs/en/memory, code.claude.com/docs/en/sub-agents

<a id="cc-mem-017"></a>
### CC-MEM-017 [LOW] Emphasis Overuse
**Requirement**: Best practice: emphasize one instruction, not many. "If Claude keeps skipping one instruction, add emphasis such as "IMPORTANT" to that line alone. If you emphasize many lines, none of them stands out." Anthropic's prompting guide adds: "The fix is to dial back any aggressive language. Where you might have said "CRITICAL: You MUST use this tool when...", you can use more normal prompting like "Use this tool when..."." The threshold of more than 5 emphasized lines is an agnix heuristic.
**Detection**: In CLAUDE.md, CLAUDE.local.md and `.claude/rules/**/*.md`, count lines outside code and HTML comments with an all-caps IMPORTANT, CRITICAL, MUST, NEVER, ALWAYS, MANDATORY or DO NOT. Documents that cite RFC 2119 or RFC 8174 are skipped.
**Fix**: Manual - write the instructions plainly and give the reason; keep emphasis for the one line Claude keeps skipping
**Source**: code.claude.com/docs/en/best-practices, platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices

<a id="cc-mem-018"></a>
### CC-MEM-018 [LOW] AGENTS.md Not Imported
**Requirement**: A CLAUDE.md SHOULD import AGENTS.md rather than tell Claude to read it: "**A `CLAUDE.md` that tells Claude in words to read `AGENTS.md`**: Claude sees `AGENTS.md` only if it decides to open the file. Delete the `CLAUDE.md` so Claude reads `AGENTS.md` directly, or replace the sentence with an `@AGENTS.md` import." A CLAUDE.local.md alone also hides AGENTS.md: "Because `CLAUDE.local.md` counts, adding one to keep your own uncommitted instructions in a project that relies on `AGENTS.md` stops Claude from reading `AGENTS.md` for you."
**Detection**: An AGENTS.md sits in the memory file's project directory and the file has no `@AGENTS.md` import. For CLAUDE.md: prose such as "Read AGENTS.md" or "see the `AGENTS.md` file" outside code blocks. For CLAUDE.local.md: no CLAUDE.md or `.claude/CLAUDE.md` in that directory or any directory above it.
**Fix**: Manual - add `@AGENTS.md`, or set Project instructions to `claude-md-and-agents-md`
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-019"></a>
### CC-MEM-019 [MEDIUM] Import Path Quoted or Split by a Space
**Requirement**: An import path with spaces MUST escape them: "Without the backslashes, the path ends at the first space, even when the import is on a line of its own. A path wrapped in quotes isn't imported at all, with or without the backslashes."
**Detection**: Outside code and comments: `@"path"` or `@'path'` whose path exists; `@name more words` where `name` does not exist but the name joined with the following words does.
**Fix**: Manual - remove the quotes and write `@Design\ Docs/api-conventions.md`
**Source**: code.claude.com/docs/en/memory ("Import additional files")

<a id="cc-mem-020"></a>
### CC-MEM-020 [LOW] Personal Claude File Not Gitignored
**Requirement**: Personal files SHOULD be gitignored: "Add `CLAUDE.local.md` to your `.gitignore` so it isn't committed." For a hand-made `.claude/settings.local.json`: "If you created the file by hand and Claude Code hasn't written to it yet, add it to `.gitignore` yourself." Local subagent memory in `.claude/agent-memory-local/` "shouldn't be checked into version control".
**Detection**: Inside a git repository, CLAUDE.local.md, `.claude/agent-memory-local/<agent>/MEMORY.md` or `.claude/settings.local.json` that no `.gitignore` from the repository root down (or `.git/info/exclude`) ignores. For settings.local.json the global excludes file also counts, because Claude Code adds `**/.claude/settings.local.json` there when it writes the file.
**Fix**: Manual - add the file to `.gitignore`
**Source**: code.claude.com/docs/en/memory, code.claude.com/docs/en/settings, code.claude.com/docs/en/sub-agents

<a id="cc-mem-021"></a>
### CC-MEM-021 [LOW] Directory Tree or File Listing in CLAUDE.md
**Requirement**: Best practice: leave out content Claude can derive from the code. The best-practices Exclude column lists "File-by-file descriptions of the codebase", and `/doctor` "cuts content Claude can derive from the codebase, such as directory layouts, dependency lists, and architecture overviews". A short layout note is fine ("project layout" is among the facts to keep), so the thresholds (a 20-line tree, a 15-item file listing) are agnix heuristics.
**Detection**: In CLAUDE.md, the longest run of directory-tree lines (`├──`, `└──`, `│`, `|-- `) of at least 20 lines, or at least 15 consecutive list items of the form `` - `path/file.ext` - description ``
**Fix**: Manual - keep pitfalls, rationale and conventions; cut the listing
**Source**: code.claude.com/docs/en/best-practices, code.claude.com/docs/en/memory

---

## AGENTS.MD RULES (CROSS-PLATFORM)

<a id="agm-001"></a>
### AGM-001 [HIGH] Valid Markdown Structure
**Requirement**: AGENTS.md MUST be valid markdown
**Detection**: Parse as markdown, check for syntax errors
**Fix**: [AUTO-FIX] Fix markdown syntax issues
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

<a id="agm-002"></a>
### AGM-002 [MEDIUM] Missing Section Headers
**Requirement**: AGENTS.md SHOULD have clear section headers (##)
**Detection**: `!content.contains("## ")` or `!content.contains("# ")`
**Fix**: Add section headers for organization
**Source**: docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

<a id="agm-003"></a>
### AGM-003 [MEDIUM] Character Limit (Windsurf)
**Requirement**: Rules files SHOULD be under 12000 characters for Windsurf compatibility
**Detection**: `content.len() > 12000`
**Fix**: Split into multiple files or reduce content
**Source**: docs.windsurf.com/windsurf/cascade/memories

<a id="agm-004"></a>
### AGM-004 [MEDIUM] Missing Project Context
**Requirement**: AGENTS.md SHOULD describe project purpose/stack
**Detection**: Check for project description section
**Fix**: Add "# Project" or "## Overview" section
**Source**: Best practices across platforms

<a id="agm-005"></a>
### AGM-005 [MEDIUM] Platform-Specific Features Without Guard
**Requirement**: Platform-specific instructions SHOULD be labeled
**Detection**: Claude-specific (hooks, context: fork) or Cursor-specific features without platform label
**Fix**: Add platform guard comment (e.g., "## Claude Code Specific")
**Source**: Multi-platform compatibility

## CLAUDE CODE RULES (PLUGINS)

<a id="cc-pl-001"></a>
### CC-PL-001 [HIGH] Plugin Manifest Not in .claude-plugin/
**Requirement**: The manifest is optional. When present, plugin.json MUST be in `.claude-plugin/`.
**Detection**: For an existing plugin.json, check that its parent directory is `.claude-plugin/`.
**Fix**: Move to correct location
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-002"></a>
### CC-PL-002 [HIGH] Components in .claude-plugin/
**Requirement**: No component directory may be inside `.claude-plugin/`. The documented set is `commands/`, `agents/`, `skills/`, `workflows/`, `output-styles/`, `themes/`, `monitors/`, and `hooks/` - "All other directories ... must be at the plugin root, not inside `.claude-plugin/`".
**Detection**: Check for each of those eight directories under `.claude-plugin/`
**Fix**: Move to plugin root
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-003"></a>
### CC-PL-003 [LOW] Invalid Semver
**Requirement**: Semver is an optional interoperability recommendation for plugins consumed through version-constrained dependencies. Claude Code accepts arbitrary version strings and does not check manifest versions against semver.
**Detection**: Warn when a non-empty version string does not parse as semver, including prerelease and build forms. This is advice, not a loader rejection.
**Fix**: Suggest valid semver. Partial versions can be normalized with an unsafe autofix because changing a version string can change dependency resolution.
**Source**: code.claude.com/docs/en/plugins/manifest-reference#version, code.claude.com/docs/en/plugins/dependencies#tag-plugin-releases-for-version-resolution

<a id="cc-pl-004"></a>
### CC-PL-004 [HIGH] Missing Required/Recommended Plugin Field
**Requirement**: plugin.json REQUIRES name; description and version are RECOMMENDED
**Detection**: Parse JSON, check required fields (error for name, warning for description/version)
**Fix**: Add missing fields
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-005"></a>
### CC-PL-005 [HIGH] Empty Plugin Name
**Requirement**: name field MUST NOT be empty
**Detection**: `name.trim().is_empty()`
**Fix**: Add plugin name
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-006"></a>
### CC-PL-006 [HIGH] Plugin Parse Error
**Requirement**: plugin.json MUST be valid JSON
**Detection**: JSON parse error on plugin.json
**Fix**: Fix JSON syntax errors in plugin.json
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-007"></a>
### CC-PL-007 [HIGH] Invalid Component Path
**Requirement**: Paths in every documented path-bearing manifest field MUST be relative to the plugin root, start with `./`, stay inside the plugin root and exist. Vendor text: "Every component path in a manifest is relative to the plugin root and must start with `./`. A path such as `commands/foo.md` fails validation.", "`agents` entries must be `.md` files, and `skills` entries must be directories." and "Every component path must resolve inside the plugin root and must exist." (plugins manifest reference, "Path rules", "Path-only fields", "Containment and existence"). The full set is `commands`, `agents`, `skills`, `workflows`, `hooks`, `mcpServers`, `outputStyles`, `lspServers`, `experimental.themes`, and `experimental.monitors`. Claude Code 2.1.221+ explicitly accepts `skills: "."` for a root-level `SKILL.md`. `mcpServers` also accepts a valid HTTPS URL ending in `.mcpb` or `.dxt`. agnix conservatively limits its URL exception to this field and literal bundle endpoints without query or fragment suffixes; this scope does not establish whether upstream loads URLs with those suffixes.
**Detection**: Check path fields for absolute paths (`/`, `C:\`), parent traversal (`..`), or a missing `./` prefix; exempt the documented `skills: "."` root path. For a manifest in `.claude-plugin/`, also report `agents` entries that are not `.md` paths (directories and inline objects), `skills` entries that name a `.md` file, and `./` paths that don't exist under the plugin root (paths with `$` placeholders are skipped).
**Fix**: Prepend `./` to relative paths other than the root `skills: "."` form [safe autofix]
**Source**: code.claude.com/docs/en/plugins/manifest-reference, github.com/anthropics/claude-code/releases/tag/v2.1.221

<a id="cc-pl-008"></a>
### CC-PL-008 [HIGH] Component Inside .claude-plugin
**Requirement**: Component paths in manifest MUST NOT point inside `.claude-plugin/` directory
**Detection**: Check if path fields reference `.claude-plugin/` subdirectories
**Fix**: Suggest moving components to plugin root
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-009"></a>
### CC-PL-009 [MEDIUM] Invalid Author Object
**Requirement**: If `author` field is present, `author.name` SHOULD be a non-empty string
**Detection**: Check `author.name` exists and is non-empty when `author` is present
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-010"></a>
### CC-PL-010 [HIGH] Invalid Homepage URL
**Requirement**: If `homepage` is present, it MUST parse as a URL. Vendor text: "Documentation URL. Must parse as a URL, or the plugin fails to load" (plugins manifest reference, "Fields")
**Detection**: `url::Url::parse(homepage)` fails, or `homepage` is not a string (error). A URL that parses but isn't `http` or `https` gets an info note, since the doc only says "parse as a URL". An empty string is not reported.
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-011"></a>
### CC-PL-011 [HIGH] LSP Server Missing Required Fields
**Requirement**: LSP servers MUST have command and extensionToLanguage
**Detection**: Check lspServers entries for required fields
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-012"></a>
### CC-PL-012 [MEDIUM] Invalid UserConfig Key
**Requirement**: userConfig keys SHOULD be valid identifiers
**Detection**: Check keys match identifier pattern
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-013"></a>
### CC-PL-013 [MEDIUM] Channel Missing Server Reference
**Requirement**: channels entries SHOULD have server field
**Detection**: Check each channel has server key
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-014"></a>
### CC-PL-014 [MEDIUM] Plugin Agent Field Ignored
**Requirement**: Plugin agent files SHOULD NOT set fields Claude Code ignores for plugin agents. Vendor text: "Ignored fields: `permissionMode`, `hooks`, `mcpServers`, and `initialPrompt`. An agent file can't add hooks or MCP servers on its own, so add those as plugin hooks and MCP servers instead" (plugins components, "Frontmatter fields in plugin agents")
**Detection**: frontmatter of an agent Markdown file under a plugin's `agents/` directory (the plugin root has `.claude-plugin/`) sets `permissionMode`, `hooks`, `mcpServers` or `initialPrompt`. Inline agent objects in `plugin.json` are not a documented shape ("`agents` entries must be `.md` files") and are reported by CC-PL-007.
**Fix**: Manual - remove the field, or declare hooks and MCP servers at plugin level
**Source**: code.claude.com/docs/en/plugins/components, code.claude.com/docs/en/sub-agents

<a id="cc-pl-015"></a>
### CC-PL-015 [MEDIUM] Default Component Folder Shadowed by Manifest
**Requirement**: If a root default component folder for a **replace-semantics** field exists, `plugin.json` SHOULD include that folder in the matching manifest field or avoid overriding that field. Per the "Path behavior rules" section of the plugins reference, only `commands`, `agents`, `workflows`, `outputStyles`, `experimental.themes`, and `experimental.monitors` replace their default. `skills` **adds** to the default scan (the default `skills/` directory is always scanned), and `hooks`/`mcpServers`/`lspServers` have their own merge rules, so none of those can shadow and none are checked.
**Detection**: Check `.claude-plugin/plugin.json` for a replace-semantics component field while the matching root folder exists and is not one of the configured paths. Folder names differ from manifest keys for `outputStyles` (`output-styles/`) and the `experimental.*` keys (`themes/`, `monitors/`).
**Fix**: Manual - add `./<component>` to the manifest field, or move the files into a configured component path.
**Source**: code.claude.com/docs/en/plugins/manifest-reference, github.com/anthropics/claude-code/releases/tag/v2.1.140

---

<a id="cc-pl-016"></a>

### CC-PL-016 [MEDIUM] Plugin Name Not Kebab-Case
**Requirement**: A plugin manifest `name` SHOULD be a kebab-case identifier - lowercase letters, digits, and hyphens, with no spaces, no leading or trailing hyphen, and no doubled hyphen.
**Detection**: Reject spaces, `@`, `:`, path separators, control and invisible characters. Warn for other accepted names that are not kebab-case, such as uppercase letters or underscores.
**Fix**: Manual - rename the plugin.
**Source**: code.claude.com/docs/en/plugins/manifest-reference, github.com/anthropics/claude-code/releases/tag/v2.1.247

<a id="cc-pl-017"></a>
### CC-PL-017 [MEDIUM] Unknown Plugin Manifest Field
**Requirement**: Manifest keys SHOULD match the documented top-level fields. Claude Code strips unknown keys at load time.
**Detection**: Warn for keys outside the manifest reference, including misspellings such as `homepag`.
**Fix**: Correct the key spelling or remove it.
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-018"></a>
### CC-PL-018 [HIGH] Reserved Plugin Name
**Requirement**: A plugin name MUST NOT pass as one of Anthropic's own. Vendor text: "`claude plugin validate` also checks that the name doesn't pass as one of Anthropic's own plugins. The check ignores case and treats any run of separators as one", with "Starts with `claude-`, `anthropic-`, `anthropics-`, or `cc-plugin-` | Error", "Is `claude`, `anthropic`, `anthropics`, `claude-code`, or `claude-mods` | Error", "Puts `official` beside `claude` or `anthropic`, such as `official-claude-tools` | Error" and "Has `claude`, `anthropic`, or `anthropics` as a whole word anywhere else, such as `mcp-for-claude` | Warning". "`claude plugin init` and `claude plugin tag` refuse a name that draws the error. Claude Code still installs and loads a plugin whose name they refuse." (plugins manifest reference, "`name`")
**Detection**: the `plugin.json` `name`, split into words on any non-alphanumeric run and lowercased: the error set is an error, the warning set a warning. Plugins whose `author` is Anthropic, or that sit in a marketplace owned by Anthropic, are exempt. Marketplace entry names get the same check under CC-MP-002.
**Fix**: Manual - rename the plugin
**Source**: code.claude.com/docs/en/plugins/manifest-reference, code.claude.com/docs/en/plugins/marketplace-reference

<a id="cc-pl-019"></a>
### CC-PL-019 [MEDIUM] Invalid Dependency Version Range
**Requirement**: A dependency `version` MUST be a semantic-version range. Vendor text: "`version` | A semantic-version range such as `~2.1.0`, `^2.0`, `>=1.4`, or `=2.1.0`. The dependency installs at the highest git tag that satisfies this range, so the dependency's maintainer must tag releases." (plugin dependencies, "Declare a dependency with a version constraint")
**Detection**: `dependencies[i].version` that is not a node-semver range: `||` alternatives of comparators (`<`, `<=`, `>`, `>=`, `=`, `~`, `~>`, `^`, X-ranges) or a hyphen range
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/dependencies, code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-020"></a>
### CC-PL-020 [MEDIUM] Unbraced Plugin Variable in Plugin Content
**Requirement**: Plugin skill, command and agent content SHOULD write path variables in braces. Vendor text: "The variables aren't present in the environment of commands Claude runs through the Bash tool, in the main session or in a subagent. In skill, command, and agent content, write the `${...}` reference in the Markdown body instead, and Claude Code substitutes the path inline when it loads the content." (plugins manifest reference, "Where each variable resolves")
**Detection**: the body of a plugin skill (SKILL.md under a plugin root) or plugin agent contains `$CLAUDE_PLUGIN_ROOT` or `$CLAUDE_PLUGIN_DATA` without braces. Hook commands get the variables from their environment, so lines that mention hooks, fenced JSON blocks and inline code spans holding only the variable name are skipped. Plugin command files are not detected as a file type.
**Fix**: Manual - write `${CLAUDE_PLUGIN_ROOT}`
**Source**: code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-021"></a>
### CC-PL-021 [MEDIUM] Plugin Settings Key Dropped
**Requirement**: A plugin's settings SHOULD only use the keys that take effect. Vendor text: "Only two keys take effect from a plugin, `agent` and `subagentStatusLine`; every other key is dropped." (plugins components) and, for the manifest key, "Only `agent` and `subagentStatusLine` take effect; other keys are dropped at load." (plugins manifest reference, "`settings`")
**Detection**: keys other than `agent`, `subagentStatusLine` (and `$schema`) in a `settings.json` beside `.claude-plugin/`, or in the manifest `settings` object
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/components, code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-022"></a>
### CC-PL-022 [HIGH] Monitor Command References user_config
**Requirement**: A monitor `command` MUST NOT reference `${user_config.*}`. Vendor text: "`command` gets the path variables and `${ENV_VAR}` from the environment, but never `${user_config.*}`. A monitor that references one doesn't start, and monitor processes don't receive `CLAUDE_PLUGIN_OPTION_<KEY>` either" (plugins components, "Monitors")
**Detection**: monitor entries inline in `experimental.monitors` (or a top-level `monitors` array), in the file `experimental.monitors` names, or in the default `monitors/monitors.json`
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/components, code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-pl-023"></a>
### CC-PL-023 [HIGH] Invalid Key or Value in Strict Manifest Object
**Requirement**: Strict manifest objects MUST only use their documented keys, and `userConfig` options MUST follow the options rules. Vendor text: "Strict objects: `userConfig` options, `channels` entries, `lspServers` configs, and `monitors` entries are strict. An unknown key inside one is an error, and the plugin doesn't load" (plugins manifest reference, "Unrecognized fields") and "`options` applies to a `string` field that isn't `multiple` or `sensitive`. Set `default` to one of the listed values, or set `required: true` so the user must pick one. Each option is a plain label of 1 to 64 characters ... A plugin whose `options` break these rules fails to load." ("Limit a field to fixed options")
**Detection**: unknown keys in `userConfig` options (top level and inside `channels[].userConfig`), `channels` entries, inline `lspServers` configs and `.lsp.json`/listed LSP files, and monitor entries; `options` on a non-string, `multiple` or `sensitive` field, labels that are not 1-64-character strings, and a `default` outside `options` without `required: true`
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/manifest-reference

---

## CLAUDE CODE RULES (PLUGIN MARKETPLACES)

> **Scope:** `.claude-plugin/marketplace.json` (a new file type). The rules mirror the `claude plugin validate` messages in the marketplace reference ("Validation messages") and the load-time failures it lists. Positions point at the field each message names.

<a id="cc-mp-001"></a>
### CC-MP-001 [HIGH] Marketplace Parse Error or Missing Required Field
**Requirement**: The marketplace file MUST parse and set the required fields. Vendor text: "`name`, `owner`, and `plugins` are required.", `owner` "`name` is required", and plugin entries: "`name` and `source` are required." (marketplace reference, "Top-level fields", "Plugin entries")
**Detection**: invalid JSON, a non-object root, a missing or empty `name` or `owner.name`, a missing `plugins` array, and entries that are not objects or lack `name` or `source`
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/marketplace-reference

<a id="cc-mp-002"></a>
### CC-MP-002 [HIGH] Invalid or Reserved Marketplace or Plugin Name
**Requirement**: Marketplace and plugin entry names MUST be ids Claude Code accepts and MUST NOT be reserved. Vendor text: marketplace `name` "letters, digits, `.`, `_`, and `-`, starting with a letter or digit, and no `..`. `claude plugin validate` fails any other name, because Claude Code can't install plugins from a marketplace that uses one", and plugin entry `name` "letters, digits, `.`, `_`, and `-`, starting with a letter or digit. `claude plugin validate` fails other names, which Claude Code can't install" (marketplace reference, "Top-level fields", "Plugin entries"); "You can't give your marketplace any of the following names" ("Reserved names")
**Detection**:
  - marketplace `name`: control or bidirectional-formatting characters, non-ASCII ("any name containing a non-ASCII character"), spaces, `/`, `\`, `..` or `.`, other characters outside the id set, the `claudeai-` prefix, the official, community and directory names (unless the `owner` is Anthropic), another spelling of a reserved name (trailing dot, or a symbol other than `_` in place of `-`), `inline`, `builtin`, `skills-dir`, `synced`, `claude-plugin-test`, `npm`/`pip`/`uv`/`cargo`/`github`/`gh` in any casing, and JavaScript object member names
  - entry `name`: control characters, spaces, characters outside the id set, and the CC-PL-018 reserved-name table (error set as errors, warning set as warnings; skipped for an Anthropic owner or author)
  - not covered: the open-ended "names that impersonate an official marketplace" heuristic beyond non-ASCII, and the `<owner>-<repo>` download-folder clash, which depends on what the user has registered
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/marketplace-reference, code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-mp-003"></a>
### CC-MP-003 [HIGH] Invalid Plugin Source, Duplicate Entry or Unresolved Rename
**Requirement**: Plugin sources, entry names and renames MUST validate. Vendor text: "`plugins.i.source: Invalid string: must start with "./"`", "`plugins[i].source: Path contains "..": <path>`", "`Duplicate plugin name "x" found in marketplace`", "`plugins.i.source: Invalid input`" and "`chain does not resolve (<reason>) — target must be a name in plugins[], a key in renames, or null`" are errors (marketplace reference, "Validation messages"); "On macOS and Linux, Claude Code refuses an entry path that contains a backslash anywhere after the leading `./`" ("Relative path plugin source")
**Detection**: a string `source` that contains `..`, has a backslash after `./`, or doesn't start with `./` (other than `.` or a bare name under `metadata.pluginRoot`); an object `source` whose `source` type is unknown, the `unsupported` placeholder, or missing its required field (`github` `repo`, `url` `url`, `git-subdir` `url` and `path`, `npm` `package`, `archive` `url`, `command` `command`); a duplicate entry `name`; a `renames` value that is not null, a plugin name in `plugins`, or a key in `renames`
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/marketplace-reference

<a id="cc-mp-004"></a>
### CC-MP-004 [HIGH] Marketplace Entry Fails to Load
**Requirement**: Marketplace entries MUST NOT use field forms that fail at load time. Vendor text: "Write entry `hooks` as an inline object that maps hook event names to matcher arrays. If you write a file path or an array instead, `claude plugin validate` passes it. Those hooks never run" ("Hooks in an entry"); "`Plugin "x" sets headersHelper but is not "strict": false`" is an error ("Validation messages"); and with `strict: false`, plugin.json present and component fields on the entry, "the plugin fails to load with `Plugin <name> has conflicting manifests: both plugin.json and marketplace entry specify components`" ("Strict mode")
**Detection**: entry `hooks` that is a string or array; `headersHelper` without `"strict": false`; `"strict": false` with `commands`, `agents`, `skills`, `hooks`, `outputStyles` or `themes` on a relative-path entry whose directory has `.claude-plugin/plugin.json`
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/marketplace-reference, code.claude.com/docs/en/plugins/manifest-reference

<a id="cc-mp-005"></a>
### CC-MP-005 [LOW] Marketplace Field Ignored or Missing Metadata
**Requirement**: Marketplace files SHOULD avoid fields Claude Code ignores and SHOULD describe the marketplace. Vendor text: "Claude Code ignores an unknown top-level key or plugin-entry key rather than rejecting it, so a typo loads silently. `claude plugin validate` reports each unknown key as a warning." ("Marketplace file"), plus the warnings "No marketplace description provided", "Marketplace has no plugins defined", "Entry declares version "x" but <path>/plugin.json says "y". At install time, plugin.json wins" and "sets headers/headersHelper, which only apply to "archive" sources; they have no effect on this entry." ("Validation messages")
**Detection**: unknown keys at the top level, under `metadata` and in entries (directory listing fields such as `icon` count as unknown in an entry); no `description` or `metadata.description`; an empty `plugins` array; an entry `version` that differs from the relative-path plugin's `plugin.json`; `headers` or `headersHelper` on a non-archive entry
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins/marketplace-reference

## MCP RULES

<a id="mcp-001"></a>


### MCP-001 [HIGH] Invalid JSON-RPC Version
**Requirement**: MUST use JSON-RPC 2.0
**Detection**: `message.jsonrpc != "2.0"`
**Fix**: Set `"jsonrpc": "2.0"`
**Source**: modelcontextprotocol.io/specification

<a id="mcp-002"></a>
### MCP-002 [HIGH] Missing Required Tool Field
**Requirement**: Tool MUST have `name` and `inputSchema`. `description` is optional.
**Detection**: Parse tool definitions and check the required fields. Missing descriptions are informational advice under MCP-004, not schema errors.
**Fix**: Add missing required fields
**Source**: modelcontextprotocol.io/specification/2026-07-28/server/tools

<a id="mcp-003"></a>
### MCP-003 [HIGH] Invalid JSON Schema
**Requirement**: `inputSchema` MUST be valid JSON Schema (JSON Schema 2020-12 compatible)
**Detection**: Validate schema structure and field types
**Fix**: Correct JSON Schema structure errors
**Source**: modelcontextprotocol.io/specification

<a id="mcp-004"></a>
### MCP-004 [LOW] Missing Tool Description
**Requirement**: Consider a clear description to help clients and models understand the tool. The field is optional in the MCP schema.
**Detection**: Missing, empty, or fewer than ten trimmed description characters. This heuristic emits informational advice only.
**Fix**: Add or improve the description manually
**Source**: modelcontextprotocol.io/specification/2026-07-28/server/tools

<a id="mcp-006"></a>
### MCP-006 [HIGH] Untrusted Annotations
**Requirement**: Tool annotations MUST be treated as untrusted and annotation keys SHOULD use known hint names (`readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`, `title`)
**Detection**: Warn when annotations are present and when unknown annotation keys are used
**Fix**: Restrict annotation keys to known spec hint names
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-007"></a>
### MCP-007 [HIGH] MCP Parse Error
**Requirement**: MCP configuration MUST be valid JSON
**Detection**: JSON parse error on MCP configuration file
**Fix**: Fix JSON syntax errors in MCP configuration
**Source**: modelcontextprotocol.io/specification

<a id="mcp-008"></a>
### MCP-008 [MEDIUM] Protocol Version Mismatch
**Requirement**: MCP initialize messages SHOULD use the expected protocol version
**Detection**: Check `protocolVersion` in initialize request params or response result. A pinned revision requires an exact match; unpinned validation tolerates the current revision strings `2025-11-25` and `2026-07-28`.
**Fix**: Update protocolVersion to match expected version, or configure `mcp_protocol_version` in agnix config to match your target version
**Note**: This warning checks declarations on legacy initialization messages. MCP 2026-07-28 uses per-request `_meta` and `UnsupportedProtocolVersionError`, with no initialization handshake. Accepting its revision string here does not validate the modern protocol flow; per-request metadata and fallback behavior remain outside MCP-008's scope.
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle, modelcontextprotocol.io/specification/2026-07-28/basic/versioning
**Version-Aware**: When MCP protocol version is not pinned in `.agnix.toml [spec_revisions]`, an assumption note is added indicating default protocol version is being used. Pin the version with `mcp_protocol = "2025-11-25"` for explicit control.

<a id="mcp-009"></a>
### MCP-009 [HIGH] Missing command for stdio server
**Requirement**: Stdio MCP servers MUST have a `command` field
**Detection**: Server entry has `type: "stdio"`, or omits `type` and has no usable `command`. In `.cursor/mcp.json` only, a URL-only entry is inferred as HTTP, matching Cursor's documented remote-server form.
**Fix**: Add a `command` field specifying the executable to run
**Source**: modelcontextprotocol.io/specification, cursor.com/docs/mcp

<a id="mcp-010"></a>
### MCP-010 [HIGH] Missing url for http/sse server
**Requirement**: HTTP and SSE MCP servers MUST have a `url` field
**Detection**: Server entry has `type: "http"` or `type: "sse"` but no `url` field; URL-only entries in `.cursor/mcp.json` need no redundant `type`
**Fix**: Add a `url` field specifying the server endpoint
**Source**: modelcontextprotocol.io/specification, cursor.com/docs/mcp

<a id="mcp-011"></a>
### MCP-011 [HIGH] Invalid MCP server type
**Requirement**: MCP server `type` MUST be `stdio`, `http`, or `sse`
**Detection**: Server entry has a `type` field with an unrecognized value
**Fix**: Auto-fix (unsafe) -- replace with closest valid server type
**Source**: modelcontextprotocol.io/specification

<a id="mcp-012"></a>
### MCP-012 [HIGH] Deprecated SSE transport
**Requirement**: SSE transport SHOULD be replaced with Streamable HTTP
**Detection**: Server entry has `type: "sse"`
**Fix**: Change `type` from `"sse"` to `"http"` (unsafe: server may not support Streamable HTTP)
**Note**: Raised to high severity because SSE is deprecated and behind current transport guidance
**Source**: modelcontextprotocol.io/specification

<a id="mcp-013"></a>
### MCP-013 [HIGH] Invalid Tool Name Format
**Requirement**: Tool name MUST be 1-128 chars and match `[a-zA-Z0-9_.-]+`
**Detection**: Check `tools[].name` length and allowed characters
**Fix**: [AUTO-FIX] Rename tool to a compliant identifier
**Source**: modelcontextprotocol.io/specification/2025-11-25/server/tools

<a id="mcp-014"></a>
### MCP-014 [HIGH] Invalid outputSchema Definition
**Requirement**: `outputSchema` MUST be valid JSON Schema when provided
**Detection**: Validate `tools[].outputSchema` object structure/types
**Fix**: Correct `outputSchema` to valid JSON Schema
**Source**: modelcontextprotocol.io/specification/2025-11-25/server/tools

<a id="mcp-015"></a>
### MCP-015 [HIGH] Missing Resource Required Fields
**Requirement**: Resource definitions MUST include `uri` and `name`
**Detection**: Check each `resources[]` entry for missing/empty `uri` or `name`
**Fix**: Add required fields
**Source**: modelcontextprotocol.io/specification/2025-11-25/server/resources

<a id="mcp-016"></a>
### MCP-016 [HIGH] Missing Prompt Required Name
**Requirement**: Prompt definitions MUST include `name`
**Detection**: Check each `prompts[]` entry for missing/empty `name`
**Fix**: Add non-empty `name`
**Source**: modelcontextprotocol.io/specification/2025-11-25/server/prompts

<a id="mcp-017"></a>
### MCP-017 [HIGH] Non-HTTPS Remote HTTP Server URL
**Requirement**: Non-localhost HTTP MCP endpoints MUST use HTTPS
**Detection**: For `type: "http"`, flag `http://` URLs when host is not localhost/loopback
**Fix**: [AUTO-FIX] Change remote MCP URL to `https://`
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/transports

<a id="mcp-018"></a>
### MCP-018 [MEDIUM] Potential Plaintext Secret in MCP Env
**Requirement**: Secret-like env vars SHOULD avoid plaintext values
**Detection**: In stdio server `env`, flag keys matching `API_KEY`, `SECRET`, `TOKEN`, `PASSWORD` with non-empty literal values
**Fix**: Use runtime secret injection or env indirection
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices

<a id="mcp-019"></a>
### MCP-019 [MEDIUM] Potentially Dangerous Stdio Command
**Requirement**: Stdio server commands SHOULD avoid risky shell patterns
**Detection**: Flag patterns like `curl|sh`, `wget|sh`, `sudo rm`, and simple exfiltration command signatures
**Fix**: Replace with audited, explicit command execution flow
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices

<a id="mcp-020"></a>
### MCP-020 [MEDIUM] Unknown Capability Declaration Key
**Requirement**: Capability keys SHOULD use recognized core keys; this is advisory typo detection because implementations can declare additional capabilities.
**Detection**: Validate keys under `capabilities` against the known list, including legacy `tasks` and modern `extensions`. Extension identifiers and settings inside `extensions` are not checked by this rule.
**Fix**: Remove or rename unknown capability keys
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle, modelcontextprotocol.io/specification/2026-07-28/basic/versioning

<a id="mcp-021"></a>
### MCP-021 [MEDIUM] Wildcard HTTP Interface Binding
**Requirement**: HTTP servers SHOULD avoid wildcard/all-interface binds by default
**Detection**: Flag `http://0.0.0.0...` and IPv6 wildcard binds
**Fix**: [AUTO-FIX] Prefer localhost binding unless remote exposure is required
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/security_best_practices

<a id="mcp-022"></a>
### MCP-022 [HIGH] Invalid args Array Type
**Requirement**: `args` MUST be an array of strings when present
**Detection**: Validate `mcpServers.*.args` type and element types
**Fix**: Convert to array of string arguments
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/transports

<a id="mcp-023"></a>
### MCP-023 [HIGH] Duplicate MCP Server Names
**Requirement**: `mcpServers` keys MUST be unique
**Detection**: Scan raw JSON for duplicate keys inside `mcpServers`
**Fix**: Rename duplicate server entries
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/transports

<a id="mcp-024"></a>
### MCP-024 [HIGH] Empty MCP Server Configuration
**Requirement**: Each MCP server entry MUST define meaningful config fields
**Detection**: Flag empty objects in `mcpServers`
**Fix**: Add at least one meaningful field (`type`, `command`, `url`, `args`, `env`)
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/transports

---

## CLAUDE CODE RULES (MCP)

> **Scope:** Claude Code's own handling of MCP server entries. CC-MCP-001, -002, -004 and -005 run only on the MCP files Claude Code reads - a project `.mcp.json` and a plugin's `.mcp.json` (the directory has `.claude-plugin/`) - so other clients' MCP files (`.cursor/mcp.json`, `.vscode/mcp.json`, ...) don't get Claude Code facts. CC-MCP-003 runs on MCP tool listings. The generic MCP rules (MCP-*) still run on every MCP file.

<a id="cc-mcp-001"></a>
### CC-MCP-001 [HIGH] Credential Variable Reads as Empty in Remote MCP Server
**Requirement**: A remote server's `url` and `headers` MUST NOT reference the credential variables Claude Code reads as empty. Vendor text: "In a remote server's `url` and `headers`, Claude Code reads credential variables from your environment as empty rather than expanding them. ... If you write `Bearer ${ANTHROPIC_AUTH_TOKEN}`, the server receives `Bearer ` with no credential and rejects the request, usually with a `401`." and "A covered name reads as empty whether or not you have set the variable, and a `:-default` fallback on it is ignored." (mcp, "Credential variables that read as empty")
**Detection**: on servers with `type` `http`, `streamable-http`, `sse` or `ws`, a `${NAME}` or `${NAME:-...}` in `url` or a `headers` value, where NAME is one the page names: `ANTHROPIC_API_KEY`, `ANTHROPIC_AUTH_TOKEN`, `AWS_BEARER_TOKEN_BEDROCK`, `HTTPS_PROXY`, `NPM_TOKEN`. The page introduces them with "such as", so the list is the documented subset. `ANTHROPIC_BASE_URL` still expands and is not reported.
**Fix**: Manual - copy the value into a variable with a name of your own
**Source**: code.claude.com/docs/en/mcp

<a id="cc-mcp-002"></a>
### CC-MCP-002 [MEDIUM] CLAUDE_PROJECT_DIR Without Default in Project .mcp.json
**Requirement**: A project `.mcp.json` `command` or `args` MUST give `${CLAUDE_PROJECT_DIR}` a default. Vendor text: "This variable is set in the server's environment, not in Claude Code's own environment, so referencing it via `${VAR}` expansion in the `command` or `args` of a project-scoped `.mcp.json` entry or a local- or user-scoped server entry in `~/.claude.json` requires a default such as `${CLAUDE_PROJECT_DIR:-.}`. Plugin-provided MCP configurations substitute `${CLAUDE_PROJECT_DIR}` directly and don't need the default." (mcp, "Option 3: Add a local stdio server")
**Detection**: project `.mcp.json` (not a plugin's) server whose `command` or an `args` element contains `${CLAUDE_PROJECT_DIR}`
**Fix**: Manual - write `${CLAUDE_PROJECT_DIR:-.}`
**Source**: code.claude.com/docs/en/mcp

<a id="cc-mcp-003"></a>
### CC-MCP-003 [LOW] MCP Tool Description or Instructions Truncated
**Requirement**: MCP tool descriptions and server instructions SHOULD fit what Claude Code keeps. Vendor text: "**Server instructions, and the description of a tool that loads upfront**: 4,096 characters" and "Claude Code cuts a tool description or server instructions that run past these default limits, and Claude sees only the text before the cut." (mcp, "Tool search for MCP server authors")
**Detection**: a `tools[]` (or `result.tools[]`) description longer than 4,096 characters (16,384 when the tool sets `_meta["anthropic/alwaysLoad"]: false`), or `instructions` (or `result.instructions`) longer than 4,096 characters (info). With a pinned Claude Code version older than 2.1.296 (released 2026-10-09), the upfront limit is 2,048.
**Fix**: Manual
**Source**: code.claude.com/docs/en/mcp

<a id="cc-mcp-004"></a>
### CC-MCP-004 [MEDIUM] Non-boolean alwaysLoad in Claude Code MCP Config
**Requirement**: `alwaysLoad` MUST be a boolean when present. Vendor text: "If a server's tools should always be visible to Claude without a search step, set `alwaysLoad` to `true` in that server's configuration." and "To keep all of a server's tools behind tool search, set `"alwaysLoad": false`" (mcp, "Exempt a server from deferral")
**Detection**: string, number, array or object `alwaysLoad` on a server in a Claude Code `.mcp.json` (JSON `null` counts as absent)
**Fix**: Opt-in unsafe autofix converts exact quoted `true` or `false` to a boolean. This can activate ignored settings or change security policy; plain `--fix` leaves them unchanged. Other invalid values require manual correction.
**Source**: code.claude.com/docs/en/mcp

<a id="cc-mcp-005"></a>
### CC-MCP-005 [HIGH] Reserved MCP Server Name
**Requirement**: MCP server names MUST NOT be names Claude Code reserves. Vendor text: "Claude Code reserves the names of its built-in servers, including `workspace`, `claude-in-chrome`, `computer-use`, `Claude Preview`, and `Claude Browser`. If your configuration defines a server with a reserved name, Claude Code skips it at load time and shows a warning asking you to rename it." (mcp, "Configuration warnings") and "Prompts from a server named `anthropic-skills` don't appear, because Claude Code reserves that name for skills synced from claude.ai. The server's tools still work."
**Detection**: top-level server keys of a project `.mcp.json`, compared case-sensitively with the five reserved names (error); `anthropic-skills` gets an info note. A plugin's servers are scoped as `plugin:<plugin>:<server>`, so plugin `.mcp.json` keys are not checked.
**Fix**: Manual - rename the server
**Source**: code.claude.com/docs/en/mcp

---

## GITHUB COPILOT RULES

<a id="cop-001"></a>
### COP-001 [HIGH] Empty Copilot Instruction File
**Requirement**: Copilot instruction files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful instructions
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions, code.visualstudio.com/docs/agent-customization/custom-instructions

<a id="cop-002"></a>
### COP-002 [HIGH] Scoped Instructions Without applyTo or Valid Frontmatter
**Requirement**: A path-specific instruction file (`.github/instructions/**/*.instructions.md`) MUST have valid YAML frontmatter and SHOULD set `applyTo`. GitHub.com and Copilot CLI apply the file only through it ("At the start of the file, create a frontmatter block containing the `applyTo` keyword"). VS Code documents `applyTo` as optional: "If you omit both `description` and `applyTo`, attach the file manually when you want to use it", and a `description` enables on-demand loading.
**Detection**: YAML parse error or unclosed frontmatter: error. No frontmatter or no `applyTo`: warning. No `applyTo` but a non-empty `description`: info (VS Code loads the file on demand; GitHub.com and Copilot CLI do not apply it).
**Fix**: Auto-fix (unsafe) - insert frontmatter with `applyTo: "**"` when the file has none (it changes which requests use the file)
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions ("Creating path-specific custom instructions"), code.visualstudio.com/docs/agent-customization/custom-instructions ("Instructions file format")

<a id="cop-003"></a>
### COP-003 [HIGH] Invalid Glob Pattern in applyTo
**Requirement**: `applyTo` field MUST contain valid glob patterns. "You can specify multiple patterns by separating them with commas."
**Detection**: Split on commas outside brace and bracket groups, then parse each pattern; reported at the `applyTo` line
**Fix**: Correct the glob syntax
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions

<a id="cop-004"></a>
### COP-004 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: Scoped instruction frontmatter SHOULD only contain keys Copilot reads: `applyTo` and `excludeAgent` (GitHub), and `name` ("Display name shown in the UI") and `description` (VS Code)
**Detection**: Top-level frontmatter keys outside that set
**Fix**: [AUTO-FIX, safe] Remove the whole entry (key line and continuation lines); offered only when the edited frontmatter re-parses to the original mapping without the key
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions, code.visualstudio.com/docs/agent-customization/custom-instructions

<a id="cop-005"></a>
### COP-005 [HIGH] Invalid excludeAgent Value
**Requirement**: The `excludeAgent` frontmatter field in scoped instruction files MUST be `"code-review"` or `"cloud-agent"` ("Use either `\"code-review\"` or `\"cloud-agent\"`"). `coding-agent`, which earlier agnix versions suggested, is accepted with a LOW note.
**Detection**: Parse frontmatter, validate `excludeAgent` value against allowed set
**Fix**: Auto-fix (unsafe) - replace with the closest documented value 
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/add-custom-instructions/add-repository-instructions, docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions

<a id="cop-006"></a>
### COP-006 [MEDIUM] Instruction File Over About 1,000 Lines
**Requirement**: A Copilot instruction file SHOULD stay within about 1,000 lines: "**Best practice**: Limit any single instruction file to a maximum of about 1,000 lines. Beyond this, the quality of responses may deteriorate."
**Detection**: `.github/copilot-instructions.md` and `.github/instructions/**/*.instructions.md` with more than 1,000 lines, reported at line 1,001. The earlier 4,000-character figure appears only inside a sample prompt of a 2025 GitHub blog post, not in the docs.
**Fix**: Remove redundant or generic instructions, or move language- and path-specific guidance into `*.instructions.md` files with `applyTo`
**Source**: docs.github.com/en/copilot/tutorials/customize-code-review ("Keep instructions short and focused")

<a id="cop-007"></a>
### COP-007 [HIGH] Custom Agent Missing Description
**Requirement**: Custom Copilot agent files MUST include a non-empty `description` frontmatter field ("`description` | **Required** string" on GitHub.com and in Copilot CLI). Agents with `target: vscode` are exempt: VS Code says "All fields in this table are optional."
**Detection**: Parse frontmatter and verify `description` exists and is non-empty, unless `target` is `vscode`
**Fix**: Add `description` to frontmatter
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration, code.visualstudio.com/docs/agents/reference/custom-agent-file-reference

<a id="cop-008"></a>
### COP-008 [MEDIUM] Custom Agent Unknown or Invalid Frontmatter Field
**Requirement**: Custom agent frontmatter SHOULD only use the documented fields with documented types: `name`, `description`, `target`, `tools`, `model`, `disable-model-invocation`, `user-invocable`, `infer`, `mcp-servers`, `metadata` (GitHub) and `argument-hint`, `agents`, `reasoning-effort`, `handoffs`, `hooks` (VS Code). VS Code types `model` as "String or array of strings" and `reasoning-effort` as "`low`, `medium`, `high`, `xhigh`, or `max`".
**Detection**: Unknown top-level keys; wrong types for `disable-model-invocation`, `user-invocable`, `metadata`, `model`, `tools`, `agents` (array of strings), `hooks` (mapping) and `reasoning-effort` (one of the five values)
**Fix**: [AUTO-FIX, safe] Remove an unknown field when the whole entry can be removed exactly (verified by re-parsing); value-type problems are not auto-fixed
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration, code.visualstudio.com/docs/agents/reference/custom-agent-file-reference

<a id="cop-009"></a>
### COP-009 [HIGH] Custom Agent Invalid Target
**Requirement**: Custom agent `target` MUST be `vscode` or `github-copilot`
**Detection**: Parse `target` and validate against allowed values
**Fix**: [AUTO-FIX] Set `target` to `vscode` or `github-copilot`
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration, code.visualstudio.com/docs/agents/reference/custom-agent-file-reference

<a id="cop-010"></a>
### COP-010 [MEDIUM] Custom Agent infer Field Must Be Boolean
**Requirement**: Custom agent `infer` field MUST be a boolean when present
**Detection**: Parse custom agent frontmatter and validate that `infer` is boolean
**Fix**: Set `infer` to either `true` or `false`
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration, code.visualstudio.com/docs/agents/reference/custom-agent-file-reference

<a id="cop-011"></a>
### COP-011 [HIGH] Custom Agent Prompt Body Exceeds Length Limit
**Requirement**: Custom agent prompt body MUST be at most 30,000 characters ("The prompt can be a maximum of 30,000 characters.")
**Detection**: Count body characters after frontmatter and check `> 30000`
**Fix**: Reduce prompt body length
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-012"></a>
### COP-012 [LOW] Custom Agent Field Ignored on GitHub.com
**Requirement**: "The `argument-hint` and `handoffs` properties from VS Code and other IDE custom agents are currently not supported for Copilot cloud agent on GitHub.com. They are ignored to ensure compatibility." VS Code uses both, so an agent shared between GitHub.com and an IDE may keep them.
**Detection**: Info when an agent without `target: vscode` sets `argument-hint` or `handoffs`. `model` is a documented GitHub.com property and is not reported.
**Fix**: No auto-fix (deleting the fields changes VS Code behavior); set `target: vscode` for an IDE-only agent
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-013"></a>
### COP-013 [HIGH] Prompt File Empty Body
**Requirement**: Reusable prompt files (`.github/prompts/*.prompt.md`) MUST contain non-empty prompt body content ("The body contains the prompt text in Markdown format.")
**Detection**: Parse optional frontmatter and check body for non-whitespace content
**Fix**: Add prompt body content
**Source**: code.visualstudio.com/docs/agent-customization/prompt-files

<a id="cop-014"></a>
### COP-014 [MEDIUM] Prompt File Unknown Frontmatter Field
**Requirement**: Prompt file frontmatter SHOULD only use the documented fields `description`, `name`, `argument-hint`, `agent`, `model` and `tools`
**Detection**: Parse frontmatter and detect unknown top-level keys
**Fix**: [AUTO-FIX, safe] Remove the whole unknown entry when it can be removed exactly (verified by re-parsing)
**Source**: code.visualstudio.com/docs/agent-customization/prompt-files

<a id="cop-015"></a>
### COP-015 [LOW] Prompt File Agent Not Found
**Requirement**: The prompt file `agent` field SHOULD name "`ask`, `agent`, `plan`, or the name of a custom agent"
**Detection**: Info when `agent` is none of the three built-in values and the repository has a `.github/agents/` folder in which no agent matches by file ID (file name without `.agent.md` or `.md`) or `name` (`.claude/agents/` is searched too). Not reported when the repository has no `.github/agents/` folder, since user and organization agents are not visible. A case-only mismatch is named in the suggestion.
**Fix**: No auto-fix (the earlier fix rewrote valid values such as `agent` and `plan`)
**Source**: code.visualstudio.com/docs/agent-customization/prompt-files ("Prompt file format")

<a id="cop-017"></a>
### COP-017 [HIGH] Copilot Hook File Schema
**Requirement**: Copilot hook files MUST match the hooks reference. Copilot loads every `.github/hooks/*.json` file ("stored in your repository at `.github/hooks/*.json`") and the CLI also `~/.copilot/hooks/*.json`; "The JSON must contain a `version` field with a value of `1` and a `hooks` object containing arrays of hook definitions." Events are `agentStop`, `errorOccurred`, `notification`, `permissionRequest`, `postToolUse`, `postToolUseFailure`, `preCompact`, `preToolUse`, `sessionEnd`, `sessionStart`, `subagentStart`, `subagentStop`, `userPromptSubmitted` and `userPromptTransformed`, plus their documented PascalCase aliases. Command entries default to `type: "command"` ("Defaults to `\"command\"` when omitted") and need `bash`, `powershell`, `command` or `exec`; `http` entries need `url`; `prompt` entries need `prompt`. VS Code's native format ("PascalCase events without a numeric `version` property") is accepted with a note that Copilot CLI and cloud agent skip it. Inline `hooks` in `.github/copilot/settings.json` and `~/.copilot/settings.json` use the same entries and are strict: "any item-level validation error rejects the whole `hooks` field".
**Detection**: Parse JSON with spans; report invalid JSON, a missing or wrong `version`, a `hooks` value that is not an event map (the undocumented array form included), unknown events (with a near-miss suggestion), non-array event lists, malformed entries, a `command` written as an object, and wrongly typed fields, each at the offending key or value
**Fix**: Correct the hook file structure
**Source**: docs.github.com/en/copilot/reference/hooks-reference, docs.github.com/en/copilot/concepts/agents/hooks, code.visualstudio.com/docs/agent-customization/hooks

<a id="cop-018"></a>
### COP-018 [HIGH] Invalid copilot-setup-steps Job
**Requirement**: `copilot-setup-steps.yml` MUST define a job named `copilot-setup-steps` ("The job MUST be called `copilot-setup-steps` or it will not be picked up by Copilot.") on a supported runner: "Copilot cloud agent is only compatible with Ubuntu x64 and Windows 64-bit runners. Runners with macOS or other operating systems are not supported." Larger, self-hosted and ARC runner labels are documented.
**Detection**: Parse workflow YAML; report a missing `jobs.copilot-setup-steps` job and a `runs-on` label (string, list or `labels` of a group) that names macOS. Other labels and expressions are accepted; `steps` is optional.
**Fix**: Add or correct the `copilot-setup-steps` job in the workflow
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment

<a id="cop-019"></a>
### COP-019 [HIGH] Plugin Manifest Missing Required Fields
**Requirement**: A Copilot CLI plugin manifest MUST be valid JSON with a `name`. The legacy manifest's only required field is `name` (`description` and `version` are "Optional metadata fields"); an Agent Plugins 1.0 manifest requires `$schema` and `name`, and "If a plugin declares an Agent Plugins version that Copilot CLI doesn't support, the CLI rejects the plugin".
**Detection**: `.github/plugin/plugin.json` and `.plugin/plugin.json`: invalid JSON, a non-object root, a missing `name`, or an agent-plugins.org `$schema` other than the 1.0.0 and 1.1.0 schemas
**Fix**: Manual
**Source**: docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference

<a id="cop-020"></a>
### COP-020 [MEDIUM] Plugin Invalid Field Types
**Requirement**: Plugin manifest fields SHOULD have the documented types: strings for `name`, `version`, `description`, `homepage`, `repository`, `license` and `category`; string arrays for `keywords` and `tags`; an object for `author`; "string | string[]" for the legacy component paths `agents`, `skills` and `commands`; "string | object" for `hooks`, `mcpServers` and `lspServers`
**Detection**: Check each present field's JSON type, at the value
**Fix**: Manual
**Source**: docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference

<a id="cop-025"></a>
### COP-025 [LOW] Agent File Wrong Location
**Requirement**: `.agent.md` files SHOULD be in a folder Copilot reads agents from: `.github/agents/` (any depth), `~/.copilot/agents/`, a root `agents/` folder (organization and enterprise agents in the `.github` or `.github-private` repository, and CLI plugin agents), or VS Code's profile `prompts/` folder
**Detection**: Info for a `*.agent.md` file outside every such folder (Claude Code agent folders and documentation folders are not checked)
**Fix**: Manual - move file
**Source**: docs.github.com/en/copilot/concepts/agents/copilot-cli/about-custom-agents, code.visualstudio.com/docs/agent-customization/custom-agents, docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference

<a id="cop-027"></a>
### COP-027 [LOW] Deprecated Infer Field
**Requirement**: Agents SHOULD replace `infer` with `disable-model-invocation` and `user-invocable`: GitHub marks it "**Retired**. Use `disable-model-invocation` and `user-invocable` instead." and VS Code "**Deprecated.**" VS Code still reads it, so this is a LOW migration note.
**Detection**: Check for infer key in .agent.md frontmatter
**Fix**: [AUTO-FIX, unsafe] Migrate an unambiguous boolean `infer` to `disable-model-invocation` (inverted) and default `user-invocable: true` independently of `infer`. Preserve explicit boolean replacement fields; refuse ambiguous or wrong-type fields. Review invocation behavior before applying.
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration, code.visualstudio.com/docs/agents/reference/custom-agent-file-reference

<a id="cop-028"></a>
### COP-028 [HIGH] Custom Agent MCP Secret Without COPILOT_MCP_ Prefix
**Requirement**: Secrets and variables referenced by a custom agent's `mcp-servers` MUST be named `COPILOT_MCP_*`: "Only Agents secrets and variables with names prefixed with `COPILOT_MCP_` will be available to your MCP configuration." and "Referenced names must start with `COPILOT_MCP_`."
**Detection**: In `mcp-servers.<server>.env` and `.headers` values: `${{ secrets.NAME }}` or `${{ vars.NAME }}` without the prefix is an error; `$NAME`, `${NAME}` or `${NAME:-default}` without the prefix is a warning (the CLI may read the variable from the local environment). Skipped for `target: vscode` agents, since `mcp-servers` is "Not used in VS Code and other IDE custom agents".
**Fix**: Manual - rename the Agents secret or variable
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/configure-mcp-servers, docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-029"></a>
### COP-029 [MEDIUM] copilot-setup-steps Job Setting Ignored
**Requirement**: The `copilot-setup-steps` job SHOULD use only the settings Copilot honors: "you can only customize the following settings of the `copilot-setup-steps` job. If you try to customize other settings, your changes will be ignored." The list is `steps`, `permissions`, `runs-on`, `services`, `snapshot` and `timeout-minutes` ("maximum value: `59`").
**Detection**: Warning for each other job key (a display `name` is not reported) and for `timeout-minutes` above 59, at the key's line
**Fix**: Manual - move job-level settings such as `env` into the steps
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/customize-the-agent-environment

<a id="cop-030"></a>
### COP-030 [MEDIUM] File Reference Copilot CLI Does Not Load
**Requirement**: `@` file references SHOULD be ones Copilot CLI loads: "Referenced files must remain within the repository, or within the custom instructions directory for local instructions. Absolute paths and paths beginning with `~/` are not loaded. File references are not expanded in `GEMINI.md` or `*.instructions.md` files."
**Detection**: In `.github/copilot-instructions.md`: `@` imports that are absolute, start with `~`, or resolve outside the project root. In `*.instructions.md`: any `@` import. AGENTS.md, CLAUDE.md and GEMINI.md are checked only when the run targets Copilot (`tools = ["copilot"]`), since Claude Code and Gemini CLI expand their own imports. Uses the shared import parser (code spans and fences excluded).
**Fix**: Manual - reference a repository file or copy the content
**Source**: docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-custom-instructions ("Referencing other files")

<a id="cop-031"></a>
### COP-031 [HIGH] Copilot Hook Entry Violates Documented Constraints
**Requirement**: Hook entries MUST respect the hooks reference: "By default, only `https://` URLs are allowed. Non-TLS `http://` requests are rejected, except for `http://localhost`, `http://127.*`, and `http://[::1]` when `COPILOT_HOOK_ALLOW_LOCALHOST=1` is set"; for `preToolUse` and `permissionRequest` the URL "must use `https://` because the response can grant tool permissions"; with `allowedEnvVars` "`url` must use `https://`"; prompt hooks "are only supported on `sessionStart`"; "Do not combine `exec` with `bash`, `powershell`, or `command`"; and a matcher "is compiled as `^(?:PATTERN)$` ... Invalid regexes cause the hook entry to be skipped".
**Detection**: Error for each violation, at the URL, key or matcher; info for an http localhost URL (allowed only with the environment opt-in). Matchers are compiled with an ECMAScript regex engine. PascalCase `PreToolUse` and `PermissionRequest` matchers follow Claude's semantics, so `*`, `**`, an empty value and `|`-separated names are not regexes there.
**Fix**: Manual
**Source**: docs.github.com/en/copilot/reference/hooks-reference ("HTTP hooks", "Prompt hooks", "Command hooks", "Matcher filtering")

<a id="cop-032"></a>
### COP-032 [LOW] Legacy Chat Mode File
**Requirement**: Chat mode files SHOULD be migrated: "Custom agents were previously known as custom chat modes." and "If you have existing `.chatmode.md` files, rename them to `.agent.md` and place them in one of the supported custom agent locations."
**Detection**: Any `*.chatmode.md` file; a LOW migration note only, the file is not validated as an agent
**Fix**: Manual - rename to `.agent.md` and move to `.github/agents/`
**Source**: code.visualstudio.com/docs/agent-customization/custom-agents ("Are custom agents different from chat modes?")

<a id="cop-033"></a>
### COP-033 [MEDIUM] Invalid Custom Agent Filename
**Requirement**: A custom agent file name MUST use only the documented characters: "the filename may only contain the following characters: `.`, `-`, `_`, `a-z`, `A-Z`, `0-9`."
**Detection**: Agent files in Copilot agent folders whose name, without `.agent.md` or `.md`, has any other character
**Fix**: Manual - rename the file
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/create-custom-agents

<a id="cop-034"></a>
### COP-034 [MEDIUM] Custom Agent MCP Server Tools List
**Requirement**: Each MCP server in a custom agent MUST list its `tools` ("Required keys for local and remote MCP servers"), and SHOULD allowlist specific tools: "We strongly recommend that you allowlist specific read-only tools, since the agent will be able to use these tools autonomously and will not ask you for approval first."
**Detection**: `mcp-servers.<server>` without `tools`: warning. `tools` containing `"*"`: info. Skipped for `target: vscode` agents.
**Fix**: Manual
**Source**: docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/configure-mcp-servers, docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-035"></a>
### COP-035 [MEDIUM] MCP File Uses Unsupported Feature or Wrong Server Key
**Requirement**: MCP files SHOULD use the features their format supports: "Portable files use a top-level `mcpServers` object. The VS Code format uses `servers`." and "Portable configurations don't support VS Code input variables or environment files. Servers requiring interactive inputs aren't forwarded to Copilot harness sessions." Workspace-root `.mcp.json` supports "stdio and HTTP" transports and no working directory.
**Detection**: `.vscode/mcp.json` with `mcpServers` and no `servers`; `~/.copilot/mcp-config.json` with `servers` and no `mcpServers`, `envFile`, or `${input:...}`; workspace `.mcp.json` (only when the run targets Copilot, since it is also Claude Code's file) with `servers`, `envFile`, `cwd`, `type: "sse"` or `${input:...}`
**Fix**: Manual
**Source**: code.visualstudio.com/docs/agents/reference/mcp-configuration ("Configuration file"), code.visualstudio.com/docs/agent-customization/mcp-servers ("Choose a configuration destination")

<a id="cop-036"></a>
### COP-036 [LOW] Copilot Hook Skipped by Cloud Agent or Missing Script
**Requirement**: Repository hooks SHOULD run where they are deployed: under cloud agent "Only the `bash` field is honored; `powershell` entries are ignored. The cross-platform `command` field is honored as a fallback.", and the troubleshooting steps ask to "Verify the script you are calling from your hook is executable" and "Check that the script has a proper shebang".
**Detection**: Info for a `.github/hooks/*.json` command entry with only `powershell`; for a `bash` or `command` whose first word is a relative script path (`./x.sh`, `scripts/x.py`) that does not exist at the repository root (entries with `cwd` are skipped); and for such a script without a `#!` first line. The executable bit is not checked.
**Fix**: Manual
**Source**: docs.github.com/en/copilot/reference/hooks-reference ("Cloud agent execution environment"), docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/use-hooks ("Troubleshooting")

<a id="cop-037"></a>
### COP-037 [MEDIUM] Literal Secret in Copilot Hook
**Requirement**: Hook files SHOULD NOT hold credentials: "**Never log sensitive data, such as tokens or passwords**". Hook files in `.github/hooks/` are committed.
**Detection**: `env` and `headers` values whose key names a credential (token, secret, password, API key, authorization) and whose value is a literal secret rather than a variable reference or placeholder (shared plaintext-secret heuristic)
**Fix**: Manual - read the value from the environment
**Source**: docs.github.com/en/copilot/concepts/agents/hooks ("Security considerations"), docs.github.com/en/copilot/reference/hooks-reference

<a id="cop-038"></a>
### COP-038 [LOW] Code Review Instruction Points to an External Link
**Requirement**: Code review instructions SHOULD NOT depend on external links: "**Follow external links**" is listed under unsupported instruction types, for example "`Review this code according to the standards at https://example.com/standards`", with the workaround "Copy the relevant content directly into your instruction file instead".
**Detection**: Info for a line outside code fences that has an http(s) URL with a directive phrase (follow, according to, refer to, adhere to, comply with, as described at, standards/guidelines/conventions at) in `.github/copilot-instructions.md` and `*.instructions.md`, unless the file sets `excludeAgent: "code-review"`
**Fix**: Manual - copy the guidance in
**Source**: docs.github.com/en/copilot/tutorials/customize-code-review ("What not to do")

<a id="cop-039"></a>
### COP-039 [MEDIUM] Copilot Plugin Name or Manifest Constraints
**Requirement**: Plugin names and descriptions MUST meet the CLI plugin reference: the legacy `name` is a "Kebab-case plugin name (letters, numbers, and hyphens only). Maximum 64 characters."; an Agent Plugins 1.0 name must "Contain between 1 and 64 characters", "Contain only lowercase ASCII letters, digits, hyphens, and periods", "Start and end with an alphanumeric character" and "Not contain `--` or `..`"; `description` is "Max 1024 chars". The Agent Plugins 1.0 manifest is closed: "Unknown top-level fields are reported and ignored. Component path fields such as `agents`, `skills`, `hooks`, `mcpServers`, and `lspServers` are not Agent Plugins 1.0 manifest fields."
**Detection**: Warning for a name or description that breaks these limits; info for a field outside the Agent Plugins 1.0 schema in a manifest that declares it
**Fix**: Manual
**Source**: docs.github.com/en/copilot/reference/copilot-cli-reference/cli-plugin-reference ("Name constraints", "Optional metadata fields", "Agent Plugins 1.0 manifest fields")

---


## CURSOR PROJECT RULES

<a id="cur-001"></a>
### CUR-001 [HIGH] Empty Cursor Rule File
**Requirement**: Cursor .mdc rule files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful rules content
**Source**: cursor.com/docs/rules

<a id="cur-002"></a>
### CUR-002 [MEDIUM] Missing Frontmatter in .mdc File
**Requirement**: Cursor .mdc files SHOULD have YAML frontmatter with metadata
**Detection**: File doesn't start with `---` markers
**Fix**: Auto-fix (unsafe) - insert template frontmatter with description and globs fields
**Source**: cursor.com/docs/rules

<a id="cur-003"></a>
### CUR-003 [HIGH] Invalid YAML Frontmatter
**Requirement**: .mdc file frontmatter MUST be valid YAML
**Detection**: YAML parse error on frontmatter content
**Fix**: Fix YAML syntax errors in frontmatter
**Source**: cursor.com/docs/rules

<a id="cur-004"></a>
### CUR-004 [HIGH] Invalid Glob Pattern in globs Field
**Requirement**: `globs` field MUST contain valid glob patterns. The scalar form carries several patterns in one string - "Separate multiple patterns with commas", with `docs/**/*.md, docs/**/*.mdx` as the doc's own example.
**Detection**: Split on commas at brace/bracket depth 0, then parse each pattern. Validating the whole string was intermittently wrong: the doc's literal example happens to parse as one pattern, while `src/**, tests/**` does not.
**Fix**: Correct the glob syntax
**Source**: cursor.com/docs/rules

<a id="cur-005"></a>
### CUR-005 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: .mdc frontmatter SHOULD only contain known keys (description, globs, alwaysApply)
**Detection**: Check for keys other than known keys in frontmatter
**Fix**: Remove unknown keys
**Source**: cursor.com/docs/rules

<a id="cur-006"></a>
### CUR-006 [LOW] Legacy .cursorrules File Detected
**Requirement**: Projects SHOULD migrate from `.cursorrules` to `.cursor/rules/*.mdc`: the file "is legacy and will be deprecated". Cursor still reads it, so this is a LOW migration note. Cline reads `.cursorrules` but not `.cursor/rules/` ("Cursor Rules | `.cursorrules` | Automatically detected"), so rules both tools should see belong in AGENTS.md.
**Detection**: File named `.cursorrules` (info)
**Fix**: Create `.cursor/rules/` and move the rules into .mdc files, or into AGENTS.md
**Source**: cursor.com/help/customization/rules ("How do I migrate from .cursorrules?"), cursor.com/docs/rules, docs.cline.bot/customization/cline-rules ("Supported Rule Types")

<a id="cur-007"></a>
### CUR-007 [MEDIUM] alwaysApply with Redundant globs
**Requirement**: When `alwaysApply: true`, the `globs` field SHOULD NOT be set (it is redundant)
**Detection**: Frontmatter has both `alwaysApply: true` and a `globs` field
**Fix**: [AUTO-FIX] Remove the `globs` field (safe)
**Source**: cursor.com/docs/rules

<a id="cur-008"></a>
### CUR-008 [HIGH] Invalid alwaysApply Type
**Requirement**: `alwaysApply` MUST be a boolean (`true`/`false`), not a quoted string
**Detection**: `alwaysApply` value is a string (e.g., `"true"` or `"false"`) instead of a boolean
**Fix**: Auto-fix (safe) - convert quoted string to unquoted boolean
**Source**: cursor.com/docs/rules

<a id="cur-009"></a>
### CUR-009 [MEDIUM] Rule Applied Only When Mentioned
**Requirement**: A rule with none of `alwaysApply`, `globs` and `description` SHOULD say how it applies: the behavior table gives it "Included only when you `@`-mention the rule in chat.", and "For `Apply Intelligently`, ensure a description is defined." A rule that sets `alwaysApply: false` explicitly is manual on purpose and not reported.
**Detection**: Frontmatter has no `alwaysApply`, no `globs`, and no (or an empty) `description`
**Fix**: Add a `description` (Apply Intelligently), `globs` (Apply to Specific Files) or `alwaysApply: false` (manual on purpose)
**Source**: cursor.com/docs/rules ("Rule anatomy", "FAQ")

<a id="cur-010"></a>
### CUR-010 [HIGH] Invalid .cursor/hooks.json Schema
**Requirement**: `.cursor/hooks.json` MUST define an object `hooks` map and a `version` field. The per-file options table documents `version` as `| version | number | required | Config schema version. Must be a positive integer (use 1). |`, and every hooks.json example in the doc carries `"version": 1`. Zero, negative, fractional, and non-numeric values are invalid; `1.0` is accepted because JSON has one number type and its value is the integer 1.
**Detection**: Parse JSON, validate the `hooks` shape, report a missing `version`, and check that a present `version` is an integer >= 1
**Fix**: Add `"version": 1` and the `hooks` object; correct `version` if it is not a positive integer
**Source**: cursor.com/docs/hooks

<a id="cur-011"></a>
### CUR-011 [MEDIUM] Unknown Cursor Hook Event Name
**Requirement**: Hook event names in `.cursor/hooks.json` SHOULD use documented Cursor events, including `workspaceOpen`
**Detection**: Validate each `hooks.<event>` key against allowlisted event names
**Fix**: [AUTO-FIX] Rename event keys to supported Cursor hook events
**Source**: cursor.com/docs/hooks

<a id="cur-012"></a>
### CUR-012 [HIGH] Hook Entry Missing Required Command Field
**Requirement**: Each hook entry MUST include a `command` field, except `type: "prompt"` entries. "Prompt hooks use an LLM to evaluate a natural language condition" and carry `prompt` instead - the doc's example has only `type`, `prompt` and `timeout`. CUR-018 checks `prompt` for those.
**Detection**: Parse `hooks.<event>[]` objects and check for missing `command`, skipping entries whose `type` is `prompt`
**Fix**: Add a non-empty command to each hook object
**Source**: cursor.com/docs/hooks

<a id="cur-013"></a>
### CUR-013 [HIGH] Invalid Cursor Hook Type Value
**Requirement**: Hook `type` MUST be `command` or `prompt` when present
**Detection**: Parse hook entries and validate `type` values
**Fix**: [AUTO-FIX] Change invalid `type` values to supported values
**Source**: cursor.com/docs/hooks

<a id="cur-014"></a>
### CUR-014 [HIGH] Invalid Cursor Subagent Frontmatter
**Requirement**: `.cursor/agents/**/*.md` files MUST have valid YAML frontmatter. All fields are optional: `name` defaults to the filename and `model` defaults to `inherit`. Specific model IDs may append `[id=value]` parameter lists, including empty `[]`.
**Detection**: Parse frontmatter and validate optional typed fields (`name`, `description`, `model`, `readonly`, `is_background`) when present
**Fix**: Correct frontmatter keys, naming format, and value types
**Source**: cursor.com/docs/subagents

<a id="cur-015"></a>
### CUR-015 [MEDIUM] Empty Cursor Subagent Body
**Requirement**: `.cursor/agents/**/*.md` Cursor subagent markdown files SHOULD include body instructions after frontmatter
**Detection**: Parse file and check that body content is non-empty after frontmatter
**Fix**: Add clear subagent instructions below frontmatter
**Source**: cursor.com/docs/subagents

<a id="cur-016"></a>
### CUR-016 [HIGH] Invalid .cursor/environment.json Schema
**Requirement**: `.cursor/environment.json` MUST match the published schema at cursor.com/schemas/environment.schema.json. Comments are allowed, trailing commas are not, no root field is required, `build` needs `dockerfile` or `dockerfileContents` (the schema's `anyOf`) with an optional `context`, and unknown root/build fields are rejected except for the conventional root `$schema` association key. The root accepts the union of the schema's `common` and `container` definitions: `name`, `user`, `install`, `start`, `repositoryDependencies`, `disableAllMcpServers`, `mcpServerAllowlist`, `egressAllowlist` (array of strings), `egressMode` (`allow_all`, `parent_plus_network_settings`, `default_with_network_settings`, or `network_settings_only`), `chromeExecutablePath`, `enable_testing` (boolean or the strings `"true"`/`"false"`), `ports`, `terminals`, `build`, `image`, `snapshot`, and `agentCanUpdateSnapshot`. Each `mcpServerAllowlist` entry identifies an HTTP server by `serverUrl` or a stdio server by `command`, with an optional `toolAllowlist` array of strings. Terminal entries require only `command`; `name` and `description` are optional. `update` is not in the schema and is reported as renamed to `install`.
**Detection**: Strip JSON comments, parse JSON, enforce the root/build/MCP-server closed-field sets, and validate setup strings, repository dependencies, MCP policy, egress policy, testing flag, ports, build fields, image and snapshot fields, and terminal entries
**Fix**: Correct field types; rename `update` to `install`
**Source**: cursor.com/docs/cloud-agent/setup

<a id="cur-017"></a>
### CUR-017 [MEDIUM] Invalid Hook Entry Field Types
**Requirement**: Hook entry fields SHOULD have correct types (timeout: positive number, loop_limit: number|null, failClosed: boolean, matcher: regex string where an empty string or `"*"` matches everything)
**Detection**: Check field types in hook entries
**Fix**: Manual
**Source**: cursor.com/docs/hooks

<a id="cur-018"></a>
### CUR-018 [MEDIUM] Invalid or Missing Prompt Hook Field
**Requirement**: Prompt-type hooks SHOULD have a non-empty string `prompt` field
**Detection**: Check for `type: "prompt"` without a non-empty string prompt
**Fix**: Manual
**Source**: cursor.com/docs/hooks

<a id="cur-019"></a>
### CUR-019 [LOW] Invalid Prompt Hook Model Type
**Requirement**: model field on prompt hooks MAY be validated as string
**Detection**: Check model field type
**Fix**: Manual
**Source**: cursor.com/docs/hooks

<a id="cur-020"></a>
### CUR-020 [HIGH] Ignored Plain Markdown Cursor Rule
**Requirement**: Project rules under `.cursor/rules/` MUST use the `.mdc` extension. Cursor explicitly ignores plain `.md` files in this directory.
**Detection**: Classify `.cursor/rules/**/*.md` so agnix can report the ignored extension before validating content
**Fix**: Rename the file with a `.mdc` extension
**Source**: cursor.com/docs/rules

<a id="cur-021"></a>
### CUR-021 [MEDIUM] Cursor Rule Over 500 Lines
**Requirement**: A project rule SHOULD stay under 500 lines: "Keep rules under 500 lines" and "Split large rules into multiple, composable rules" (docs), "Keep rules under 500 lines. Split large rules into smaller, focused files." (help)
**Detection**: `.cursor/rules/**/*.mdc` with more than 500 lines, reported at line 501. Legacy `.cursorrules` files get only the CUR-006 migration note.
**Fix**: Split the rule into focused rules and reference files instead of copying them
**Source**: cursor.com/docs/rules ("Best practices"), cursor.com/help/customization/rules

<a id="cur-022"></a>
### CUR-022 [HIGH] Project Hook Script Path Relative to the Wrong Base
**Requirement**: Script paths in a project `.cursor/hooks.json` MUST be relative to the project root: "Project hooks run from the **project root**, so use `.cursor/hooks/format.sh` (not `./hooks/format.sh`)" and "For project hooks, use paths like `.cursor/hooks/script.sh` (relative to project root), not `./hooks/script.sh` (which would look for `<project>/hooks/script.sh`)."
**Detection**: For each command hook whose first word is a relative path: an error when the path does not exist at the project root but does under `.cursor/`; a warning when it starts with `./hooks/` or `hooks/` and the project has no `hooks/` directory. The user file `~/.cursor/hooks.json`, which runs from `~/.cursor/`, is not checked.
**Fix**: [AUTO-FIX, unsafe] Prefix the path with `.cursor/` when that file exists (it changes which script runs)
**Source**: cursor.com/docs/hooks ("Project hooks (.cursor/)", "Configuration")

<a id="cur-023"></a>
### CUR-023 [LOW] Cursor Skill Uses Legacy globs
**Requirement**: Skills SHOULD scope files with `paths`: "The legacy `globs` field is still accepted as a fallback for older skills, but new skills should use `paths`."
**Detection**: A `globs` frontmatter key in a skill Cursor loads: `.cursor/skills/`, and `.agents/skills/` when the run targets Cursor (other tools read that folder too). A LOW migration note; when `paths` is also set, the note says `globs` is only a fallback.
**Fix**: [AUTO-FIX, unsafe] Rename `globs` to `paths` when `paths` is absent; offered only when the edited frontmatter re-parses to the same mapping with the key renamed
**Source**: cursor.com/docs/skills ("Scoping a skill to specific files")

<a id="cur-024"></a>
### CUR-024 [MEDIUM] MCP Field Not Supported for the Transport
**Requirement**: MCP server fields MUST match the transport: "The `envFile` option is only available for STDIO servers. Remote servers (HTTP/SSE) do not support `envFile`." and "Add an `auth` object to remote server entries that use `url`".
**Detection**: In `.cursor/mcp.json` and `~/.cursor/mcp.json`: `envFile` on a server with `url`, and `auth` on a stdio server (with `command` and no `url`), at the key
**Fix**: Manual - use `${env:NAME}` interpolation for remote servers
**Source**: cursor.com/docs/mcp ("STDIO server configuration", "Static OAuth for remote servers")

<a id="cur-025"></a>
### CUR-025 [MEDIUM] Cursor Permission Allows Everything
**Requirement**: Allowlists SHOULD name specific domains and tools. The CLI permissions reference marks "`WebFetch(*)` | Allow fetches from any domain (use with caution)" and "`Mcp(*:*)` | Allow all MCP tools (use with caution)"; for `permissions.json`, `*:*` matches "All tools from all servers" and "Allowlists and `autoRun` instructions are best-effort convenience. They are not a security guarantee."
**Detection**: Warning for `WebFetch(*)` and `Mcp(*:*)` in `permissions.allow` of `.cursor/cli.json` or `~/.cursor/cli-config.json`; info for `*:*` in a `permissions.json` `mcpAllowlist`
**Fix**: Manual
**Source**: cursor.com/docs/cli/reference/permissions, cursor.com/docs/reference/permissions

<a id="cur-026"></a>
### CUR-026 [MEDIUM] CLI Setting Ignored in a Project cli.json
**Requirement**: A project `.cursor/cli.json` SHOULD hold only permissions: "Only permissions can be configured at the project level. All other CLI settings must be set globally."
**Detection**: Top-level CLI settings in `.cursor/cli.json` other than `permissions` and `version` that the configuration reference lists (`editor`, `channel`, `model`, `maxMode`, `hasChangedDefaultModel`, `notifications`, `hints`, `rewind`, `suggestNextPrompt`, `display`, `approvalMode`, `sandbox`, `network`, `attribution`)
**Fix**: Manual - move the setting to `~/.cursor/cli-config.json`
**Source**: cursor.com/docs/cli/reference/configuration ("File location")

<a id="cur-027"></a>
### CUR-027 [MEDIUM] Ignored permissions.json Entry
**Requirement**: `permissions.json` entries MUST be in a form Cursor reads: "Entries that do not contain a `:` are ignored." (MCP allowlist), "Non-string entries inside either array are silently dropped.", "All fields are optional. Unknown keys are ignored.", and "If neither file is present, both are unparseable, or no file contains a given key, Cursor falls back to the IDE allowlist for that key."
**Detection**: In `.cursor/permissions.json` and `~/.cursor/permissions.json` (JSONC): an unparseable file, a non-object root, an unknown top-level key (with a near-miss suggestion), a non-array allowlist, a non-string entry, and an `mcpAllowlist` entry without `:`
**Fix**: Manual
**Source**: cursor.com/docs/reference/permissions ("Top-level fields", "MCP allowlist format", "Precedence")

<a id="cur-028"></a>
### CUR-028 [LOW] Cursor Subagent Without Description
**Requirement**: A subagent SHOULD have a description: "**Invest in descriptions** — The `description` field determines when Agent delegates to your subagent." (`description` itself is optional.)
**Detection**: Info for `.cursor/agents/**/*.md` without a non-empty string `description`
**Fix**: Manual - say what the subagent does and when to use it
**Source**: cursor.com/docs/subagents ("Best practices", "Configuration fields")

<a id="cur-029"></a>
### CUR-029 [LOW] Cursor Subagent Prompt Too Long
**Requirement**: Subagent prompts SHOULD be concise: "**Overly long prompts** — A 2,000-word prompt doesn't make a subagent smarter. It makes it slower and harder to maintain."
**Detection**: Info when the body of a `.cursor/agents/**/*.md` file has more than 2,000 whitespace-separated words outside code fences
**Fix**: Manual
**Source**: cursor.com/docs/subagents ("Anti-patterns to avoid")

<a id="cur-030"></a>
### CUR-030 [HIGH] Invalid Cursor Plugin Manifest
**Requirement**: A Cursor plugin manifest MUST be valid JSON with the required fields. `.cursor-plugin/plugin.json` requires `name`: "Plugin identifier. Lowercase, kebab-case (alphanumerics, hyphens, and periods). Must start and end with an alphanumeric character."; `author` needs "`name` (required)". `.cursor-plugin/marketplace.json` requires `name`, `owner` (with `name`) and `plugins`, and each plugin entry a `name`.
**Detection**: Invalid JSON, a non-object root, a missing or non-string `name`, a name outside the format, an `author` or `owner` without `name`, and missing marketplace fields
**Fix**: Manual
**Source**: cursor.com/docs/reference/plugins ("Required fields", "Optional fields", "Marketplace manifest fields")

<a id="cur-031"></a>
### CUR-031 [MEDIUM] Literal Secret in environment.json Command
**Requirement**: Cloud agent credentials SHOULD stay out of the repository: "The Secrets tab remains the recommended approach for security and management."
**Detection**: `install`, `start` and `terminals[].command` strings in `.cursor/environment.json` that assign a literal to a credential-named variable (`API_TOKEN=...`), pass one to a credential flag (`--token ...`), or contain a known token prefix (`sk-`, `ghp_`, `github_pat_`, `xox?-`, `AKIA`, `glpat-`); variable references and placeholders are skipped (shared plaintext-secret heuristic)
**Fix**: Manual - store the value in the Secrets tab
**Source**: cursor.com/docs/cloud-agent/setup ("Monorepos with multiple .env files")

<a id="cur-032"></a>
### CUR-032 [LOW] MCP Permission Hook Fails Open
**Requirement**: Security-critical MCP hooks SHOULD fail closed: "Crashes, timeouts, and non-zero exit codes other than `2` fail open by default ... Set `failClosed: true` on the hook definition to block on those failures too. This is recommended for security-critical `beforeMCPExecution` hooks."
**Detection**: Info for a project `beforeMCPExecution` command hook without `failClosed: true`
**Fix**: Manual - set `failClosed: true` if the hook enforces a policy
**Source**: cursor.com/docs/hooks ("beforeShellExecution / beforeMCPExecution")

<a id="cur-033"></a>
### CUR-033 [LOW] Hook Not Run by Cloud Agents
**Requirement**: Hooks meant for cloud agents SHOULD use what they run: "Cloud agents run **command-based hooks** only." and "Hooks not available in cloud agents": `sessionStart`, `sessionEnd`, `beforeMCPExecution`, `afterMCPExecution`, `beforeTabFileRead`, `afterTabFileEdit`, `workspaceOpen`.
**Detection**: Info for those events and for `type: "prompt"` entries in a project `.cursor/hooks.json`, only when the project has `.cursor/environment.json` (a sign it uses cloud agents)
**Fix**: Manual
**Source**: cursor.com/docs/hooks ("Cloud agent support")

<a id="cur-034"></a>
### CUR-034 [LOW] .cursorignore Negation Cannot Re-include
**Requirement**: Negations SHOULD NOT target files in a nested directory of a `dir/*` exclusion: "When using negation patterns (prefixed with `!`), you cannot re-include a file if a parent directory is excluded via \*." The workaround is to "Explicitly exclude nested directories".
**Detection**: Info for `!dir/sub/.../file` after `dir/*` without a `dir/sub/*` line
**Fix**: Manual - add the nested exclusion before the negation
**Source**: cursor.com/docs/reference/ignore-file ("Negation pattern limitations")

<a id="cur-035"></a>
### CUR-035 [LOW] Too Many Cursor Subagents
**Requirement**: A project SHOULD keep a small set of focused subagents: "Having 50+ subagents with vague instructions like "helps with coding" is ineffective." and "Start with 2-3 focused subagents."
**Detection**: Info, once per project on the first file in path order, when `.cursor/agents/` holds 50 or more `.md` files (the vendor's figure)
**Fix**: Manual - merge or remove overlapping subagents
**Source**: cursor.com/docs/subagents ("Anti-patterns to avoid")

<a id="cur-036"></a>
### CUR-036 [LOW] Long Code Block in a Cursor Rule
**Requirement**: Rules SHOULD reference code rather than copy it: "Reference files instead of copying their contents. This keeps rules short and prevents them from becoming stale as code changes." and "**Duplicating what's already in your codebase**: Point to canonical examples instead of copying code."
**Detection**: Info for a fenced code block of more than 40 lines in a `.cursor/rules/**/*.mdc` body. The 40-line threshold is an agnix heuristic; Cursor's own example rule has a 20-line block.
**Fix**: Manual - reference the example file
**Source**: cursor.com/docs/rules ("Best practices", "What to avoid in rules")

<a id="cur-037"></a>
### CUR-037 [LOW] Long-running Process in environment.json install
**Requirement**: `install` SHOULD finish: "Keep long-running processes out of `install`. Put Docker, databases, tunnels, and dev servers in startup commands."
**Detection**: Info when `install` runs a dev server or watcher (`npm run dev`, `pnpm dev`, `next dev`, `python -m http.server`), `tail -f`, `sleep infinity`, `docker compose up` without `-d`, or ends with a background `&` (agnix heuristic patterns)
**Fix**: Manual - move the process to `start` or `terminals`
**Source**: cursor.com/docs/cloud-agent/setup ("How to decide what to put in your install script")

<a id="cur-039"></a>
### CUR-039 [HIGH] Invalid Cursor CLI Config
**Requirement**: Cursor CLI config files MUST be valid: "Pure JSON format (no comments)" and "Corrupted files are backed up as `.bad` and recreated"; `permissions.allow` and `permissions.deny` are string arrays of `Shell(commandBase)`, `Read(pathOrGlob)`, `Write(pathOrGlob)`, `WebFetch(domainOrPattern)` and `Mcp(server:tool)` tokens.
**Detection**: In `.cursor/cli.json` and `~/.cursor/cli-config.json`: invalid JSON, comments or trailing commas, a non-object root, a non-object `permissions`, non-array lists and non-string entries (errors); an entry that is not one of the five token forms (warning, with a near-miss suggestion)
**Fix**: Manual
**Source**: cursor.com/docs/cli/reference/configuration ("Notes"), cursor.com/docs/cli/reference/permissions

<a id="cur-040"></a>
### CUR-040 [MEDIUM] Cursor Plugin Submission Checklist
**Requirement**: Cursor plugins SHOULD meet the submission checklist: "All paths in manifest are relative and valid (no `..`, no absolute paths)", "Cursor Plugins using variables declare every `${VAR}` from `mcp.json` in the manifest schema", and "Cursor multi-plugin repositories have `.cursor-plugin/marketplace.json` at the repo root with unique plugin names".
**Detection**: Manifest paths (`rules`, `agents`, `skills`, `commands`, `hooks`, `mcpServers`, `logo`, and marketplace `source`) that are absolute or contain `..` (URLs allowed for `logo`); `${VAR}` placeholders in the plugin's `mcp.json` (root `mcp.json`, the `mcpServers` path, or inline servers) not declared under `variables.properties` (`${env:...}`, `${CURSOR_PLUGIN_ROOT}` and the other built-in variables excepted); duplicate plugin names in a marketplace
**Fix**: Manual
**Source**: cursor.com/docs/reference/plugins ("Submission checklist", "Variables")

---

## CLINE RULES

<a id="cln-001"></a>
### CLN-001 [HIGH] Empty Cline Rules File
**Requirement**: `.clinerules` file and Markdown rules in `.clinerules/` or `.cline/rules/` MUST have content after any frontmatter. The Cline SDK loader throws "Missing instructions body in rule file.", and the VS Code extension skips a file whose trimmed content is empty.
**Detection**: Parse file, strip optional YAML frontmatter, check remaining body is non-whitespace. Files whose frontmatter Cline cannot parse get CLN-010 instead.
**Fix**: No auto-fix (content must be authored by user)
**Source**: docs.cline.bot/customization/cline-rules ("Where Rules Live"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseRuleConfigFromMarkdown`), apps/vscode/src/core/context/instructions/user-instructions/rule-helpers.ts

<a id="cln-002"></a>
### CLN-002 [HIGH] Invalid Paths Glob in Cline Rules
**Requirement**: `paths` field in `.clinerules/*.{md,txt}` and `.cline/rules/*.md` frontmatter MUST contain valid glob patterns: "Currently, `paths` is the supported conditional. It takes an array of glob patterns".
**Detection**: Parse YAML frontmatter, extract `paths` field, validate each glob pattern
**Fix**: No auto-fix (glob patterns must be manually corrected)
**Source**: docs.cline.bot/customization/cline-rules ("Conditional Rules")

<a id="cln-003"></a>
### CLN-003 [MEDIUM] Unknown Frontmatter Key in Cline Rules
**Requirement**: Frontmatter in `.clinerules/*.{md,txt}` and `.cline/rules/*.md` SHOULD only use keys Cline reads: `paths` ("Currently, `paths` is the supported conditional"), and `name`, `disabled` and `enabled`, which the SDK loader reads. Other keys have no effect ("Unknown conditional keys are ignored for forward compatibility."). `disabled` and `enabled` must be booleans and `name` a string: the SDK loader throws "Frontmatter field '${fieldName}' must be a boolean." (or "must be a string.") and drops the file.
**Detection**: Parse YAML frontmatter; report keys outside `paths`, `name`, `disabled`, `enabled`, and those three with the wrong type, at the key line
**Fix**: [AUTO-FIX unsafe] Remove an unknown frontmatter key; offered only when the edited frontmatter re-parses to the same mapping without the key. No fix for wrong types.
**Source**: docs.cline.bot/customization/cline-rules ("Conditional Rules"); vendor code apps/vscode/src/core/context/instructions/user-instructions/rule-conditionals.ts, sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseRuleConfigFromMarkdown`)

<a id="cln-004"></a>
### CLN-004 [HIGH] Scalar Paths in Cline Rules
**Requirement**: `paths` field in `.clinerules/*.{md,txt}` and `.cline/rules/*.md` frontmatter MUST be a YAML array, not a scalar string. Cline then drops the condition ("// Invalid type -> ignore conditional (fail-open)"), so the rule loads for every request.
**Detection**: Parse YAML frontmatter, check if `paths` is a scalar string
**Fix**: [AUTO-FIX safe] Convert scalar paths to a one-item array; offered only when the edited frontmatter re-parses to the same mapping with `paths` as that array
**Source**: docs.cline.bot/customization/cline-rules ("Conditional Rules"); vendor code apps/vscode/src/core/context/instructions/user-instructions/rule-conditionals.ts (`evaluatePathsConditional`)

<a id="cln-005"></a>
### CLN-005 [HIGH] Empty Workflow File
**Requirement**: Workflow files MUST have steps: the SDK loader throws "Missing instructions body in workflow file." Project workflows live in `.cline/workflows/` ("# Project workflows"), and "Cline also reads the older `.clinerules/` layout (rules, plus its `workflows/`, `skills/` and `hooks/` folders)".
**Detection**: `.cline/workflows/*.md` and `.clinerules/workflows/*.md` that are empty or have no content after the frontmatter
**Fix**: Manual - add workflow content
**Source**: docs.cline.bot/getting-started/config ("Configuration Directory Layout"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseWorkflowConfigFromMarkdown`)

<a id="cln-006"></a>
### CLN-006 [MEDIUM] Unknown Frontmatter Key in Cline Workflow
**Requirement**: Workflow frontmatter SHOULD only use keys Cline reads: the SDK loader reads `name`, `disabled` and `enabled` (booleans; "Frontmatter field '${fieldName}' must be a boolean."). Other keys have no effect. Frontmatter itself is allowed.
**Detection**: Parse the frontmatter of `.cline/workflows/*.md` and `.clinerules/workflows/*.md`; report keys outside `name`, `disabled`, `enabled`, and those three with the wrong type, at the key line
**Fix**: Manual - remove the key
**Source**: docs.cline.bot/getting-started/config ("Configuration Directory Layout"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseWorkflowConfigFromMarkdown`)

<a id="cln-009"></a>
### CLN-009 [MEDIUM] Unknown Hook Event Name
**Requirement**: Hook files SHOULD be named after a file hook event: TaskStart, TaskResume, UserPromptSubmit, PreToolUse, PostToolUse, TaskComplete, TaskError, TaskCancel, SessionShutdown ("File Hooks vs Runtime Hooks"). PreCompact is listed as "not wired for file hooks today", so a PreCompact file hook never runs. The VS Code extension reads `.clinerules/hooks/`; "The CLI discovers hooks from `.cline/hooks/` in the workspace".
**Detection**: Files directly in `.clinerules/hooks/` or `.cline/hooks/` whose name without extension is not one of the events; PreCompact gets a "not wired" message. Markdown and text notes and files starting with `_` or `.` are skipped.
**Fix**: Manual - rename file
**Source**: github.com/cline/cline sdk/examples/hooks/README.md ("File Hooks vs Runtime Hooks"), docs.cline.bot/customization/clineignore ("Enforcing .clineignore with a Hook")

<a id="cln-010"></a>
### CLN-010 [HIGH] Unparseable Cline Rule Frontmatter
**Requirement**: Rule and workflow frontmatter MUST be valid YAML: "**Invalid YAML**: If frontmatter can't be parsed, Cline fails open. The rule activates with raw content visible to help debugging." The SDK loader throws "Failed to parse YAML frontmatter" and skips the file.
**Detection**: Rules (`.clinerules/*.{md,txt}`, `.cline/rules/*.md`) and workflows whose frontmatter is not valid YAML or has no closing `---`. Reported at the line of an unquoted glob that starts with `*` (a YAML alias, for example `- *.md`) with a hint to quote it, otherwise at the parser's line.
**Fix**: Manual - fix the YAML (quote glob patterns)
**Source**: docs.cline.bot/customization/cline-rules ("Behavior Details"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts, apps/vscode/src/core/context/instructions/user-instructions/rule-helpers.ts

<a id="cln-011"></a>
### CLN-011 [HIGH] Invalid paths Entries in Cline Rules
**Requirement**: `paths` MUST be an array of non-empty strings. Cline checks `isNonEmptyStringArray` and otherwise ignores the condition ("// Invalid type -> ignore conditional (fail-open)"), so the rule loads for every request. `paths: []` is valid: "`paths: []` means the rule never activates."
**Detection**: `paths` arrays with an empty string or a non-string item, and `paths` values that are neither a string (CLN-004) nor an array (number, boolean, mapping, or empty), at the `paths` line
**Fix**: Manual
**Source**: vendor code apps/vscode/src/core/context/instructions/user-instructions/rule-conditionals.ts (`isNonEmptyStringArray`, `evaluatePathsConditional`); docs.cline.bot/customization/cline-rules ("Behavior Details")

<a id="cln-012"></a>
### CLN-012 [MEDIUM] Cline Hook Not Runnable in the VS Code Extension
**Requirement**: Hooks for the VS Code extension SHOULD follow its naming and format: "Hooks in the extension live in `.clinerules/hooks/` (workspace) or `~/Documents/Cline/Hooks/` (global), must be named exactly after their event with no file extension, and need a shebang and the executable bit". The CLI reads `.cline/hooks/` and accepts extensions (`.cline/hooks/PreToolUse.sh`).
**Detection**: In `.clinerules/hooks/`: a file named `<Event>.<ext>` for a valid event, and a hook whose first line is not a `#!` shebang. `.cline/hooks/` is not checked. The executable bit is not checked.
**Fix**: Manual - rename the file to the event name, add a shebang
**Source**: docs.cline.bot/customization/clineignore ("Install in the VS Code extension", "Install in the CLI"), github.com/cline/cline sdk/examples/hooks/README.md

<a id="cln-013"></a>
### CLN-013 [LOW] Deprecated .clineignore
**Requirement**: Projects SHOULD move off `.clineignore`: "| .clineignore | VS Code extension | Deprecating soon | Block Ignored File Access plugin |" and "To avoid creating a false sense of security, we are removing `.clineignore` as a supported feature." The file "was never designed to prevent access to those files".
**Detection**: A `.clineignore` file, as a LOW note. Not reported when `.clinerules/hooks/` or `.cline/hooks/` next to it has a PreToolUse hook (the documented guard hook reads `.clineignore`).
**Fix**: Manual - enforce the patterns with the PreToolUse guard hook or the Block Ignored File Access plugin
**Source**: docs.cline.bot/resources/deprecations ("`.clineignore`"), docs.cline.bot/customization/clineignore ("Enforcing .clineignore with a Hook")

<a id="cln-014"></a>
### CLN-014 [MEDIUM] Literal Secret in Cline Rule or Workflow
**Requirement**: Rules and workflows SHOULD NOT hold credentials: "Commit `.cline/` files you want to share with your team. Keep secrets out of the repo."
**Detection**: Heuristic: the first `api_key`, `token`, `password` or `*secret` assignment with a literal value (placeholders, variable references and code identifiers excluded), or a known provider token prefix (`sk-`, `ghp_`, `github_pat_`, `xox?-`, `AKIA`, `glpat-`), in Cline rules and workflows. One finding per file. Skills are left to the cross-tool skill secret rule.
**Fix**: Manual - move the value to an environment variable
**Source**: docs.cline.bot/getting-started/config ("What Goes Where?")

<a id="cln-015"></a>
### CLN-015 [MEDIUM] Cline paths Pattern Never Matches
**Requirement**: `paths` patterns SHOULD be relative to the workspace root: "Candidate paths are expected to be workspace-root-relative POSIX paths." A pattern with a leading `/` or `\`, a drive letter or a `..` segment never matches, and the docs examples are all relative.
**Detection**: `paths` entries that start with `/`, `\` or a drive letter, or contain a `..` segment. `./` prefixes are not reported.
**Fix**: Manual
**Source**: vendor code apps/vscode/src/core/context/instructions/user-instructions/rule-conditionals.ts (header comment); docs.cline.bot/customization/cline-rules ("Conditional Rules")

<a id="cln-016"></a>
### CLN-016 [MEDIUM] Remote MCP Server Without type in Cline Settings
**Requirement**: Remote servers in `cline_mcp_settings.json` SHOULD set `type`: "The `type` field selects the transport. Omitting it defaults to the legacy `sse` transport for backward compatibility, so set `"type": "streamableHttp"` explicitly for the recommended transport."
**Detection**: A server under `mcpServers` with `url` and no `type`, at the server key. The file usually lives in `~/.cline/data/settings/`, so repository runs rarely see it.
**Fix**: Manual - set `"type": "streamableHttp"` (or `"sse"` for SSE-only servers)
**Source**: docs.cline.bot/mcp/mcp-overview ("Remote server (Streamable HTTP)")

<a id="cl-sk-004"></a>
### CL-SK-004 [MEDIUM] Cline Skill Without Instructions
**Requirement**: A Cline skill MUST have instructions after its frontmatter: the SDK loader throws "Missing instructions body in skill file."
**Detection**: SKILL.md in `.cline/skills/` or `.clinerules/skills/` (and `.claude/skills/` or `.agents/skills/` when the run targets Cline) that is empty or has only frontmatter
**Fix**: Manual - add the skill instructions
**Source**: docs.cline.bot/customization/skills ("Where Skills Live"); vendor code sdk/packages/core/src/extensions/config/user-instruction-config-loader.ts (`parseSkillConfigFromMarkdown`)

---

## OPENCODE RULES

OpenCode 1 (`opencode-ai`) and OpenCode 2 (`@opencode/cli`, released 2026-09-11) read the same files. Each OpenCode file is checked against the release lines it targets: `[tool_versions] opencode` pins one line (1.x or 2.x); unpinned, a file that uses native OpenCode 2 fields (`agents`, `permissions`, `policies`, `mcp.servers`, ...) targets OpenCode 2, and any other file targets both. Checks that hold for one line only run when that is the only line targeted, so documented OpenCode 1 configuration gets no OpenCode 2 finding.

<a id="oc-001"></a>
### OC-001 [HIGH] Invalid Share Mode
**Requirement**: The `share` field MUST be `"manual"`, `"auto"` or `"disabled"` (schema `share` enum: "Control whether sessions may be shared manually, automatically, or not at all")
**Detection**: Parse JSON/JSONC, validate `share` at the value
**Fix**: Auto-fix (unsafe) - replace with the closest valid mode
**Source**: opencode.ai/config.json

<a id="oc-002"></a>
### OC-002 [HIGH] Invalid Instruction Path
**Requirement**: Entries of `instructions` MUST resolve: OpenCode loads "files, glob patterns, or HTTP(S) URLs" and skips missing files. "Relative path or glob | Searched in each directory from the workspace up to the project root"; "~/ path | Resolved from your home directory"; "Absolute path | Used as written".
**Detection**: Error for a non-array or non-string entry, an invalid glob, or a relative path that exists neither next to a project config nor (for `.opencode/opencode.json`) at the project root. `~/` and absolute entries are not checked (machine-specific; they were errors before), nor are entries of the global or system config, whose relative paths resolve per project.
**Fix**: Fix or remove the entry
**Source**: opencode.ai/docs/rules, opencode.ai/v2/docs/instructions

<a id="oc-003"></a>
### OC-003 [HIGH] opencode.json Parse Error
**Requirement**: opencode.json(c) MUST parse: "OpenCode supports JSON and JSONC. Use JSONC when you want comments or trailing commas." (the V1 docs' own first example has trailing commas; the schema sets `allowComments` and `allowTrailingCommas`). The root MUST be an object.
**Detection**: Parse as JSONC (comments and trailing commas allowed, formerly an error), report the syntax error position or a non-object root
**Fix**: Fix the syntax
**Source**: opencode.ai/v2/docs/config, opencode.ai/docs/config

<a id="oc-004"></a>
### OC-004 [MEDIUM] Unknown Config Key
**Requirement**: Top-level keys SHOULD be keys of https://opencode.ai/config.json (top-level `additionalProperties: false`), which lists the native OpenCode 2 keys and the OpenCode 1 keys it still reads ("supported V1 and native V2 fields may coexist at the top level").
**Detection**: Warning at each key outside the schema (plus the OpenCode 1 TUI keys `theme`, `keybinds`, `tui`, which OC-DEP-005 reports), with a closest-key hint. When `[tool_versions] opencode` pins 1.x, native OpenCode 2 keys (`agents`, `permissions`, `policies`, `commands`, `providers`, `plugins`, `snapshots`, `update`, `media`, `websearch`, `worktree`, `warming`) are reported as keys OpenCode 1 does not read.
**Fix**: No auto-fix
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/migrate-v1

<a id="oc-006"></a>
### OC-006 [LOW] Remote URL in Instructions
**Requirement**: Remote instructions are fetched at load: "Remote instructions are fetched with a 5 second timeout." (V1) / "Fetched when config loads, with a five-second timeout" (V2). That a remote entry can slow or skip loading is agnix inference.
**Detection**: Info at each `http://` or `https://` entry of `instructions`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/rules, opencode.ai/v2/docs/instructions

<a id="oc-007"></a>
### OC-007 [MEDIUM] Invalid Agent Definition
**Requirement**: Agents SHOULD have a description. OpenCode 1: "Use the `description` option to provide a brief description of what the agent does and when to use it." ... "This is a **required** config option." (its own `build`/`plan` overrides omit it). OpenCode 2: "description explains the agent's purpose. Add it to subagents because OpenCode shows it to the model choosing which agent to launch".
**Detection**: Warning for an `agent.<id>` entry without `description` when the file targets OpenCode 1 (custom agents with no mode or mode `subagent`/`all`), and for an `agents.<id>` (or OpenCode 2 targeted) entry with mode `subagent` or `all`. Built-in agents (build, plan, general, explore, scout, compaction, title, summary) and disabled agents are skipped. Error when `agent`/`agents` is not an object.
**Fix**: Add a description
**Source**: opencode.ai/docs/agents, opencode.ai/v2/docs/agents

<a id="oc-008"></a>
### OC-008 [HIGH] Invalid Permission Config
**Requirement**: Permission values MUST be `"allow"`, `"ask"` or `"deny"`: OpenCode 1 `permission` is a shorthand action or an object of patterns to actions (top level and per agent); OpenCode 2 `permissions` is an ordered array where "Each rule requires three string fields" `action`, `resource`, `effect` (`Permission.Rule`, `additionalProperties: false`).
**Detection**: Error at each invalid value or rule (top level, agent entries, Markdown agent frontmatter).
**Fix**: [AUTO-FIX, unsafe] Replace an invalid action with the closest valid one
**Source**: opencode.ai/docs/permissions, opencode.ai/v2/docs/permissions

<a id="oc-009"></a>
### OC-009 [MEDIUM] Invalid Variable Substitution
**Requirement**: Substitutions MUST use `{env:VARIABLE_NAME}` or `{file:path/to/file}`: "You can use variable substitution in your config files to reference environment variables and file contents."
**Detection**: Warning at each string with a `{word:...}` pattern that has another prefix or an empty value
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config, opencode.ai/v2/docs/config

<a id="oc-cfg-001"></a>
### OC-CFG-001 [HIGH] Invalid Model Format
**Requirement**: Model references MUST be `provider/model`: "model uses provider/model with an optional #variant" (OpenCode 2; schema pattern `^[^/#]+\/[^#]+(?:#[^#]+)?$`), and "JSON configuration also accepts the expanded form" `{ providerID, model, variant }`.
**Detection**: Error for `model`, `small_model`, agent and command `model` values that are not `provider/model[#variant]`, an `{env:}`/`{file:}` reference, or the expanded object (the object form was an error before)
**Fix**: No auto-fix
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/agents

<a id="oc-cfg-002"></a>
### OC-CFG-002 [HIGH] Invalid autoupdate or update Value
**Requirement**: `autoupdate` (OpenCode 1) MUST be a boolean or `"notify"`; `update` (OpenCode 2) MUST be `"disable"`, `"notify"` or `"auto"`
**Detection**: Error at an invalid value
**Fix**: No auto-fix
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/config

<a id="oc-cfg-004"></a>
### OC-CFG-004 [MEDIUM] Invalid Default Agent
**Requirement**: `default_agent` SHOULD name an existing primary agent: "If the specified agent doesn't exist or is a subagent, OpenCode will fall back to `\"build\"` with a warning."
**Detection**: Warning when the name is not a built-in agent, not defined under `agent`, `agents` or `mode`, and no `agents/` (or `agent/`, `modes/`, `mode/`) Markdown file of that name exists in the project `.opencode/` or the global config directory (Markdown agents were reported before)
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-005"></a>
### OC-CFG-005 [MEDIUM] Hardcoded Secret in Config
**Requirement**: Secrets SHOULD stay out of the config file: "Use configuration for other options. Store secrets in environment variables rather than in the file" (OpenCode 2 MCP servers); the config docs show `"apiKey": "{env:ANTHROPIC_API_KEY}"` and `"apiKey": "{file:~/.secrets/openai-key}"`.
**Detection**: Warning (formerly an error) for a literal provider `apiKey` (`provider.<id>.options`, `providers.<id>.settings`), credential headers of providers and MCP servers, credential-named MCP `environment` variables, and OAuth `clientSecret`/`client_secret`. `{env:...}` and `{file:...}` (formerly reported), shell variables, all-caps placeholders such as `Bearer MY_API_KEY` and short dummy keys (`"ollama"`) are not reported; a value is literal when it has a known token prefix or 16 or more characters (agnix heuristic).
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/mcp-servers, opencode.ai/docs/config

<a id="oc-cfg-006"></a>
### OC-CFG-006 [HIGH] Invalid MCP Server Structure
**Requirement**: MCP servers MUST be objects with `"type": "local"` or `"remote"`. OpenCode 1 lists servers under `mcp`; OpenCode 2 under `mcp.servers` ("V2 does not place server names directly under mcp"), with defaults in `mcp.timeout`. An OpenCode 1 entry with only `enabled` toggles a server from remote organization config.
**Detection**: Error for a non-object `mcp`, a non-object server, or a server without a valid `type`. `mcp.servers` and `mcp.timeout` are containers, not servers (they were reported as a server named `servers`), and enabled-only OpenCode 1 entries are valid.
**Fix**: No auto-fix
**Source**: opencode.ai/docs/mcp-servers, opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-007"></a>
### OC-CFG-007 [HIGH] Invalid MCP Server Command, URL, cwd, or Environment
**Requirement**: Local servers MUST have `command` ("Executable followed by its arguments", required); remote servers MUST have an absolute `url` ("requires an absolute URL"); `environment` holds strings; `cwd` is a path; OpenCode 2 `protocol` is `legacy`, `auto` or `2026-07-28`. `env` is not a field.
**Detection**: Error at the offending field
**Fix**: No auto-fix
**Source**: opencode.ai/docs/mcp-servers, opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-008"></a>
### OC-CFG-008 [HIGH] Invalid Log Level
**Requirement**: `logLevel` MUST be one of the schema enum `DEBUG`, `INFO`, `WARN`, `ERROR` (case-sensitive; lowercase and `trace`/`fatal` were accepted before). OpenCode 2 ignores the field ("logLevel: use OPENCODE_LOG_LEVEL when starting OpenCode"), so the check runs only for files that target OpenCode 1.
**Detection**: Error at the value when the file targets OpenCode 1 (OC-DEP-009 covers OpenCode 2)
**Fix**: Auto-fix (unsafe) - replace with the matching level
**Source**: opencode.ai/config.json

<a id="oc-cfg-009"></a>
### OC-CFG-009 [HIGH] Invalid Compaction Token Budget
**Requirement**: `compaction.reserved` (OpenCode 1), `compaction.buffer` and `compaction.keep.tokens` (OpenCode 2) MUST be non-negative integers (schema `minimum: 0`)
**Detection**: Error at the value
**Fix**: No auto-fix
**Source**: opencode.ai/config.json

<a id="oc-cfg-010"></a>
### OC-CFG-010 [HIGH] Invalid Skills URL
**Requirement**: `skills.urls` (OpenCode 1 object form) MUST hold http(s) URLs. The OpenCode 2 array form mixes paths and URLs ("Add more local directories or HTTP catalogs with the skills array") and is not checked.
**Detection**: Error at each non-URL or non-string entry
**Fix**: No auto-fix
**Source**: opencode.ai/config.json

<a id="oc-cfg-011"></a>
### OC-CFG-011 [HIGH] Invalid MCP Timeout
**Requirement**: OpenCode 1: "`timeout` | Number | Timeout in ms for fetching tools from the MCP server" (positive integer). OpenCode 2: "Timeouts are positive integer milliseconds. Set defaults under mcp.timeout; a server's timeout object overrides matching defaults" with `startup`, `catalog`, `execution`.
**Detection**: Error at a non-positive OpenCode 1 timeout, and at unknown keys or non-positive values of OpenCode 2 timeout objects
**Fix**: No auto-fix
**Source**: opencode.ai/docs/mcp-servers, opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-012"></a>
### OC-CFG-012 [HIGH] Invalid MCP OAuth Config
**Requirement**: `oauth` MUST be an object or `false`. No member is required: "OAuth client ID. If not provided, dynamic client registration will be attempted", and `"oauth": {}` is a documented example. Members: `clientId`, `clientSecret`, `scope`, `callbackPort`, `redirectUri` (OpenCode 1, `additionalProperties: false`); `client_id`, `client_secret`, `scope`, `callback_port`, `redirect_uri`, `auth_server_metadata_url` under `mcp.servers` ("use V2's snake_case OAuth fields"). The rule formerly required `client_id` and `authorization_url`, which no OpenCode page mentions.
**Detection**: Error for `oauth: true` or another non-object, an unknown member (with a closest-name hint), a non-string member, or a callback port outside 1-65535. OpenCode 1 members under `mcp.servers` are OC-CFG-021.
**Fix**: No auto-fix
**Source**: opencode.ai/docs/mcp-servers, opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-013"></a>
### OC-CFG-013 [HIGH] Invalid Server Config
**Requirement**: `server` options MUST have the documented types (`port` positive integer, `hostname` string, `mdns` boolean, `mdnsDomain` string, `cors` array). OpenCode 2 ignores `server` ("use the V2 service and explicit server options"), so the check runs only for files that target OpenCode 1.
**Detection**: Warning at each mistyped option when the file targets OpenCode 1
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-014"></a>
### OC-CFG-014 [MEDIUM] Invalid subagent_depth Value
**Requirement**: `subagent_depth` (OpenCode 1) and `experimental.subagent_depth` (OpenCode 2) MUST be non-negative integers: "The default is `1` ... or `0` to prevent all subagent launches." OpenCode 2 ignores the top-level key ("Top-level subagent_depth: use experimental.subagent_depth instead").
**Detection**: Warning at an invalid value; the top-level key only when the file targets OpenCode 1
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-015"></a>
### OC-CFG-015 [MEDIUM] Shell-Style Variable Not Expanded
**Requirement**: "Use {env:NAME} for environment substitution. Shell expressions such as $NAME are not expanded in JSON strings" - the literal text is sent.
**Detection**: Warning at strings with `$NAME` or `${NAME}` (all-caps names) in MCP `environment`, `headers`, `url` and `oauth`, and provider `options`/`settings`/`headers`. Command arrays are not checked (a shell may expand them).
**Fix**: No auto-fix (write `{env:NAME}`)
**Source**: opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-017"></a>
### OC-CFG-017 [MEDIUM] Mixed OpenCode 1 and 2 Fields in One Entry
**Requirement**: "Nested mixing is intentionally bounded. OpenCode recognizes mixed V1 and V2 members within mcp, compaction, and experimental, but it does not recursively infer formats inside individual agents, providers, commands, or models. Keep each of those nested entries entirely in one format." / "Do not use legacy top-level fields such as temperature, top_p, prompt, permission, tools, disable, or maxSteps in new V2 agent configuration."
**Detection**: Warning when one agent (JSON or Markdown), command, provider or model entry has members of both shapes (agent: `prompt`, `permission`, `tools`, `disable`, `maxSteps`, `variant`, `temperature`, `top_p`, `options` with `system`, `permissions`, `disabled`, `request`; command: `variant` with `subagent`; provider: `api`, `id`, `npm`, `whitelist`, `blacklist`, `options` with `canonical`, `package`, `compatibility`, `settings`, `headers`, `body`; model: `id`, `release_date`, `tool_call`, `modalities`, `options`, `status` with `modelID`, `capabilities`, `settings`, `releaseDate`, `disabled`)
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/migrate-v1

<a id="oc-cfg-018"></a>
### OC-CFG-018 [LOW] Reference Without Description
**Requirement**: "OpenCode includes references with descriptions in agent context. ... References without descriptions remain available through autocomplete and direct use, but are not advertised to agents."
**Detection**: Info at a `references` (or `reference`) entry that is a string shorthand or an object without a non-empty `description`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/references, opencode.ai/v2/docs/references

<a id="oc-cfg-019"></a>
### OC-CFG-019 [LOW] Bare Reference Shorthand Read as Git Repository
**Requirement**: "A shorthand string is a local path only when it starts with ., /, or ~. Use ./docs, not docs; a bare docs value is interpreted as a Git repository."
**Detection**: Info at a string reference that does not start with `.`, `/` or `~` and contains no `/` or `:` (so it is not `owner/repo`, a host path or a Git URL)
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/references

<a id="oc-cfg-020"></a>
### OC-CFG-020 [LOW] Many Enabled MCP Servers
**Requirement**: "When you use an MCP server, it adds to the context. This can quickly add up if you have a lot of tools." / "MCP tools consume model context, so add only the servers you need." The threshold of 8 enabled servers is an agnix heuristic; OpenCode gives no number.
**Detection**: Info at `mcp` when more than 8 servers in one file are enabled (`enabled: false` / `disabled: true` servers are not counted)
**Fix**: No auto-fix
**Source**: opencode.ai/docs/mcp-servers, opencode.ai/v2/docs/mcp-servers

<a id="oc-cfg-021"></a>
### OC-CFG-021 [MEDIUM] OpenCode 1 Field in an mcp.servers Entry
**Requirement**: Entries under `mcp.servers` use the OpenCode 2 fields: "Use disabled, not an enabled field, to keep one configured without connecting it", timeouts are objects ("separates timeout purposes"), and "Remote OAuth fields use snake case: clientId becomes client_id, clientSecret becomes client_secret, callbackPort becomes callback_port, and redirectUri becomes redirect_uri." (`Mcp.*ConfigEncoded`, `additionalProperties: false`)
**Detection**: Warning at `enabled`, a numeric `timeout`, or a camelCase OAuth member of an `mcp.servers` entry
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/mcp-servers, opencode.ai/v2/docs/migrate-v1

<a id="oc-cfg-022"></a>
### OC-CFG-022 [MEDIUM] Global-Only Setting in Project Config
**Requirement**: "Control update checks from the global config. ... Project-level values are ignored." (`update`)
**Detection**: Warning at `update` in a config outside the global (`~/.config/opencode`) and system directories
**Fix**: No auto-fix (move it to the global config)
**Source**: opencode.ai/v2/docs/config

<a id="oc-ag-001"></a>
### OC-AG-001 [HIGH] Invalid Agent Mode Value
**Requirement**: Agent `mode` MUST be `primary`, `subagent` or `all` ("The `mode` option can be set to `primary`, `subagent`, or `all`.")
**Detection**: Error at the value in `agent`, `agents`, `mode` entries and Markdown agent frontmatter
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents, opencode.ai/v2/docs/agents

<a id="oc-ag-002"></a>
### OC-AG-002 [HIGH] Invalid Color Format
**Requirement**: Hex agent colors MUST be six-digit `#RRGGBB` (schema pattern `^#[0-9a-fA-F]{6}$`; "color sets the agent's UI color using a six-digit hex value"); native OpenCode 2 `agents` entries accept only hex.
**Detection**: Error for a `#` value that is not six hex digits (three-digit values were accepted before), and for a non-hex value in an `agents` entry. Theme names in OpenCode 1 entries are OC-AG-006.
**Fix**: No auto-fix
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/agents

<a id="oc-ag-003"></a>
### OC-AG-003 [MEDIUM] Temperature Out of Range
**Requirement**: `temperature` (OpenCode 1) MUST be a number. OpenCode documents "Temperature values typically range from 0.0 to 1.0"; the 0-2 bound is an agnix heuristic (provider limits), so out-of-range values are warnings (formerly errors).
**Detection**: Warning for a non-number or a value outside 0-2 in OpenCode 1 agent entries and Markdown agents
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents

<a id="oc-ag-004"></a>
### OC-AG-004 [HIGH] Steps Not a Positive Integer
**Requirement**: `steps` MUST be a positive integer ("steps sets a positive maximum number of model steps"; schema `exclusiveMinimum: 0`)
**Detection**: Error at the value
**Fix**: No auto-fix
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/agents

<a id="oc-ag-005"></a>
### OC-AG-005 [HIGH] top_p Out of Range
**Requirement**: `top_p` (OpenCode 1) MUST be within range: "Values range from 0.0 to 1.0."
**Detection**: Error at a non-number or a value outside 0-1
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents

<a id="oc-ag-006"></a>
### OC-AG-006 [MEDIUM] Invalid Named Color
**Requirement**: Named colors SHOULD be theme colors: "Use a valid hex color (e.g., `#FF5733`) or theme color: `primary`, `secondary`, `accent`, `success`, `warning`, `error`, `info`."
**Detection**: Warning at a non-hex color that is not a theme color, in OpenCode 1 entries and Markdown agents
**Fix**: Auto-fix (unsafe) - replace with the closest theme color
**Source**: opencode.ai/docs/agents

<a id="oc-ag-007"></a>
### OC-AG-007 [MEDIUM] Redundant steps and maxSteps
**Requirement**: "The legacy `maxSteps` field is deprecated. Use `steps` instead."
**Detection**: Warning at `maxSteps` when `steps` is also set
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents

<a id="oc-ag-008"></a>
### OC-AG-008 [HIGH] Invalid hidden Type
**Requirement**: `hidden` MUST be a boolean (schema `type: boolean`)
**Detection**: Error at a non-boolean value
**Fix**: No auto-fix
**Source**: opencode.ai/config.json

<a id="oc-ag-009"></a>
### OC-AG-009 [HIGH] Invalid Agent disable or disabled Type
**Requirement**: `disable` (OpenCode 1) and `disabled` (OpenCode 2) MUST be booleans (schema `type: boolean`)
**Detection**: Error at a non-boolean value
**Fix**: No auto-fix
**Source**: opencode.ai/config.json

<a id="oc-ag-010"></a>
### OC-AG-010 [MEDIUM] OpenCode Agent Without Description
**Requirement**: Markdown agents (`.opencode/agents/**.md`, `~/.config/opencode/agents/**.md`) SHOULD have a description: "description explains the agent's purpose. Add it to subagents because OpenCode shows it to the model choosing which agent to launch" (OpenCode 2); "This is a **required** config option." (OpenCode 1). "The markdown file name becomes the agent name"; nested paths become part of the ID.
**Detection**: Warning on a Markdown agent without `description` whose mode is `subagent` or `all`, or whose mode is omitted when the file also targets OpenCode 1 (default `all`; OpenCode 2 defaults to `primary`). These files are an OpenCode agent file type: the OpenCode agent field checks (OC-AG-*, OC-008, OC-PM-*, OC-DEP-002, OC-CFG-001, OC-CFG-017) run on the frontmatter, and the Claude Code subagent rules (CC-AG-*) no longer do. Files in `mode/`/`modes/` are primary agents and are not reported.
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/agents, opencode.ai/docs/agents

<a id="oc-ag-011"></a>
### OC-AG-011 [LOW] hidden on a Non-Subagent
**Requirement**: OpenCode 1: "Hide a subagent from the `@` autocomplete menu with `hidden: true`." ... "Only applies to `mode: subagent` agents." (OpenCode 2 hides any agent.)
**Detection**: Info at `hidden: true` on an agent whose mode is not `subagent`, when the file targets OpenCode 1 (not for native `agents` entries)
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents

<a id="oc-pm-002"></a>
### OC-PM-002 [LOW] Possibly Misspelled Permission Key
**Requirement**: "Permission keys are matched as wildcard patterns against the underlying tool name, so the same syntax works for built-ins, custom tools, and MCP tools - for example `\"mymcp_*\": \"deny\"`". Keys are not limited to built-in names, so only near misses of built-in keys are reported (every non-built-in key was a warning before, including `*`).
**Detection**: Info at an OpenCode 1 permission key without wildcards that is within a small edit distance of a built-in key (`raed` for `read`)
**Fix**: No auto-fix
**Source**: opencode.ai/docs/agents

<a id="oc-pm-003"></a>
### OC-PM-003 [MEDIUM] Permission Rule Shadowed by Later Catch-All
**Requirement**: "Rules are evaluated by pattern match, with the **last matching rule winning**. A common pattern is to put the catch-all `\"*\"` rule first, and more specific rules after it." / "The last matching rule wins, so the specific exceptions follow the broad rule." / policies: "When several statements match, the last one wins."
**Detection**: Warning at a `"*"` key of an OpenCode 1 permission object (top level, per tool, per agent) placed after keys with another action, and at an OpenCode 2 `permissions` rule or policy statement with resource `*` (and the same action, or action `*`) placed after rules of that action with another effect
**Fix**: No auto-fix (move the catch-all first)
**Source**: opencode.ai/docs/permissions, opencode.ai/v2/docs/permissions

<a id="oc-pm-004"></a>
### OC-PM-004 [MEDIUM] Broad external_directory Allow
**Requirement**: "Keep the list focused on trusted paths" (OpenCode 1) / "Explicit external paths are canonicalized before matching, so authorize only trusted directory boundaries." (OpenCode 2)
**Detection**: Warning at `external_directory: "allow"` or an allow pattern covering everything or the home directory (`*`, `**`, `/`, `/**`, `~`, `~/**`, `$HOME/**`, ...), in both shapes
**Fix**: No auto-fix
**Source**: opencode.ai/docs/permissions, opencode.ai/v2/docs/permissions

<a id="oc-pm-005"></a>
### OC-PM-005 [MEDIUM] OpenCode 1 Action Name in a permissions Array
**Requirement**: "Permission actions also changed: bash is now shell, task is now subagent, and write and patch are now edit." / "doom_loop and lsp are not current V2 Core permission actions."
**Detection**: Warning at an OpenCode 2 `permissions` rule whose action is `bash`, `task`, `write`, `patch`, `doom_loop` or `lsp`
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/migrate-v1

<a id="oc-pm-006"></a>
### OC-PM-006 [LOW] Shell Allow-All With Deny Patterns
**Requirement**: "shell runs with the host user's filesystem, process, and network authority. Directory inference from command text is best effort, so prefer a narrow shell allowlist instead of patterns intended to recognize every dangerous command."
**Detection**: Info at `bash: {"*": "allow", ...}` with deny patterns (OpenCode 1) and at a shell `*` allow rule followed by shell deny rules (OpenCode 2)
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/permissions

<a id="oc-pm-007"></a>
### OC-PM-007 [LOW] Shell Pattern Without Argument Wildcard
**Requirement**: "Use pattern matching for commands with arguments. `\"grep *\"` allows `grep pattern file.txt`, while `\"grep\"` alone would block it." (OpenCode 2: "A shell pattern ending in * also matches the command without arguments.")
**Detection**: Info at an allowed `bash` pattern (OpenCode 1) or shell allow rule resource (OpenCode 2) without `*` or `?`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/permissions

<a id="oc-pol-001"></a>
### OC-POL-001 [HIGH] Invalid Policy Statement
**Requirement**: "Each statement has three fields" (`action` provider.use, tool.use or integration.use; `resource`; `effect` allow or deny) and "Statements that fail validation are dropped with a warning in the server log and the rest of the list still applies." `tool.use` resources are `<action>:<value>`; `integration.use` resources are `mcp:<server>`, `skill:<id>` or `plugin:<target>`. OpenCode 1 `experimental.policies`: "Currently, policies can control which providers OpenCode may use."
**Detection**: Error for a statement OpenCode drops (non-object, unknown member, invalid action or effect, non-string resource) in `policies` or `experimental.policies`; warning for a `tool.use`/`integration.use` resource without its prefix form (it matches nothing); error for a non-provider.use statement in `experimental.policies` when OpenCode 1 is pinned
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/policies

<a id="oc-cmd-001"></a>
### OC-CMD-001 [MEDIUM] Command Argument Inside Shell Block
**Requirement**: "Do not place untrusted arguments inside shell blocks." / "Shell blocks run when OpenCode evaluates the command, outside the agent's tool permission flow. Only use commands from sources you trust."
**Detection**: Warning at each ``!`...` `` block containing `$ARGUMENTS` or `$N` in Markdown command bodies and JSON command templates
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/commands

<a id="oc-cmd-002"></a>
### OC-CMD-002 [HIGH] JSON Command Without Template
**Requirement**: JSON commands MUST have a template: "This is a **required** config option." / "Each command requires a template." (schema `required: ["template"]`)
**Detection**: Error for a `command`/`commands` entry that is not an object or has no string `template`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/commands, opencode.ai/v2/docs/commands

<a id="oc-cmd-003"></a>
### OC-CMD-003 [LOW] template in Markdown Command Frontmatter
**Requirement**: "Do not put template in Markdown frontmatter; the body supplies it." / "The trimmed Markdown body is always the prompt template."
**Detection**: Info at `template` in the frontmatter of `.opencode/commands/**.md` (or `~/.config/opencode/commands/`)
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/commands

<a id="oc-dep-001"></a>
### OC-DEP-001 [MEDIUM] Deprecated mode Map
**Requirement**: The `mode` map is deprecated (schema: "@deprecated Use `agent` field instead."; "The singular agent and deprecated mode maps become agents." ... "Entries from the old mode map become primary agents.")
**Detection**: Warning at `mode`
**Fix**: Auto-fix (unsafe, formerly safe) - rename `mode` to `agent` when every entry is an object and `agent` is absent; entries then default to mode `all`
**Source**: opencode.ai/config.json, opencode.ai/v2/docs/migrate-v1

<a id="oc-dep-002"></a>
### OC-DEP-002 [MEDIUM] Deprecated tools Field
**Requirement**: "As of `v1.1.1`, the legacy `tools` boolean config is deprecated and has been merged into `permission`. The old `tools` config is still supported for backwards compatibility." / "`tools` is **deprecated**. Prefer the agent's `permission` field"
**Detection**: Warning at top-level `tools` and at agent-level `tools` in JSON agents and Markdown agent frontmatter (agent level was not checked before)
**Fix**: No auto-fix (booleans map to permission rules; a key rename would be invalid)
**Source**: opencode.ai/docs/permissions, opencode.ai/docs/agents

<a id="oc-dep-003"></a>
### OC-DEP-003 [MEDIUM] Deprecated autoshare Field
**Requirement**: `autoshare` is deprecated (schema: "@deprecated Use 'share' field instead. Share newly created sessions automatically")
**Detection**: Warning at `autoshare`
**Fix**: Auto-fix (unsafe) - replace `"autoshare": true|false` with `"share": "auto"|"manual"` (the old safe fix renamed the key only and left a boolean `share`)
**Source**: opencode.ai/config.json

<a id="oc-dep-005"></a>
### OC-DEP-005 [MEDIUM] Deprecated TUI Keys
**Requirement**: "Legacy `theme`, `keybinds`, and `tui` keys in `opencode.json` are deprecated and automatically migrated when possible." TUI settings belong in `tui.json` (OpenCode 2: `~/.config/opencode/cli.json`).
**Detection**: Warning at each legacy key
**Fix**: Auto-fix (unsafe) - remove the legacy keys after migration
**Source**: opencode.ai/docs/config

<a id="oc-dep-006"></a>
### OC-DEP-006 [MEDIUM] Deprecated maxSteps Field
**Requirement**: "The legacy `maxSteps` field is deprecated. Use `steps` instead."
**Detection**: Warning at `maxSteps` without `steps` (JSON agents and Markdown agents)
**Fix**: Auto-fix (unsafe, formerly safe) - rename to `steps`
**Source**: opencode.ai/docs/agents

<a id="oc-dep-007"></a>
### OC-DEP-007 [MEDIUM] Deprecated reference Field
**Requirement**: "Rename the deprecated singular reference map to references" ... "Reference entries keep the same shapes." (schema: "@deprecated Use 'references' field instead.")
**Detection**: Warning at `reference`
**Fix**: Auto-fix (unsafe, formerly safe) - rename to `references` when absent
**Source**: opencode.ai/v2/docs/migrate-v1

<a id="oc-dep-008"></a>
### OC-DEP-008 [LOW] Legacy Singular .opencode Subdirectory
**Requirement**: "The `.opencode` and `~/.config/opencode` directories use **plural names** for subdirectories ... Singular names (e.g., `agent/`) are also supported for backwards compatibility." / "The legacy singular directories command/ are also discovered, but use commands/ for new files." / "Files under a V1 mode/ or modes/ directory represent primary agents. When moving one into agents/, add mode: primary to its frontmatter."
**Detection**: Info on files in `agent/`, `command/`, `tool/`, `plugin/`, `skill/`, `mode/` or `modes/` under `.opencode/` or the global or system config directory
**Fix**: No auto-fix (move the file)
**Source**: opencode.ai/docs/config, opencode.ai/v2/docs/migrate-v1

<a id="oc-dep-009"></a>
### OC-DEP-009 [MEDIUM] Field Ignored by OpenCode 2
**Requirement**: "The V1 schema also accepted fields that have no supported V2 behavior. V2 ignores these values and emits a warning so they are not mistaken for active configuration": `logLevel`, `server`, top-level `subagent_depth`, `compaction.tail_turns` and `compaction.prune`, agent `name` in JSON, an enabled-only MCP entry without a type, experimental `batch_tool`, `openTelemetry`, `primary_tools`, `continue_loop_on_deny`, provider `id`, `whitelist`, `blacklist`, and provider-model `release_date`, `attachment`, `reasoning`, `temperature`, `experimental`, a non-deprecated `status`, and boolean `interleaved`.
**Detection**: Warning at each such field when the file targets OpenCode 2 only (it uses native OpenCode 2 fields or `[tool_versions] opencode` pins 2.x). Unpinned OpenCode 1 configuration is not reported.
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/migrate-v1

<a id="oc-lsp-001"></a>
### OC-LSP-001 [MEDIUM] LSP Command Without Extensions
**Requirement**: Custom LSP servers SHOULD list the file extensions they handle (`extensions`). OpenCode 2 "accepts and preserves lsp configuration, but it does not run language servers", so the check runs only for files that target OpenCode 1.
**Detection**: Warning at an `lsp` entry with `command` and no `extensions`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/lsp

<a id="oc-lsp-002"></a>
### OC-LSP-002 [HIGH] Invalid LSP Extensions
**Requirement**: `extensions` MUST be a non-empty array of strings (schema `type: array, items: string`). Checked only for files that target OpenCode 1.
**Detection**: Error at the value
**Fix**: No auto-fix
**Source**: opencode.ai/docs/lsp

<a id="oc-tui-001"></a>
### OC-TUI-001 [MEDIUM] Unknown TUI Key
**Requirement**: Keys of the legacy `tui` object SHOULD be TUI settings (`theme`, `keybinds`, `mouse`, `scroll_speed`, `scroll_acceleration`, `diff_style`)
**Detection**: Warning at each other key
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-tui-002"></a>
### OC-TUI-002 [HIGH] Invalid scroll_speed
**Requirement**: `tui.scroll_speed` MUST be a number of at least 0.001
**Detection**: Error at the value
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-tui-003"></a>
### OC-TUI-003 [HIGH] Invalid diff_style
**Requirement**: `tui.diff_style` MUST be `auto` or `stacked`
**Detection**: Error at the value
**Fix**: Auto-fix (unsafe) - replace with the closest style
**Source**: opencode.ai/docs/config

<a id="oc-agm-002"></a>
### OC-AGM-002 [MEDIUM] Secrets in AGENTS.md
**Requirement**: AGENTS.md SHOULD NOT contain credentials: "Commit project instruction files so everyone working in the repository receives the same guidance." / "You should commit your project's `AGENTS.md` file to Git." OpenCode does not reject such a file (formerly HIGH/MUST).
**Detection**: Warning at each line of an AGENTS.md in an OpenCode project (an `opencode.json(c)` or `.opencode/` in the file's directory or an ancestor) or in the global config directory that contains a known credential prefix (`sk-ant-`, `sk-proj-`, `xoxb-`, `xoxp-`, `AKIA`, `AIza` (formerly the never-matching `AIZA`), `ghp_`, `gho_`, `github_pat_`); it no longer runs on every AGENTS.md
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/instructions, opencode.ai/docs/rules

<a id="oc-agm-003"></a>
### OC-AGM-003 [MEDIUM] CLAUDE.md Ignored by OpenCode 2
**Requirement**: "OpenCode V2 recognizes AGENTS.md only. It does not use CLAUDE.md as a fallback." / "If a V1 setup relied on a CLAUDE.md fallback, move that guidance into the applicable AGENTS.md." (OpenCode 1: "`CLAUDE.md` in your project directory (used if no `AGENTS.md` exists)")
**Detection**: For a CLAUDE.md with no AGENTS.md in the same directory, in an OpenCode project: warning when the project targets OpenCode 2 only (native OpenCode 2 config or a 2.x pin), info when it targets both lines, nothing when OpenCode 1 is pinned
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/instructions

<a id="oc-sk-002"></a>
### OC-SK-002 [LOW] Duplicate OpenCode Skill ID
**Requirement**: "Skills are selected by ID. If two sources define git-release, the source registered later supplies the skill that OpenCode loads" (order: `.claude/skills`, `.agents/skills`, `~/.config/opencode/skills`, project `.opencode/skills`) / "Avoid duplicate IDs unless you intend to override an earlier skill." (OpenCode 1: "Ensure skill names are unique across all locations")
**Detection**: Info on a project skill in `.claude/skills/<id>/` or `.agents/skills/<id>/` when a higher-precedence source in the same project root (`.agents/skills`, `.opencode/skills`) defines the same ID, in an OpenCode project. Mirrored copies in multi-tool repositories are common, hence LOW.
**Fix**: No auto-fix
**Source**: opencode.ai/v2/docs/skills

<a id="oc-tool-001"></a>
### OC-TOOL-001 [LOW] Custom Tool Shadows Built-in Tool
**Requirement**: "Custom tools are keyed by tool name. If a custom tool uses the same name as a built-in tool, the custom tool takes precedence." ... "Prefer unique names unless you intentionally want to replace a built-in tool." "The **filename** becomes the **tool name**" for the default export.
**Detection**: Info on `.opencode/tools/<name>.ts|js` (or `~/.config/opencode/tools/`) whose file name is a built-in tool (bash, shell, read, write, edit, patch, grep, glob, list, skill, task, subagent, webfetch, websearch, question, lsp, todowrite, todoread, execute) and that has a default export (named exports become `<file>_<export>`)
**Fix**: No auto-fix
**Source**: opencode.ai/docs/custom-tools

## GEMINI CLI RULES

<a id="gm-001"></a>
### GM-001 [HIGH] Invalid Markdown Structure in GEMINI.md
**Requirement**: GEMINI.md MUST have valid markdown (no unclosed code blocks or malformed links)
**Detection**: Parse markdown, check for unclosed ``` blocks and malformed [text]( links
**Fix**: [AUTO-FIX] No auto-fix (manual correction required)
**Source**: geminicli.com/docs/cli/gemini-md/

<a id="gm-002"></a>
### GM-002 [MEDIUM] Missing Section Headers in GEMINI.md
**Requirement**: GEMINI.md SHOULD have markdown section headers for organization
**Detection**: Scan for `^#+\s+.+` patterns, report if none found
**Fix**: No auto-fix (headers must be authored by user)
**Source**: geminicli.com/docs/cli/gemini-md/

<a id="gm-003"></a>
### GM-003 [MEDIUM] Missing Project Context in GEMINI.md
**Requirement**: GEMINI.md SHOULD include a project context section describing purpose and tech stack
**Detection**: Check for headers matching project/overview/about/description patterns or content referencing "this project"
**Fix**: No auto-fix (project context must be authored by user)
**Source**: geminicli.com/docs/cli/gemini-md/

<a id="gm-004"></a>
### GM-004 [MEDIUM] Invalid Hooks Configuration in Gemini Settings
**Requirement**: hooksConfig in .gemini/settings.json MUST use valid event names and hook structure
**Detection**: Parse hooksConfig object, validate event names against known set, check required fields (type, command)
**Fix**: No auto-fix (manual correction required)
**Source**: geminicli.com/docs/hooks

<a id="gm-005"></a>
### GM-005 [HIGH] Invalid Extension Manifest
**Requirement**: gemini-extension.json MUST have valid JSON with required fields (name, version, description)
**Detection**: Parse JSON, check required fields exist and are non-empty strings, validate name format
**Fix**: No auto-fix (manual correction required)
**Source**: geminicli.com/docs/extensions/reference

<a id="gm-006"></a>
### GM-006 [LOW] Invalid .geminiignore File
**Requirement**: .geminiignore MAY have valid gitignore-style patterns
**Detection**: Check for empty content and unmatched brackets in glob patterns
**Fix**: No auto-fix (manual correction required)
**Source**: geminicli.com/docs/cli/settings

<a id="gm-007"></a>
### GM-007 [MEDIUM] @import File Not Found in GEMINI.md
**Requirement**: @import directives in GEMINI.md SHOULD reference existing files
**Detection**: Scan for @import lines, resolve paths relative to GEMINI.md, check file existence
**Fix**: No auto-fix (create the file or fix the path)
**Source**: geminicli.com/docs/cli/gemini-md/

<a id="gm-008"></a>
### GM-008 [LOW] Invalid Context File Name Configuration
**Requirement**: contextFileName in gemini-extension.json MAY reference a valid filename
**Detection**: Check if contextFileName contains path separators (should be a filename only)
**Fix**: [AUTO-FIX] No auto-fix (manual correction required)
**Source**: geminicli.com/docs/extensions/reference

<a id="gm-009"></a>
### GM-009 [HIGH] Settings.json Parse Error
**Requirement**: .gemini/settings.json MUST have valid JSON/JSONC syntax
**Detection**: Attempt to parse as JSONC; report parse errors with line/column. Detect unknown top-level keys.
**Fix**: [AUTO-FIX] No auto-fix (correct the JSON syntax)
**Source**: geminicli.com/docs/cli/settings

<a id="gm-010"></a>
### GM-010 [MEDIUM] memoryManager Without autoMemory After v0.40 Split
**Requirement**: Users who set `experimental.memoryManager: true` SHOULD also set `experimental.autoMemory: true` on Gemini CLI v0.40+
**Detection**: Parse `.gemini/settings.json`; warn when `experimental.memoryManager === true` and `experimental.autoMemory` is missing or false
**Fix**: No auto-fix (upstream declined to ship a migration shim - users may legitimately want only the subagent)
**Rationale**: Gemini CLI v0.40 (PR #25601) split the combined `memoryManager` flag. Pre-v0.40 it gated both the Memory Manager subagent and background skill extraction + `/memory inbox`. Post-v0.40 `memoryManager` gates only the subagent; extraction and the inbox move to the new `autoMemory` flag. Users carrying forward only `memoryManager: true` lose the inbox silently.
**Source**: github.com/google-gemini/gemini-cli/pull/25601

## GEMINI CLI AGENT RULES

Rules for local Gemini agent markdown files at `.gemini/agents/*.md`. These define `kind: local` agents with YAML frontmatter containing `name`, `description`, `tools`, `mcp_servers`, and `system_prompt`. Schema documented in the gemini-cli source at `packages/core/src/agents/agentLoader.ts`.

<a id="gm-ag-001"></a>
### GM-AG-001 [HIGH] Invalid auth block in Gemini agent MCP server
**Requirement**: The `auth` block inside `mcp_servers.<name>` MUST follow the schema added in gemini-cli v0.39.0 (google-gemini/gemini-cli#24770):
- Variant `type: "google-credentials"` - only `scopes` (optional array of strings) is accepted
- Variant `type: "oauth"` - `client_id`, `client_secret`, `scopes`, `authorization_url`, `token_url` are all optional; URLs must be valid http(s); scopes must be an array of strings
**Detection**: Parse YAML frontmatter; walk `mcp_servers.*.auth`; enforce discriminator, reject unknown fields per variant, type-check string/array values, validate URL shape for `authorization_url` and `token_url`
**Fix**: Manual - align the auth block with the documented variant schema
**Source**: google-gemini/gemini-cli#24770 (gemini-cli v0.39.0+)

---

## CODEX CLI RULES

<a id="cdx-000"></a>
### CDX-000 [HIGH] TOML Parse Error
**Requirement**: Codex config.toml files MUST have valid TOML syntax
**Detection**: Attempt to parse as TOML; report parse errors with line/column
**Fix**: Correct the TOML syntax
**Source**: learn.chatgpt.com/docs/config-file/config-basic, learn.chatgpt.com/docs/config-file/config-reference


<a id="cdx-004"></a>
### CDX-004 [MEDIUM] Unknown Config Key
**0.154 schema refresh**: Accept `allow_symlinked_codex_home` and `thread_unload_delay_secs` added by the tagged Codex 0.154.0 schema.
**Requirement**: Top-level keys in `.codex/config.toml` SHOULD be from the current configuration schema (`config-schema.json`, top-level `additionalProperties: false`); Codex ignores unknown keys
**Detection**: Parse TOML, compare top-level keys against known key and table allowlists. `approvalMode` and `fullAutoErrorMode` are settings of the retired TypeScript CLI that the current CLI ignores, and get a message saying so. `allow_managed_hooks_only` gets a message pointing to requirements.toml: "This setting is only supported in `requirements.toml`; putting it in `config.toml` does not enable managed-hooks-only mode."
**Fix**: [AUTO-FIX, unsafe] Remove unrecognized keys
**Source**: learn.chatgpt.com/docs/config-file/config-reference, learn.chatgpt.com/docs/config-schema.json, learn.chatgpt.com/docs/hooks

<a id="cdx-005"></a>
### CDX-005 [HIGH] Invalid project_doc_max_bytes
**Requirement**: `project_doc_max_bytes` in `.codex/config.toml` MUST be a non-negative integer. Codex `config.schema.json` specifies minimum 0 and no maximum (default 32768). Zero is valid.
**Detection**: Parse TOML, flag a non-integer or negative `project_doc_max_bytes`
**Fix**: Set a non-negative integer (default: 32768)
**Source**: developers.openai.com/codex/config-schema.json

<a id="cdx-006"></a>
### CDX-006 [HIGH] Invalid project_doc_fallback_filenames
**Requirement**: `project_doc_fallback_filenames` in `.codex/config.toml` MUST be an array of unique, non-empty filename strings
**Detection**: Parse TOML, validate array type, ensure all entries are non-empty strings, flag duplicate and path-like entries
**Fix**: Use a unique array of bare filenames (e.g., `["AGENTS.md", "README.md"]`)
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md/

<a id="cdx-cfg-001"></a>
### CDX-CFG-001 [HIGH] Invalid approval_policy Value
**Requirement**: `approval_policy` MUST be `"on-request"`, `"never"`, or an object with exactly one outer key, `granular`. The nested `approval_policy.granular` object requires boolean `mcp_elicitations`, `rules`, and `sandbox_approval`; optional `request_permissions` and `skill_approval` must also be booleans. This union is identical in the Codex `rust-v0.163.0` and `main` schemas. `untrusted` is a project `trust_level`, not an approval policy.
**Detection**: Parse `.codex/config.toml|json|yaml`; reject other strings, non-object/non-string values, a missing or non-object `granular` wrapper, missing required booleans, and wrong known-field types. Unknown nested keys receive advisory CDX-CFG-023 rather than an invented schema rejection.
**Fix**: No auto-fix (set to `on-request`, `never`, or a granular object)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-002"></a>
### CDX-CFG-002 [HIGH] Invalid sandbox_mode Value
**Requirement**: `sandbox_mode` in Codex config MUST be `read-only`, `workspace-write`, or `danger-full-access`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `sandbox_mode` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-003"></a>
### CDX-CFG-003 [HIGH] Invalid model_reasoning_effort Value
**Requirement**: `model_reasoning_effort` in Codex config MUST be a non-empty string. Codex rust-v0.138.0 replaced the fixed `none|minimal|low|medium|high|xhigh` enum with model-defined reasoning efforts - the upstream schema accepts any non-empty effort the model advertises (openai/codex#26444), so unknown values are no longer flagged.
**Detection**: Parse `.codex/config.toml|json|yaml`; flag non-string types and empty strings only
**Fix**: No auto-fix (set to an effort the model advertises; standard values: `none`, `minimal`, `low`, `medium`, `high`, `xhigh`)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json, github.com/openai/codex/releases/tag/rust-v0.138.0

<a id="cdx-cfg-004"></a>
### CDX-CFG-004 [HIGH] Invalid model_verbosity Value
**Requirement**: `model_verbosity` in Codex config MUST be one of `low|medium|high`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `model_verbosity` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-005"></a>
### CDX-CFG-005 [HIGH] Invalid personality Value
**Requirement**: `personality` in Codex config MUST be one of `none|friendly|pragmatic`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `personality` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-006"></a>
### CDX-CFG-006 [MEDIUM] Unknown Codex Config Field
**0.154 schema refresh**: Accept the new `tui.question_esc_back` and `tui.whimsy` preferences and the feature names listed under CDX-CFG-011.
**Requirement**: Codex config keys SHOULD match the official schema at top-level and known nested sections (`features`, `tui`, `shell_environment_policy`, `mcp_servers`, `apps`)
**Detection**: Parse `.codex/config.toml|json|yaml`, compare observed keys against allowlists, and report unknown keys
**Fix**: No auto-fix (remove or rename unsupported fields)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-007"></a>
### CDX-CFG-007 [MEDIUM] Full Access Sandbox Posture
**Requirement**: A Codex config SHOULD NOT remove the sandbox outside a controlled environment: "`danger-full-access`: The agent runs without sandbox restrictions. This removes the filesystem and network boundaries and should be used only when you want the agent to act with full access." and "Use `danger-full-access` only in a controlled environment (for example, an isolated CI runner or container)." Combined with `approval_policy = "never"` the agents page marks the posture "No sandbox; no approvals _(not recommended)_".
**Detection**: Warning on `sandbox_mode = "danger-full-access"` or `default_permissions = ":danger-full-access"`, at the key; the message names the no-approvals combination when `approval_policy = "never"`. `notice.hide_full_access_warning` no longer suppresses the rule: the config reference defines it as "Track acknowledgement of the full access warning prompt.", UI state rather than an author acknowledgment. Use agnix disable comments or config for intentional CI configs.
**Fix**: No auto-fix (use workspace-write or a permission profile)
**Source**: learn.chatgpt.com/docs/sandboxing, learn.chatgpt.com/docs/agent-approvals-security, learn.chatgpt.com/docs/non-interactive-mode, learn.chatgpt.com/docs/config-file/config-reference

<a id="cdx-cfg-008"></a>
### CDX-CFG-008 [HIGH] Invalid shell_environment_policy Value
**Requirement**: `shell_environment_policy.inherit` MUST be one of `core|all|none`; `filters` MUST map patterns to `include|exclude` and MUST NOT coexist with legacy `exclude` or `include_only`
**Detection**: Parse config and validate shell environment inheritance, filter actions, and mutually exclusive fields
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, github.com/openai/codex (codex-rs/core/config.schema.json @ rust-v0.146.0)

<a id="cdx-cfg-009"></a>
### CDX-CFG-009 [HIGH] Invalid MCP Server Structure in Codex Config
**Requirement**: Each `mcp_servers.<name>` entry MUST be an object and MUST define at least one transport (`command` or `url`)
**Detection**: Parse config and validate `mcp_servers` object shape and per-server transport presence
**Fix**: No auto-fix (add `command` or `url`, or fix object shape)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-010"></a>
### CDX-CFG-010 [HIGH] Hardcoded Secret in Codex Config
**Requirement**: Codex config MUST NOT hardcode credentials (API keys/tokens/secrets/passwords)
**Detection**: Scan config string values and key names for hardcoded secret patterns while allowing environment variable references
**Fix**: No auto-fix (replace with environment variable references)
**Source**: developers.openai.com/codex/config-reference

<a id="cdx-cfg-011"></a>
### CDX-CFG-011 [MEDIUM] Invalid Feature Flag Name or Shape
**0.154 schema refresh**: Accept `reasoning_effort_override`, `unified_exec_tty`, `windows_sandbox_service`, and `worktrees`. Source: [tagged configuration schema](https://github.com/openai/codex/blob/rust-v0.154.0/codex-rs/core/config.schema.json).
**Requirement**: Keys under `[features]` SHOULD use known Codex feature flag names; `non_prefixed_mcp_tool_names` MUST be a boolean or an object with optional boolean `enabled` and string-array `server_names`
**Detection**: Parse config, report unknown keys under `features`, and validate the structured `non_prefixed_mcp_tool_names` value
**Fix**: No auto-fix (remove unsupported flags or rename)
**Source**: developers.openai.com/codex/config-reference, github.com/openai/codex (codex-rs/core/config.schema.json @ rust-v0.146.0)

<a id="cdx-cfg-012"></a>
### CDX-CFG-012 [HIGH] Invalid cli_auth_credentials_store Value
**Requirement**: `cli_auth_credentials_store` MUST be one of `file|keyring|auto|ephemeral`
**Detection**: Parse config and validate `cli_auth_credentials_store` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-ag-001"></a>
### CDX-AG-001 [HIGH] Empty AGENTS.md for Codex
**Requirement**: AGENTS.md used by Codex MUST contain actionable project guidance: "Ensure instruction files contain content; Codex ignores empty files."
**Detection**: For `AGENTS.md` and `AGENTS.override.md` (the names Codex discovers; `AGENTS.local.md` is not one of them), flag files where `content.trim().is_empty()`
**Fix**: No auto-fix (add repository-specific instructions)
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="cdx-ag-002"></a>
### CDX-AG-002 [MEDIUM] Secrets in AGENTS.md for Codex
**Requirement**: AGENTS.md SHOULD NOT include hardcoded credentials or tokens. Codex documents AGENTS.md as project guidance "that travels with your repository", so anything in it is shared with everyone who clones the repository. Codex itself does not reject such a file (formerly HIGH/MUST).
**Detection**: Scan AGENTS.md and AGENTS.override.md lines for secret markers/prefixes and credential-like assignments
**Fix**: No auto-fix (remove secrets and use environment variables)
**Source**: learn.chatgpt.com/docs/customization/overview, learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="cdx-ag-003"></a>
### CDX-AG-003 [LOW] Generic AGENTS.md Guidance for Codex
**Requirement**: AGENTS.md SHOULD provide specific, actionable repo guidance instead of vague rules: "Keep it practical. A short, accurate AGENTS.md is more useful than a long file full of vague rules." The vague-rule phrases (follow best practices or coding standards, write clean code, keep code clean, be careful, use common sense) are an agnix heuristic; Codex publishes no list. The rule no longer reuses the Claude memory list of model-behavior phrases such as "be helpful" or "think step by step".
**Detection**: A short file (under 120 characters) with a vague-rule phrase and no inline code, paths or flags. Info level.
**Fix**: No auto-fix (replace vague rules with concrete repository guidance)
**Source**: learn.chatgpt.com/guides/best-practices

<a id="cdx-app-001"></a>
### CDX-APP-001 [HIGH] Invalid Tool Approval Mode
**Requirement**: Tool approval modes MUST be valid: "Supported values are `auto`, `prompt`, `writes`, and `approve`. The `writes` mode prompts for tools that aren't marked read-only." (schema `AppToolApproval`). `writes` was formerly rejected.
**Detection**: `default_tools_approval_mode` and `tools.<tool>.approval_mode` under `apps.<id>`, `apps._default`, `mcp_servers.<id>` and `plugins.<plugin>.mcp_servers.<id>`; reported at the value
**Fix**: No auto-fix (set to a supported value)
**Source**: learn.chatgpt.com/docs/extend/mcp, learn.chatgpt.com/docs/config-file/config-reference, learn.chatgpt.com/docs/config-schema.json

<a id="cdx-cfg-014"></a>
### CDX-CFG-014 [MEDIUM] Invalid model Value
**Requirement**: `model` SHOULD be a string
**Detection**: Parse config and verify `model` is a string type
**Fix**: No auto-fix (set to a valid model name string)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-015"></a>
### CDX-CFG-015 [HIGH] Invalid model_provider Value
**Requirement**: `model_provider` MUST be a string
**Detection**: Parse config and verify `model_provider` is a string type
**Fix**: No auto-fix (set to a valid provider string)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-016"></a>
### CDX-CFG-016 [MEDIUM] Invalid model_reasoning_summary Value
**Requirement**: `model_reasoning_summary` SHOULD be one of `auto|always|none|concise|detailed`
**Detection**: Parse config and validate `model_reasoning_summary` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-017"></a>
### CDX-CFG-017 [MEDIUM] Invalid history Configuration
**Requirement**: `history` SHOULD be a TOML table (object)
**Detection**: Parse config and verify `history` is an object type
**Fix**: No auto-fix (configure as a TOML table)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-018"></a>
### CDX-CFG-018 [MEDIUM] Invalid tui Configuration
**Requirement**: `tui` SHOULD be a TOML table (object)
**Detection**: Parse config and verify `tui` is an object type
**Fix**: No auto-fix (configure as a TOML table)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-019"></a>
### CDX-CFG-019 [MEDIUM] Invalid file_opener Value
**Requirement**: `file_opener` SHOULD be a string
**Detection**: Parse config and verify `file_opener` is a string type
**Fix**: No auto-fix (set to a valid opener string)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-020"></a>
### CDX-CFG-020 [HIGH] Invalid MCP OAuth Config
**Requirement**: `mcp_oauth_credentials_store` MUST be one of `file|keyring|auto|ephemeral`
**Detection**: Parse config and validate `mcp_oauth_credentials_store` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-021"></a>
### CDX-CFG-021 [MEDIUM] Invalid model_context_window Value
**Requirement**: `model_context_window` SHOULD be a positive integer
**Detection**: Parse config and verify value is a positive integer
**Fix**: No auto-fix (set to a valid positive integer)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-022"></a>
### CDX-CFG-022 [MEDIUM] Invalid model_auto_compact_token_limit Value
**Requirement**: `model_auto_compact_token_limit` SHOULD be a positive integer
**Detection**: Parse config and verify value is a positive integer
**Fix**: No auto-fix (set to a valid positive integer)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-023"></a>
### CDX-CFG-023 [MEDIUM] Invalid Approval Policy Sub-field
**Requirement**: Keys inside `approval_policy.granular` SHOULD be recognized: `sandbox_approval`, `rules`, `mcp_elicitations`, `request_permissions`, and `skill_approval`. Unknown keys are advisory because the upstream granular schema does not forbid additional properties.
**Detection**: Warn on unknown keys in the nested object, not the outer `approval_policy` wrapper. CDX-CFG-001 validates required fields and boolean types.
**Fix**: No auto-fix - review and manually remove or rename the unknown key.
**Source**: github.com/openai/codex/blob/rust-v0.163.0/codex-rs/core/config.schema.json, github.com/openai/codex/blob/main/codex-rs/core/config.schema.json

<a id="cdx-cfg-024"></a>
### CDX-CFG-024 [MEDIUM] Invalid Approvals Reviewer Value
**Requirement**: `approvals_reviewer`, `apps.<app_id>.approvals_reviewer`, and `apps._default.approvals_reviewer` SHOULD be one of `user|auto_review|guardian_subagent`
**Detection**: Parse config and validate top-level, app-level, and app-default `approvals_reviewer` enum values
**Fix**: Manual
**Source**: github.com/openai/codex (codex-rs/core/config.schema.json @ rust-v0.137.0 and rust-v0.140.0)

<a id="cdx-cfg-025"></a>
### CDX-CFG-025 [MEDIUM] Invalid Service Tier Value
**Requirement**: service_tier SHOULD be "flex" or "fast"
**Detection**: Check enum value
**Fix**: Manual
**Source**: learn.chatgpt.com/docs/config-file/config-reference, learn.chatgpt.com/docs/config-schema.json

<a id="cdx-cfg-026"></a>
### CDX-CFG-026 [LOW] Unknown Permission Profile Network Key
**Requirement**: Keys of a permission profile's `[permissions.<name>.network]` table SHOULD be ones Codex defines (`NetworkToml`, `additionalProperties: false`): `enabled`, `domains`, `unix_sockets`, `proxy_url`, `enable_socks5`, `socks_url`, `enable_socks5_udp`, `allow_upstream_proxy`, `allow_local_binding`, `dangerously_allow_non_loopback_proxy`, `dangerously_allow_all_unix_sockets`, `mode`, `mitm`
**Detection**: For each `[permissions.<name>]` profile, report unknown keys of its `network` table at the key. A bare `[permissions.network]` is a profile named `network`, not a network table (the rule formerly checked that table against an older key list).
**Fix**: Manual
**Source**: learn.chatgpt.com/docs/permissions, learn.chatgpt.com/docs/config-schema.json

<a id="cdx-cfg-027"></a>
### CDX-CFG-027 [LOW] Invalid Windows Sandbox Value
**Requirement**: windows.sandbox SHOULD be "elevated" or "unelevated"
**Detection**: Check enum value
**Fix**: Manual
**Source**: github.com/openai/codex (codex-rs/core/config.schema.json @ rust-v0.137.0)

<a id="cdx-cfg-028"></a>
### CDX-CFG-028 [HIGH] Unsupported Inline MCP bearer_token Field
**Requirement**: `mcp_servers.<name>.bearer_token` inline field MUST NOT be used - Codex runtime rejects it and requires `bearer_token_env_var` to reference an environment variable instead
**Detection**: Walk `mcp_servers.*` tables in config.toml; error when any table contains a `bearer_token` key
**Fix**: Manual - rewrite as `bearer_token_env_var = "MY_ENV_VAR"` and set the token in the named env var (keeps secret out of the config file)
**Source**: openai/codex#19294 (removed from schema in rust-v0.125.0), openai/codex#19275 (original bug)

<a id="cdx-cfg-029"></a>
### CDX-CFG-029 [HIGH] Invalid Agent Concurrency Limit
**Requirement**: `agents.max_concurrent_threads_per_session` and its `agents.max_threads` compatibility alias MUST be positive integers when present and MUST NOT both be set. Codex 0.145 applies either form to both multi-agent backends and rejects zero, negative, fractional, non-numeric, or duplicate values.
**Detection**: Parse Codex config files; inspect both `[agents]` concurrency-limit keys and emit an error for any value that is not an integer of 1 or greater or when both the current name and compatibility alias are present. A positive `agents.max_threads` value alongside `multi_agent_v2` is valid in Codex 0.145 and is not flagged.
**Fix**: Manual - keep one field set to an integer of 1 or greater; prefer `max_concurrent_threads_per_session` for new configuration, while `max_threads` remains supported as an alias.
**Source**: github.com/openai/codex/releases/tag/rust-v0.145.0, github.com/openai/codex/pull/33550, github.com/openai/codex/blob/rust-v0.145.0/codex-rs/config/src/config_toml.rs

<a id="cdx-cfg-030"></a>
### CDX-CFG-030 [MEDIUM] Invalid web_search Mode
**Requirement**: `web_search` in `.codex/config.toml` / `.json` / `.yaml` MUST be one of `"disabled"`, `"cached"`, `"indexed"`, or `"live"` when present. Codex CLI `rust-v0.142.0` added the `"indexed"` mode alongside the existing modes.
**Detection**: Parse Codex config files and validate the top-level `web_search` string enum. Case-sensitive. `null` values and absent keys are not flagged.
**Fix**: Manual - set `web_search` to `"disabled"`, `"cached"`, `"indexed"`, or `"live"`.
**Source**: github.com/openai/codex/releases/tag/rust-v0.142.0, github.com/openai/codex/blob/rust-v0.142.0/codex-rs/protocol/src/config_types.rs

<a id="cdx-cfg-031"></a>
### CDX-CFG-031 [MEDIUM] Invalid skills.max_context_tokens
**Requirement**: `skills.max_context_tokens` MUST be a positive integer ("integer (positive)"); "Explicit values are capped at `10000` tokens."
**Detection**: Warning for a non-integer or non-positive value; info for a value above 10000, which Codex caps
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/config-file/config-reference, learn.chatgpt.com/docs/build-skills, learn.chatgpt.com/docs/config-schema.json

<a id="cdx-cfg-032"></a>
### CDX-CFG-032 [MEDIUM] Key Ignored in Project Config
**Requirement**: A project `.codex/config.toml` MUST NOT rely on keys Codex reads only from user config: "Codex ignores the following keys in project-local `.codex/config.toml` and prints a startup warning when it sees them: `openai_base_url`, `chatgpt_base_url`, `apps_mcp_product_sku`, `model_provider`, `model_providers`, `notify`, `profile`, `profiles`, `experimental_realtime_ws_base_url`, and `otel`."
**Detection**: Warning at each of those top-level keys in a `.codex/config.toml` outside the Codex home (`$CODEX_HOME`, else `~/.codex`). `profile` and `profiles` are left to CDX-APP-003 while it is enabled. Not reported when no home directory is known.
**Fix**: No auto-fix (move the key to `~/.codex/config.toml`)
**Source**: learn.chatgpt.com/docs/config-file/config-advanced

<a id="cdx-cfg-033"></a>
### CDX-CFG-033 [MEDIUM] Permission Profile Shadowed by Legacy Sandbox Settings
**Requirement**: A config MUST NOT mix permission profiles with the older sandbox settings: "Configure either `default_permissions` and `[permissions]`, or `sandbox_mode` / `sandbox_workspace_write`, but not both. If `sandbox_mode` appears in any loaded config file, you pass `--sandbox`, or the selected config profile sets `sandbox_mode`, Codex uses those older sandbox settings instead of `default_permissions`."
**Detection**: Warning at `default_permissions` (or `[permissions]`) when the same file sets `sandbox_mode` or `[sandbox_workspace_write]`; the message says the profile is ignored when `sandbox_mode` is set. Settings split across user and project layers are not visible to a single-file check.
**Fix**: No auto-fix (choose one system)
**Source**: learn.chatgpt.com/docs/permissions, learn.chatgpt.com/docs/config-file/config-reference

<a id="cdx-cfg-034"></a>
### CDX-CFG-034 [MEDIUM] Profile Domain Rules Without Network Proxy
**Requirement**: Domain rules of a permission profile SHOULD be backed by the network proxy: "A profile's `network.enabled = true` permits command network access, but it does not start the network proxy. To enforce profile domain rules, also set `features.network_proxy = true` in `config.toml`, or use enabled, administrator-managed `[experimental_network]` requirements. Without an active proxy, profile domain rules do not restrict direct network access."
**Detection**: Warning at `[permissions.<name>.network.domains]` when that profile sets `network.enabled = true`, has domain rules, and the same file does not enable `features.network_proxy`. Managed requirements can start the proxy, which agnix cannot see, so this is a warning.
**Fix**: No auto-fix (set `features.network_proxy = true`)
**Source**: learn.chatgpt.com/docs/permissions

<a id="cdx-cfg-035"></a>
### CDX-CFG-035 [HIGH] Invalid Permission Profile Inheritance
**Requirement**: `extends` MUST name a profile Codex accepts: "A profile can extend `:read-only`, `:workspace`, or another named profile. It cannot extend `:danger-full-access`; Codex also rejects unknown parents and inheritance cycles." `default_permissions` "must match a profile under `[permissions]` or a built-in such as `:workspace`".
**Detection**: Error at `permissions.<name>.extends` for `:danger-full-access`, an unknown `:`-prefixed built-in, or a cycle among the profiles in the file; error at `default_permissions` for an unknown `:`-prefixed built-in. Names without `:` may be defined in another config layer and are not reported as unknown.
**Fix**: [AUTO-FIX, unsafe] Replace an unknown `default_permissions` built-in with the closest one (`:workspaces` to `:workspace`); unsafe because Codex then applies that profile
**Source**: learn.chatgpt.com/docs/permissions

<a id="cdx-ag-005"></a>
### CDX-AG-005 [MEDIUM] AGENTS.md References Missing File
**Requirement**: File references in AGENTS.md SHOULD point to existing files. Codex advises keeping AGENTS.md concise and referencing "task-specific markdown files"; a reference to a missing file sends the agent to a file that is not there. Codex has no import mechanism, so references are plain text for the model. The path-shape filter is an agnix heuristic.
**Detection**: Extract backtick-quoted relative file paths (with a directory part and a plain extension) outside fenced code and check existence relative to the file and the project root; one diagnostic per file
**Fix**: No auto-fix (fix reference or create missing file)
**Source**: learn.chatgpt.com/guides/best-practices

<a id="cdx-ag-006"></a>
### CDX-AG-006 [LOW] AGENTS.md Missing Project Context
**Requirement**: AGENTS.md SHOULD include project-specific structure. Codex: "A good AGENTS.md covers:" repo layout, how to run the project, build/test/lint commands, conventions, constraints and how to verify work.
**Detection**: Check for presence of headings, backtick commands, and file paths
**Fix**: No auto-fix (add project-specific content)
**Source**: learn.chatgpt.com/guides/best-practices

<a id="cdx-ag-008"></a>
### CDX-AG-008 [LOW] AGENTS.md Shadowed by AGENTS.override.md
**Requirement**: An AGENTS.md next to an AGENTS.override.md is not read by Codex: "In each directory along the path, it checks for `AGENTS.override.md`, then `AGENTS.md`, then any fallback names in `project_doc_fallback_filenames`. Codex includes at most one file per directory." Other tools still read that AGENTS.md, so the two files SHOULD be kept deliberately. Committed nested overrides are a documented team pattern ("Add overrides in nested directories when specific teams need different rules"), so this is informational.
**Detection**: Info on an `AGENTS.md` whose directory also has `AGENTS.override.md`
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="cdx-ag-009"></a>
### CDX-AG-009 [MEDIUM] AGENTS.md Exceeds Codex project_doc_max_bytes
**Requirement**: A Codex instruction file SHOULD fit the instruction budget: "Codex skips empty files and stops adding files once the combined size reaches the limit defined by `project_doc_max_bytes` (32 KiB by default). ... Raise the limit or split instructions across nested directories when you hit the cap." Codex has no import mechanism, so the fix is nesting or a higher limit.
**Detection**: Warning on an `AGENTS.md` or `AGENTS.override.md` over 32768 bytes. An `AGENTS.md` next to an `AGENTS.override.md` is not checked, since Codex never reads it. Codex-family twin of XP-007 with the Codex fix text.
**Fix**: No auto-fix (move guidance into nested AGENTS.md files or raise `project_doc_max_bytes`)
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="cdx-ag-010"></a>
### CDX-AG-010 [MEDIUM] Codex Instruction Chain Exceeds project_doc_max_bytes
**Requirement**: The combined instruction chain SHOULD fit `project_doc_max_bytes`: Codex "stops adding files once the combined size reaches the limit" (32 KiB by default), walking from the project root to the working directory with at most one file per directory (`AGENTS.override.md`, then `AGENTS.md`).
**Detection**: Project-level: one chain per leaf directory; a chain over 32768 bytes is reported once, on its largest file. A chain of one file is left to CDX-AG-009. Fallback names from the user's Codex config are not visible, so the check can under-report. Codex-family twin of XP-009.
**Fix**: No auto-fix (trim the largest file, nest guidance deeper, or raise `project_doc_max_bytes`)
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="cdx-app-002"></a>
### CDX-APP-002 [MEDIUM] Invalid skills Configuration
**Requirement**: `skills` SHOULD be a TOML table (object)
**Detection**: Parse config and verify `skills` is an object type
**Fix**: No auto-fix (configure as a TOML table)
**Source**: learn.chatgpt.com/docs/config-file/config-reference, learn.chatgpt.com/docs/config-schema.json

<a id="cdx-app-003"></a>
### CDX-APP-003 [MEDIUM] Removed Profile Selector or Profiles Table
**Requirement**: Codex configs MUST NOT rely on in-file profiles: "In Codex 0.134.0 and later, `--profile` no longer reads `[profiles.profile-name]` from `config.toml`, and the top-level `profile = "profile-name"` selector is no longer supported." Profiles are separate `~/.codex/<name>.config.toml` files selected with `--profile <name>`. The rule formerly checked only that `profile` was a string.
**Detection**: Warning at a top-level `profile` key or a `[profiles]` table in any Codex config. No version gating: Codex 0.134.0 is older than the releases agnix tracks.
**Fix**: No auto-fix (move each profile to `~/.codex/<name>.config.toml`)
**Source**: learn.chatgpt.com/docs/config-file/config-advanced

### Codex Plugin Rules (CDX-PL)

<a id="cdx-pl-001"></a>
### CDX-PL-001 [HIGH] Codex Plugin Manifest Location or Agent Plugins Schema
**Requirement**: Legacy plugin.json MUST be in `.codex-plugin/`; root Agent Plugins manifests MUST declare the supported `https://agent-plugins.org/schemas/1.0.0/plugin.schema.json` schema
**Detection**: Check the legacy parent directory or the root manifest `$schema` discriminator
**Fix**: Move a legacy manifest to `.codex-plugin/plugin.json` or use a supported Agent Plugins schema
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/agent_plugin_manifest.rs @ rust-v0.146.0), agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="cdx-pl-002"></a>
### CDX-PL-002 [HIGH] Invalid Plugin Manifest
**Requirement**: Plugin manifests MUST contain valid JSON; Agent Plugins 1.0 metadata MUST use the required field types accepted by Codex
**Detection**: Parse as JSON, then validate Agent Plugins string metadata, author fields, keywords, and Codex extension `apps`/`interface` types
**Fix**: Correct the JSON syntax or field type
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs, agent_plugin_manifest.rs @ rust-v0.146.0), agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="cdx-pl-003"></a>
### CDX-PL-003 [HIGH] Missing or Empty Plugin Name
**Requirement**: Plugin manifest MUST have a non-empty string `name` field; Agent Plugins 1.0 requires it
**Detection**: Check `name` field is present and non-empty after trimming
**Fix**: Auto-fix (unsafe) - derive name from directory or parent project
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-004"></a>
### CDX-PL-004 [HIGH] Invalid Plugin Name Characters
**Requirement**: Codex 0.147+ plugin `name` MUST contain only ASCII alphanumeric characters, dots, hyphens, and underscores. Dots MUST separate non-empty segments, so names cannot start or end with a dot or contain `..`.
**Detection**: Validate the name against Codex's path-safe plugin segment rules
**Fix**: No auto-fix (rename to a valid plugin name)
**Source**: github.com/openai/codex (codex-rs/plugin/src/plugin_id.rs @ rust-v0.147.0)

<a id="cdx-pl-005"></a>
### CDX-PL-005 [HIGH] Component Path Missing ./ Prefix
**Requirement**: Component `path` values MUST start with `./`, including `apps` and path-form `hooks` in `extensions.com.openai`
**Detection**: Check that each component path begins with `./`
**Fix**: Auto-fix (safe) - prepend `./` to the path
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-006"></a>
### CDX-PL-006 [HIGH] Component Path Directory Traversal
**Requirement**: Component `path` MUST NOT contain `..` segments
**Detection**: Check for `..` in normalized path components
**Fix**: No auto-fix (restructure paths to stay within plugin directory)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-007"></a>
### CDX-PL-007 [HIGH] Component Path Empty Relative
**Requirement**: Component `path` MUST reference a file, not just `./`
**Detection**: Check that path has content beyond the `./` prefix
**Fix**: No auto-fix (provide a specific file path)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-008"></a>
### CDX-PL-008 [MEDIUM] Too Many Default Prompts
**Requirement**: `default_prompts` array MUST NOT exceed the maximum entry count
**Detection**: Count entries in `default_prompts` array
**Fix**: No auto-fix (remove excess prompts)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-009"></a>
### CDX-PL-009 [MEDIUM] Default Prompt Too Long
**Requirement**: Each entry in `default_prompts` MUST NOT exceed the maximum character length
**Detection**: Check string length of each prompt entry
**Fix**: No auto-fix (shorten prompt text)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-010"></a>
### CDX-PL-010 [MEDIUM] Empty Default Prompt Entry
**Requirement**: Entries in `default_prompts` SHOULD NOT be empty or whitespace-only
**Detection**: Check each prompt entry is non-empty after trimming
**Fix**: No auto-fix (remove empty entries or add content)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-011"></a>
### CDX-PL-011 [MEDIUM] Invalid Interface URL
**Requirement**: URL fields within `interface` (websiteUrl, privacyPolicyUrl, termsOfServiceUrl) SHOULD contain valid http/https URLs
**Detection**: Validate URL format of each URL field in the `interface` object
**Fix**: No auto-fix (provide a valid URL)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-012"></a>
### CDX-PL-012 [MEDIUM] Invalid Asset Path
**Requirement**: Asset paths in `interface` (composerIcon, logo, screenshots) MUST start with `./` and MUST NOT contain directory traversal
**Detection**: Validate each asset path for `./` prefix and absence of `..`
**Fix**: No auto-fix
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-013"></a>
### CDX-PL-013 [LOW] Invalid Hooks Value
**Requirement**: Plugin manifest `hooks` MUST be a relative path string, string array, inline hooks object, or inline hooks object array
**Detection**: Validate the `hooks` shape in legacy manifests and Agent Plugins `extensions.com.openai`; path forms also receive CDX-PL-005/006/007 validation
**Fix**: No auto-fix (use one of the supported hooks forms)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs, agent_plugin_manifest.rs @ rust-v0.146.0)

<a id="cdx-pl-014"></a>
### CDX-PL-014 [LOW] Missing Required Codex-Format Package Field
**Requirement**: A Codex-format manifest (`.codex-plugin/plugin.json` with no portable root manifest) MUST define the fields the Codex format requires before it is uploaded: "`description` | Required for Codex; optional for Agent Plugins | Package summary, at most 4000 characters.", likewise `version` and `author` (`author.name`) and `interface`; "Codex package validation requires both" `logo` and `composerIcon`. Upload validation enforces this ("If validation fails, correct the reported package errors and upload again"); locally installed plugins load without these fields, so the rule is informational.
**Detection**: Info listing the missing fields among `version`, `description`, `author.name`, `interface`, `interface.composerIcon`, `interface.logo`. Not reported for a compatibility overlay next to a portable root `plugin.json`. A `description` over 4,000 characters is a warning in both formats.
**Fix**: No auto-fix
**Source**: developers.openai.com/plugins/deploy/submission, developers.openai.com/plugins/build/plugins

<a id="cdx-pl-015"></a>
### CDX-PL-015 [MEDIUM] Invalid Skills Path Type
**Requirement**: Plugin manifest `skills` SHOULD be a string path; Codex ignores malformed values with a warning
**Detection**: Check that `skills`, when present, is a JSON string
**Fix**: No auto-fix (set `skills` to a relative path string such as `./skills`, or remove the field)
**Source**: github.com/openai/codex (codex-rs/core-plugins/src/manifest.rs @ rust-v0.137.0)

<a id="cdx-pl-016"></a>
### CDX-PL-016 [MEDIUM] Invalid Dark-mode Logo Path
**Requirement**: Plugin manifest `interface.logoDark` MUST start with `./` and MUST NOT contain directory traversal
**Detection**: Validate the dark-mode logo asset path for `./` prefix and absence of `..`
**Fix**: No auto-fix (set `logoDark` to a package-relative asset path such as `./assets/logo-dark.png`)
**Source**: github.com/openai/codex (openai/codex#29488, codex-rs/core-plugins/src/manifest.rs @ rust-v0.142.2)

<a id="cdx-pl-017"></a>
### CDX-PL-017 [LOW] Codex Compatibility Manifest Without Portable Root Manifest
**Requirement**: "The root `plugin.json` is the portable entry point. OpenAI also accepts legacy and Claude-compatible manifests, but new packages should use this format." A `.codex-plugin/plugin.json` "is optional and serves as a compatibility fallback".
**Detection**: Info on a `.codex-plugin/plugin.json` when the plugin root has no `plugin.json` with the Agent Plugins `$schema`
**Fix**: No auto-fix
**Source**: developers.openai.com/plugins/build/plugins

<a id="cdx-pl-018"></a>
### CDX-PL-018 [LOW] Plugin Exceeds Public Submission Limits
**Requirement**: Plugins prepared for the public directory SHOULD meet the submission limits: `name` "at most 64 characters. Use lowercase letters, numbers, and single hyphens for packages you plan to submit"; `displayName` and `shortDescription` "at most 30 characters"; `longDescription` at most 4000; `developerName` at most 80; capabilities "at most 20, with at most 120 characters each". "Plugins containing lifecycle hooks aren't eligible for the public plugin directory", and packages with app references "cannot currently be submitted". "The limits here are for public submission; a package upload can accept longer draft text."
**Detection**: Info per limit, only for manifests that carry submission metadata (`interface.developerName`, `interface.category`, `interface.longDescription`, or OpenAI `review` / `publication` settings), so locally installed plugins are not reported
**Fix**: No auto-fix
**Source**: developers.openai.com/plugins/deploy/submission, developers.openai.com/plugins/build/plugins

<a id="cdx-pl-019"></a>
### CDX-PL-019 [HIGH] Secret in Plugin MCP Configuration
**Requirement**: A plugin package MUST NOT contain credentials: "Keep the client secret out of the package. The manifest doesn't accept a `clientSecret` field; supply it through the connection setup flow." and "Don't embed a token in server `headers` to work around this limitation: public submission rejects nonempty server headers."
**Detection**: In the plugin root's `.mcp.json` or `mcp.json` (when the plugin has `.codex-plugin/plugin.json` or OpenAI extension settings): error at a non-empty `clientSecret` / `client_secret` under a server; warning at a credential header (`Authorization`, names with token, api-key, secret, password or cookie) whose value is a literal credential. `${VAR}` references and placeholder values are not reported.
**Fix**: No auto-fix
**Source**: developers.openai.com/plugins/build/plugins, developers.openai.com/plugins/deploy/submission

---

### Codex Requirements Rules (CDX-REQ)

Validates Codex's admin-written managed `requirements.toml` (system location: `/etc/codex/requirements.toml` on Unix, `%ProgramData%\OpenAI\Codex\requirements.toml` on Windows). Codex does not reject unknown keys in this file, so typo'd constraints are silently dropped - hence the unknown-key check.

### CDX-REQ-000 [HIGH] Codex requirements.toml TOML Parse Error
**Requirement**: `requirements.toml` MUST be syntactically valid TOML
**Detection**: Parse the file as a TOML table; report the parse error location on failure
**Fix**: No auto-fix (correct the TOML syntax)
**Source**: github.com/openai/codex (codex-rs/config/src/config_requirements.rs, codex-rs/config/src/loader/mod.rs @ rust-v0.137.0)

### CDX-REQ-001 [MEDIUM] Unknown Codex requirements.toml Key
**Requirement**: Top-level keys SHOULD be recognized members of `ConfigRequirementsToml`; Codex silently ignores unknown keys (no `deny_unknown_fields`), so a typo is never enforced
**Detection**: Compare each top-level key against the rust-v0.146.0 `ConfigRequirementsToml` set, including permission profiles, remote control, browser use, model policy, feedback, and update controls; obsolete `allowed_permissions` is unknown
**Fix**: No auto-fix (remove or rename the key)
**Source**: github.com/openai/codex (codex-rs/config/src/config_requirements.rs @ rust-v0.146.0, docs/config.md)

<a id="cdx-req-002"></a>
### CDX-REQ-002 [HIGH] Requirements Rule Without Prompt or Forbidden Decision
**Requirement**: "Unlike `.rules`, requirements rules must specify `decision`, and that decision must be `"prompt"` or `"forbidden"` (not `"allow"`)."
**Detection**: Error for each `[rules].prefix_rules` entry with no `decision`, a non-string decision, or any other value; reported at the decision or the entry
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/enterprise/managed-configuration

<a id="cdx-req-003"></a>
### CDX-REQ-003 [LOW] Managed Hook Script Outside managed_dir
**Requirement**: "Managed hook commands should use absolute script paths under the configured managed directory." Codex "doesn't distribute the scripts in `managed_dir`; your enterprise tooling must install and update them separately."
**Detection**: Info for a requirements.toml command hook whose `command` has no token under `managed_dir` (or whose `command_windows` has none under `windows_managed_dir`), or a command hook with no `managed_dir` set
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/hooks, learn.chatgpt.com/docs/enterprise/managed-configuration

<a id="cdx-req-004"></a>
### CDX-REQ-004 [LOW] Conflicting experimental_network Domain Settings
**Requirement**: "Do not combine the canonical `domains` map with the legacy `allowed_domains` or `denied_domains` lists." and use `managed_allowed_domains_only = true` "only when you also define administrator-owned `"allow"` entries in `[experimental_network.domains]`".
**Detection**: Info at a legacy list next to a `domains` map, and at `managed_allowed_domains_only = true` with no allow entry
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/enterprise/managed-configuration

<a id="cdx-hk-000"></a>
### CDX-HK-000 [HIGH] Codex Hooks File Fails to Load
**Requirement**: A Codex hooks file MUST deserialize, or Codex skips every hook in it. `hooks.json` is `HooksFile` with `deny_unknown_fields` (only `description` and `hooks` at the top), and every handler is a tagged `command`, `mcp_tool`, `prompt` or `agent` entry; a type error anywhere fails the whole file ("failed to parse hooks config"). An invalid inline `[hooks]` table drops all inline hooks of that config layer ("failed to parse TOML hooks").
**Detection**: Error for invalid JSON in `.codex/hooks.json` (plain JSON: no comments or trailing commas), an unknown top-level key, a non-object `hooks`, an event value that is not an array, a matcher group or handler of the wrong type, a handler without `type` or with an unknown type, a `command` handler without a string `command`, an `mcp_tool` handler without string `server` and `tool`, and `timeout` / `additionalContextLimit` that are not non-negative integers. Also run on inline `[hooks]` in `.codex/config.toml` and managed `requirements.toml`, and on a Codex plugin's default `hooks/hooks.json` (when the plugin has `.codex-plugin/plugin.json` and its manifest does not replace the default hooks path).
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/hooks; openai/codex codex-rs/config/src/hook_config.rs and codex-rs/hooks/src/engine/discovery.rs (main, 2026-10-17)

<a id="cdx-hk-001"></a>
### CDX-HK-001 [MEDIUM] Codex Hook Timeout Out of Range
**Requirement**: Hook timeouts MUST be within what Codex runs: "`timeout` is in seconds." and "`SessionEnd` and `Interrupt` use `1` second by default and support up to `3` seconds." Codex clamps a larger value to 3 seconds (with a startup warning) and raises 0 to 1 second for every event, so a Claude-style 60-second SessionEnd hook is killed after 3 seconds.
**Detection**: Warning at `timeout` for a SessionEnd or Interrupt handler over 3, and for a timeout of 0 on any event. Medium, not high: Codex still runs the hook with the clamped value.
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/hooks; openai/codex codex-rs/hooks/src/engine/discovery.rs (`normalize_command_hook`)

<a id="cdx-hk-002"></a>
### CDX-HK-002 [MEDIUM] Codex Hook Handler Skipped
**Requirement**: Hook handlers MUST be ones Codex runs: "`command` and `mcp_tool` handlers are supported. `prompt` and `agent` handlers are parsed but skipped." and "`SessionEnd` doesn't support MCP tool hooks." Codex also skips a handler with an empty command or an empty MCP server or tool name, and a whole matcher group whose regex does not compile.
**Detection**: Warning at `type` for prompt and agent handlers and for an `mcp_tool` handler on SessionEnd; at `command` for a blank command; at the handler for a blank `server` or `tool`; at `matcher` for an invalid regex on an event whose matcher Codex compiles (not UserPromptSubmit, Stop or Interrupt). In a plugin hooks file shared with Claude Code (the plugin also has `.claude-plugin/plugin.json`), prompt and agent handlers are not reported, since Claude Code runs them.
**Fix**: No auto-fix (use a command or mcp_tool handler)
**Source**: learn.chatgpt.com/docs/hooks; openai/codex codex-rs/hooks/src/engine/discovery.rs

<a id="cdx-hk-003"></a>
### CDX-HK-003 [LOW] Hooks in Both hooks.json and Inline [hooks]
**Requirement**: "If a single layer contains both `hooks.json` and inline `[hooks]`, Codex merges them and warns at startup. Prefer one representation per layer."
**Detection**: Info on a `.codex/hooks.json` with handlers when the `config.toml` next to it also has inline hook handlers
**Fix**: No auto-fix (keep one representation)
**Source**: learn.chatgpt.com/docs/hooks, learn.chatgpt.com/docs/config-file/config-advanced

<a id="cdx-hk-004"></a>
### CDX-HK-004 [LOW] additionalContextLimit Ineffective or Unbounded
**Requirement**: "Avoid setting the limit to `0` unless the hook enforces a strict output cap; otherwise, a single hook can consume the entire context window." and "For events that can't produce additional context, Codex ignores `additionalContextLimit` and reports a configuration warning."
**Detection**: Info at `additionalContextLimit` on an event other than PreToolUse, PostToolUse, SessionStart, UserPromptSubmit and SubagentStart (the events whose limit Codex keeps), and at `additionalContextLimit = 0` on those events
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/hooks; openai/codex codex-rs/hooks/src/engine/discovery.rs

<a id="cdx-hk-005"></a>
### CDX-HK-005 [MEDIUM] Unknown Codex Hook Event
**Requirement**: Hook events MUST be Codex events: PreToolUse, PermissionRequest, PostToolUse, PreCompact, PostCompact, SessionStart, SessionEnd, UserPromptSubmit, SubagentStart, SubagentStop, Stop, Interrupt. Codex reads events as fixed fields without `deny_unknown_fields`, so hooks under any other name (a Claude Code `Notification` hook, a misspelling) are dropped without a warning.
**Detection**: Warning at each unknown event key, with a "did you mean" suggestion for near misses. Inline `[hooks]` also accepts `state` (config.toml) and `managed_dir` / `windows_managed_dir` (requirements.toml). In a plugin hooks file shared with Claude Code only near misses are reported.
**Fix**: No auto-fix
**Source**: openai/codex codex-rs/config/src/hook_config.rs (`HookEventsToml`); learn.chatgpt.com/docs/hooks

<a id="cdx-sub-001"></a>
### CDX-SUB-001 [HIGH] Invalid Codex Custom Agent File
**Requirement**: "Every standalone custom agent file must define:" `name`, `description`, `developer_instructions`. The loader rejects a file without a non-empty `name` ("must define a non-empty `name`") or `developer_instructions`, a blank `description` ("cannot be blank"), invalid TOML, and a `nickname_candidates` list that is empty or has blank, duplicate or non-ASCII names.
**Detection**: Error for each problem in `.codex/agents/*.toml` and `~/.codex/agents/*.toml`, at the key when it is present
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/agent-configuration/subagents; openai/codex codex-rs/agent-roles/src/agent_role_config.rs

<a id="cdx-sub-002"></a>
### CDX-SUB-002 [MEDIUM] Session Setting in Codex Custom Agent File
**Requirement**: "Other session settings, including `sandbox_mode`, `approval_policy`, `model_provider`, and `mcp_servers`, don't override the parent configuration when placed in a custom agent file." Custom agents "inherit the parent's permissions, provider, and MCP servers; a custom agent file can't override those settings", and "The service tier also follows the root session's preference."
**Detection**: Warning at `sandbox_mode`, `sandbox_workspace_write`, `approval_policy`, `default_permissions`, `permissions`, `model_provider`, `model_providers`, `mcp_servers` or `service_tier` in a custom agent file
**Fix**: No auto-fix (set it on the parent session; narrow the agent with developer_instructions)
**Source**: learn.chatgpt.com/docs/agent-configuration/subagents

<a id="cdx-sub-003"></a>
### CDX-SUB-003 [LOW] Custom Agent Replaces a Built-in Agent
**Requirement**: "If a custom agent name matches a built-in agent such as `explorer`, your custom agent takes precedence." Codex ships `default`, `worker` and `explorer`.
**Detection**: Info at `name` when it is `default`, `worker` or `explorer`. File names that differ from `name` are not reported: "the `name` field is the source of truth", and the docs' own examples use `pr-explorer.toml` for `pr_explorer`.
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/agent-configuration/subagents

<a id="cdx-rl-001"></a>
### CDX-RL-001 [LOW] Forbidden Rule Without Justification
**Requirement**: "When you use `forbidden`, include a recommended alternative in the justification when appropriate"; Codex "may surface it in approval prompts or rejection messages".
**Detection**: Info at `decision` of a literal `prefix_rule(...)` call in `.codex/rules/*.rules` (or `~/.codex/rules/`) with `decision = "forbidden"` and no non-empty `justification`. Non-literal Starlark is not read.
**Fix**: No auto-fix
**Source**: learn.chatgpt.com/docs/agent-configuration/rules

<a id="cdx-rl-002"></a>
### CDX-RL-002 [MEDIUM] Broad Allow Rule Outside the Sandbox
**Requirement**: "Rules let you allow, prompt, or forbid command prefixes outside the sandbox, which is often a better fit than broadly expanding access." `decision` "defaults to `"allow"`", and Codex treats a complex `bash -lc "<script>"` as one invocation matched by its prefix, so allowing a shell or interpreter allows any code outside the sandbox.
**Detection**: Warning at `pattern` when a literal `prefix_rule` with no decision or `decision = "allow"` has a one-token pattern that is a shell, interpreter, privilege or network command (bash, sh, zsh, fish, pwsh, python, python3, node, deno, bun, ruby, perl, php, sudo, doas, env, xargs, npx, uvx, curl, wget, rm), or such a command followed only by a code flag (`-c`, `-lc`, `-e`, ...). The command list is an agnix heuristic.
**Fix**: No auto-fix (allow a narrower prefix or use decision = "prompt")
**Source**: learn.chatgpt.com/docs/agent-configuration/rules, learn.chatgpt.com/docs/sandboxing

<a id="cdx-dep-001"></a>
### CDX-DEP-001 [LOW] Deprecated Codex Custom Prompt
**Requirement**: "Custom prompts are deprecated. Use [skills](https://learn.chatgpt.com/docs/build-skills) for reusable instructions"; custom prompts "live in your local Codex home directory (for example, `~/.codex`)", and "Codex scans only the top-level Markdown files in that folder".
**Detection**: Info on Markdown files in `.codex/prompts/`; the message says when the file is outside the Codex home (never loaded) and a warning is used for files in a subdirectory (never loaded)
**Fix**: No auto-fix (convert the prompt to a skill)
**Source**: learn.chatgpt.com/docs/custom-prompts

---

## ROO CODE RULES

<a id="roo-001"></a>
### ROO-001 [HIGH] Empty Roo Code Rule File
**Requirement**: Roo Code rule files (`.roorules`, `.roo/rules/*.md`) MUST contain content
**Detection**: Check if `content.trim().is_empty()`
**Fix**: Add meaningful rule content to the file
**Source**: docs.roocode.com/features/custom-modes

<a id="roo-002"></a>
### ROO-002 [HIGH] Invalid .roomodes Configuration
**Requirement**: `.roomodes` MUST be valid JSON with `customModes` array containing mode entries with slug, name, roleDefinition, and groups
**Detection**: Parse JSON, validate structure - check customModes is array, each entry has required fields, slug format is valid, groups are valid names
**Fix**: Correct the .roomodes configuration to match the expected schema
**Source**: docs.roocode.com/features/custom-modes

<a id="roo-003"></a>
### ROO-003 [MEDIUM] Invalid .rooignore File
**Requirement**: `.rooignore` SHOULD have valid gitignore-style glob patterns
**Detection**: Check for empty content, validate each non-comment line as a glob pattern
**Fix**: Add valid glob patterns or fix syntax errors
**Source**: docs.roocode.com/features/rooignore

<a id="roo-004"></a>
### ROO-004 [MEDIUM] Invalid Mode Slug in Rule Directory
**Requirement**: Mode-specific rule directories (`.roo/rules-{slug}/`) SHOULD use valid slug format (lowercase alphanumeric with hyphens)
**Detection**: Extract slug from parent directory name, validate format
**Fix**: Rename directory to use a valid slug format
**Source**: docs.roocode.com/features/custom-modes

<a id="roo-005"></a>
### ROO-005 [HIGH] Invalid .roo/mcp.json Configuration
**Requirement**: `.roo/mcp.json` MUST be valid JSON with `mcpServers` object containing server entries with required fields
**Detection**: Parse JSON, validate structure - check mcpServers is object, stdio servers have command, http/sse servers have url
**Fix**: Correct the .roo/mcp.json configuration to match the expected schema
**Source**: docs.roocode.com/features/mcp/using-mcp-in-roo

<a id="roo-006"></a>
### ROO-006 [MEDIUM] Mode Slug Not Recognized
**Requirement**: SKILL.md files in mode-specific directories SHOULD reference built-in modes or modes defined in .roomodes
**Detection**: Check if slug matches built-in modes (code, architect, ask, debug, orchestrator) for SKILL.md files
**Fix**: Define the mode in .roomodes or use a built-in mode slug
**Source**: docs.roocode.com/features/custom-modes

---

## WINDSURF RULES

<a id="ws-001"></a>
### WS-001 [MEDIUM] Empty Windsurf Rule File
**Requirement**: Windsurf rule files in `.windsurf/rules/` SHOULD have content
**Detection**: File is empty or whitespace-only
**Fix**: Add rule content to the file
**Source**: docs.windsurf.com/windsurf/cascade/memories

<a id="ws-002"></a>
### WS-002 [HIGH] Windsurf Rule File Exceeds Character Limit
**Requirement**: Windsurf rule files MUST be under 12000 characters
**Detection**: File content length exceeds 12000 characters
**Fix**: Reduce content length or split into multiple rule files
**Source**: docs.windsurf.com/windsurf/cascade/memories

<a id="ws-003"></a>
### WS-003 [MEDIUM] Empty or Oversized Windsurf Workflow File
**Requirement**: Windsurf workflow files in `.windsurf/workflows/` SHOULD have content and be under 12000 characters
**Detection**: File is empty or exceeds 12000 characters
**Fix**: Add workflow steps or reduce content length
**Source**: docs.windsurf.com/windsurf/cascade/memories

<a id="ws-004"></a>
### WS-004 [LOW] Legacy .windsurfrules File Detected
**Requirement**: Projects SHOULD migrate from `.windsurfrules` to `.windsurf/rules/` directory format
**Detection**: File named `.windsurfrules`
**Fix**: Migrate to `.windsurf/rules/` directory with individual `.md` files
**Source**: docs.windsurf.com/windsurf/cascade/memories

---

## KIRO STEERING RULES

<a id="kiro-001"></a>
### KIRO-001 [HIGH] Invalid Steering File Inclusion Mode
**Requirement**: Kiro steering files MUST use a valid inclusion mode. Kiro CLI V3 drops frontmatter with any other value and includes the whole file, frontmatter text included, in every interaction; the V2 engine loads only `always` files.
**Detection**: Top-level `inclusion` is present, not null, and not exactly (case-sensitive) one of `always`, `fileMatch`, `manual`, `auto`. Reported at the value.
**Fix**: [AUTO-FIX, unsafe] Replace the exact top-level value (never a nested key) with the closest valid mode, keeping its quote style; offered only when the edit re-parses to the same frontmatter with only `inclusion` changed. Unsafe because a new mode changes when Kiro loads the file.
**Source**: kiro.dev/docs/steering/

<a id="kiro-002"></a>
### KIRO-002 [HIGH] Inclusion Mode Without the Field It Loads By
**Requirement**: `inclusion: auto` MUST have a non-empty `description` (Kiro activates auto steering by description; without one the file is only a slash command). `inclusion: fileMatch` MUST have a non-empty `fileMatchPattern` (without one the file never loads). `name` is optional: Kiro falls back to the file name.
**Detection**: `auto` without `description` or with `description: ""`; `fileMatch` with a missing, null, empty-string or empty-array pattern. Wrong types are KIRO-015.
**Fix**: Add the missing description or pattern
**Source**: kiro.dev/docs/steering/

<a id="kiro-003"></a>
### KIRO-003 [MEDIUM] fileMatchPattern Does Not Match as Written
**Requirement**: Each `fileMatchPattern` (a string or the documented array of strings) SHOULD match the way it reads. Kiro uses minimatch: a comma outside `{...}` alternation and `[...]` classes is a literal character, and an unclosed `[` is a literal `[`.
**Detection**: For every string pattern: a comma left after brace expansion outside a class (the same check, and limits, as Kiro's `fileMatchPatternLiteralComma` issue), or a `[` with no closing `]`. Reported at the pattern.
**Fix**: Use the array form or braces (`**/*.{ts,tsx}`); close or escape the bracket
**Source**: kiro.dev/docs/steering/

<a id="kiro-004"></a>
### KIRO-004 [MEDIUM] Empty Kiro Steering File
**Requirement**: Kiro steering files in `.kiro/steering/` SHOULD have content
**Detection**: File is empty or whitespace-only
**Fix**: Add steering content to the file
**Source**: kiro.dev/docs/steering/

<a id="kiro-005"></a>
### KIRO-005 [MEDIUM] Empty Steering Body After Frontmatter
**Requirement**: Steering files SHOULD include instruction content below frontmatter
**Detection**: File has frontmatter delimiters but markdown body is empty
**Fix**: Add concrete steering instructions below frontmatter
**Source**: kiro.dev/docs/steering/

<a id="kiro-006"></a>
### KIRO-006 [HIGH] Secrets Detected in Steering File
**Requirement**: Steering files MUST NOT include hardcoded credentials
**Detection**: A secret-like key (`api_key`, `token`, `password`, `*secret`) assigned a literal value. Values that name code instead of holding a literal are skipped: identifiers (`hashedPassword`, `SecretString`), member access (`user.password`), calls, `$VAR` references and templates. Quoted literals are always checked. Reported at the match.
**Fix**: Remove plaintext secrets and use environment variable expansion
**Source**: kiro.dev/docs/steering/

<a id="kiro-007"></a>
### KIRO-007 [MEDIUM] fileMatchPattern Without fileMatch Inclusion
**Requirement**: `fileMatchPattern` SHOULD only be used with `inclusion: fileMatch`; otherwise Kiro ignores the pattern and loads the file by its actual mode (`always` when unset)
**Detection**: A non-null `fileMatchPattern` while inclusion is missing or not `fileMatch`. Reported at the key.
**Fix**: Set `inclusion: fileMatch` or remove `fileMatchPattern`
**Source**: kiro.dev/docs/steering/

<a id="kiro-008"></a>
### KIRO-008 [MEDIUM] Unknown Steering Frontmatter Field
**Requirement**: Steering frontmatter SHOULD use only documented keys; Kiro drops the others
**Detection**: Top-level key outside `inclusion`, `name`, `description`, `fileMatchPattern`. Reported at the key.
**Fix**: Rename or remove unknown frontmatter keys
**Source**: kiro.dev/docs/steering/

<a id="kiro-009"></a>
### KIRO-009 [MEDIUM] Broken Inline Steering Reference
**Requirement**: Inline `#[[file:...]]` and `#[[folder:...]]` references SHOULD resolve to existing targets. File references may select one line or an inclusive line range.
**Detection**: Inline file or folder reference points to a missing target; line selectors are removed before resolving file paths.
**Fix**: Correct the reference path or create the missing file or folder
**Source**: kiro.dev/docs/steering/

<a id="kiro-010"></a>
### KIRO-010 [MEDIUM] On-Demand Steering Fields Without Inclusion Mode
**Requirement**: A steering file that sets `name` or `description` SHOULD declare an inclusion mode. Kiro reads those fields only for `auto` steering; without `inclusion` (or with `inclusion: null`) the file is `always` included.
**Detection**: `inclusion` missing or null while `description` or `name` is set. Files without frontmatter, or with frontmatter but no such field, are the documented default and are not reported.
**Fix**: No auto-fix (add `inclusion: auto` with a description, or `inclusion: manual`)
**Source**: kiro.dev/docs/steering/

<a id="kiro-011"></a>
### KIRO-011 [LOW] Always-Included Steering Doc Excessively Long
**Requirement**: A steering file Kiro includes in every interaction (no frontmatter, `inclusion: always` or unset, or frontmatter Kiro cannot read) SHOULD stay under 50,000 characters. The number is agnix guidance: Kiro documents no steering limit, only "Keep Files Focused", and context files consume tokens on every request.
**Detection**: Character count of the whole file when the file is always included; `fileMatch`, `manual` and `auto` files are loaded on demand and are not checked
**Fix**: No auto-fix (split into smaller files or load detail on demand)
**Source**: kiro.dev/docs/steering/, kiro.dev/docs/cli/chat/context/

<a id="kiro-015"></a>
### KIRO-015 [HIGH] Steering Frontmatter Kiro Cannot Read
**Requirement**: Steering frontmatter MUST be readable by Kiro: it starts on the very first line, has a closing `---`, is valid YAML without duplicate keys, is a mapping, and `name`/`description` are strings and `fileMatchPattern` a string or string array. Otherwise Kiro CLI V3 includes the whole file, frontmatter text included, in every interaction, so the inclusion mode is lost.
**Detection**: Frontmatter after blank lines or text (a `---` block within the first ten lines that closes and holds steering keys with `inclusion` or `fileMatchPattern`, so horizontal rules are not mistaken for it; a warning when that block's mode is `always` or unset, since only its text leaks into the context), an unclosed block, a YAML error (reported at the error, or at a repeated key), or a non-mapping document: error. A wrong type for `name`, `description` (including null) or `fileMatchPattern`: the V2 engine reads only `inclusion`, so this is an error when only V3 is targeted (`[tool_versions] kiro = "3.x"`), a warning naming V3 when both engines are, and not reported when V2 is pinned. An empty or comments-only block is no frontmatter (always included) and is not reported. A repeated `inclusion` key is a YAML error reported here.
**Fix**: No auto-fix (fix the YAML or the field type)
**Source**: kiro.dev/docs/steering/

<a id="kiro-016"></a>
### KIRO-016 [MEDIUM] AGENTS.md Exceeds Kiro Truncation Limit
**Requirement**: An AGENTS.md Kiro loads (workspace root or nested) SHOULD stay within 50,000 characters: Kiro CLI V3 keeps only the first 50,000 (JavaScript string length, UTF-16 code units) and appends "[Truncated: AGENTS.md exceeds the maximum steering document size.]". The limit comes from the shipped V3 engine (@kiro/agent 0.66.48 in Kiro CLI 2.32.1); the docs do not state it.
**Detection**: Files named exactly `AGENTS.md` outside `.kiro/steering/` (which are steering documents) whose UTF-16 length exceeds 50,000, reported at the first dropped character. Runs only for Kiro projects: Kiro selected with `tools`/`target`, or no tool selected and a `.kiro` directory next to the file or above it. Not reported when `[tool_versions] kiro` pins 2.x.
**Fix**: No auto-fix (move scoped guidance into .kiro/steering/ or nested AGENTS.md files)
**Source**: kiro.dev/docs/steering/ (AGENTS.md support); Kiro CLI V3 engine

<a id="kiro-017"></a>
### KIRO-017 [MEDIUM] Kiro Steering Frontmatter in AGENTS.md
**Requirement**: AGENTS.md SHOULD NOT carry Kiro steering frontmatter: Kiro reads AGENTS.md as plain text, so `inclusion` and `fileMatchPattern` have no effect and the block reaches the model as text ("AGENTS.md files do not support inclusion modes and are always included")
**Detection**: An AGENTS.md (outside `.kiro/steering/`, Kiro projects only, as KIRO-016) that starts with a closed frontmatter block whose mapping has `inclusion` or `fileMatchPattern`; other keys such as Amp's `globs` are not reported
**Fix**: Move scoped guidance into a .kiro/steering/ file with that frontmatter, or remove the block
**Source**: kiro.dev/docs/steering/

<a id="kiro-018"></a>
### KIRO-018 [LOW] Vague Auto Steering Description
**Requirement**: An `inclusion: auto` description SHOULD say what the file covers and when to use it; Kiro matches requests against it
**Detection**: The description has fewer than four words (fewer than 12 characters for text written without spaces) or starts with a generic "Helps with" / "Assists with" phrase, the docs' own vague example. Info level.
**Fix**: No auto-fix (write "<what>. Use when <situation>.")
**Source**: kiro.dev/docs/steering/ (Auto inclusion), kiro.dev/docs/skills/ (Best practices)

<a id="kiro-026"></a>
### KIRO-026 [MEDIUM] Kiro Configuration File Never Loaded
**Requirement**: A Kiro configuration file MUST use the name and location Kiro loads. Kiro lists `.kiro/agents/` (`*.json`; V3 also `*.md`, recursively), the top level of `.kiro/hooks/` (`*.json`) and `.kiro/steering/` (`**/*.md`) and compares extensions case-sensitively; it lists the top-level `*.workflow.{json,yaml,yml}` of `.kiro/workflows/`; it opens `.kiro/settings/mcp.json`, `.kiro/settings/cli.json`, and `permissions.yaml` or `permissions.json` in `~/.kiro/settings/` and `~/.kiro/workspace-roots/<hash>/`. Kiro CLI does not read `.kiro/settings.json` (only the CLI 2.x session tool's description still names it).
**Detection**: Report, as a warning, a file no Kiro engine loads although its name shows the intent: an extension that differs only by case (`reviewer.JSON`, `style.MD`) or a JSON variant (`reviewer.jsonc`, `lint.yaml`) in `.kiro/agents/`, `.kiro/hooks/` or `.kiro/steering/`; `permissions.yml`, `mcp.jsonc`, `cli.jsonc` or a settings name with surrounding whitespace (` permissions.yaml`); `mcp.json`, `cli.json` or `permissions.*` placed directly in `.kiro/`; a hook document (`hooks` array, or `when` and `then`) in a subdirectory of `.kiro/hooks/`; a workflow recipe in a subdirectory of `.kiro/workflows/`, with a case-variant suffix (`plan.Workflow.json`), or without the `.workflow` infix when its content is a recipe (`plan.json` with a `steps` array); and a `.kiro/settings.json` that holds a Kiro CLI setting key (`chat.defaultModel`, `toolSearch.enabled`, ...). Unrelated files in those directories (scripts, `package.json`, editor settings) are not reported.
**Fix**: No auto-fix (rename or move the file)
**Source**: kiro.dev/docs/configuration, kiro.dev/docs/custom-agents, kiro.dev/docs/hooks, kiro.dev/docs/steering, kiro.dev/docs/reference/settings, kiro.dev/docs/permissions

---

## KIRO POWERS RULES

<a id="kr-pw-001"></a>
### KR-PW-001 [HIGH] POWER.md Frontmatter Kiro Cannot Use
**Requirement**: A legacy `POWER.md` MUST have frontmatter Kiro can read and SHOULD set `description` and a `keywords` array, which Kiro uses to describe and activate the power. Kiro takes the power name from its directory, so `name` is not required.
**Detection**: Error: invalid YAML or an unclosed frontmatter block (Kiro cannot read the power and skips it). Warning: no frontmatter, frontmatter not on the first line, or no `description` / no array `keywords` (a non-array `keywords` counts as none).
**Fix**: Fix the YAML; add description and keywords
**Source**: kiro.dev/docs/powers, kiro.dev/docs/powers/installation

<a id="kr-pw-002"></a>
### KR-PW-002 [MEDIUM] Empty POWER.md Keywords Array
**Requirement**: `keywords` SHOULD contain one or more activation keywords
**Detection**: `keywords` exists but is an empty array. Reported at the key.
**Fix**: Add one or more keywords used for power activation
**Source**: kiro.dev/docs/powers/

<a id="kr-pw-003"></a>
### KR-PW-003 [MEDIUM] Empty POWER.md Body
**Requirement**: `POWER.md` SHOULD include onboarding/workflow/reference content
**Detection**: Readable frontmatter with an empty markdown body
**Fix**: Add body content describing usage and behavior
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-004"></a>
### KR-PW-004 [MEDIUM] Adjacent Power mcp.json Kiro Cannot Use
**Requirement**: A legacy power's adjacent `mcp.json` MUST be strict JSON (Kiro reads it with JSON.parse; otherwise it skips the whole power) and SHOULD define an `mcpServers` object
**Detection**: Error: the adjacent `mcp.json` is unreadable or not valid JSON (comments and trailing commas included). Warning: `mcpServers` is missing or not an object. Server contents are left to the MCP rules.
**Fix**: Fix the JSON; use an `mcpServers` object keyed by server name
**Source**: kiro.dev/docs/powers/, kiro.dev/docs/powers/installation

<a id="kr-pw-006"></a>
### KR-PW-006 [LOW] Duplicate Keywords
**Requirement**: Power activation keywords SHOULD be unique (POWER.md `keywords` and plugin.json `keywords`)
**Detection**: Duplicate entries in the keywords array (case-insensitive), listed in one info diagnostic at the key
**Fix**: No auto-fix (remove duplicate keywords)
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-007"></a>
### KR-PW-007 [MEDIUM] Name Invalid Characters
**Requirement**: POWER.md name SHOULD use lowercase letters, digits and hyphens only
**Detection**: Validate name against `^[a-z0-9][a-z0-9-]*$`. Reported at the key.
**Fix**: No auto-fix (rename to valid format)
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-008"></a>
### KR-PW-008 [HIGH] Secrets in Power Body
**Requirement**: POWER.md body MUST NOT contain hardcoded credentials
**Detection**: The KIRO-006 secret check on the body, so identifiers, member access and calls in code samples are skipped. Reported at the match.
**Fix**: No auto-fix (use environment variables)
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-009"></a>
### KR-PW-009 [HIGH] plugin.json Is Not a JSON Object
**Requirement**: An Agent Plugins power's `plugin.json` MUST be strict JSON holding an object; otherwise Kiro rejects the power
**Detection**: JSON syntax error (comments and trailing commas included), reported at the error; or a top-level value that is not an object
**Fix**: Fix the JSON
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="kr-pw-010"></a>
### KR-PW-010 [HIGH] Missing or Unsupported plugin.json $schema
**Requirement**: `plugin.json` MUST set `$schema` to an Agent Plugins 1.x manifest schema (`https://agent-plugins.org/schemas/1.x.y/plugin.schema.json`); otherwise Kiro rejects the power
**Detection**: `$schema` missing, not a string, or not matching `^https://agent-plugins.org/schemas/1.\d+.\d+/plugin.schema.json$`. Reported at the value.
**Fix**: Set `"$schema": "https://agent-plugins.org/schemas/1.0.0/plugin.schema.json"`
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="kr-pw-011"></a>
### KR-PW-011 [HIGH] Missing or Invalid plugin.json Name
**Requirement**: `plugin.json` MUST set `name` to 1-64 characters of `a-z`, `0-9`, `.` and `-`, starting and ending with a letter or digit, without `--` or `..`; otherwise Kiro rejects the power
**Detection**: `name` missing, not a string, or not matching the Agent Plugins name rule. Reported at the value.
**Fix**: Rename to a valid plugin name
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="kr-pw-012"></a>
### KR-PW-012 [MEDIUM] plugin.json Field Outside the Agent Plugins Manifest
**Requirement**: `plugin.json` SHOULD contain only manifest fields (`$schema`, `name`, `displayName`, `version`, `description`, `author`, `homepage`, `repository`, `license`, `keywords`, `extensions`). Kiro ignores other keys (and reports an issue); the Agent Plugins schema rejects them. `displayName` is accepted by Kiro although the 1.0.0 schema lacks it.
**Detection**: Any other top-level key, reported at the key; `mcpServers` and component keys (`skills`, `steering`, `hooks`, ...) get a targeted suggestion
**Fix**: Remove the key, or move client-specific data under `extensions`; put MCP servers in `mcp.json`
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="kr-pw-013"></a>
### KR-PW-013 [MEDIUM] plugin.json Value Kiro Ignores
**Requirement**: `plugin.json` field values SHOULD have the manifest types; Kiro drops values of the wrong type
**Detection**: `version`, `description`, `displayName`, `homepage`, `repository`, `license` not strings; `author` not an object, or with a key other than `name`/`email`/`url` (Kiro then drops the whole author), or with non-string values; `keywords` not an array, or with non-string items (dropped); `extensions` not an object. Reported at the value or item.
**Fix**: Fix the value type
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/plugin.schema.json

<a id="kr-pw-014"></a>
### KR-PW-014 [HIGH] Plugin Skill Kiro Skips
**Requirement**: Skills of an Agent Plugins power MUST be loadable: `skills/` is a directory, each `skills/<dir>/SKILL.md` sits in a directory whose name is a valid skill name, and no two skills declare the same name
**Detection**: From the manifest's directory: `skills` exists but is not a directory (no skills load); a skill directory with a `SKILL.md` whose name is not a valid skill name (Kiro skips it); two skills declaring the same frontmatter `name` (Kiro keeps the first in name order and skips the rest). Reported on plugin.json.
**Fix**: Rename the directory or the skill
**Source**: kiro.dev/docs/powers/create, kiro.dev/docs/skills

<a id="kr-pw-015"></a>
### KR-PW-015 [MEDIUM] Power Without Activation Description or Keywords
**Requirement**: `plugin.json` SHOULD set a non-empty `description` and at least one string in `keywords`; Kiro describes and activates powers by them
**Detection**: `description` missing or blank; `keywords` missing, not an array, or with no string items
**Fix**: Add a description and keywords
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-016"></a>
### KR-PW-016 [LOW] Documented plugin.json Field Missing or Not Semver
**Requirement**: The powers docs list `version` (semantic version, e.g. 1.0.0) and `author` (with `name`) as required; Kiro loads a power without them and accepts any version string
**Detection**: `version` missing or not a semantic version, or no `author.name` string. Info level.
**Fix**: Add a MAJOR.MINOR.PATCH version and an author object with a name
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-017"></a>
### KR-PW-017 [MEDIUM] POWER.md Ignored Next to plugin.json
**Requirement**: A power directory SHOULD NOT mix formats: when `plugin.json` exists Kiro loads the power from it and never reads `POWER.md`, its `steering/` or its legacy `mcp.json` layout
**Detection**: A `POWER.md` whose directory also has a `plugin.json`; other POWER.md checks are skipped for it
**Fix**: Finish the migration and delete POWER.md, or remove plugin.json
**Source**: kiro.dev/docs/powers, kiro.dev/docs/powers/installation

<a id="kr-pw-018"></a>
### KR-PW-018 [MEDIUM] Legacy steering Directory in a plugin.json Power
**Requirement**: An Agent Plugins power SHOULD NOT rely on a `steering/` directory: Kiro loads plugin steering only from `dev.kiro/steering/`
**Detection**: A `steering/` directory next to `plugin.json`
**Fix**: Move the guidance into `skills/<name>/SKILL.md` or `dev.kiro/steering/`
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-019"></a>
### KR-PW-019 [LOW] Legacy POWER.md Power Format
**Requirement**: New powers SHOULD use the Agent Plugins format (`plugin.json`, `skills/`, `mcp.json`); POWER.md powers still load ("For new powers, we recommend using the Agent Plugins format")
**Detection**: Any POWER.md without a sibling plugin.json (with one, KR-PW-017 applies). Info level.
**Fix**: No auto-fix (migrate with Kiro's power-builder power, or keep POWER.md)
**Source**: kiro.dev/docs/powers, kiro.dev/docs/powers/create

---

## KIRO IDE HOOKS RULES

Kiro IDE 0.x stored each hook in `.kiro/hooks/*.kiro.hook`: `{ "name", "description", "version", "enabled", "when": { "type", "patterns", "toolTypes" }, "then": { "type": "askAgent" | "runCommand", "prompt" | "command" } }`. Kiro CLI never reads these files; Kiro IDE 1.0 lists them as legacy hooks and offers to upgrade each one to the `.kiro/hooks/*.json` v1 format (see the Kiro V3 hook rules below). The documented `when.type` values are `fileEdited`, `fileCreated`, `fileDeleted`, `promptSubmit`, `agentStop`, `preToolUse`, `postToolUse`, `preTaskExecution`, `postTaskExecution`, `sessionStart` and `userTriggered`, plus `agentSpawn` from the IDE 1.0 migration table. The flat `event` / `patterns` / `runCommand` / `askAgent` spellings are still read for compatibility.

<a id="kr-hk-001"></a>
### KR-HK-001 [HIGH] Invalid Kiro IDE Hook Event Type
**Requirement**: The IDE hook event (`when.type`, or `event`) MUST be one of the documented IDE 0.x trigger names
**Detection**: Error when the event is missing or unknown (for example `userPrompt`). Warning for the undocumented spellings `fileCreate`, `fileDelete` and `manual` (documented: `fileCreated`, `fileDeleted`, `userTriggered`); they are not errors because whether IDE 0.x accepted them is unverified, and the IDE-generated files in the corpus use the documented names. The CLI spellings `SessionEnd`, `AgentSpawn`, `userPromptSubmit` and `stop` get the KR-HK-014 hint instead. A non-string event is reported by KR-HK-026. The diagnostic points at the `when.type` value.
**Fix**: [AUTO-FIX, unsafe] Replace `fileCreate` / `fileDelete` / `manual` with the documented name, on the exact value span. Unsafe because the IDE's handling of the old spelling is unverified. Other unknown events are fixed manually (for example `promptSubmit` instead of `userPrompt`).
**Source**: kiro.dev/docs/ide/0x-reference, kiro.dev/docs/ide/whats-new-v1/hooks

<a id="kr-hk-002"></a>
### KR-HK-002 [HIGH] File Event Hook Missing Patterns
**Requirement**: File event IDE hooks MUST include `when.patterns`
**Detection**: The event is `fileEdited`, `fileCreated` or `fileDeleted` (or the tolerated `fileCreate` / `fileDelete`) and `when.patterns` is missing or has only blank entries. A `patterns` value of the wrong type is reported by KR-HK-026 instead.
**Fix**: Add one or more patterns in `when.patterns`
**Source**: kiro.dev/docs/ide/0x-reference

<a id="kr-hk-003"></a>
### KR-HK-003 [HIGH] IDE Hook Missing Action
**Requirement**: IDE hooks MUST define an action: `then.prompt` (askAgent) or `then.command` (runCommand)
**Detection**: Neither `then.command` / `then.runCommand` / `runCommand` nor `then.prompt` / `then.askAgent` / `askAgent` has non-blank text. Action fields of the wrong type are reported by KR-HK-026 instead.
**Fix**: Add a `then` block with a prompt or a command
**Source**: kiro.dev/docs/ide/0x-reference

<a id="kr-hk-004"></a>
### KR-HK-004 [MEDIUM] Tool Hook Missing toolTypes Filter
**Requirement**: `preToolUse`/`postToolUse` hooks SHOULD name the tools they match in `when.patterns` (the IDE 0.x reference says tool events match `when.patterns` against the tool name; the hooks best practices ask for precise tool matchers)
**Detection**: Tool hook event has no non-blank `when.patterns`. The undocumented `when.toolTypes` key is still accepted because real IDE files use it.
**Fix**: Add `when.patterns` with the tool names to match. The V3 counterpart is KR-HK-027.
**Source**: kiro.dev/docs/ide/0x-reference, kiro.dev/docs/hooks/best-practices

<a id="kr-hk-009"></a>
### KR-HK-009 [LOW] Command Uses Absolute Path
**Requirement**: IDE hook commands SHOULD avoid machine-specific absolute paths. This is an agnix portability heuristic: Kiro does not require relative paths, it runs command actions "in your project root", so relative paths work on every checkout. (Windsurf's docs recommend absolute paths for its own hooks; this rule is not applied to other tools.)
**Detection**: Info when the run command starts with `/` or a drive letter; the diagnostic points at the command value
**Fix**: No auto-fix (use relative paths or a tool on PATH)
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-010"></a>
### KR-HK-010 [HIGH] Secrets in Hook Command
**Requirement**: Kiro hook commands MUST NOT contain hardcoded credentials
**Detection**: Checks `action.command` and `confirm.confirmCommand` in `.kiro/hooks/*.json` (and `~/.kiro/hooks/*.json`), and the run command of legacy `.kiro.hook` files, enabled or not (a committed credential leaks either way). Reports a token prefix that starts a word and is followed by a token-shaped body (`sk-` with 16+ characters, `ghp_` / `gho_` / `github_pat_` with 30+, `xoxb-` / `xoxp-` with 10+, `glpat-` with 20+, `AKIA` + 16 uppercase characters), or a literal value after `api_key=`, `apikey=`, `api-key=`, `token=`, `password=`, `secret=` or `Bearer `. Values that are references (`$VAR`, `${VAR}`, `$(cmd)`, `%VAR%`, `<...>`, `{{...}}`) or shorter than 8 characters are not secrets, so `flask-lint`, `task-check` and `--token=$DEPLOY_TOKEN` are not reported. The diagnostic points at the command value. (This section lists the rule because it started with IDE hooks; since 0.59.0 it also covers v1 hook files.)
**Fix**: No auto-fix (use environment variables; the hook process inherits Kiro's environment)
**Source**: kiro.dev/docs/mcp/security ("Never hardcode API keys or tokens in configuration files"), kiro.dev/docs/hooks, kiro.dev/docs/ide/0x-reference. The hooks pages say nothing about secrets.

<a id="kr-hk-016"></a>
### KR-HK-016 [HIGH] V3 Hook File Saved as .kiro.hook
**Requirement**: v1 hook documents MUST be saved as `.kiro/hooks/*.json`
**Detection**: A `.kiro.hook` file has a `hooks` array, or a v1 `trigger` / `action`, and no IDE 0.x `when` / `then` / `event` fields. Kiro CLI V3 and IDE 1.0 load v1 hooks only from `.json` files, so these hooks never run. No other IDE hook rule is reported for the file.
**Fix**: No auto-fix (rename the file to `.json`)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/ide/whats-new-v1/hooks

<a id="kr-hk-026"></a>
### KR-HK-026 [HIGH] Invalid Kiro IDE Hook File
**Requirement**: An IDE hook file MUST be a JSON object whose fields have the documented types
**Detection**: Error for invalid JSON or a non-object document. Warning, at the offending value, for `when` / `then` that are not objects, `when.type` and command or prompt fields that are not strings, `patterns` / `toolTypes` that are not arrays of strings, and a `timeout` / `then.timeout` that is not a number or is negative (the IDE documents `timeout` in seconds, example 10; 0 is allowed and there is no upper bound). The remaining checks still run. (JSON syntax and type errors were previously reported as KR-HK-001; the former millisecond range check for legacy timeouts is retired.)
**Fix**: No auto-fix (repair the JSON)
**Source**: kiro.dev/docs/ide/0x-reference

---

## KIRO V3 HOOK RULES

Kiro CLI V3 (opt-in in CLI 2.x with `kiro-cli --v3`, the default from CLI 3.0) and Kiro IDE 1.0 load `.kiro/hooks/*.json` in the workspace and, for the CLI, `~/.kiro/hooks/*.json`: `{ "version": "v1", "hooks": [ { "name", "trigger", "matcher"?, "action": { "type": "command" | "agent", "command" | "prompt" }, "timeout"?, "enabled"?, "description"?, "confirm"? } ] }`. Kiro CLI 2.x does not load these files. The engine reads each file with strict `JSON.parse` (no comments; the last copy of a duplicate key wins) and validates the whole document with one schema: a single malformed hook makes Kiro skip the whole file ("Hook file does not match v2 schema"), while an unrecognized trigger name skips only that hook. Unknown keys are stripped silently. Every diagnostic points at the offending key or value. These rules follow the V3 engine bundled with kiro-cli 2.32.1 (`@kiro/agent` 0.66.48) where it is more precise than the docs.

<a id="kr-hk-011"></a>
### KR-HK-011 [HIGH] Invalid Kiro V3 Hook Document Version
**Requirement**: A standalone hook file MUST set `version` to the string `"v1"`; any other value makes Kiro skip the whole file
**Detection**: Error when `version` is missing, not a string, or not exactly `"v1"`. Not reported when the file also has no `hooks` array (KR-HK-012 covers it) or is a legacy when/then document (KR-HK-015 covers it).
**Fix**: [AUTO-FIX, unsafe] Replace only the decoded version strings `"1"` or `"1.0"` with `"v1"`, on the exact top-level value span and only when `version` is not duplicated. Unsafe because it activates hooks Kiro currently skips. Numeric, missing, legacy (`"1.0.0"`) and future versions are not guessed.
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-hk-012"></a>
### KR-HK-012 [HIGH] Invalid Kiro V3 Hook Entry
**Requirement**: A hook file MUST be a JSON object with a non-empty `hooks` array of objects, and each hook MUST have a non-empty string `name`, a recognized `trigger` and an `action` object
**Detection**: Invalid JSON (with serde's line and column); a non-object document; `hooks` missing, not an array or empty; a bare hook definition without the wrapper; an entry that is not an object; `name` / `trigger` missing, not a string or empty (whitespace-only passes the schema); `action` missing (with a hint when `command` or `prompt` sits at the hook level). These skip the whole file. An unrecognized trigger name skips only that hook: the lookup is exact and case-sensitive over SessionStart, SessionEnd, Stop, PreToolUse, PostToolUse, PreTaskExec, PostTaskExec, UserPromptSubmit, PostFileCreate, PostFileSave, PostFileDelete, Manual and the aliases sessionStart, agentSpawn, agentStop, stop, promptSubmit, userPromptSubmit, preToolUse, postToolUse, preTaskExecution, postTaskExecution, fileEdited, AfterFileEdit, fileCreated, fileDeleted and userTriggered. `AgentSpawn` is not accepted, although the hook-types docs say it is.
**Fix**: [AUTO-FIX, unsafe] An unrecognized trigger that matches a canonical name or alias ignoring case (for example `AgentSpawn`, `postfilesave`) is replaced with the canonical name. Unsafe because the hook starts running.
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/types, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-hk-013"></a>
### KR-HK-013 [HIGH] Invalid Kiro V3 Hook Action
**Requirement**: `action.type` MUST be `command` (with a non-empty string `command`) or `agent` (with a non-empty string `prompt`)
**Detection**: Error, because Kiro skips the whole file, when `action` is not an object, `type` is missing, not a string or another value, or the matching payload is missing, not a string or empty. Warning when the payload contains only whitespace (it passes the schema but does nothing). The timeout check moved to KR-HK-017.
**Fix**: [AUTO-FIX, unsafe] `type` values `runCommand` / `askAgent` (IDE 0.x spellings) and case variants of `command` / `agent` are replaced when the matching `command` / `prompt` field is present. Unsafe because it activates the file.
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/actions

<a id="kr-hk-014"></a>
### KR-HK-014 [LOW] Kiro Hook Migration and Canonical Trigger Hint
**Requirement**: Prefer canonical trigger names when editing V3 hooks; the accepted aliases remain valid. Legacy IDE manual hooks may stay in their original format.
**Detection**: In standalone `.kiro/hooks/*.json`, any accepted alias (see KR-HK-012) gets an info diagnostic naming its canonical trigger. In legacy `.kiro.hook` files, advise migration for the CLI-only spellings `SessionEnd`, `AgentSpawn`, `userPromptSubmit` and `stop` only when no parsed, enabled `v1` sibling contains the same canonical trigger and action. Unrelated, disabled, or invalid siblings do not suppress the hint; an unreadable directory is inconclusive. Lowercase `agentSpawn` and legacy manual `userTriggered` are not misplaced-CLI hints.
**Fix**: [AUTO-FIX, safe] Replace the alias with its canonical name using the exact structural string span (never when the trigger or a containing key is duplicated). Safe because the engine resolves the alias to that canonical name before anything else, and the stdin payload already carries the canonical name. Legacy file migration is manual; do not rename a file or convert an IDE hook automatically.
**Source**: kiro.dev/docs/hooks/types, kiro.dev/docs/ide/whats-new-v1/hooks, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-hk-015"></a>
### KR-HK-015 [HIGH] Legacy IDE Hook Saved as a V3 Hook File
**Requirement**: `.kiro/hooks/*.json` files MUST use the v1 format; Kiro CLI V3 and IDE 1.0 skip a legacy IDE `when` / `then` document saved there
**Detection**: A `.json` hook file with no `hooks` array and a top-level `when` or `then`, or a `hooks` entry with `when` / `then` and no `trigger` / `action` (the "Current format (1.0)" example on the IDE 0.x reference page has this shape and is wrong). One diagnostic replaces the former KR-HK-011 plus KR-HK-012 pair.
**Fix**: No auto-fix. Rename the file to `.kiro.hook` to keep it as a legacy IDE hook (IDE 1.0 offers to upgrade it), or convert it: `when.type` becomes `trigger` (for example `fileEdited` becomes `PostFileSave`), `when.patterns` becomes a regex `matcher`, and `then` becomes `action`.
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/ide/whats-new-v1/hooks, kiro.dev/docs/ide/0x-reference

<a id="kr-hk-017"></a>
### KR-HK-017 [HIGH] Invalid Kiro V3 Hook Field Value
**Requirement**: `description` and `matcher` MUST be strings, `enabled` MUST be a boolean, and `timeout` MUST be a non-negative whole number of seconds (default 60, `0` disables it)
**Detection**: Error, because Kiro skips the whole file, for any other type, a fractional or a negative timeout. There is no upper bound: the engine passes `timeout` x 1000 ms to the process runner (agnix's former 3,600-second ceiling had no vendor basis and was removed). `timeout` is ignored for agent actions.
**Fix**: [AUTO-FIX, unsafe] `"true"` / `"false"` strings in `enabled` become booleans and plain integer strings in `timeout` become numbers. Unsafe because the file starts loading.
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-018"></a>
### KR-HK-018 [HIGH] Invalid Kiro V3 Hook Confirm Block
**Requirement**: `confirm` MUST be an object with a non-empty string `question` and a non-empty `options` array of `{ "id": non-empty string, "label": non-empty string, "run": boolean, "continueReason"?: string }`, plus an optional non-empty string `confirmCommand`
**Detection**: Error (Kiro skips the whole file) for any shape violation. Warning when a valid `confirm` sits on a hook that is not a Stop hook with a command action, where Kiro ignores it.
**Fix**: No auto-fix
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-019"></a>
### KR-HK-019 [HIGH] Kiro V3 Hook Matcher Is Not a Valid Regex
**Requirement**: A hook `matcher` MUST compile as a JavaScript regular expression, unless it is a glob selector on a tool trigger
**Detection**: The matcher fails to compile with ECMAScript syntax (checked with the `regress` engine, which never rejects a pattern V8 accepts). On PreToolUse / PostToolUse a matcher without the metacharacters `\ ^ $ ( ) [ ] { } | +` that contains `*` or `?` is kept as a glob selector and is not reported. Otherwise the engine logs "Hook matcher regex failed to compile" and the hook never fires, on any trigger. The usual cause is a glob such as `*.ts` or `src/**` on a file trigger.
**Fix**: No auto-fix (write a regex such as `\.ts$`)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/types

<a id="kr-hk-020"></a>
### KR-HK-020 [MEDIUM] Kiro V3 Hook Matcher Has No Effect
**Requirement**: A `matcher` SHOULD only be set on triggers that use it: PreToolUse, PostToolUse, PostFileCreate, PostFileSave and PostFileDelete
**Detection**: Warning for a non-empty, compilable matcher on any other trigger. The engine ignores it (logging "Hook matcher has no effect for this trigger") and the hook runs on every event; the hook-types page says "Stop Hooks do not use matchers." The IDE 1.0 page says UserPromptSubmit matches the prompt text; the engine does not. A matcher that does not compile disables the hook on any trigger (KR-HK-019).
**Fix**: No auto-fix (remove the matcher or filter inside the command)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/types

<a id="kr-hk-021"></a>
### KR-HK-021 [MEDIUM] Kiro V3 Tool Matcher Uses Non-V3 Tool Names
**Requirement**: Tool trigger matchers SHOULD use V3 tool ids or tags
**Detection**: Warning when a PreToolUse / PostToolUse matcher cannot match any V3 built-in tool id or tag under the V3 rules (a selector-shaped matcher matches ids or tags, with `*` / `?` globs and the execute_bash / execute_pwsh equivalence, and is also an unanchored regex over ids; any other matcher is only a regex over ids) and names Kiro CLI 2.x tools (`fs_read`, `execute_cmd`, `use_aws`, `aws`, `glob`, ...; the hook-types page: "V3 does not expand CLI 2.x tool names") or Claude Code tools (`Bash`, `Write`, `Edit`, `Task`, ...). Matchers that mention `@` or `mcp_` (MCP selectors and ids) are not checked.
**Fix**: No auto-fix (use ids such as `read_file`, `fs_write`, `execute_bash` or tags such as `read`, `write`, `shell`, `@mcp`)
**Source**: kiro.dev/docs/hooks/types, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-hk-022"></a>
### KR-HK-022 [MEDIUM] Misplaced or Misspelled Kiro V3 Hook Field
**Requirement**: Hook fields SHOULD be spelled and placed as documented; Kiro strips unknown keys silently
**Detection**: Warning, at the key, only where dropping the key changes what the hook does: `timeout` inside `action` (the hook uses the default 60 seconds), the CLI 2.x `timeout_ms` / `timeoutMs`, the IDE 0.x `patterns` / `toolTypes` (the hook runs on every event), hook-level `command` / `prompt` / `type` next to an `action`, and case variants of documented keys such as `Matcher`. Other unknown keys are not reported.
**Fix**: No auto-fix (move or rename the key)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-hk-023"></a>
### KR-HK-023 [MEDIUM] Kiro V3 Hook Command Uses Unsupported Placeholder
**Requirement**: Hook commands SHOULD read event data from the JSON on stdin; Kiro substitutes only `${WORKSPACE_ROOT}` in `action.command` and `$HOME` in `confirm.confirmCommand`, and adds only `KIRO_SESSION_ID` to the environment
**Detection**: Warning for `{{filePath}}`-style templates (filePath, file, fileName, prompt, toolName, toolInput and their snake_case forms; the IDE 1.0 docs example uses `{{filePath}}` but the V3 engine does not substitute it), bare `$WORKSPACE_ROOT` (the shell expands it to an empty string), `${WORKSPACE_ROOT}` in `confirmCommand`, and the variables `USER_PROMPT` (set by CLI 2.x only), `FILE_PATH`, `TOOL_NAME`, `TOOL_INPUT` and `TOOL_RESPONSE`. Hook commands run in a shell, so other `$VAR` and `${VAR}` references expand from the process environment as usual.
**Fix**: No auto-fix (for example `jq -r .file_path`)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/hooks/actions

<a id="kr-hk-024"></a>
### KR-HK-024 [MEDIUM] Kiro V3 Hook Never Runs
**Requirement**: Hooks SHOULD use a trigger and action combination that Kiro runs
**Detection**: Warning for a SessionEnd hook with an agent action (SessionEnd runs only command actions, within a 5-second budget). Info for a Manual (or `userTriggered`) hook: Kiro CLI V3 lists it but `/hooks` has no action to run it.
**Fix**: No auto-fix
**Source**: kiro.dev/docs/hooks/types

<a id="kr-hk-025"></a>
### KR-HK-025 [LOW] Standalone Hook File Ignored by Pinned Kiro CLI 2.x
**Requirement**: Projects that pin Kiro CLI 2.x SHOULD know that the default 2.x engine does not load `.kiro/hooks/*.json`
**Detection**: Info once per standalone hook file when `[tool_versions] kiro` selects the 2.x engine. The file is still validated, because Kiro IDE 1.0 and the V3 engine load it.
**Fix**: No auto-fix (define CLI 2.x hooks in an agent's `hooks` field)
**Source**: kiro.dev/docs/hooks, kiro.dev/docs/cli/v3/hooks-migration

<a id="kr-hk-027"></a>
### KR-HK-027 [LOW] Kiro Tool Hook Without a Specific Matcher
**Requirement**: PreToolUse / PostToolUse hooks SHOULD target specific tools ("Use precise `matcher` values ... rather than matching all tools with `"*"` when possible")
**Detection**: Info when a tool hook's matcher is missing, empty, `*`, `**` or a match-all regex such as `.*`; the engine also warns "Hook has no matcher and will run on every tool call". Deliberate audit-everything hooks are a known false positive, hence the info level. (The legacy IDE counterpart is KR-HK-004.)
**Fix**: No auto-fix (use a tag such as `shell` or `write`, an MCP selector such as `@github`, or an anchored regex)
**Source**: kiro.dev/docs/hooks/best-practices, kiro.dev/docs/hooks

<a id="kr-hk-028"></a>
### KR-HK-028 [LOW] Kiro File Hook Matching Every File
**Requirement**: File hooks SHOULD use specific file patterns ("Use specific file glob patterns (e.g., `src/**/*.ts`) rather than broad patterns that match everything")
**Detection**: Info for a PostFileCreate / PostFileSave / PostFileDelete hook (or alias) whose matcher is missing, empty or a match-all regex such as `.*`, and for a legacy `.kiro.hook` file event whose `when.patterns` are all match-all globs (`*`, `**`, `**/*`, `**/*.*`, `*.*`). Globs such as `*` on a V3 file trigger do not compile and are KR-HK-019 errors instead; a legacy file hook with no patterns is KR-HK-002.
**Fix**: No auto-fix (match the files the hook is for, for example `\.ts$`)
**Source**: kiro.dev/docs/hooks/best-practices, kiro.dev/docs/hooks

<a id="kr-hk-029"></a>
### KR-HK-029 [LOW] Kiro Tool Matcher Is an Unanchored Regex
**Requirement**: Regex tool matchers SHOULD be anchored ("A matcher that contains regex metacharacters is evaluated only as an unanchored regular expression against the tool ID. Anchor it (`^...$`) to match exactly.")
**Detection**: Info for a PreToolUse / PostToolUse matcher in regex form (it contains one of `\ ^ $ ( ) [ ] { } | +`) with an alternation or group that does not both start with `^` and end with `$`, for example `fs_write|str_replace`, which also matches any id containing those names. Not reported when KR-HK-021 already reports the matcher, or for file triggers.
**Fix**: No auto-fix (anchoring can break intentional substring matching such as `([Ss]hell|[Bb]ash)`)
**Source**: kiro.dev/docs/hooks/types

<a id="kr-hk-030"></a>
### KR-HK-030 [LOW] Kiro Hook File Name Not Kebab-Case
**Requirement**: Hook files SHOULD have descriptive kebab-case names ("Any `.json` filename works - use descriptive kebab-case names (e.g., `lint-on-save.json`, `guard-writes.json`)")
**Detection**: Info when the file name without `.json` has uppercase letters, underscores, spaces, other characters, or empty words. Dot-separated parts such as `lint.hook.json` are accepted. Style only: Kiro loads any `.json` name.
**Fix**: No auto-fix (rename the file)
**Source**: kiro.dev/docs/hooks

---

## KIRO PERMISSIONS RULES

Capability policies are read by the V3 engine (and Kiro IDE 1.0) from `~/.kiro/settings/permissions.yaml` (or `permissions.json`; the YAML file wins when both exist), from `~/.kiro/workspace-roots/<hash>/permissions.yaml` (workspace scope, stored outside the repository) and from an administrator `managed-settings.json` (`/Library/Application Support/Kiro/`, `/etc/kiro/`, `C:\ProgramData\Kiro\`; deny and ask only). `$KIRO_HOME` replaces `~/.kiro`. `permissions.yml` is never read (KIRO-026). A repository-local `.kiro/settings/permissions.yaml` is linted as a template; Kiro does not read it (KR-PM-009). The engine's parser is strict: a fatal problem drops every rule of the file and the session fails closed, denying all tool calls until the file is fixed. An unknown capability, the unsupported effect `evaluate` and an unknown preset only skip that entry. All corrections below are manual because permissions affect access.

<a id="kr-pm-001"></a>
### KR-PM-001 [HIGH] Invalid Kiro Permissions Document
**Requirement**: A permissions document MUST parse (YAML for `.yaml`/`.yml`, strict JSON otherwise) to a mapping with a `rules` array whose entries are mappings. `rules: []` is valid.
**Detection**: Flag parse errors (duplicate keys included), an empty or comment-only file, a non-mapping document, a missing or non-array `rules`, and non-mapping rule entries. Each is fatal: Kiro rejects the file and fails closed.
**Fix**: No auto-fix - repair the document shape without guessing access policy.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions, kiro.dev/docs/enterprise/governance/permissions

<a id="kr-pm-002"></a>
### KR-PM-002 [HIGH] Invalid Kiro Permission Capability
**Requirement**: Each rule MUST have a case-sensitive string `capability` the engine knows: `fs_read`, `fs_write`, `shell`, `web_fetch`, `web_search`, `mcp`, `subagent`, `skill`, `power`, `context`, `diagnostics`, `workflow` (accepted by the engine, not listed on kiro.dev), `sandbox_network`, or the meta-capabilities `all`, `builtin`, `filesystem`.
**Detection**: A missing or non-string capability is fatal for the file. An unknown capability string (for example `*` or `execute_bash`) makes Kiro skip that rule before reading its other fields; problems in the rest of such a rule are reported by KR-PM-003..005 as warnings because they become fatal once the capability is fixed.
**Fix**: No auto-fix - choose the intended capability after reviewing access scope.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions

<a id="kr-pm-003"></a>
### KR-PM-003 [HIGH] Invalid Kiro Permission Effect
**Requirement**: Each rule MUST have a string `effect` of `allow`, `deny`, or `ask`; an administrator `managed-settings.json` may use only `deny` and `ask`.
**Detection**: A missing, wrong-type or unknown effect (`permit`, `Allow`) and an `allow` rule in managed-settings.json are fatal (error). `effect: evaluate` is parsed but skipped as "not supported yet" (warning).
**Fix**: No auto-fix - explicitly select the intended access decision.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions, kiro.dev/docs/enterprise/governance/permissions

<a id="kr-pm-004"></a>
### KR-PM-004 [HIGH] Invalid Kiro Permission Patterns
**Requirement**: Optional `match` and `exclude` MUST be arrays of strings. Omitting `match` means every resource.
**Detection**: Flag a scalar, mapping, null, or any non-string array member; each is fatal for the file. `match: ["git *"]` is valid; `match: "git *"` is not.
**Fix**: No auto-fix - supply string arrays after reviewing their scope.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions

<a id="kr-pm-005"></a>
### KR-PM-005 [HIGH] Unknown Field in Kiro Permission Rule
**Requirement**: A rule MUST contain only `capability`, `effect`, `match` and `exclude`.
**Detection**: Any other key in a rule (`comment`, `note`, `description`) is fatal: the engine reports "Unknown field(s) ... in rule N" and rejects the file. kiro.dev documents the same for administrator files ("Rules with extra fields not in the schema cause the entire file to be rejected").
**Fix**: No auto-fix - remove the field or move the note into a YAML comment.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/enterprise/governance/permissions

<a id="kr-pm-006"></a>
### KR-PM-006 [HIGH] Invalid Kiro Permission Policy Presets
**Requirement**: The optional top-level `policies` list MUST be an array of preset id strings: `allow-all`, `edit-workspace`, `read-workspace`, `read-all`, `read-only-shell`, `dev-shell`.
**Detection**: A non-array `policies` or a non-string entry is fatal (error). An unknown preset id is skipped (warning). In managed-settings.json every preset is skipped because presets contain only allow rules (warning).
**Fix**: No auto-fix - list valid preset ids or write explicit rules.
**Source**: kiro.dev/docs/permissions ("Policy presets"), kiro.dev/docs/cli/v3/permissions

<a id="kr-pm-007"></a>
### KR-PM-007 [MEDIUM] Kiro Permission Pattern Has No Effect
**Requirement**: Patterns SHOULD use the resource form Kiro compares for the capability and target paths an allow rule can open.
**Detection**: Warn on an `mcp` pattern that starts with `@` (resources are `server/tool`; `@server` is the CLI 2.x tools selector form), a `web_fetch` pattern containing `://` or `/` (rules are compared with the URL host name, as read from the V3 engine; for example the upgrader's `https://docs.aws.amazon.com/*`), a `web_search` or `diagnostics` match pattern other than `*` (those requests carry no other resource), and an `fs_write` allow for `.git`, `.vscode`, `.kiroignore`, `*.code-workspace` or the `.kiro` agents, hooks, workflows, powers, settings and workspace-roots directories, which Kiro always asks about or denies whatever allow rule exists.
**Fix**: No auto-fix - rewrite the pattern (`@github` becomes `github/*`, `https://docs.example.com/*` becomes `docs.example.com`) or remove the ineffective allow.
**Source**: kiro.dev/docs/permissions ("Pattern matching", "Default behavior"), kiro.dev/docs/enterprise/governance/permissions

<a id="kr-pm-008"></a>
### KR-PM-008 [MEDIUM] Empty Kiro Permission match List Matches Everything
**Requirement**: A rule meant to cover specific resources SHOULD list them; `match: []` is treated like an omitted `match` (and like `["*"]`).
**Detection**: Warn on `match: []`, which applies the rule's effect to every resource of the capability (an `allow` rule allows everything).
**Fix**: No auto-fix - list the intended resources, or omit `match` to make the full scope explicit.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions

<a id="kr-pm-009"></a>
### KR-PM-009 [LOW] Kiro Does Not Read This Permissions File
**Requirement**: Policies take effect only in the locations Kiro reads.
**Detection**: Info for a permissions file outside `<kiro>/settings/` and `<kiro>/workspace-roots/<hash>/` (`<kiro>` = `~/.kiro`, `$KIRO_HOME`, or another account's `/home/<user>/.kiro` or `/Users/<user>/.kiro`), for a managed-settings.json outside its operating-system path, and for a `permissions.json` next to a `permissions.yaml` (the YAML file is read instead).
**Fix**: No auto-fix - install the policy where Kiro reads it, or keep the file as a template.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/cli/v3/permissions

<a id="kr-pm-010"></a>
### KR-PM-010 [MEDIUM] Unrestricted Kiro Allow Rule for a Write-Capable Capability
**Requirement**: Allow rules for `shell`, `fs_write`, `filesystem`, `mcp`, `all` and `builtin` SHOULD name the commands, paths or servers they cover ("Use specific patterns over wildcards", "Start restrictive").
**Detection**: Warn on an `allow` rule for one of these capabilities whose `match` is omitted or only `*` / `**` and that has no `exclude`. `match: []` is KR-PM-008.
**Fix**: No auto-fix - list what the agent needs, or add exclude patterns.
**Source**: kiro.dev/docs/permissions, kiro.dev/docs/custom-agents/configuration-reference ("Security best practices")

<a id="kr-pm-011"></a>
### KR-PM-011 [MEDIUM] Unsupported Pattern Syntax in Kiro Shell, Web or MCP Rule
**Requirement**: Shell, web and MCP patterns MUST use only `*` as a wildcard; `**`, `?` and character classes are not supported.
**Detection**: Warn on a `shell`, `web_fetch`, `web_search` or `mcp` pattern containing `**` (treated as `*`), `?` or `[...]` (matched literally). File-system patterns support these globs and are not checked.
**Fix**: No auto-fix - rewrite the pattern with `*`.
**Source**: kiro.dev/docs/permissions ("Pattern matching")

<a id="kr-pm-012"></a>
### KR-PM-012 [MEDIUM] Shadowed Kiro sandbox_network Deny
**Requirement**: A specific `sandbox_network` deny SHOULD come before a broader allow that also matches; host rules are first-match in order.
**Detection**: Within one file, warn on a `sandbox_network` deny whose host an earlier `sandbox_network` allow already matches (omitted match, `*`, or a `*` pattern such as `*.example.com`) without excluding it.
**Fix**: No auto-fix - move the deny before the allow.
**Source**: kiro.dev/docs/permissions ("Sandbox network rules")

---

## KIRO MCP RULES

Kiro reads MCP servers from `~/.kiro/settings/mcp.json` and `<workspace>/.kiro/settings/mcp.json`, from a legacy power's `mcp.json` (next to `POWER.md`) and from an Agent Plugins power's `mcp.json` (next to `plugin.json`). Kiro CLI 2.x (the V2 engine) parses settings files with strict JSON and rejects the whole file when any field has the wrong type; the V3 engine accepts comments and trailing commas and skips only an invalid server. Severity follows the targeted engines: a condition fatal in every targeted engine is an error, fatal in one engine a warning naming it. Pin the engine with `[tool_versions] kiro`. Powers are V3 only.

<a id="kr-mcp-001"></a>
### KR-MCP-001 [HIGH] Kiro MCP Config Fails to Load
**Requirement**: An mcp.json MUST parse and contain an `mcpServers` object, and each server MUST define `command` (local) or `url` (remote).
**Detection**: Error on invalid JSON, a missing or non-object `mcpServers` (Kiro loads no server from the file), and a server with neither `command` nor `url` (skipped; `"disabled": true` and `"type": "registry"` servers are exempt). Comments or trailing commas are accepted by the V3 engine in settings files but make Kiro CLI 2.x reject the file (warning naming V2, or an error when V2 is pinned). Power mcp.json files are read with `JSON.parse` (plain JSON); for a legacy power with a POWER.md, parse and `mcpServers` problems are reported once, by KR-PW-004 on the POWER.md.
**Fix**: No auto-fix - repair the JSON or define `command` or `url`.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-mcp-002"></a>
### KR-MCP-002 [MEDIUM] Hardcoded Secret in Kiro MCP Server Config
**Requirement**: Credentials in MCP `env`, `headers` and `args` SHOULD use `${VAR}` expansion instead of plaintext ("Never hardcode API keys or tokens in configuration files"; powers: "Use environment variables for API keys and secrets").
**Detection**: Warn when an `env` or `headers` entry whose name denotes a credential (API_KEY, SECRET, TOKEN, PASSWORD, PAT, Authorization, ...; not names ending in DIR, PATH, URL, NAME and similar) holds a value that looks like a real secret, and on command-line credentials in `args` (`--api-key=VALUE`, `--token VALUE`, `Authorization: Bearer VALUE`). References (`${VAR}`, `$VAR`, `%VAR%`), placeholders (`YOUR_API_KEY`, `your-token-here`, `<KEY>`, an upper-case variable name), file paths and values shorter than 8 characters are not flagged. A `Bearer` / `Basic` / `Token` scheme prefix is skipped. Settings, legacy power and Agent Plugins mcp.json files are checked.
**Fix**: No auto-fix - move the secret to an environment variable and reference it as `${NAME}`.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/docs/mcp/security, kiro.dev/docs/powers/create

<a id="kr-mcp-003"></a>
### KR-MCP-003 [MEDIUM] Kiro MCP Command Includes Arguments
**Requirement**: `command` SHOULD name only the executable; arguments belong in `args` (which is optional, `[]` included).
**Detection**: Warn when `command` contains whitespace and its first word has no path separator or quote (`"npx -y @scope/server"`). Kiro starts the command without a shell (Kiro CLI 2.x on every platform, the V3 engine through cross-spawn on macOS and Linux), so such a server fails to start. Paths with spaces and commands containing `${` are not flagged.
**Fix**: No auto-fix - split the command into `command` and `args`.
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-004"></a>
### KR-MCP-004 [HIGH] Invalid Kiro MCP Server URL
**Requirement**: A remote server `url` MUST be a valid `https://` URL, or `http://` for localhost, 127.0.0.1 or ::1.
**Detection**: Error on a URL that does not parse and on another scheme (`ws://`, `wss://`, `sse://`, `ftp://`) for a non-local host, which the V3 engine rejects and an HTTP transport cannot use. A non-HTTP scheme on a loopback host is a warning (the V3 engine accepts it at load, but kiro.dev documents only HTTPS and localhost HTTP). URLs containing `${VAR}` are not checked (expanded first). Checked only when `url` is the transport (no `command`).
**Fix**: No auto-fix - correct the URL.
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-005"></a>
### KR-MCP-005 [MEDIUM] Duplicate Kiro MCP Server Name in One File
**Requirement**: A server name SHOULD appear once per `mcpServers` object.
**Detection**: Warn on the earlier definition of a repeated name; JSON keeps the last one, so the earlier entry is silently ignored. Same-named servers in different files are documented overrides and are not flagged.
**Fix**: No auto-fix - remove or rename one definition.
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-006"></a>
### KR-MCP-006 [MEDIUM] Invalid OAuth Configuration
**Requirement**: An `oauth` block SHOULD be used on an HTTP(S) remote server. `clientId` and `clientSecret` SHOULD be non-empty, `clientSecret` requires `clientId`, `redirectUri` SHOULD be `:port`, `localhost:port`, `127.0.0.1:port` or `http://localhost:port[/path]` (port 1-65535, no query or fragment), and `clientMetadataUrl` SHOULD be an https URL with a path. `clientId` is optional (dynamic client registration).
**Detection**: Warn on the problems above, on `clientMetadataUrl` together with `clientId` ("When a `clientId` is present, Kiro uses it and never consults `clientMetadataUrl`") and on `clientMetadataUrl` without a pinned `redirectUri` ("an unpinned callback is rejected on every connect"). An invalid `redirectUri` or `clientMetadataUrl` makes the V3 engine skip the server (warning naming V3, error when V3 is pinned). Wrong value types are KR-MCP-007.
**Fix**: No auto-fix - correct the OAuth fields or remove the block.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/changelog/cli/2-12/

<a id="kr-mcp-007"></a>
### KR-MCP-007 [HIGH] Invalid Kiro MCP Server Field Type
**Requirement**: Server fields MUST have the documented types: `command`, `url`, `cwd`, `type` strings; `args`, `disabledTools`, `autoApprove`, `oauthScopes` arrays of strings; `env`, `headers` objects of strings; `disabled`, `waitForReady` booleans; `timeout` a whole number of milliseconds of at least 1; `versionNegotiation` `"auto"` or `"legacy"`; `oauth` an object with string `clientId`, `redirectUri`, `clientMetadataUrl`, `clientSecret` and a string-array `oauthScopes`.
**Detection**: Kiro CLI 2.x rejects the whole file on any wrong type it reads (`null` included, except for `env`, `type` and `oauth`), observed with `kiro-cli mcp list` 2.32.1; the V3 engine skips the server (`null` never passes). Severity follows the engines each value fails in, for example `timeout: 0` fails only in V3 and `timeout: 1.5` only in V2. A server entry that is not an object is an error.
**Fix**: No auto-fix - write the value with the documented type.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-mcp-008"></a>
### KR-MCP-008 [HIGH] Kiro MCP Remote URL Uses Plain HTTP
**Requirement**: A remote server on a host other than localhost, 127.0.0.1 or ::1 MUST use `https://`.
**Detection**: The V3 engine skips such a server ("must use https or a localhost url"); Kiro CLI 2.x does not check. Warning naming V3 when both engines are targeted, error when V3 is pinned, nothing when 2.x is pinned.
**Fix**: No auto-fix - switch the endpoint to HTTPS.
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-009"></a>
### KR-MCP-009 [MEDIUM] Unexpanded Variable Reference in Kiro MCP Config
**Requirement**: Environment references SHOULD use `${NAME}`, the only form Kiro expands.
**Detection**: Warn on a bare `$NAME`, `${env:NAME}` / `${input:...}` and VS Code variables such as `${workspaceFolder}` in `command`, `args`, `url`, `cwd`, `env` and `headers` values; the server receives the text literally. Arguments passed to a shell command (`bash -c ...`) are not checked. Skipped when only Kiro CLI 2.x is targeted (its expansion beyond `${NAME}` is undocumented).
**Fix**: No auto-fix - use `${NAME}` or write the value out.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/changelog/cli/2-4/

<a id="kr-mcp-010"></a>
### KR-MCP-010 [LOW] Unknown Kiro MCP Server Field
**Requirement**: Server entries SHOULD use only fields Kiro reads.
**Detection**: Info for any other field (`transport`, `description`, typos such as `evn`), with a suggestion when it is close to a known field. Kiro ignores these fields.
**Fix**: No auto-fix - remove the field or correct its spelling.
**Source**: kiro.dev/docs/mcp/configuration, kiro.dev/docs/custom-agents/configuration-reference

<a id="kr-mcp-011"></a>
### KR-MCP-011 [LOW] Kiro MCP Server Has Both command and url
**Requirement**: A server SHOULD define either `command` or `url`.
**Detection**: Info when both are set; the V3 engine starts the command and ignores `url`.
**Fix**: No auto-fix - keep the transport you intend.
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-012"></a>
### KR-MCP-012 [HIGH] Agent Plugins mcp.json Schema Missing or Mismatched
**Requirement**: A power's Agent Plugins mcp.json MUST set `$schema` to `https://agent-plugins.org/schemas/1.x.y/mcp.schema.json` with the same version as its plugin.json.
**Detection**: Error when `$schema` is missing, not an Agent Plugins MCP schema, or targets another version than plugin.json; Kiro then loads none of the power's servers.
**Fix**: No auto-fix - set the matching `$schema`.
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/mcp.schema.json

<a id="kr-mcp-013"></a>
### KR-MCP-013 [HIGH] Unknown Top-Level Field in Agent Plugins mcp.json
**Requirement**: A power's Agent Plugins mcp.json MUST contain only `$schema` and `mcpServers` at the top level.
**Detection**: Error on any other top-level key; Kiro disables all of the power's MCP servers.
**Fix**: No auto-fix - remove the field.
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/mcp.schema.json

<a id="kr-mcp-014"></a>
### KR-MCP-014 [HIGH] Invalid Agent Plugins MCP Server Entry
**Requirement**: Each Agent Plugins server MUST set `type` to `stdio`, `streamable-http` or `sse`. A stdio server takes only `type`, `command` (a bare executable name or a `./` path inside the power), `args`, `env` (no `PLUGIN_ROOT` / `PLUGIN_DATA`) and `cwd` (starting with `./`, `${PLUGIN_ROOT}` or `${PLUGIN_DATA}`, without `..`). A remote server takes only `type`, `url` (http or https, https for a non-local host, no user info or fragment) and `headers` (strings, no names differing only in case). Kiro-specific fields such as `disabled`, `autoApprove` and `disabledTools` are not part of the format.
**Detection**: Error on the first reason the loader skips the server.
**Fix**: No auto-fix - follow the Agent Plugins MCP schema.
**Source**: kiro.dev/docs/powers/create, agent-plugins.org/schemas/1.0.0/mcp.schema.json, github.com/kirodotdev/powers power-builder migrate-plugin

<a id="kr-mcp-015"></a>
### KR-MCP-015 [MEDIUM] Kiro MCP autoApprove Wildcard
**Requirement**: `autoApprove` SHOULD list specific trusted tools; Kiro advises auto-approving only tools that do not have write access to sensitive systems, come from trusted sources and have limited scope.
**Detection**: Warn when `autoApprove` contains `"*"`, which approves every tool of the server without prompting (Kiro IDE; Kiro CLI V3 approves MCP tools through permission rules and ignores `autoApprove`).
**Fix**: No auto-fix - list the tool names to auto-approve.
**Source**: kiro.dev/docs/mcp/security ("Auto-approval guidelines"), kiro.dev/docs/mcp/configuration

---

## KIRO CLI SETTINGS RULES

Rules for the Kiro CLI settings files `~/.kiro/settings/cli.json` (global, `$KIRO_HOME/settings/cli.json` when set) and `<workspace>/.kiro/settings/cli.json`: a flat JSON object keyed by dotted setting names that `kiro-cli settings <key> <value>` and `/settings` write. The key list and types are those of `kiro-cli settings list --all` 2.32.1 (140 keys). A wrongly typed value is not coerced; Kiro keeps the default. Kiro CLI does not read `.kiro/settings.json`.

<a id="kr-set-001"></a>
### KR-SET-001 [MEDIUM] Invalid toolSearch.enabled Value
**Requirement**: `toolSearch.enabled` SHOULD be a boolean (true or false); other values are not read.
**Detection**: Warn on a non-boolean top-level `toolSearch.enabled` (the last occurrence when the key repeats).
**Fix**: [AUTO-FIX, unsafe] Replace an exact quoted `"true"`/`"false"` (any case) with the JSON boolean, on the value span of the top-level key. Unsafe because Kiro ignores the quoted value today, so the conversion activates the setting.
**Source**: kiro.dev/docs/reference/settings, kiro.dev/docs/mcp/tool-search/

<a id="kr-set-002"></a>
### KR-SET-002 [MEDIUM] Invalid toolSearch.minPct Value
**Requirement**: `toolSearch.minPct` SHOULD be a number from 0 to 100 (default 5; 0 means always active; the V3 engine accepts only 0 to 100).
**Detection**: Warn on a non-number, a negative value and a value above 100.
**Fix**: [AUTO-FIX, unsafe] Replace a quoted plain decimal (`"5"`, `"2.5"`) with the bare number. Exponents, signs, leading zeros and out-of-range values are left manual, so the result always parses.
**Source**: kiro.dev/docs/reference/settings, kiro.dev/docs/mcp/tool-search/

<a id="kr-set-003"></a>
### KR-SET-003 [MEDIUM] Invalid toolSearch.minTokens Value
**Requirement**: `toolSearch.minTokens` SHOULD be a non-negative integer (default 50000; 0 means always active).
**Detection**: Warn on a non-number, a negative value and a fractional value.
**Fix**: [AUTO-FIX, unsafe] Replace a quoted plain integer (`"50000"`) with the bare integer.
**Source**: kiro.dev/docs/reference/settings, kiro.dev/docs/mcp/tool-search/

<a id="kr-set-004"></a>
### KR-SET-004 [HIGH] Unparseable Kiro CLI Settings File
**Requirement**: cli.json MUST be strict JSON with a top-level object.
**Detection**: Error on a syntax error, comments, trailing commas, an empty file or a non-object root. kiro-cli 2.32.1 then exits with "failed to parse" for every command, chat included, for a global or a workspace file.
**Fix**: No auto-fix - repair the JSON.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-005"></a>
### KR-SET-005 [LOW] Unknown Kiro CLI Setting
**Requirement**: Keys SHOULD be Kiro CLI settings (`kiro-cli settings` refuses to write other keys).
**Detection**: Info for any other top-level key (`chat.autoCompact`, third-party keys), with a suggestion for a close or known spelling. Kiro ignores these keys.
**Fix**: No auto-fix - correct or remove the key.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-006"></a>
### KR-SET-006 [MEDIUM] Config Section Misplaced in Kiro CLI Settings
**Requirement**: Sections of other Kiro files SHOULD NOT be pasted into cli.json.
**Detection**: Warn on top-level `mcpServers`/`powers` (belongs in .kiro/settings/mcp.json), `hooks` (agent or .kiro/hooks), `rules`/`permissions`/`policies` (permissions.yaml) and agent fields (`tools`, `allowedTools`, `toolsSettings`, `resources`, `prompt`, `includeMcpJson`); they have no effect here.
**Fix**: No auto-fix - move the section to the file Kiro reads it from.
**Source**: kiro.dev/docs/reference/settings, kiro.dev/docs/mcp/configuration

<a id="kr-set-007"></a>
### KR-SET-007 [MEDIUM] Nested Kiro CLI Settings Object
**Requirement**: Settings SHOULD be written as flat dotted keys.
**Detection**: Warn on a top-level object such as `{"toolSearch": {"enabled": true}}` whose nested paths are settings; Kiro reads only `"toolSearch.enabled"`, so the nested values stay unset (observed with kiro-cli 2.32.1).
**Fix**: No auto-fix - rewrite each setting as a dotted key.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-008"></a>
### KR-SET-008 [MEDIUM] Kiro CLI Setting Has Wrong Value Type
**Requirement**: Each setting SHOULD have its declared type (boolean, number, string, array of strings, object).
**Detection**: Warn when a known setting other than `toolSearch.*` has another type, for example `"chat.enableKnowledge": "yes"` or `"cleanup.periodDays": "7d"`. Kiro does not read the value and keeps the default.
**Fix**: [AUTO-FIX, unsafe] Convert an exact quoted `"true"`/`"false"` for a boolean setting, or a quoted plain integer for a number setting, on the exact value span. Unsafe because it activates a value Kiro ignores today.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-009"></a>
### KR-SET-009 [MEDIUM] Unrecognized Kiro CLI Setting Value
**Requirement**: Settings with documented values SHOULD use one of them.
**Detection**: Warn on values outside the documented sets: `chat.agentEngine` (v1, v2, v3), `chat.uiMode` (lite, tui, structured), `chat.ui` (legacy, classic, tui), `chat.notificationMethod`, `chat.screenReader`, `chat.historyMode`, `chat.defaultInterruptBehavior`, `chat.modelPicker.sortBy`, `chat.sessionDashboard.scope`, `memory.mode`, `compaction.excludeContextWindowPercent` (0-100) and `chat.fullscreenWheelRows` (1-3).
**Fix**: No auto-fix - choose a documented value.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-010"></a>
### KR-SET-010 [MEDIUM] Duplicate Kiro CLI Setting Key
**Requirement**: A key SHOULD appear once.
**Detection**: Warn on the earlier occurrence of a repeated top-level key; Kiro keeps only the last value (observed with kiro-cli 2.32.1). Other KR-SET rules check the last value.
**Fix**: No auto-fix - remove one occurrence.
**Source**: kiro.dev/docs/reference/settings

<a id="kr-set-011"></a>
### KR-SET-011 [LOW] Global-Only Kiro CLI Setting in Workspace File
**Requirement**: Global-only settings SHOULD be set in `~/.kiro/settings/cli.json`.
**Detection**: Info when a workspace `.kiro/settings/cli.json` (any cli.json outside `~/.kiro` or `$KIRO_HOME`) sets one of the 76 keys `kiro-cli settings --workspace` refuses ("cannot be overridden at workspace level"), for example `chat.uiMode`, `chat.agentEngine` or `telemetry.enabled`; Kiro ignores the workspace value.
**Fix**: No auto-fix - move the key to the global file.
**Source**: kiro.dev/docs/reference/settings


<a id="kr-wf-001"></a>
### KR-WF-001 [HIGH] Workflow Recipe Exceeds Kiro Limits
**Requirement**: A recipe MUST have at most 50 `step` nodes, nest nodes at most 8 levels deep, give every node a unique `id`, and use at most 50 `include` nodes.
**Detection**: Count step nodes, track nesting depth (a root-level node is depth 1) and node ids across the static recipe; report the first excess step, the first too-deep node, each duplicate id (at the duplicate), and an include count above 50.
**Fix**: No auto-fix (split the recipe or rename nodes)
**Source**: kiro.dev/docs/workflows/authoring ("Node types", "Validate before launch")

<a id="kr-wf-002"></a>
### KR-WF-002 [HIGH] Invalid Workflow Repeat Cap
**Requirement**: Every `repeat` MUST set `maxIterations` (a positive integer, at most 1000) and `onMaxIterations` (`abort`, `continue` or `pause`), and MUST NOT set both `stopCondition` and `stopWhen`.
**Detection**: Check each repeat node for the two required fields, the integer range and the enum, and for both stop forms.
**Fix**: No auto-fix (choose the cap and its behavior deliberately)
**Source**: kiro.dev/docs/workflows/authoring ("Node types", "Repeat", "Define stop conditions", "Authoring checklist")

<a id="kr-wf-003"></a>
### KR-WF-003 [MEDIUM] Workflow Recipe Suffix Conflict
**Requirement**: A recipe name SHOULD use one suffix. When `<name>.workflow.json` exists, Kiro loads it and reports `<name>.workflow.yaml` / `.workflow.yml` as a conflict.
**Detection**: For a `.workflow.yaml` or `.workflow.yml` recipe, check for a `.workflow.json` sibling with the same stem.
**Fix**: No auto-fix (delete or rename one file)
**Source**: kiro.dev/docs/workflows/authoring (introduction)

<a id="kr-wf-004"></a>
### KR-WF-004 [HIGH] Invalid Workflow Recipe Document
**Requirement**: A recipe MUST parse (JSON for `.workflow.json`, YAML otherwise) and match the workflow schema, or Kiro does not load it: an object with a string `name` and a `steps` array; `inputs` an object of string type hints; each node an object with a `type` of `step`, `sequence`, `repeat`, `parallel`, `watch` or `include`, a string `id`, and its required fields (`step`: `agent`, `prompt`; `sequence`/`repeat`: `steps`; `parallel`: `branches`, `joinPolicy` of `all`, `allSettled` or `any`; `watch`: `handler`; `include`: `path`).
**Detection**: Report parse errors at the parser's location and schema violations at the node: wrong types (`captureOutput`, `artifacts`, `modelId`, `effortLevel`, `config`, `idleTimeoutSec`), the removed step `input` field, unknown keys and malformed ids or paths in `include` nodes (absolute path, `bundled://<name>` or `generated://<id>`; relative paths are rejected).
**Fix**: No auto-fix
**Source**: kiro.dev/docs/workflows/authoring ("Root fields", "Node types", "Configure a step"); Kiro V3 recipe schema

<a id="kr-wf-005"></a>
### KR-WF-005 [HIGH] Interactive Workflow Step Inside Parallel
**Requirement**: A step with `completion` (interactive) MUST NOT be nested inside a `parallel` node.
**Detection**: Report a step that has `completion` and a `parallel` ancestor.
**Fix**: No auto-fix (move the step outside the parallel node)
**Source**: kiro.dev/docs/workflows/authoring ("Parallel")

<a id="kr-wf-006"></a>
### KR-WF-006 [HIGH] Invalid Workflow Stop Condition
**Requirement**: A `stopCondition` or step `completion` MUST set at least one of `containsText`, `fileCheck` or `completionSignal` (`success`, `need_input`, `error`); a `fileCheck` takes exactly `path`, `jsonPath` (a dotted key path) and `value` (not an empty array). A `stopWhen` MUST be `<watchId>.terminal` naming a watch node of the recipe, or `{{expr}} contains <text>`.
**Detection**: Validate each stop condition, completion and `stopWhen` against those forms.
**Fix**: No auto-fix
**Source**: kiro.dev/docs/workflows/authoring ("Define stop conditions"); Kiro V3 `stopWhen` parser

<a id="kr-wf-007"></a>
### KR-WF-007 [HIGH] Invalid Workflow Template Reference
**Requirement**: A structured template (`{{previous.output}}`, `{{<id>.output}}`, `{{steps.<id>.output}}`, `{{artifacts.<name>}}`) MUST point to a producer that runs before the consumer, or the consuming step fails. `{{previous.output}}` needs an earlier sibling that captures output and cannot be used inside a parallel branch or a stop condition. A bare `{{name}}` that is not a declared input stays literal text.
**Detection**: Apply Kiro's happens-before rule to step prompts, artifact paths and stop-condition templates (errors); warn on undeclared bare inputs in step prompts, as Kiro does.
**Fix**: No auto-fix
**Source**: kiro.dev/docs/workflows/authoring ("Use template variables")

<a id="kr-wf-008"></a>
### KR-WF-008 [HIGH] Invalid Workflow Watch Configuration
**Requirement**: A `command` watch MUST set `config.command` to one static command line (no `{{...}}` template, no `args` key) and poll no faster than every 10 seconds; `commandTimeoutSec` is positive. A `github-pr` watch MUST set `config.prRef` or `config.url` and poll no faster than every 30 seconds.
**Detection**: Check the config of watch nodes with the built-in handlers; other handlers are checked by Kiro when the run is created.
**Fix**: No auto-fix
**Source**: kiro.dev/docs/workflows/authoring ("Watch")

<a id="kr-wf-009"></a>
### KR-WF-009 [MEDIUM] Workflow Loop Stop File Never Written
**Requirement**: The `fileCheck` file of a repeat's stop condition SHOULD be written by a step inside the loop, or the loop never stops before its cap.
**Detection**: Warn when no step prompt or artifact path inside the repeat contains the `fileCheck` path (Kiro logs the same warning).
**Fix**: No auto-fix
**Source**: kiro.dev/docs/workflows/authoring ("Repeat"); Kiro V3 recipe validation

---

## KIRO IGNORE FILE RULES

Rules for `.kiroignore` (workspace root or any subdirectory; Kiro CLI V3 and IDE) and the global `~/.kiro/settings/kiroignore`. Both use gitignore syntax.

<a id="kr-ign-001"></a>
### KR-IGN-001 [MEDIUM] Ineffective Kiro Ignore Negation
**Requirement**: A negation SHOULD NOT try to re-include a file below an excluded directory: "You cannot re-include a file if a parent directory is excluded."
**Detection**: For a negation that names its parent directories literally (`!secrets/public.txt`), report it when the file's other patterns leave one of those directories excluded (`secrets/`, `/secrets`, `**/secrets`), with gitignore semantics: the last matching pattern wins and shorter directories are checked first.
**Fix**: No auto-fix (exclude the directory's contents, for example `secrets/*`, instead of the directory)
**Source**: kiro.dev/docs/kiroignore ("Pattern syntax")

---

## UNIVERSAL RULES (XML)

<a id="xml-001"></a>
### XML-001 [MEDIUM] Unclosed XML Tag
**Requirement**: XML tags used to structure a prompt SHOULD be closed. Anthropic recommends tags as structure ("Use consistent, descriptive tag names across your prompts." / "Nest tags when content has a natural hierarchy"); no tool rejects an unbalanced tag, but an unclosed one leaves the rest of the file inside it.
**Detection**: Parse tags, check balance with stack
**Fix**: [AUTO-FIX, unsafe] Insert the matching closing tag at the end of the content
**Source**: platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices

<a id="xml-002"></a>
### XML-002 [MEDIUM] Mismatched Closing Tag
**Requirement**: A closing tag SHOULD match the open tag
**Detection**: `stack.last().name != closing_tag.name`
**Fix**: Replace with correct closing tag
**Source**: XML parsing standard (w3.org/TR/xml), platform.claude.com prompting best practices

<a id="xml-003"></a>
### XML-003 [MEDIUM] Unmatched Closing Tag
**Requirement**: A closing tag SHOULD have a corresponding opening tag
**Detection**: `stack.is_empty() && found_closing_tag`
**Fix**: Remove or add opening tag
**Source**: XML parsing standard (w3.org/TR/xml), platform.claude.com prompting best practices

---

## UNIVERSAL RULES (REFERENCES)

<a id="ref-001"></a>
### REF-001 [HIGH] Import File Not Found
**Requirement**: @import references MUST point to existing files
**Detection**: Resolve path, check existence
**Fix**: Show resolved path, suggest alternatives
**Source**: code.claude.com/docs/en/memory, agentskills.io

<a id="ref-002"></a>
### REF-002 [HIGH] Broken Markdown Link
**Requirement**: Markdown links SHOULD point to existing files
**Detection**: Extract `[text](path)`, check existence
**Fix**: Show available files
**Source**: Standard markdown validation

<a id="ref-003"></a>
### REF-003 [MEDIUM] Duplicate Import
**Requirement**: Each @import path SHOULD appear only once per file
**Detection**: Extract @imports, normalize paths (strip `./` prefix), flag duplicates
**Fix**: [AUTO-FIX] Remove the duplicate @import line
**Source**: Claude Code memory docs

---

## PROMPT ENGINEERING RULES

Each rule runs only on files whose vendor documents its advice: CLAUDE.md and CLAUDE.local.md get PE-002, PE-004, PE-005 and PE-006; AGENTS.md, AGENTS.local.md and AGENTS.override.md get PE-004 and PE-005; Cursor rule files get PE-004; GEMINI.md gets none (Gemini advises "**Use negative constraints:** Explicitly telling the agent what _not_ to do ... is often more effective than vague positive instructions.").

<a id="pe-002"></a>
### PE-002 [LOW] Chain-of-Thought on Simple Task
**Requirement**: SHOULD NOT add "think step by step" to a simple operation. Anthropic: "**Prefer general instructions over prescriptive steps.** A prompt like "think thoroughly" often produces better reasoning than a hand-written step-by-step plan."
**Detection**: A chain-of-thought phrase within 5 lines of a simple task (file reads, basic commands) in CLAUDE.md or CLAUDE.local.md
**Fix**: Remove the step-by-step instruction
**Source**: platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices

<a id="pe-004"></a>
### PE-004 [MEDIUM] Ambiguous Instructions
**Requirement**: Instructions SHOULD be specific. Claude Code: "Write instructions that are concrete enough to verify". Cursor: "Avoid vague guidance". Codex: "A short, accurate AGENTS.md is more useful than a long file full of vague rules." Cline: "**Be specific, not vague.**"
**Detection**: Hedges such as usually, sometimes, if possible, when appropriate outside code and parentheses, on lines that read as instructions (a directive word such as should, must, always, prefer, avoid, or an imperative verb at the start), in CLAUDE.md, CLAUDE.local.md, AGENTS.md variants and Cursor rules. Descriptions, headings and table rows are skipped.
**Fix**: Replace the hedge with a condition that can be checked
**Source**: code.claude.com/docs/en/memory, cursor.com/docs/rules, learn.chatgpt.com/guides/best-practices, docs.cline.bot/customization/cline-rules

<a id="pe-005"></a>
### PE-005 [MEDIUM] Redundant Generic Instructions
**Requirement**: Instructions SHOULD NOT include generic directives. Claude Code's best practices exclude "Self-evident practices like "write clean code"". Windsurf: "There's no need to add generic rules (e.g. "write good code")". VS Code: "Avoid generic advice that applies to any project." Copilot code review: "Vague quality improvements" such as "Be more accurate" "add noise". Response-length instructions such as "be concise" are not flagged: Anthropic now says to "Prompt explicitly for conciseness" for Claude Opus 5.
**Detection**: Phrases like "be helpful", "be accurate", "follow instructions", "do your best", "write clean code", "write good code" outside code blocks, in CLAUDE.md, CLAUDE.local.md and AGENTS.md variants
**Fix**: [AUTO-FIX, unsafe] Delete the line, offered only when the line holds nothing but generic phrases
**Source**: code.claude.com/docs/en/best-practices, docs.devin.ai/desktop/cascade/memories, code.visualstudio.com/docs/agents/guides/customize-copilot-guide, docs.github.com/en/copilot/tutorials/customize-code-review

<a id="pe-006"></a>
### PE-006 [MEDIUM] Negative-Only Instructions
**Requirement**: In Claude memory files, a negative instruction SHOULD say what to do instead: "**Tell Claude what to do instead of what not to do**". Not applied to GEMINI.md or AGENTS.md, since Gemini advises the opposite.
**Detection**: Check for "don't/never/avoid" without "instead/rather/prefer" within a 3-line window, in CLAUDE.md and CLAUDE.local.md. A second clause on the same line after a dash, semicolon, colon, full stop or "because" counts as the alternative or the reason.
**Fix**: Add positive alternative (e.g., "Instead, use..."), or give the reason
**Source**: platform.claude.com/docs/en/build-with-claude/prompt-engineering/claude-prompting-best-practices

---

## CROSS-PLATFORM RULES

<a id="xp-001"></a>
### XP-001 [LOW] Platform-Specific Feature in Generic Config
**Requirement**: Generic configs MUST NOT use platform-specific features
**Detection**: Check for Claude-only features (hooks, context: fork) in AGENTS.md
**Fix**: Move to CLAUDE.md or wrap in a Claude-specific section header
**Example**: Valid guarded section:
```markdown
## Claude Code Specific
- type: PreToolExecution
  command: echo "lint"
context: fork
agent: reviewer
```
**Source**: multi-platform research

<a id="xp-002"></a>
### XP-002 [MEDIUM] AGENTS.md Platform Compatibility
**Requirement**: AGENTS.md is a widely-adopted standard used by multiple platforms
**Supported Platforms**:
- Codex CLI (OpenAI)
- OpenCode
- GitHub Copilot coding agent
- Cursor (alongside `.cursor/rules/`)
- Cline (alongside `.clinerules`)
**Note**: Claude Code uses `CLAUDE.md` (not AGENTS.md)
**Detection**: Validate AGENTS.md follows markdown conventions
**Fix**: Ensure AGENTS.md is valid markdown with clear sections
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md, opencode.ai/docs/rules, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

<a id="xp-003"></a>
### XP-003 [MEDIUM] Hard-Coded Platform Paths
**Requirement**: Paths SHOULD use environment variables
**Detection**: Check for tool directories (`.claude/`, `.opencode/`, `.cursor/`, `.cline/`, `.github/copilot/`) in every validated file. Machine paths (`~/.<dir>/`, `~/Library/`, `/Users/<name>/`, `/home/<name>/`, `C:\Users\<name>\`) are checked only in files agents load, not in generic Markdown, where they mostly document where a tool keeps its data
**Fix**: Use `$CLAUDE_PROJECT_DIR` or equivalent
**Source**: multi-platform best practices

<a id="xp-004"></a>
### XP-004 [MEDIUM] Conflicting Build/Test Commands
**Requirement**: Instruction files SHOULD use consistent package managers
**Detection**: Extract build commands (npm/pnpm/yarn/bun) from multiple instruction files, detect conflicts when different managers are used for the same command type
**Fix**: Standardize on a single package manager across all instruction files
**Source**: cross-layer consistency best practices

<a id="xp-005"></a>
### XP-005 [HIGH] Conflicting Tool Constraints
**Requirement**: Tool constraints MUST NOT conflict across instruction layers
**Detection**: Extract tool allow/disallow patterns from multiple instruction files, detect when one file allows a tool and another disallows it
**Fix**: Resolve the conflict by consistently allowing or disallowing the tool
**Source**: cross-layer consistency requirements

<a id="xp-006"></a>
### XP-006 [MEDIUM] Multiple Layers Without Documented Precedence
**Requirement**: When multiple instruction layers exist, precedence SHOULD be documented
**Detection**: Detect multiple instruction files (CLAUDE.md, AGENTS.md, .cursor/rules/, etc.) without documented precedence
**Fix**: Document which file takes precedence (e.g., "CLAUDE.md takes precedence over AGENTS.md")
**Source**: multi-platform clarity requirements

<a id="xp-007"></a>
### XP-007 [MEDIUM] AGENTS.md Exceeds Codex Byte Limit
**Requirement**: AGENTS.md SHOULD stay under Codex CLI's 32768-byte `project_doc_max_bytes` default. `AGENTS.override.md` is checked too, since Codex reads it first in each directory and it draws on the same budget.
**Detection**: Check byte length of `AGENTS.md` / `AGENTS.override.md` content against the 32768-byte threshold. The documented cap is cumulative across the root-to-cwd chain; that dimension is covered by [XP-009](#xp-009), while this rule catches a single file that blows the limit on its own.

Known asymmetry with XP-009: this is a per-file validator, so it has no view of which files Codex actually loads. An `AGENTS.md` shadowed by an `AGENTS.override.md` in the same directory is never read by Codex, and XP-009 correctly excludes its bytes - but XP-007 still reports it on size alone. Resolving that needs the project-level shadowing model XP-009 uses, which a per-file rule cannot reach.
**Fix**: Reduce content or split into multiple files using @import
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md

<a id="xp-008"></a>
### XP-008 [MEDIUM] Claude-specific Features in CLAUDE.md for Cursor
**Requirement**: CLAUDE.md SHOULD guard Claude-specific features under a `## Claude Code` section when targeting Cursor
**Detection**: When target tool is Cursor, check CLAUDE.md and CLAUDE.local.md for Claude-specific directives (context:fork, agent fields, allowed-tools, hooks, @import) outside guarded sections
**Fix**: No auto-fix - move Cursor-compatible instructions to .cursor/rules/ or guard Claude-specific content under a `## Claude Code` section header
**Source**: docs.cursor.com/context/rules-for-ai

<a id="xp-009"></a>
### XP-009 [MEDIUM] Codex Instruction Chain Exceeds project_doc_max_bytes
**Requirement**: The *combined* size of the Codex instruction chain SHOULD stay under the 32768-byte `project_doc_max_bytes` default. The cap is cumulative, not per-file - Codex "stops adding files once the combined size reaches the limit", so a project split across several mid-size files is truncated even when every individual file passes XP-007.
**Detection**: Project-level check. Builds **one chain per leaf directory** the way Codex discovers them - start at the project root, walk down, at most one file per directory, preferring `AGENTS.override.md` over `AGENTS.md` - then sums each chain independently and reports on the file where that chain's running total crosses the limit, since that file and everything deeper is what Codex drops. Sibling subtrees are separate chains and are never summed together: a single whole-tree total would blame a sibling for bytes it does not share, and would eventually trip on any project with enough packages - nested instruction directories are supported, not a defect. A leaf inside a nested Git repository starts its chain at that repository, since Codex starts at the Git root. A chain of one file over the cap is left to XP-007, which already reports it. Conservative by design: `project_doc_fallback_filenames` lives in the user's Codex config rather than anything agnix reads, so a project using fallbacks has a longer real chain and this check under-reports rather than over-reports.
**Fix**: Manual - trim the chain, or raise `project_doc_max_bytes` in the Codex config
**Source**: learn.chatgpt.com/docs/agent-configuration/agents-md


<a id="xp-sk-001"></a>
### XP-SK-001 [LOW] Skill Uses Client-Specific Features
**Requirement**: Skills SHOULD avoid client-specific frontmatter fields for maximum portability
**Detection**: Skill frontmatter uses extension fields (model, context, agent, hooks, etc.) that are not part of the universal Agent Skills spec. Not reported on Kiro skills while KR-SK-001 is enabled, which reports the same fields.
**Fix**: No auto-fix - review whether the field is needed or can be removed for portability
**Source**: agentskills.io/specification

<a id="xp-sk-002"></a>
### XP-SK-002 [MEDIUM] Hardcoded User Directory Path In Skill
**Requirement**: A skill owned by a client other than Claude Code SHOULD NOT contain hardcoded user-home paths (`/Users/<name>/`, `/home/<name>/`, `C:\Users\<name>\`). The Agent Skills spec says to reference other files with relative paths from the skill root; a home path leaks the author's identity and does not resolve on another machine. This is the client-neutral twin of CC-SK-021, which covers Claude Code and unscoped skills, so a path is reported by exactly one of the two rules.
**Detection**: Same scan as CC-SK-021: the SKILL.md body, `.md` bodies bundled in the skill directory, and bundled scripts, skipping placeholder names such as `<name>`, `user` and `...`.
**Fix**: Manual - use a path relative to the skill root, `~/`, `$HOME/`, or an environment variable.
**Source**: agentskills.io/specification

---

## VERSION AWARENESS RULES (VER)

<a id="ver-001"></a>
### VER-001 [LOW] No Tool/Spec Versions Pinned
**Requirement**: Projects SHOULD pin tool/spec versions for deterministic validation
**Detection**: Check if any versions are configured in .agnix.toml [tool_versions] or [spec_revisions]
**Fix**: Add version configuration to .agnix.toml:
```toml
[tool_versions]
claude_code = "2.1.3"

[spec_revisions]
mcp_protocol = "2025-11-25"
```
**Source**: Best practice for reproducible validation

---

## PRIORITY MATRIX

### P0 (MVP - Week 3)
Implement these 30 rules first:
- AS-001 through AS-009 (Skills frontmatter)
- CC-SK-001 through CC-SK-008 (Claude skills)
- CC-HK-001 through CC-HK-008 (Hooks)
- CC-MEM-001 (Memory critical)
- XML-001 through XML-003 (XML balance)
- REF-001 through REF-003 (Import/reference validation)

### P1 (Week 4)
Add these 15 rules:
- AS-011 through AS-013 and AS-015 (Skills best practices)
- CC-MEM-004, CC-MEM-010, CC-MEM-014 through CC-MEM-021 (Memory quality)
- CC-AG-001 through CC-AG-012 (Agents)
- CC-PL-001 through CC-PL-010 (Plugins)

### P2 (Week 5-6)
Complete coverage:
- MCP-001 through MCP-004, MCP-006 (MCP protocol)
- PE-002, PE-004, PE-005, PE-006 (Prompt engineering)
- XP-001 through XP-008, XP-SK-001 through XP-SK-002 (Cross-platform)
- CR-SK-001, CL-SK-001, CP-SK-001, CX-SK-001, OC-SK-001, WS-SK-001, KR-SK-001, KR-AG-001 to KR-AG-006, KR-AG-008 to KR-AG-030, KR-HK-001 through KR-HK-006, KR-PW-001 through KR-PW-004, KR-MCP-001 through KR-MCP-002, AMP-SK-001, RC-SK-001 (Per-client and Kiro rules)
- Remaining MEDIUM/LOW certainty rules

---

## Implementation Reference

### Detection Pseudocode

```rust
pub fn validate_skill(path: &Path, content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // AS-001: Check frontmatter exists
    if !content.starts_with("---") {
        diagnostics.push(Diagnostic::error(
            path, 1, 0, "AS-001",
            "Missing YAML frontmatter".to_string()
        ));
        return diagnostics; // Can't continue without frontmatter
    }

    // Parse frontmatter
    let (frontmatter, body) = parse_frontmatter::<SkillSchema>(content)?;

    // AS-002: Check name exists
    if frontmatter.name.is_empty() {
        diagnostics.push(Diagnostic::error(
            path, 2, 0, "AS-002",
            "Missing required field: name".to_string()
        ));
    }

    // AS-004: Check name format
    if !is_valid_skill_name(&frontmatter.name) {
        diagnostics.push(Diagnostic::error(
            path, 2, 0, "AS-004",
            format!("Invalid name format: {}", frontmatter.name)
        ).with_suggestion("Use a bare skill name or '<plugin>:<skill-name>' with kebab-case segments"));
    }

    // Continue with other rules...
    diagnostics
}
```

### Auto-Fix Priority

| Rule | Auto-Fix | Safety |
|------|----------|--------|
| AS-004 | Convert name to kebab-case | safe/unsafe |
| AS-005 | Strip leading/trailing hyphens | safe |
| AS-006 | Collapse consecutive hyphens | safe |
| CC-SK-001 | Default invalid model to sonnet | unsafe |
| CC-SK-002 | Normalize context to fork | unsafe |
| CC-SK-004 | Insert context: fork before agent key | unsafe |
| CC-SK-007 | Suggest Bash(git:*) matcher | unsafe |
| CC-SK-014 | Convert string to boolean | safe |
| CC-SK-015 | Convert string to boolean | safe |
| CC-HK-001 | Correct event name casing/typo | safe/unsafe |
| CC-HK-011 | Remove redundant wildcard matcher | unsafe |
| CC-HK-013 | Remove async field | safe |
| CC-HK-015 | Remove model field | safe |
| CC-HK-018 | Remove matcher field | safe |
| CC-AG-003 | Default invalid model to sonnet | unsafe |
| CC-AG-004 | Default invalid permission mode | unsafe |
| CC-AG-008 | Replace with closest memory scope | unsafe |
| CC-PL-005 | Normalize plugin name | unsafe |
| CC-PL-007 | Prepend ./ to relative path | safe |
| MCP-001 | Set jsonrpc to "2.0" | safe |
| MCP-008 | Update protocolVersion | unsafe |
| MCP-011 | Replace with closest server type | unsafe |
| MCP-012 | Change sse to http | unsafe |
| COP-002 | Insert template frontmatter with applyTo | unsafe |
| COP-004 | Remove unknown frontmatter key | safe |
| COP-005 | Replace with closest excludeAgent value | unsafe |
| CUR-005 | Remove unknown frontmatter key | safe |
| CUR-007 | Remove redundant globs field | safe |
| CUR-008 | Convert quoted string to boolean | safe |
| CLN-003 | Remove unknown frontmatter key | unsafe |
| XML-001 | Add missing closing tag | unsafe |
| XML-002 | Fix mismatched closing tag | unsafe |
| XML-003 | Remove orphaned closing tag | unsafe |
| AS-001 | Insert empty frontmatter block | unsafe |
| AS-002 | Insert name field derived from filename | unsafe |
| AS-003 | Insert description placeholder | unsafe |
| AS-009 | Strip XML tags from skill description | unsafe |
| CC-AG-001 | Insert name field derived from filename | unsafe |
| CC-AG-002 | Insert description placeholder | unsafe |
| CC-HK-033 | Append `__.*` to a bare MCP server matcher | unsafe |
| CC-MCP-004 | Convert quoted true/false alwaysLoad to a JSON boolean | unsafe |
| CC-SK-006 | Insert disable-model-invocation: true | unsafe |
| CC-PL-003 | Normalize partial semver | unsafe |
| AGM-001 | Append closing code fence for unclosed blocks | unsafe |
| GM-001 | Append closing code fence for unclosed blocks | unsafe |
| GM-008 | Strip directory prefix from contextFileName | unsafe |
| PE-005 | Delete redundant instruction line | unsafe |
| REF-003 | Delete duplicate import line | unsafe |
| CUR-011 | Replace invalid cursor hook event with closest match | unsafe |
| CUR-013 | Replace invalid cursor hook type with closest match | unsafe |
| KIRO-001 | Replace invalid inclusion mode with closest match | unsafe |
| KR-SK-001 | Move unsupported skill field under metadata, or delete it | safe/unsafe |
| OC-008 | Replace invalid permission mode with closest match | unsafe |
| OC-DEP-001 | Rename `mode` key to `agent` | safe |
| OC-DEP-005 | Remove TUI keys after manual migration to tui.json | unsafe |
| CC-SET-016 | Replace never-consulted Write/NotebookEdit/MultiEdit/Glob path rules with Edit/Read | unsafe |
| CC-OS-007 | Replace outputStyle with the exact style name | unsafe |
| COP-027 | Migrate infer while preserving explicit invocation fields | unsafe |
| KR-HK-001 | Replace fileCreate / fileDelete / manual with the documented IDE event | unsafe |
| KR-HK-011 | Normalize version strings 1 / 1.0 to v1 | unsafe |
| KR-HK-012 | Replace a case-mismatched trigger with its canonical name | unsafe |
| KR-HK-013 | Replace runCommand / askAgent action types with command / agent | unsafe |
| KR-HK-014 | Normalize supported V3 trigger aliases | safe |
| KR-SET-001 | Convert quoted true/false to a JSON boolean | unsafe |
| KR-SET-002 | Convert a quoted plain decimal to a JSON number | unsafe |
| KR-SET-003 | Convert a quoted plain integer to a JSON number | unsafe |
| KR-SET-008 | Convert a quoted boolean or integer to its JSON type | unsafe |
| KR-HK-005 | Rename a mis-cased agent hook trigger to the accepted name | unsafe |
| KR-AG-015 | Remove empty CLI 2.x trust fields so V3 loads the agent | unsafe |
| KR-HK-017 | Unquote boolean enabled and integer timeout values | unsafe |
| OC-DEP-003 | Rename `autoshare` key to `share` | safe |
| OC-CFG-008 | Replace logLevel with closest valid value | unsafe |
| OC-AG-006 | Replace color with closest named color | unsafe |
| OC-TUI-003 | Replace diff_style with closest valid value | unsafe |
| MCP-013 | Sanitize invalid tool name characters | unsafe |
| MCP-017 | Replace http:// with https:// in non-localhost URL | unsafe |
| MCP-021 | Replace 0.0.0.0 with localhost in URL | unsafe |
| COP-008 | Delete unknown agent frontmatter key | safe |
| COP-009 | Replace invalid agent target | unsafe |
| COP-012 | Delete unsupported GitHub.com agent field | safe |
| COP-014 | Delete unknown prompt frontmatter key | safe |
| COP-015 | Replace invalid prompt type | unsafe |
| AMP-001 | Delete unknown check frontmatter key | unsafe |
| AMP-004 | Delete unknown settings JSON key | unsafe |
| GM-009 | Delete unknown settings JSON key | unsafe |
| CDX-004 | Delete unknown TOML config key | unsafe |
| AMP-002 | Replace invalid severity-default with closest match | unsafe |

---

## Rule Count Summary

| Category | Total Rules | HIGH | MEDIUM | LOW | Auto-Fixable |
|----------|-------------|------|--------|-----|--------------|
| Agent Skills | 14 | 11 | 3 | 0 | 7 |
| AGENTS.md | 5 | 1 | 4 | 0 | 1 |
| Amp Checks | 4 | 2 | 2 | 0 | 3 |
| Amp Skills | 1 | 0 | 1 | 0 | 1 |
| Claude Agents | 21 | 10 | 10 | 1 | 7 |
| Claude Hooks | 32 | 19 | 10 | 3 | 11 |
| Claude Memory | 15 | 4 | 6 | 5 | 1 |
| Claude Output Styles | 6 | 2 | 3 | 1 | 2 |
| Claude Plugins | 23 | 12 | 10 | 1 | 3 |
| Claude Marketplaces | 5 | 4 | 0 | 1 | 0 |
| Claude MCP | 5 | 2 | 2 | 1 | 1 |
| Claude Settings | 35 | 2 | 32 | 1 | 12 |
| Claude Skills | 28 | 9 | 9 | 10 | 7 |
| Cline | 14 | 6 | 7 | 1 | 2 |
| Cline Skills | 2 | 0 | 2 | 0 | 1 |
| Codex CLI | 85 | 31 | 38 | 16 | 8 |
| Codex Skills | 4 | 0 | 2 | 2 | 1 |
| GitHub Copilot | 33 | 13 | 13 | 7 | 7 |
| Copilot Skills | 2 | 0 | 2 | 0 | 1 |
| Cross-Platform | 11 | 1 | 8 | 2 | 0 |
| Cursor | 39 | 13 | 15 | 11 | 8 |
| Cursor Skills | 1 | 0 | 1 | 0 | 1 |
| Gemini Agents | 1 | 1 | 0 | 0 | 0 |
| Gemini CLI | 10 | 3 | 5 | 2 | 3 |
| Kiro Agents | 29 | 11 | 15 | 3 | 1 |
| Kiro Hooks | 28 | 15 | 6 | 7 | 7 |
| Kiro Permissions | 12 | 6 | 5 | 1 | 0 |
| Kiro MCP | 15 | 7 | 6 | 2 | 0 |
| Kiro Powers | 18 | 6 | 9 | 3 | 0 |
| Kiro Settings | 11 | 1 | 8 | 2 | 4 |
| Kiro Workflows | 9 | 7 | 2 | 0 | 0 |
| Kiro Ignore | 1 | 0 | 1 | 0 | 0 |
| Kiro Skills | 2 | 0 | 1 | 1 | 1 |
| Kiro Steering | 16 | 4 | 10 | 2 | 1 |
| MCP | 23 | 17 | 5 | 1 | 7 |
| OpenCode | 66 | 25 | 29 | 12 | 10 |
| OpenCode Skills | 1 | 0 | 1 | 0 | 1 |
| Prompt Eng | 4 | 0 | 3 | 1 | 1 |
| References | 3 | 2 | 1 | 0 | 1 |
| Roo Code | 6 | 3 | 3 | 0 | 0 |
| Roo Code Skills | 1 | 0 | 1 | 0 | 1 |
| Version Awareness | 1 | 0 | 0 | 1 | 0 |
| Windsurf | 4 | 1 | 2 | 1 | 0 |
| Windsurf Skills | 1 | 0 | 1 | 0 | 1 |
| XML | 3 | 0 | 3 | 0 | 3 |
| **TOTAL** | **650** | **251** | **297** | **102** | **127** |


---

## Sources

### Standards
- agentskills.io (Agent Skills specification)
- modelcontextprotocol.io (MCP specification)
- code.claude.com/docs (Claude Code documentation)
- cursor.com/docs (Cursor AI documentation)
- docs.windsurf.com (Windsurf/Codeium documentation)
- github.com/cline/cline (Cline repository)

### Research Papers
- Zhao et al. (2021) - Few-shot calibration

### Production Code
- agentsys/plugins/enhance/* (70 patterns, tested on 1000+ files)

### Community
- 15+ platforms researched
- GitHub repos and documentation
- Community conventions and patterns

---

**Total Coverage**: 650 validation rules across 45 categories

**Knowledge Base**: 11,036 lines, 320KB, 75+ sources
**Certainty**: 251 HIGH, 297 MEDIUM, 102 LOW
**Auto-Fixable**: 127 rules (20%)
