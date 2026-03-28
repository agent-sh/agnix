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
**Requirement**: `name` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `name` key
**Fix**: [AUTO-FIX] Add `name: directory-name`
**Source**: agentskills.io/specification

<a id="as-003"></a>
### AS-003 [HIGH] Missing Required Field: description
**Requirement**: `description` field REQUIRED in frontmatter
**Detection**: Parse YAML, check for `description` key
**Fix**: [AUTO-FIX] Add `description: "Use when..."`
**Source**: agentskills.io/specification

<a id="as-004"></a>
### AS-004 [HIGH] Invalid Name Format
**Requirement**: name MUST be 1-64 chars, lowercase letters/numbers/hyphens only
**Regex**: `^[a-z0-9]+(-[a-z0-9]+)*$`
**Detection**:
```rust
!Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").matches(name) || name.len() > 64
```
**Fix**: [AUTO-FIX] Convert name to kebab-case (lowercase, replace `_` with `-`, remove invalid chars, collapse consecutive hyphens, truncate to 64 chars)
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

<a id="as-007"></a>
### AS-007 [HIGH] Reserved Name
**Requirement**: name MUST NOT be reserved word (anthropic, claude)
**Detection**: `["anthropic", "claude", "skill"].contains(name.as_str())`
**Fix**: Suggest alternative name
**Source**: platform.claude.com/docs

<a id="as-008"></a>
### AS-008 [HIGH] Description Too Short
**Requirement**: description MUST be 1-1024 characters
**Detection**: `description.len() < 1 || description.len() > 1024`
**Fix**: Add minimal description or truncate
**Source**: agentskills.io/specification

<a id="as-009"></a>
### AS-009 [HIGH] Description Contains XML
**Requirement**: description MUST NOT contain XML tags
**Detection**: `Regex::new(r"<[^>]+>").is_match(description)`
**Fix**: [AUTO-FIX] Remove XML tags
**Source**: platform.claude.com/docs

<a id="as-010"></a>
### AS-010 [MEDIUM] Missing Trigger Phrase
**Requirement**: description SHOULD include "Use when" trigger
**Detection**: `!description.to_lowercase().contains("use when")`
**Fix**: [AUTO-FIX] Prepend "Use when user wants to " to description
**Source**: agentsys/enhance-skills, platform.claude.com/docs

<a id="as-011"></a>
### AS-011 [HIGH] Compatibility Too Long
**Requirement**: compatibility field MUST be 1-500 chars if present
**Detection**: `compatibility.len() > 500`
**Fix**: Truncate to 500 chars
**Source**: agentskills.io/specification

<a id="as-012"></a>
### AS-012 [MEDIUM] Content Exceeds 500 Lines
**Requirement**: SKILL.md SHOULD be under 500 lines
**Detection**: `body.lines().count() > 500`
**Fix**: Suggest moving to references/
**Source**: platform.claude.com/docs, agentskills.io

<a id="as-013"></a>
### AS-013 [HIGH] File Reference Too Deep
**Requirement**: File references MUST be one level deep
**Detection**: Check references like `references/guide.md` vs `refs/deep/nested/file.md`
**Fix**: Flatten directory structure
**Source**: agentskills.io/specification

<a id="as-014"></a>
### AS-014 [HIGH] Windows Path Separator
**Requirement**: Paths MUST use forward slashes, even on Windows
**Detection**: `path.contains("\\")`
**Fix**: Replace `\\` with `/`
**Source**: agentskills.io/specification

<a id="as-015"></a>
### AS-015 [HIGH] Upload Size Exceeds 8MB
**Requirement**: Skill directory MUST be under 8MB total
**Detection**: `directory_size > 8 * 1024 * 1024`
**Fix**: Remove large assets or split skill
**Source**: platform.claude.com/docs

<a id="as-016"></a>
### AS-016 [HIGH] Skill Parse Error
**Requirement**: SKILL.md frontmatter MUST be valid YAML
**Detection**: YAML parse error on frontmatter content
**Fix**: Fix YAML syntax errors in frontmatter
**Source**: agentskills.io/specification

<a id="as-017"></a>
### AS-017 [HIGH] Name Must Match Parent Directory
**Requirement**: Skill name MUST match parent directory name
**Detection**: name field does not match directory containing SKILL.md
**Fix**: Manual fix required - rename directory or update name field
**Source**: agentskills.io/specification

<a id="as-018"></a>
### AS-018 [MEDIUM] Description Uses First or Second Person
**Requirement**: Description SHOULD NOT use first or second person pronouns
**Detection**: Description contains "I", "we", "you", "your", etc.
**Fix**: Manual fix required - rewrite description in imperative mood
**Source**: agentskills.io/specification

<a id="as-019"></a>
### AS-019 [MEDIUM] Vague Skill Name
**Requirement**: Skill name SHOULD be descriptive and specific
**Detection**: Name contains vague terms like "helper", "utility", "handler"
**Fix**: Manual fix required - use more descriptive name
**Source**: agentskills.io/specification

---

## CLAUDE CODE RULES (SKILLS)

<a id="cc-sk-001"></a>
### CC-SK-001 [HIGH] Invalid Model Value
**Requirement**: model MUST be one of: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with closest valid option
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-002"></a>
### CC-SK-002 [HIGH] Invalid Context Value
**Requirement**: context MUST be "fork" or omitted
**Detection**: `context.is_some() && context != "fork"`
**Fix**: Change to "fork" or remove
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-003"></a>
### CC-SK-003 [HIGH] Context Without Agent
**Requirement**: `context: fork` REQUIRES `agent` field
**Detection**: `context == "fork" && agent.is_none()`
**Fix**: Add `agent: general-purpose`
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
**Fix**: Auto-fix (unsafe) -- replace invalid agent with 'general-purpose'
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-sk-006"></a>
### CC-SK-006 [HIGH] Dangerous Auto-Invocation
**Requirement**: Side-effect skills MUST have `disable-model-invocation: true`
**Detection**: `name.contains("deploy|ship|publish|delete|drop") && !disable_model_invocation`
**Fix**: [AUTO-FIX] Add `disable-model-invocation: true`
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-007"></a>
### CC-SK-007 [MEDIUM] Unrestricted Bash
**Requirement**: Bash in allowed-tools SHOULD be scoped
**Detection**: `allowed_tools.contains("Bash") && !allowed_tools.contains("Bash(")`
**Fix**: [AUTO-FIX] Replace unrestricted Bash with scoped version (e.g., `Bash(git:*)`)
**Source**: agentsys/enhance-skills

<a id="cc-sk-008"></a>
### CC-SK-008 [HIGH] Unknown Tool Name
**Requirement**: Tool names MUST match Claude Code tools
**Known Tools**: Bash, Read, Write, Edit, Grep, Glob, Task, WebFetch, WebSearch, AskUserQuestion, TodoRead, TodoWrite, MultiTool, NotebookEdit, EnterPlanMode, ExitPlanMode, Skill, StatusBarMessageTool, SendMessageTool, TaskOutput
**Detection**: Check against tool list; MCP tools with lowercase `mcp__<server>__<tool>` format are accepted (case-sensitive prefix)
**Fix**: Suggest closest match
**Source**: code.claude.com/docs/en/settings

<a id="cc-sk-009"></a>
### CC-SK-009 [MEDIUM] Too Many Injections
**Requirement**: Limit dynamic injections (!`cmd`) to 3
**Detection**: `content.matches("!\`").count() > 3`
**Fix**: Remove or move to scripts/
**Source**: platform.claude.com/docs

<a id="cc-sk-010"></a>
### CC-SK-010 [HIGH] Invalid Hooks in Skill Frontmatter
**Requirement**: `hooks` field in skill frontmatter MUST follow the same schema as settings.json hooks (valid events, handler types, required fields)
**Detection**: Parse hooks YAML value and validate against HooksSchema rules
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-011"></a>
### CC-SK-011 [HIGH] Unreachable Skill
**Requirement**: Skill MUST NOT set both `user-invocable: false` and `disable-model-invocation: true`
**Detection**: `user_invocable == false && disable_model_invocation == true`
**Fix**: Auto-fix (unsafe) -- remove `disable-model-invocation: true` line
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-012"></a>
### CC-SK-012 [MEDIUM] Argument Hint Without $ARGUMENTS
**Requirement**: If `argument-hint` is set, body SHOULD reference `$ARGUMENTS`
**Detection**: `argument_hint.is_some() && !body.contains("$ARGUMENTS")`
**Fix**: Auto-fix (unsafe) - append `$ARGUMENTS` to skill body
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-013"></a>
### CC-SK-013 [MEDIUM] Fork Context Without Actionable Instructions
**Requirement**: Skills with `context: fork` SHOULD contain imperative instructions for the forked agent
**Detection**: Check body for imperative verbs when context is fork
**Fix**: No auto-fix
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-014"></a>
### CC-SK-014 [HIGH] Invalid disable-model-invocation Type
**Requirement**: `disable-model-invocation` MUST be a boolean, not a string
**Detection**: Raw YAML parsing detects quoted "true"/"false" strings
**Fix**: [AUTO-FIX, safe] Convert string to boolean
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-015"></a>
### CC-SK-015 [HIGH] Invalid user-invocable Type
**Requirement**: `user-invocable` MUST be a boolean, not a string
**Detection**: Raw YAML parsing detects quoted "true"/"false" strings
**Fix**: [AUTO-FIX, safe] Convert string to boolean
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-016"></a>
### CC-SK-016 [MEDIUM] Indexed $ARGUMENTS Without argument-hint
**Requirement**: If body uses indexed $ARGUMENTS (e.g., $ARGUMENTS[0]), SHOULD have argument-hint field
**Detection**: Body contains indexed $ARGUMENTS syntax without argument-hint field
**Fix**: Manual fix required - add argument-hint field describing expected arguments
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-017"></a>
### CC-SK-017 [MEDIUM] Unknown Frontmatter Field
**Requirement**: Skill frontmatter SHOULD only use recognized fields
**Detection**: Frontmatter contains fields not in the Claude Code skill schema
**Fix**: Manual fix required - remove unknown field or correct typo
**Source**: code.claude.com/docs/en/skills

<a id="cc-sk-018"></a>
### CC-SK-018 [MEDIUM] Invalid Effort Value
**Requirement**: effort SHOULD be low, medium, high, or max
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

---

## PER-CLIENT SKILL RULES

<a id="cr-sk-001"></a>
### CR-SK-001 [MEDIUM] Cursor Skill Uses Unsupported Field
**Requirement**: Skills in `.cursor/skills/` SHOULD NOT use frontmatter fields unsupported by Cursor
**Detection**: SKILL.md path contains `.cursor/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.cursor.com/en/context/skills

<a id="cl-sk-001"></a>
### CL-SK-001 [MEDIUM] Cline Skill Uses Unsupported Field
**Requirement**: Skills in `.cline/skills/` SHOULD NOT use frontmatter fields unsupported by Cline
**Detection**: SKILL.md path contains `.cline/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.cline.bot/features/custom-instructions

<a id="cp-sk-001"></a>
### CP-SK-001 [MEDIUM] Copilot Skill Uses Unsupported Field
**Requirement**: Skills in `.github/skills/` SHOULD NOT use frontmatter fields unsupported by GitHub Copilot
**Detection**: SKILL.md path contains `.github/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.github.com/en/copilot/reference/custom-instructions-support

<a id="cx-sk-001"></a>
### CX-SK-001 [MEDIUM] Codex Skill Uses Unsupported Field
**Requirement**: Skills in `.agents/skills/` SHOULD NOT use frontmatter fields unsupported by Codex CLI
**Detection**: SKILL.md path contains `.agents/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: developers.openai.com/codex/guides/agents-md

<a id="oc-sk-001"></a>
### OC-SK-001 [MEDIUM] OpenCode Skill Uses Unsupported Field
**Requirement**: Skills in `.opencode/skills/` SHOULD NOT use frontmatter fields unsupported by OpenCode
**Detection**: SKILL.md path contains `.opencode/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: opencode.ai/docs/rules

<a id="ws-sk-001"></a>
### WS-SK-001 [MEDIUM] Windsurf Skill Uses Unsupported Field
**Requirement**: Skills in `.windsurf/skills/` SHOULD NOT use frontmatter fields unsupported by Windsurf
**Detection**: SKILL.md path contains `.windsurf/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: docs.windsurf.com/windsurf/memories

<a id="kr-sk-001"></a>
### KR-SK-001 [MEDIUM] Kiro Skill Uses Unsupported Field
**Requirement**: Skills in `.kiro/skills/` SHOULD NOT use frontmatter fields unsupported by Kiro
**Detection**: SKILL.md path contains `.kiro/skills/` AND frontmatter has unsupported fields
**Fix**: [AUTO-FIX, safe] Remove unsupported field
**Source**: kiro.dev/docs/context/steering

<a id="kr-ag-001"></a>
### KR-AG-001 [MEDIUM] Unknown Field in Kiro Agent JSON
**Requirement**: Kiro agent JSON SHOULD only use documented top-level fields
**Detection**: `.kiro/agents/*.json` contains unknown top-level keys outside the documented schema
**Fix**: Remove unsupported keys or rename to documented fields
**Source**: kiro.dev/docs/cli/custom-agents/configuration-reference

<a id="kr-ag-002"></a>
### KR-AG-002 [HIGH] Invalid Kiro Agent Resource Protocol
**Requirement**: Agent `resources` entries MUST use valid Kiro resource forms
**Detection**: Resource is not `file://`, not `skill://`, and not an object with `type: knowledgeBase`
**Fix**: Use valid resource URIs or supported structured resource object
**Source**: kiro.dev/docs/cli/custom-agents/creating

<a id="kr-ag-003"></a>
### KR-AG-003 [MEDIUM] allowedTools Not Subset of tools
**Requirement**: `allowedTools` SHOULD be a subset of `tools`
**Detection**: One or more `allowedTools` entries are not present in `tools`
**Fix**: Remove mismatched `allowedTools` entries or add them to `tools`
**Source**: kiro.dev/docs/cli/custom-agents/configuration-reference

<a id="kr-ag-004"></a>
### KR-AG-004 [MEDIUM] Invalid Kiro Agent Model Value
**Requirement**: Agent `model` SHOULD use a documented Kiro model value
**Detection**: `model` value is not one of the known Kiro model identifiers
**Fix**: Replace `model` with a documented value
**Source**: kiro.dev/docs/cli/custom-agents/configuration-reference

<a id="kr-ag-005"></a>
### KR-AG-005 [LOW] Kiro Agent Has No MCP Access
**Requirement**: Agent MCP access SHOULD be explicit when `includeMcpJson` is disabled
**Detection**: `includeMcpJson: false` and no inline `mcpServers` are defined
**Fix**: Enable `includeMcpJson` or configure inline `mcpServers`
**Source**: kiro.dev/docs/cli/custom-agents/configuration-reference

<a id="kr-ag-006"></a>
### KR-AG-006 [MEDIUM] Kiro Agent References Unknown Subagent
**Requirement**: Kiro agent prompts SHOULD reference only subagents defined in `.kiro/agents/*.json`
**Detection**: Prompt contains `@agent-name` mention where agent name is not present in sibling Kiro agent definitions
**Fix**: Add the missing agent file or remove the unresolved `@agent-name` reference
**Source**: github.com/kirodotdev/kiro/issues/5743, github.com/kirodotdev/kiro/issues/4262

<a id="kr-ag-007"></a>
### KR-AG-007 [MEDIUM] Kiro Agent Tool Scope Broader Than Referenced Subagent
**Requirement**: Orchestrator agent tool scope SHOULD NOT exceed the referenced subagent tool scope
**Detection**: For each referenced `@agent-name`, parent `allowedTools/tools` contains entries not present in the referenced subagent
**Fix**: Narrow parent tool scope or align referenced subagent permissions
**Source**: github.com/kirodotdev/kiro/issues/5071, github.com/kirodotdev/kiro/issues/5449

<a id="kr-ag-008"></a>
### KR-AG-008 [HIGH] Agent Missing Name
**Requirement**: Kiro agent JSON MUST include a non-empty `name` field
**Detection**: Check if `name` field is present and non-empty
**Fix**: No auto-fix (add a name field)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-ag-009"></a>
### KR-AG-009 [HIGH] Agent Missing Prompt
**Requirement**: Kiro agent JSON MUST include a non-empty `prompt` field
**Detection**: Check if `prompt` field is present and non-empty
**Fix**: No auto-fix (add a prompt field)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-ag-010"></a>
### KR-AG-010 [MEDIUM] Duplicate Tool Entries
**Requirement**: Kiro agent `tools` array SHOULD NOT contain duplicate entries
**Detection**: Check for duplicate tool names (case-insensitive)
**Fix**: No auto-fix (remove duplicates)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-ag-011"></a>
### KR-AG-011 [LOW] Empty Tools Array
**Requirement**: Kiro agent `tools` array SHOULD contain at least one entry if present
**Detection**: Check if `tools` is an empty array
**Fix**: No auto-fix (add tools or remove the field)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-ag-012"></a>
### KR-AG-012 [MEDIUM] toolAliases References Unknown Tool
**Requirement**: `toolAliases` targets SHOULD reference tools in the `tools` array
**Detection**: Check each alias target against the tools set
**Fix**: No auto-fix (fix alias or add missing tool)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-ag-013"></a>
### KR-AG-013 [HIGH] Secrets in Agent Prompt
**Requirement**: Kiro agent `prompt` MUST NOT contain hardcoded credentials
**Detection**: Scan prompt for secret patterns (API keys, tokens, passwords)
**Fix**: No auto-fix (use environment variables)
**Source**: kiro.dev/docs/agents, kiro.dev/docs/configuration

<a id="kr-hk-005"></a>
### KR-HK-005 [HIGH] Invalid Kiro CLI Hook Event Key
**Requirement**: Agent JSON `hooks` keys MUST use valid CLI hook event names
**Detection**: `hooks` object contains key outside `agentSpawn`, `userPromptSubmit`, `preToolUse`, `postToolUse`, `stop`
**Fix**: Rename event key to a valid CLI hook event
**Source**: kiro.dev/docs/cli/hooks

<a id="kr-hk-006"></a>
### KR-HK-006 [HIGH] Kiro CLI Hook Missing Command
**Requirement**: Each CLI hook entry MUST define a non-empty `command`
**Detection**: Hook object under a valid CLI event lacks `command` or has an empty value
**Fix**: Add a non-empty `command` value for each hook entry
**Source**: kiro.dev/docs/cli/hooks

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
**Requirement**: `.amp/settings.json` MUST be valid JSON and use known top-level keys
**Detection**: JSON parse error OR unknown top-level key in `.amp/settings.json` / `.amp/settings.local.json`
**Fix**: [AUTO-FIX] Fix JSON syntax and remove unknown keys
**Source**: ampcode.com/manual#settings

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
**Requirement**: Event MUST be one of 15 valid names (case-sensitive)
**Valid**: SessionStart, UserPromptSubmit, PreToolUse, PermissionRequest, PostToolUse, PostToolUseFailure, SubagentStart, SubagentStop, Stop, PreCompact, Setup, SessionEnd, Notification, TeammateIdle, TaskCompleted
**Detection**: `!VALID_EVENTS.contains(event)`
**Fix**: [AUTO-FIX] Replace with closest matching valid event name
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-002"></a>
### CC-HK-002 [HIGH] Prompt Hook on Wrong Event
**Requirement**: `type: "prompt"` or `type: "agent"` only on supported events
**Supported**: PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, UserPromptSubmit, Stop, SubagentStop, TaskCompleted
**Detection**: `hook.type in ["prompt", "agent"] && !PROMPT_EVENTS.contains(event)`
**Fix**: Change to `type: "command"` for unsupported events
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-003"></a>
### CC-HK-003 [LOW] Matcher Hint for Tool Events
**Requirement**: Tool events support an optional matcher field; omitting it matches all tools
**Detection**: `["PreToolUse", "PermissionRequest", "PostToolUse", "PostToolUseFailure"].contains(event) && matcher.is_none()`
**Fix**: Consider adding `"matcher": "Bash"` or `"*"` to target specific tools
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-004"></a>
### CC-HK-004 [HIGH] Matcher on Non-Tool Event
**Requirement**: Stop/SubagentStop/UserPromptSubmit MUST NOT have matcher
**Detection**: `["Stop", "SubagentStop", "UserPromptSubmit"].contains(event) && matcher.is_some()`
**Fix**: Remove matcher field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-005"></a>
### CC-HK-005 [HIGH] Missing Type Field
**Requirement**: Hook MUST have `type: "command"` or `type: "prompt"`
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
**Requirement**: `type: "prompt"` REQUIRES `prompt` field
**Detection**: `hook.type == "prompt" && hook.prompt.is_none()`
**Fix**: Add prompt field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-008"></a>
### CC-HK-008 [HIGH] Script File Not Found
**Requirement**: Hook command script MUST exist on filesystem
**Detection**: Check if script path exists (resolve $CLAUDE_PROJECT_DIR)
**Fix**: Show error with correct path
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-009"></a>
### CC-HK-009 [HIGH] Dangerous Command Pattern
**Requirement**: Hooks SHOULD NOT contain destructive commands
**Patterns**: `rm -rf`, `git reset --hard`, `drop database`, `curl.*|.*sh`
**Detection**: Regex match against dangerous patterns
**Fix**: Warn, suggest safer alternative
**Source**: agentsys/enhance-hooks

<a id="cc-hk-010"></a>
### CC-HK-010 [MEDIUM] Timeout Policy
**Requirement**: Hooks SHOULD have explicit timeout; excessive timeouts warn
**Detection**:
  - `hook.timeout.is_none()` - missing timeout
  - Command: `timeout > 600` exceeds 10-min default
  - Prompt: `timeout > 30` exceeds 30s default
**Fix**: [AUTO-FIX] Add explicit timeout within default limits (600s for commands, 30s for prompts)
**Source**: code.claude.com/docs/en/hooks
**Version-Aware**: When Claude Code version is not pinned in `.agnix.toml [tool_versions]`, an assumption note is added indicating default timeout behavior is assumed. Pin the version for version-specific validation.

<a id="cc-hk-011"></a>
### CC-HK-011 [HIGH] Invalid Timeout Value
**Requirement**: timeout MUST be positive integer
**Detection**: `timeout <= 0`
**Fix**: Set to 30
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-012"></a>
### CC-HK-012 [HIGH] Hooks Parse Error
**Requirement**: Hooks configuration MUST be valid JSON
**Detection**: JSON parse error on settings.json
**Fix**: Fix JSON syntax errors in hooks configuration
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-013"></a>
### CC-HK-013 [HIGH] Async on Non-Command Hook
**Requirement**: `async: true` MUST only appear on `type: "command"` hooks
**Detection**: Check for `async` field on prompt or agent hook types
**Fix**: Auto-fix (safe) -- remove the `async` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-014"></a>
### CC-HK-014 [MEDIUM] Once Outside Skill/Agent Frontmatter
**Requirement**: `once` field SHOULD only appear in skill/agent frontmatter hooks
**Detection**: Check for `once` field in settings.json hooks
**Fix**: [AUTO-FIX] Remove the once field from settings.json hooks
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-015"></a>
### CC-HK-015 [MEDIUM] Model on Command Hook
**Requirement**: `model` field MUST only appear on prompt or agent hooks
**Detection**: Check for `model` field on command hook types
**Fix**: Auto-fix (safe) -- remove the `model` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-016"></a>
### CC-HK-016 [HIGH] Validate Hook Type Agent
**Requirement**: `type: "agent"` MUST be recognized as a valid hook handler type
**Detection**: Ensure agent type is accepted alongside command and prompt
**Fix**: Auto-fix (unsafe) -- replace unknown hook type with closest valid type (command, prompt, agent)
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-017"></a>
### CC-HK-017 [MEDIUM] Prompt/Agent Hook Missing $ARGUMENTS
**Requirement**: Prompt and agent hooks SHOULD reference `$ARGUMENTS` to receive event data
**Detection**: Check prompt or agent hook text for `$ARGUMENTS` reference
**Fix**: [AUTO-FIX] Include `$ARGUMENTS` in the prompt or agent hook
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-018"></a>
### CC-HK-018 [LOW] Matcher on UserPromptSubmit/Stop
**Requirement**: Matchers on UserPromptSubmit and Stop events are silently ignored
**Detection**: Check for matcher field on UserPromptSubmit or Stop events
**Fix**: Auto-fix (safe) -- remove the `matcher` field line
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-019"></a>
### CC-HK-019 [MEDIUM] Deprecated Setup Event
**Requirement**: The `Setup` hook event SHOULD be replaced with `SessionStart`
**Detection**: Check if `Setup` is used as a hook event name
**Fix**: Auto-fix (unsafe) -- replace `Setup` with `SessionStart`
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-020"></a>
### CC-HK-020 [HIGH] HTTP Hook Missing URL
**Requirement**: HTTP hooks (type: "http") MUST have a url field
**Detection**: Check for type:"http" entries missing url key
**Fix**: Manual - add url field
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-021"></a>
### CC-HK-021 [MEDIUM] Invalid If Field
**Requirement**: The if field SHOULD be a non-empty string, only on tool events
**Detection**: Check if field presence and type
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
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-024"></a>
### CC-HK-024 [MEDIUM] Headers Missing AllowedEnvVars
**Requirement**: HTTP hook headers with $VAR interpolation SHOULD have allowedEnvVars
**Detection**: Check for $ patterns in headers without allowedEnvVars
**Fix**: Manual - add allowedEnvVars array
**Source**: code.claude.com/docs/en/hooks

<a id="cc-hk-025"></a>
### CC-HK-025 [LOW] Invalid Matcher Value
**Requirement**: Matcher values MAY be validated against known values per event
**Detection**: Check matcher values against event-specific allowlists
**Fix**: Manual
**Source**: code.claude.com/docs/en/hooks

---

## CLAUDE CODE RULES (SUBAGENTS)

<a id="cc-ag-001"></a>
### CC-AG-001 [HIGH] Missing Name Field
**Requirement**: Agent frontmatter REQUIRES `name` field
**Detection**: Parse frontmatter, check for `name`
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
**Requirement**: model MUST be: sonnet, opus, haiku, inherit
**Detection**: `!["sonnet", "opus", "haiku", "inherit"].contains(model)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-004"></a>
### CC-AG-004 [HIGH] Invalid Permission Mode
**Requirement**: permissionMode MUST be: default, acceptEdits, dontAsk, bypassPermissions, plan, delegate
**Detection**: `!VALID_MODES.contains(permission_mode)`
**Fix**: Replace with valid value
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-005"></a>
### CC-AG-005 [HIGH] Referenced Skill Not Found
**Requirement**: Skills in `skills` array MUST exist
**Detection**: Check `.claude/skills/{name}/SKILL.md` exists
**Fix**: Remove reference or create skill
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-006"></a>
### CC-AG-006 [HIGH] Tool/Disallowed Conflict
**Requirement**: Tool cannot be in both `tools` and `disallowedTools`
**Detection**: `tools.intersection(disallowedTools).is_empty()`
**Fix**: Remove from one list
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-007"></a>
### CC-AG-007 [HIGH] Agent Parse Error
**Requirement**: Agent frontmatter MUST be valid YAML
**Detection**: YAML parse error on agent frontmatter
**Fix**: Fix YAML syntax errors in agent frontmatter
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-008"></a>
### CC-AG-008 [HIGH] Invalid Memory Scope
**Requirement**: `memory` field MUST be `user`, `project`, or `local`
**Detection**: Check `memory` value against allowed list
**Fix**: Auto-fix (unsafe) -- replace with closest valid memory scope
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-009"></a>
### CC-AG-009 [HIGH] Invalid Tool Name in Tools List
**Requirement**: Tool names in `tools` MUST match known Claude Code tools
**Detection**: Check each tool name against known tools list; MCP tools with lowercase `mcp__<server>__<tool>` format are accepted (case-sensitive prefix)
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-010"></a>
### CC-AG-010 [HIGH] Invalid Tool Name in DisallowedTools
**Requirement**: Tool names in `disallowedTools` MUST match known Claude Code tools
**Detection**: Check each disallowed tool name against known tools list; MCP tools with lowercase `mcp__<server>__<tool>` format are accepted (case-sensitive prefix)
**Fix**: Use a known Claude Code tool name
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-011"></a>
### CC-AG-011 [HIGH] Invalid Hooks in Agent Frontmatter
**Requirement**: `hooks` object MUST follow the same schema as settings.json hooks
**Detection**: Validate hooks object structure (event names, hook types, required fields)
**Fix**: Ensure hooks follow the settings.json hooks schema
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-012"></a>
### CC-AG-012 [HIGH] Bypass Permissions Warning
**Requirement**: `permissionMode: bypassPermissions` SHOULD NOT be used (disables all safety checks)
**Detection**: Check if permissionMode equals `bypassPermissions`
**Fix**: Auto-fix (unsafe) -- replace 'bypassPermissions' with 'default'
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-013"></a>
### CC-AG-013 [MEDIUM] Invalid Skill Name Format
**Requirement**: Skill names in `skills` array SHOULD follow valid naming format (lowercase, hyphens)
**Detection**: Check skill name matches kebab-case pattern
**Fix**: [AUTO-FIX] Use kebab-case format (e.g., 'my-skill-name')
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-014"></a>
### CC-AG-014 [MEDIUM] Invalid Effort Value
**Requirement**: effort SHOULD be low, medium, high, or max
**Detection**: Check effort field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-015"></a>
### CC-AG-015 [MEDIUM] Invalid Isolation Value
**Requirement**: isolation SHOULD be "worktree"
**Detection**: Check isolation field value
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-016"></a>
### CC-AG-016 [MEDIUM] Invalid Background Type
**Requirement**: background SHOULD be boolean
**Detection**: Check background field type
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-017"></a>
### CC-AG-017 [MEDIUM] Invalid MaxTurns Value
**Requirement**: maxTurns SHOULD be a positive integer
**Detection**: Check maxTurns is > 0
**Fix**: Manual
**Source**: code.claude.com/docs/en/sub-agents

<a id="cc-ag-019"></a>
### CC-AG-019 [LOW] Unknown Agent Frontmatter Field
**Requirement**: Agent frontmatter fields MAY be validated against known set
**Detection**: Check for keys not in known agent fields
**Fix**: Manual - remove or fix typo
**Source**: code.claude.com/docs/en/sub-agents

---

## CLAUDE CODE RULES (MEMORY)

<a id="cc-mem-001"></a>
### CC-MEM-001 [HIGH] Invalid Import Path
**Requirement**: @import paths MUST exist on filesystem
**Detection**: Extract `@path` references, check existence
**Fix**: Show error with resolved path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-002"></a>
### CC-MEM-002 [HIGH] Circular Import
**Requirement**: @imports MUST NOT create circular references
**Detection**: Build import graph, detect cycles
**Fix**: Show cycle path
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-003"></a>
### CC-MEM-003 [HIGH] Import Depth Exceeds 5
**Requirement**: @import chain MUST NOT exceed 5 hops
**Detection**: Track import depth during resolution
**Fix**: Flatten import hierarchy
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-004"></a>
### CC-MEM-004 [MEDIUM] Invalid Command Reference
**Requirement**: npm scripts referenced SHOULD exist in package.json
**Detection**: Extract `npm run <script>`, check package.json
**Fix**: Show available scripts
**Source**: agentsys/enhance-claude-memory

<a id="cc-mem-005"></a>
### CC-MEM-005 [HIGH] Generic Instruction
**Requirement**: Avoid redundant "be helpful" instructions
**Patterns**: `be helpful`, `be accurate`, `think step by step`, `be concise`
**Detection**: Regex match against 8 generic patterns
**Fix**: Remove line
**Source**: agentsys/enhance-claude-memory, research papers

<a id="cc-mem-006"></a>
### CC-MEM-006 [HIGH] Negative Without Positive
**Requirement**: Negative instructions ("don't") SHOULD include positive alternative
**Detection**: Line contains `don't|never|avoid` without follow-up positive
**Fix**: Suggest "Instead, do..."
**Source**: research: positive framing improves compliance

<a id="cc-mem-007"></a>
### CC-MEM-007 [HIGH] Weak Constraint Language
**Requirement**: Critical rules MUST use strong language (must/always/never)
**Detection**: In critical section, check for `should|try to|consider|maybe`
**Fix**: Replace with `must|always|required`
**Source**: research: constraint strength affects compliance

<a id="cc-mem-008"></a>
### CC-MEM-008 [HIGH] Critical Content in Middle
**Requirement**: Important rules SHOULD be at START or END (lost in the middle)
**Detection**: "critical" appears after 40% of content
**Fix**: Move to top
**Source**: Liu et al. (2023), TACL

<a id="cc-mem-009"></a>
### CC-MEM-009 [MEDIUM] Token Count Exceeded
**Requirement**: File SHOULD be under 1500 tokens (~6000 chars)
**Detection**: `content.len() / 4 > 1500`
**Fix**: Suggest using @imports
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-010"></a>
### CC-MEM-010 [MEDIUM] README Duplication
**Requirement**: CLAUDE.md SHOULD complement README, not duplicate
**Detection**: Compare with README.md, check >40% overlap
**Fix**: Remove duplicated sections
**Source**: agentsys/enhance-claude-memory

<a id="cc-mem-011"></a>
### CC-MEM-011 [HIGH] Invalid Paths Glob in Rules
**Requirement**: Glob patterns in `.claude/rules/*.md` frontmatter `paths` field MUST be valid
**Detection**: Parse YAML frontmatter, validate each glob pattern in `paths` array
**Fix**: Manual - fix glob syntax
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-012"></a>
### CC-MEM-012 [MEDIUM] Rules File Unknown Frontmatter Key
**Requirement**: `.claude/rules/*.md` frontmatter SHOULD only contain known keys (`paths`)
**Detection**: Parse YAML frontmatter, flag keys not in known set
**Fix**: Auto-fix (unsafe) - remove unknown key line (may miss multi-line values)
**Source**: code.claude.com/docs/en/memory

<a id="cc-mem-014"></a>
### CC-MEM-014 [MEDIUM] CLAUDE.md Exceeds Line Limit
**Requirement**: CLAUDE.md SHOULD be under 200 lines
**Detection**: Count non-empty lines
**Fix**: Manual - split or trim content
**Source**: code.claude.com/docs/en/memory

---

## AGENTS.MD RULES (CROSS-PLATFORM)

<a id="agm-001"></a>
### AGM-001 [HIGH] Valid Markdown Structure
**Requirement**: AGENTS.md MUST be valid markdown
**Detection**: Parse as markdown, check for syntax errors
**Fix**: [AUTO-FIX] Fix markdown syntax issues
**Source**: developers.openai.com/codex/guides/agents-md, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions

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

<a id="agm-006"></a>
### AGM-006 [MEDIUM] Nested AGENTS.md Hierarchy
**Requirement**: Some tools load AGENTS.md hierarchically (multiple files may apply)
**Detection**: Multiple AGENTS.md files in directory tree
**Fix**: Document inheritance behavior
**Source**: developers.openai.com/codex/guides/agents-md, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

---

## CLAUDE CODE RULES (PLUGINS)

<a id="cc-pl-001"></a>
### CC-PL-001 [HIGH] Plugin Manifest Not in .claude-plugin/
**Requirement**: plugin.json MUST be in `.claude-plugin/` directory
**Detection**: Check `!.claude-plugin/plugin.json` exists
**Fix**: Move to correct location
**Source**: code.claude.com/docs/en/plugins

<a id="cc-pl-002"></a>
### CC-PL-002 [HIGH] Components in .claude-plugin/
**Requirement**: skills/agents/hooks MUST NOT be inside .claude-plugin/
**Detection**: Check for `.claude-plugin/skills/`, etc.
**Fix**: Move to plugin root
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-003"></a>
### CC-PL-003 [HIGH] Invalid Semver
**Requirement**: version MUST be semver format (major.minor.patch)
**Detection**: `!Regex::new(r"^\d+\.\d+\.\d+$").matches(version)`
**Fix**: [AUTO-FIX] Suggest valid semver
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-004"></a>
### CC-PL-004 [HIGH] Missing Required/Recommended Plugin Field
**Requirement**: plugin.json REQUIRES name; description and version are RECOMMENDED
**Detection**: Parse JSON, check required fields (error for name, warning for description/version)
**Fix**: Add missing fields
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-005"></a>
### CC-PL-005 [HIGH] Empty Plugin Name
**Requirement**: name field MUST NOT be empty
**Detection**: `name.trim().is_empty()`
**Fix**: Add plugin name
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-006"></a>
### CC-PL-006 [HIGH] Plugin Parse Error
**Requirement**: plugin.json MUST be valid JSON
**Detection**: JSON parse error on plugin.json
**Fix**: Fix JSON syntax errors in plugin.json
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-007"></a>
### CC-PL-007 [HIGH] Invalid Component Path
**Requirement**: Paths in `commands`, `agents`, `skills`, `hooks` MUST be relative (no absolute paths or `..` traversal)
**Detection**: Check path fields for absolute paths (`/`, `C:\`) or parent traversal (`..`)
**Fix**: Prepend `./` to relative paths [safe autofix]
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-008"></a>
### CC-PL-008 [HIGH] Component Inside .claude-plugin
**Requirement**: Component paths in manifest MUST NOT point inside `.claude-plugin/` directory
**Detection**: Check if path fields reference `.claude-plugin/` subdirectories
**Fix**: Suggest moving components to plugin root
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-009"></a>
### CC-PL-009 [MEDIUM] Invalid Author Object
**Requirement**: If `author` field is present, `author.name` SHOULD be a non-empty string
**Detection**: Check `author.name` exists and is non-empty when `author` is present
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-010"></a>
### CC-PL-010 [MEDIUM] Invalid Homepage URL
**Requirement**: If `homepage` field is present, it SHOULD be a valid URL (http/https)
**Detection**: Validate URL format with http/https scheme check
**Fix**: Manual fix required
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-011"></a>
### CC-PL-011 [HIGH] LSP Server Missing Required Fields
**Requirement**: LSP servers MUST have command and extensionToLanguage
**Detection**: Check lspServers entries for required fields
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-012"></a>
### CC-PL-012 [MEDIUM] Invalid UserConfig Key
**Requirement**: userConfig keys SHOULD be valid identifiers
**Detection**: Check keys match identifier pattern
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-013"></a>
### CC-PL-013 [MEDIUM] Channel Missing Server Reference
**Requirement**: channels entries SHOULD have server field
**Detection**: Check each channel has server key
**Fix**: Manual
**Source**: code.claude.com/docs/en/plugins-reference

<a id="cc-pl-014"></a>
### CC-PL-014 [MEDIUM] Plugin Agent Unsupported Field
**Requirement**: Plugin agents SHOULD NOT use hooks, mcpServers, or permissionMode
**Detection**: Check plugin agent frontmatter for unsupported keys
**Fix**: Manual - remove unsupported fields
**Source**: code.claude.com/docs/en/plugins-reference

---

## MCP RULES

<a id="mcp-001"></a>
### MCP-001 [HIGH] Invalid JSON-RPC Version
**Requirement**: MUST use JSON-RPC 2.0
**Detection**: `message.jsonrpc != "2.0"`
**Fix**: Set `"jsonrpc": "2.0"`
**Source**: modelcontextprotocol.io/specification

<a id="mcp-002"></a>
### MCP-002 [HIGH] Missing Required Tool Field
**Requirement**: Tool MUST have `name`, `description`, `inputSchema`
**Detection**: Parse tool definition, check required fields while allowing optional `title`, `outputSchema`, and `icons`
**Fix**: Add missing fields
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-003"></a>
### MCP-003 [HIGH] Invalid JSON Schema
**Requirement**: `inputSchema` MUST be valid JSON Schema (JSON Schema 2020-12 compatible)
**Detection**: Validate schema structure and field types
**Fix**: Correct JSON Schema structure errors
**Source**: modelcontextprotocol.io/specification

<a id="mcp-004"></a>
### MCP-004 [HIGH] Missing Tool Description
**Requirement**: Tool SHOULD have clear description
**Detection**: `description.is_empty()`
**Fix**: Add description
**Source**: modelcontextprotocol.io/docs/concepts/tools

<a id="mcp-005"></a>
### MCP-005 [HIGH] Tool Without User Consent
**Requirement**: Tools MUST have user consent before invocation
**Detection**: Check for permission flow
**Fix**: Document consent requirement
**Source**: modelcontextprotocol.io/specification (Security)

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
**Detection**: Check `protocolVersion` field in initialize request params or response result against configured expected version (default: `2025-11-25`)
**Fix**: Update protocolVersion to match expected version, or configure `mcp_protocol_version` in agnix config to match your target version
**Note**: This is a warning (not error) because MCP allows version negotiation between client and server
**Source**: modelcontextprotocol.io/specification (Protocol Versioning)
**Version-Aware**: When MCP protocol version is not pinned in `.agnix.toml [spec_revisions]`, an assumption note is added indicating default protocol version is being used. Pin the version with `mcp_protocol = "2025-11-25"` for explicit control.

<a id="mcp-009"></a>
### MCP-009 [HIGH] Missing command for stdio server
**Requirement**: Stdio MCP servers MUST have a `command` field
**Detection**: Server entry has `type: "stdio"` (or no type, since stdio is default) but no `command` field
**Fix**: Add a `command` field specifying the executable to run
**Source**: modelcontextprotocol.io/specification

<a id="mcp-010"></a>
### MCP-010 [HIGH] Missing url for http/sse server
**Requirement**: HTTP and SSE MCP servers MUST have a `url` field
**Detection**: Server entry has `type: "http"` or `type: "sse"` but no `url` field
**Fix**: Add a `url` field specifying the server endpoint
**Source**: modelcontextprotocol.io/specification

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
**Requirement**: Capability keys MUST come from the spec-defined set
**Detection**: Validate keys under `capabilities` against known list
**Fix**: Remove or rename unknown capability keys
**Source**: modelcontextprotocol.io/specification/2025-11-25/basic/lifecycle

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

## GITHUB COPILOT RULES

<a id="cop-001"></a>
### COP-001 [HIGH] Empty Copilot Instruction File
**Requirement**: Copilot instruction files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful instructions
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-002"></a>
### COP-002 [HIGH] Invalid Frontmatter in Scoped Instructions
**Requirement**: Scoped instruction files (.github/instructions/*.instructions.md) MUST have valid YAML frontmatter with `applyTo` field
**Detection**: Parse YAML between `---` markers, check for `applyTo` key
**Fix**: Auto-fix (unsafe) -- insert template frontmatter with applyTo field (missing frontmatter only)
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-003"></a>
### COP-003 [HIGH] Invalid Glob Pattern in applyTo
**Requirement**: `applyTo` field MUST contain valid glob patterns
**Detection**: Attempt to parse as glob pattern
**Fix**: Correct the glob syntax
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-004"></a>
### COP-004 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: Scoped instruction frontmatter SHOULD only contain known keys (`applyTo`, `excludeAgent`)
**Detection**: Check for keys other than `applyTo` and `excludeAgent` in frontmatter
**Fix**: Remove unknown keys
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-005"></a>
### COP-005 [HIGH] Invalid excludeAgent Value
**Requirement**: The `excludeAgent` frontmatter field in scoped instruction files MUST be either `"code-review"` or `"coding-agent"`
**Detection**: Parse frontmatter, validate `excludeAgent` value against allowed set
**Fix**: Auto-fix (unsafe) -- replace with closest valid excludeAgent value
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-006"></a>
### COP-006 [MEDIUM] File Length Limit
**Requirement**: Global instruction files (`.github/copilot-instructions.md`) SHOULD not exceed ~4000 characters
**Detection**: Check `content.chars().count() > 4000`
**Fix**: Reduce content or split into scoped instruction files
**Source**: docs.github.com/en/copilot/how-tos/custom-instructions/adding-repository-custom-instructions-for-github-copilot

<a id="cop-007"></a>
### COP-007 [HIGH] Custom Agent Missing Description
**Requirement**: Custom Copilot agent files (`.github/agents/*.agent.md`) MUST include a non-empty `description` frontmatter field
**Detection**: Parse frontmatter and verify `description` exists and is non-empty
**Fix**: Add `description` to frontmatter
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-008"></a>
### COP-008 [MEDIUM] Custom Agent Unknown or Invalid Frontmatter Field
**Requirement**: Custom agent frontmatter SHOULD only use supported keys and supported value types
**Detection**: Parse frontmatter and detect unknown top-level keys plus invalid value types for typed fields (`disable-model-invocation`, `user-invocable`, `metadata`)
**Fix**: [AUTO-FIX] Remove unsupported keys (typed value violations are warning-only and not auto-fixed)
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-009"></a>
### COP-009 [HIGH] Custom Agent Invalid Target
**Requirement**: Custom agent `target` MUST be `vscode` or `github-copilot`
**Detection**: Parse `target` and validate against allowed values
**Fix**: [AUTO-FIX] Set `target` to `vscode` or `github-copilot`
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-010"></a>
### COP-010 [MEDIUM] Custom Agent infer Field Must Be Boolean
**Requirement**: Custom agent `infer` field MUST be a boolean when present
**Detection**: Parse custom agent frontmatter and validate that `infer` is boolean
**Fix**: Set `infer` to either `true` or `false`
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-011"></a>
### COP-011 [HIGH] Custom Agent Prompt Body Exceeds Length Limit
**Requirement**: Custom agent prompt body MUST be at most 30,000 characters
**Detection**: Count body characters after frontmatter and check `> 30000`
**Fix**: Reduce prompt body length
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-012"></a>
### COP-012 [MEDIUM] Custom Agent Uses GitHub.com Unsupported Fields
**Requirement**: Custom agents for GitHub.com SHOULD NOT use unsupported fields (`model`, `argument-hint`, `handoffs`)
**Detection**: Parse frontmatter and detect unsupported field presence
**Fix**: [AUTO-FIX] Remove unsupported fields for GitHub.com compatibility
**Source**: docs.github.com/en/copilot/reference/custom-agents-configuration

<a id="cop-013"></a>
### COP-013 [HIGH] Prompt File Empty Body
**Requirement**: Reusable prompt files (`.github/prompts/*.prompt.md`) MUST contain non-empty prompt body content
**Detection**: Parse optional frontmatter and check body for non-whitespace content
**Fix**: Add prompt body content
**Source**: code.visualstudio.com/docs/copilot/customization/prompt-files

<a id="cop-014"></a>
### COP-014 [MEDIUM] Prompt File Unknown Frontmatter Field
**Requirement**: Prompt file frontmatter SHOULD only use supported keys
**Detection**: Parse frontmatter and detect unknown top-level keys
**Fix**: [AUTO-FIX] Remove unsupported keys
**Source**: code.visualstudio.com/docs/copilot/customization/prompt-files

<a id="cop-015"></a>
### COP-015 [HIGH] Prompt File Invalid Agent Mode
**Requirement**: Prompt file `agent` field MUST be one of `none`, `ask`, or `always`
**Detection**: Parse frontmatter and validate `agent` value
**Fix**: [AUTO-FIX] Set `agent` to a supported mode
**Source**: code.visualstudio.com/docs/copilot/customization/prompt-files

<a id="cop-017"></a>
### COP-017 [HIGH] Copilot Hooks Schema Validation
**Requirement**: `.github/hooks/hooks.json` MUST use version `1`, valid event names, `type: "command"`, and valid command structure
**Detection**: Parse JSON and validate version, events, required hook `type`, and command object shape
**Fix**: Correct hooks schema structure
**Source**: docs.github.com/en/copilot/concepts/agents/coding-agent/about-hooks

<a id="cop-018"></a>
### COP-018 [HIGH] Copilot Setup Steps Missing or Invalid copilot-setup-steps Job
**Requirement**: `copilot-setup-steps.yml` MUST define `jobs.copilot-setup-steps` with an Ubuntu runner and non-empty `steps`
**Detection**: Parse workflow YAML and verify `jobs.copilot-setup-steps` exists, `runs-on` targets Ubuntu (or expression), and `steps` is non-empty
**Fix**: Add or correct `copilot-setup-steps` job in the workflow
**Source**: docs.github.com/en/copilot/how-tos/agents/copilot-coding-agent/customizing-the-development-environment-for-copilot-coding-agent

<a id="cop-019"></a>
### COP-019 [HIGH] Plugin Missing Required Fields
**Requirement**: Copilot plugin.json MUST have name, description, and version
**Detection**: Check plugin manifest for required fields
**Fix**: Manual
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-020"></a>
### COP-020 [MEDIUM] Plugin Invalid Field Types
**Requirement**: Plugin manifest fields SHOULD have correct types
**Detection**: Check name/version/description are strings, keywords is array, agents/skills are objects
**Fix**: Manual
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-022"></a>
### COP-022 [HIGH] CLI Skill Missing Frontmatter
**Requirement**: Copilot CLI SKILL.md MUST have name and description frontmatter
**Detection**: Parse frontmatter, check for required fields
**Fix**: Manual
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-023"></a>
### COP-023 [MEDIUM] CLI Skill Name Format
**Requirement**: Copilot CLI skill name SHOULD be kebab-case
**Detection**: Check name against lowercase-hyphen pattern
**Fix**: Manual
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-024"></a>
### COP-024 [MEDIUM] CLI Skill Unknown Field
**Requirement**: Copilot CLI SKILL.md frontmatter fields SHOULD be from known set
**Detection**: Check for unknown frontmatter keys
**Fix**: Manual
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-025"></a>
### COP-025 [LOW] Agent File Wrong Location
**Requirement**: .agent.md files MAY be under .github/agents/ or ~/.copilot/agents/
**Detection**: Check path of .agent.md files
**Fix**: Manual - move file
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-026"></a>
### COP-026 [LOW] Deprecated SSE Transport
**Requirement**: MCP servers MAY use HTTP instead of deprecated SSE
**Detection**: Check for type:"sse" in mcp-config.json
**Fix**: Manual - switch to type:"http"
**Source**: docs.github.com/en/copilot/customizing-copilot

<a id="cop-027"></a>
### COP-027 [LOW] Deprecated Infer Field
**Requirement**: Agent infer field MAY be replaced with disable-model-invocation
**Detection**: Check for infer key in .agent.md frontmatter
**Fix**: Manual - use disable-model-invocation and user-invocable
**Source**: docs.github.com/en/copilot/customizing-copilot

---

## CURSOR PROJECT RULES

<a id="cur-001"></a>
### CUR-001 [HIGH] Empty Cursor Rule File
**Requirement**: Cursor .mdc rule files MUST have non-empty content
**Detection**: `content.trim().is_empty()` after stripping frontmatter
**Fix**: Add meaningful rules content
**Source**: cursor.com/docs/context/rules

<a id="cur-002"></a>
### CUR-002 [MEDIUM] Missing Frontmatter in .mdc File
**Requirement**: Cursor .mdc files SHOULD have YAML frontmatter with metadata
**Detection**: File doesn't start with `---` markers
**Fix**: Auto-fix (unsafe) - insert template frontmatter with description and globs fields
**Source**: cursor.com/docs/context/rules

<a id="cur-003"></a>
### CUR-003 [HIGH] Invalid YAML Frontmatter
**Requirement**: .mdc file frontmatter MUST be valid YAML
**Detection**: YAML parse error on frontmatter content
**Fix**: Fix YAML syntax errors in frontmatter
**Source**: cursor.com/docs/context/rules

<a id="cur-004"></a>
### CUR-004 [HIGH] Invalid Glob Pattern in globs Field
**Requirement**: `globs` field MUST contain valid glob patterns
**Detection**: Attempt to parse as glob pattern
**Fix**: Correct the glob syntax
**Source**: cursor.com/docs/context/rules

<a id="cur-005"></a>
### CUR-005 [MEDIUM] Unknown Frontmatter Keys
**Requirement**: .mdc frontmatter SHOULD only contain known keys (description, globs, alwaysApply)
**Detection**: Check for keys other than known keys in frontmatter
**Fix**: Remove unknown keys
**Source**: cursor.com/docs/context/rules

<a id="cur-006"></a>
### CUR-006 [MEDIUM] Legacy .cursorrules File Detected
**Requirement**: Projects SHOULD migrate from .cursorrules to .cursor/rules/*.mdc format
**Detection**: File named `.cursorrules`
**Fix**: Create `.cursor/rules/` directory and migrate rules to .mdc files
**Source**: cursor.com/docs/context/rules

<a id="cur-007"></a>
### CUR-007 [MEDIUM] alwaysApply with Redundant globs
**Requirement**: When `alwaysApply: true`, the `globs` field SHOULD NOT be set (it is redundant)
**Detection**: Frontmatter has both `alwaysApply: true` and a `globs` field
**Fix**: [AUTO-FIX] Remove the `globs` field (safe)
**Source**: cursor.com/docs/context/rules

<a id="cur-008"></a>
### CUR-008 [HIGH] Invalid alwaysApply Type
**Requirement**: `alwaysApply` MUST be a boolean (`true`/`false`), not a quoted string
**Detection**: `alwaysApply` value is a string (e.g., `"true"` or `"false"`) instead of a boolean
**Fix**: Auto-fix (safe) - convert quoted string to unquoted boolean
**Source**: cursor.com/docs/context/rules

<a id="cur-009"></a>
### CUR-009 [MEDIUM] Missing Description for Agent-Requested Rule
**Requirement**: Rules with no `alwaysApply` and no `globs` (agent-requested rules) SHOULD have a `description`
**Detection**: Frontmatter has no `alwaysApply`, no `globs`, and no `description` (or empty description)
**Fix**: Add a `description` field explaining when the rule should apply
**Source**: cursor.com/docs/context/rules

<a id="cur-010"></a>
### CUR-010 [HIGH] Invalid .cursor/hooks.json Schema
**Requirement**: `.cursor/hooks.json` MUST define an integer `version` and object `hooks` map
**Detection**: Parse JSON and validate top-level shape and required fields
**Fix**: Add required fields and correct schema types for `version` and `hooks`
**Source**: cursor.com/docs/agent/hooks

<a id="cur-011"></a>
### CUR-011 [MEDIUM] Unknown Cursor Hook Event Name
**Requirement**: Hook event names in `.cursor/hooks.json` SHOULD use documented Cursor events
**Detection**: Validate each `hooks.<event>` key against allowlisted event names
**Fix**: [AUTO-FIX] Rename event keys to supported Cursor hook events
**Source**: cursor.com/docs/agent/hooks

<a id="cur-012"></a>
### CUR-012 [HIGH] Hook Entry Missing Required Command Field
**Requirement**: Each hook entry MUST include a `command` field
**Detection**: Parse `hooks.<event>[]` objects and check for missing `command`
**Fix**: Add a non-empty command to each hook object
**Source**: cursor.com/docs/agent/hooks

<a id="cur-013"></a>
### CUR-013 [HIGH] Invalid Cursor Hook Type Value
**Requirement**: Hook `type` MUST be `command` or `prompt` when present
**Detection**: Parse hook entries and validate `type` values
**Fix**: [AUTO-FIX] Change invalid `type` values to supported values
**Source**: cursor.com/docs/agent/hooks

<a id="cur-014"></a>
### CUR-014 [HIGH] Invalid Cursor Subagent Frontmatter
**Requirement**: `.cursor/agents/**/*.md` files MUST have valid YAML frontmatter with required fields and valid optional field types
**Detection**: Parse frontmatter and validate required keys (`name`, `description`), plus optional typed fields (`model`, `readonly`, `is_background`) when present
**Fix**: Correct frontmatter keys, naming format, and value types
**Source**: cursor.com/docs/context/subagents

<a id="cur-015"></a>
### CUR-015 [MEDIUM] Empty Cursor Subagent Body
**Requirement**: `.cursor/agents/**/*.md` Cursor subagent markdown files SHOULD include body instructions after frontmatter
**Detection**: Parse file and check that body content is non-empty after frontmatter
**Fix**: Add clear subagent instructions below frontmatter
**Source**: cursor.com/docs/context/subagents

<a id="cur-016"></a>
### CUR-016 [HIGH] Invalid .cursor/environment.json Schema
**Requirement**: `.cursor/environment.json` MUST be an object with string `install` (required), optional string `start`, optional array `terminals`, optional object `build` (with string `dockerfile` and `context`), and optional string `update`
**Detection**: Parse JSON and validate required fields plus terminal entry and build object structure
**Fix**: Provide required fields and valid terminal objects (`name`, `command`)
**Source**: cursor.com/docs/cloud-agent/setup

<a id="cur-017"></a>
### CUR-017 [MEDIUM] Invalid Hook Entry Field Types
**Requirement**: Hook entry fields SHOULD have correct types (timeout: number, loop_limit: number|null, failClosed: boolean)
**Detection**: Check field types in hook entries
**Fix**: Manual
**Source**: cursor.com/docs/agent/hooks

<a id="cur-018"></a>
### CUR-018 [MEDIUM] Prompt Hook Missing Prompt Field
**Requirement**: Prompt-type hooks SHOULD have a prompt field
**Detection**: Check for type:"prompt" without prompt key
**Fix**: Manual
**Source**: cursor.com/docs/agent/hooks

<a id="cur-019"></a>
### CUR-019 [LOW] Invalid Prompt Hook Model Type
**Requirement**: model field on prompt hooks MAY be validated as string
**Detection**: Check model field type
**Fix**: Manual
**Source**: cursor.com/docs/agent/hooks

---

## CLINE RULES

<a id="cln-001"></a>
### CLN-001 [HIGH] Empty Cline Rules File
**Requirement**: `.clinerules` file or files in `.clinerules/` folder MUST have non-empty content after frontmatter
**Detection**: Parse file, strip optional YAML frontmatter, check remaining body is non-whitespace
**Fix**: No auto-fix (content must be authored by user)
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

<a id="cln-002"></a>
### CLN-002 [HIGH] Invalid Paths Glob in Cline Rules
**Requirement**: `paths` field in `.clinerules/*.md` and `.clinerules/*.txt` frontmatter MUST contain valid glob patterns
**Detection**: Parse YAML frontmatter, extract `paths` field, validate each glob pattern
**Fix**: No auto-fix (glob patterns must be manually corrected)
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

<a id="cln-003"></a>
### CLN-003 [MEDIUM] Unknown Frontmatter Key in Cline Rules
**Requirement**: Frontmatter in `.clinerules/*.md` and `.clinerules/*.txt` files SHOULD only use documented keys (`paths`)
**Detection**: Parse YAML frontmatter, check all keys against allowlist
**Fix**: [AUTO-FIX unsafe] Remove unknown frontmatter keys
**Source**: docs.cline.bot/improving-your-workflow/cline-rules

<a id="cln-004"></a>
### CLN-004 [HIGH] Scalar Paths in Cline Rules
**Requirement**: `paths` field in `.clinerules/*.md` and `.clinerules/*.txt` frontmatter MUST be a YAML array, not a scalar string
**Detection**: Parse YAML frontmatter, check if `paths` is a scalar string (Cline silently ignores scalar values)
**Fix**: [AUTO-FIX safe] Convert scalar paths to array format
**Source**: docs.cline.bot/features/cline-rules

<a id="cln-005"></a>
### CLN-005 [HIGH] Empty Workflow File
**Requirement**: Workflow files MUST have content
**Detection**: Check .clinerules/workflows/*.md for empty content
**Fix**: Manual - add workflow content
**Source**: docs.cline.bot/features/cline-rules/overview

<a id="cln-006"></a>
### CLN-006 [MEDIUM] Workflow With Frontmatter
**Requirement**: Workflow files SHOULD be plain markdown without frontmatter
**Detection**: Check for --- delimiter at start of workflow file
**Fix**: Manual - remove frontmatter
**Source**: docs.cline.bot/features/cline-rules/overview

<a id="cln-009"></a>
### CLN-009 [MEDIUM] Unknown Hook Event Name
**Requirement**: Hook filenames SHOULD match valid events: TaskStart, TaskResume, TaskCancel, TaskComplete, PreToolUse, PostToolUse, UserPromptSubmit, PreCompact
**Detection**: Check filename against event list
**Fix**: Manual - rename file
**Source**: docs.cline.bot/features/cline-rules/overview

<a id="cl-sk-002"></a>
### CL-SK-002 [HIGH] Missing Skill Name
**Requirement**: Cline SKILL.md MUST have name frontmatter field
**Detection**: Parse frontmatter, check for name
**Fix**: Manual - add name field
**Source**: docs.cline.bot/features/cline-rules/overview

<a id="cl-sk-003"></a>
### CL-SK-003 [HIGH] Missing Skill Description
**Requirement**: Cline SKILL.md MUST have description frontmatter field
**Detection**: Parse frontmatter, check for description
**Fix**: Manual - add description field
**Source**: docs.cline.bot/features/cline-rules/overview

---

## OPENCODE RULES

<a id="oc-001"></a>
### OC-001 [HIGH] Invalid Share Mode
**Requirement**: The `share` field in `opencode.json` MUST be `"manual"`, `"auto"`, or `"disabled"`
**Detection**: Parse JSON, validate `share` value against allowed set
**Fix**: Auto-fix (unsafe) - replace with closest valid share mode
**Source**: opencode.ai/docs/config

<a id="oc-002"></a>
### OC-002 [HIGH] Invalid Instruction Path
**Requirement**: Paths in the `instructions` array MUST exist on disk or be valid glob patterns
**Detection**: Parse JSON, resolve each path in `instructions` array relative to config file location
**Fix**: Fix or remove broken instruction paths
**Source**: opencode.ai/docs/config

<a id="oc-003"></a>
### OC-003 [HIGH] opencode.json Parse Error
**Requirement**: `opencode.json` MUST be valid JSON (or JSONC with comments stripped)
**Detection**: Attempt JSON parse, report errors with line/column location
**Fix**: Fix JSON syntax errors
**Source**: opencode.ai/docs/config

<a id="oc-004"></a>
### OC-004 [MEDIUM] Unknown Config Key
**Requirement**: Top-level keys in `opencode.json` SHOULD be from the known configuration schema
**Detection**: Parse JSON, compare top-level keys against known key allowlist
**Fix**: Remove unrecognized keys
**Source**: opencode.ai/docs/config

<a id="oc-006"></a>
### OC-006 [LOW] Remote URL in Instructions
**Requirement**: Remote URLs in `instructions` MAY slow startup (5-second timeout per URL)
**Detection**: Check if instruction paths start with `http://` or `https://`
**Fix**: No auto-fix (user preference)
**Source**: opencode.ai/docs/config

<a id="oc-007"></a>
### OC-007 [MEDIUM] Invalid Agent Definition
**Requirement**: Custom agents in `agent` object SHOULD have a `description` field
**Detection**: Parse JSON, check each agent object for `description` key
**Fix**: Add description field to agent definitions
**Source**: opencode.ai/docs/config

<a id="oc-008"></a>
### OC-008 [HIGH] Invalid Permission Config
**Requirement**: Permission values MUST be `"allow"`, `"ask"`, or `"deny"`
**Detection**: Parse JSON, validate each permission value against allowed set
**Fix**: [AUTO-FIX] Replace invalid permission value with the closest valid mode (`"allow"`, `"ask"`, or `"deny"`)
**Source**: opencode.ai/docs/config

<a id="oc-009"></a>
### OC-009 [MEDIUM] Invalid Variable Substitution
**Requirement**: Variable substitution patterns MUST use `{env:NAME}` or `{file:path}` syntax
**Detection**: Scan all string values for `{prefix:value}` patterns, flag unknown prefixes or empty values
**Fix**: No auto-fix (must be manually corrected)
**Source**: opencode.ai/docs/config

---


<a id="oc-cfg-001"></a>
### OC-CFG-001 [HIGH] Invalid Model Format
**Requirement**: The `model` field must be formatted as `provider/model`
**Detection**: Parse JSON, validate `model` and `small_model` against the required format
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-002"></a>
### OC-CFG-002 [HIGH] Invalid autoupdate value
**Requirement**: `autoupdate` MUST be a boolean or the string `notify`
**Detection**: Parse JSON, validate `autoupdate` type/value
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-003"></a>
### OC-CFG-003 [MEDIUM] Unknown Top-level Config Field
**Requirement**: Top-level config keys SHOULD match documented OpenCode fields
**Detection**: Parse JSON object keys, flag unknown top-level keys
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-004"></a>
### OC-CFG-004 [MEDIUM] Invalid Default Agent
**Requirement**: The `default_agent` field must refer to a valid agent
**Detection**: Parse JSON, validate `default_agent`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-005"></a>
### OC-CFG-005 [HIGH] Hardcoded API Key
**Requirement**: The `apiKey` field in provider options MUST NOT be hardcoded
**Detection**: Parse JSON, scan `provider.options.apiKey` for hardcoded strings
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-006"></a>
### OC-CFG-006 [HIGH] Invalid MCP Server Structure
**Requirement**: The MCP server configuration MUST be valid
**Detection**: Parse JSON, validate `mcp` configuration objects
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-007"></a>
### OC-CFG-007 [HIGH] MCP Server Missing Command or URL
**Requirement**: Local MCP servers MUST have `command`, remote MUST have `url`
**Detection**: Parse JSON, validate `mcp` server requirements
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-001"></a>
### OC-AG-001 [HIGH] Invalid Agent Mode Value
**Requirement**: The `mode` field MUST be `subagent`, `primary`, or `all`
**Detection**: Parse JSON, validate agent `mode`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-002"></a>
### OC-AG-002 [HIGH] Invalid Color Format
**Requirement**: The `color` field MUST be a hex string or valid theme color
**Detection**: Parse JSON, validate agent `color`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-003"></a>
### OC-AG-003 [HIGH] Temperature Out of Range
**Requirement**: The `temperature` field MUST be between 0 and 2
**Detection**: Parse JSON, validate agent `temperature`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-004"></a>
### OC-AG-004 [HIGH] Steps Not a Positive Integer
**Requirement**: The `steps` field MUST be a positive integer
**Detection**: Parse JSON, validate agent `steps`
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-pm-002"></a>
### OC-PM-002 [MEDIUM] Unknown Permission Key
**Requirement**: Permission keys MUST be known actions/rules
**Detection**: Parse JSON, validate permission keys
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-pm-001"></a>
### OC-PM-001 [HIGH] Invalid Permission Action
**Requirement**: Permission actions MUST be `allow`, `ask`, or `deny`
**Detection**: Parse JSON, validate permission action values across root and nested permission maps
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-dep-001"></a>
### OC-DEP-001 [MEDIUM] Deprecated `mode` Field
**Requirement**: The top-level `mode` field is deprecated - use `agent` instead
**Detection**: Parse JSON, flag presence of `mode` key at top-level
**Fix**: [AUTO-FIX] Rename `mode` key to `agent` (safe)
**Source**: opencode.ai/docs/config

<a id="oc-dep-002"></a>
### OC-DEP-002 [MEDIUM] Deprecated `tools` Field
**Requirement**: The top-level `tools` field is deprecated - use `permission` instead
**Detection**: Parse JSON, flag presence of `tools` key at top-level
**Fix**: [AUTO-FIX] Rename `tools` key to `permission` (safe)
**Source**: opencode.ai/docs/config

<a id="oc-dep-003"></a>
### OC-DEP-003 [MEDIUM] Deprecated `autoshare` Field
**Requirement**: The top-level `autoshare` field is deprecated - use `share` instead
**Detection**: Parse JSON, flag presence of `autoshare` key at top-level
**Fix**: [AUTO-FIX] Rename `autoshare` key to `share` (safe)
**Source**: opencode.ai/docs/config

<a id="oc-dep-004"></a>
### OC-DEP-004 [MEDIUM] Deprecated CONTEXT.md Filename
**Requirement**: CONTEXT.md is deprecated - rename to AGENTS.md
**Detection**: Check if file path contains CONTEXT.md
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-dep-005"></a>
### OC-DEP-005 [MEDIUM] Deprecated TUI Keys
**Requirement**: theme, keybinds, tui keys SHOULD be in tui.json, not opencode.json
**Detection**: Check for deprecated TUI keys in opencode.json
**Fix**: Manual - move to tui.json
**Source**: opencode.ai/docs/

<a id="oc-dep-006"></a>
### OC-DEP-006 [MEDIUM] Deprecated MaxSteps Field
**Requirement**: maxSteps SHOULD be replaced with steps
**Detection**: Check for maxSteps without steps in agent config
**Fix**: Manual - rename to steps
**Source**: opencode.ai/docs/

<a id="oc-cfg-008"></a>
### OC-CFG-008 [HIGH] Invalid Log Level
**Requirement**: The `logLevel` field MUST be one of: fatal, error, warn, info, debug, trace
**Detection**: Parse JSON, validate `logLevel` value (case-insensitive)
**Fix**: [AUTO-FIX] Replace with closest valid log level (unsafe)
**Source**: opencode.ai/docs/config

<a id="oc-cfg-009"></a>
### OC-CFG-009 [HIGH] Invalid Compaction Reserved
**Requirement**: The `compaction.reserved` field MUST be a non-negative integer
**Detection**: Parse JSON, validate `compaction.reserved` is integer >= 0
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-010"></a>
### OC-CFG-010 [HIGH] Invalid Skills URL
**Requirement**: Each URL in `skills.urls` MUST start with http:// or https://
**Detection**: Parse JSON, validate each URL in skills.urls array
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-011"></a>
### OC-CFG-011 [HIGH] Invalid MCP Timeout
**Requirement**: MCP server `timeout` MUST be a positive integer
**Detection**: Parse JSON, validate timeout field in each MCP server entry
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-012"></a>
### OC-CFG-012 [HIGH] Invalid MCP OAuth Config
**Requirement**: MCP server `oauth` MUST include `client_id` and `authorization_url`
**Detection**: Parse JSON, validate oauth object structure in MCP server entries
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-cfg-013"></a>
### OC-CFG-013 [HIGH] Invalid Server Config
**Requirement**: Server config fields MUST have correct types (port: number, hostname: string, mdns: boolean, cors: array)
**Detection**: Check server object field types
**Fix**: Manual
**Source**: opencode.ai/docs/

<a id="oc-ag-005"></a>
### OC-AG-005 [HIGH] top_p Out of Range
**Requirement**: The agent `top_p` field MUST be between 0.0 and 1.0
**Detection**: Parse JSON, validate agent `top_p` value range
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-006"></a>
### OC-AG-006 [MEDIUM] Invalid Named Color
**Requirement**: The agent `color` field SHOULD be a hex color or one of: primary, secondary, accent, success, warning, error, info
**Detection**: Parse JSON, validate agent `color` against named colors and hex format
**Fix**: [AUTO-FIX] Replace with closest named color (unsafe)
**Source**: opencode.ai/docs/config

<a id="oc-ag-007"></a>
### OC-AG-007 [MEDIUM] Redundant steps and maxSteps
**Requirement**: Agents SHOULD use `steps` only - `maxSteps` is redundant when both are present
**Detection**: Parse JSON, detect both `steps` and `maxSteps` in agent definition
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-008"></a>
### OC-AG-008 [HIGH] Invalid hidden Type
**Requirement**: The agent `hidden` field MUST be a boolean
**Detection**: Parse JSON, validate agent `hidden` is boolean
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-ag-009"></a>
### OC-AG-009 [HIGH] Invalid Agent Disable Type
**Requirement**: Agent disable field MUST be boolean
**Detection**: Check disable field type
**Fix**: Manual
**Source**: opencode.ai/docs/

<a id="oc-lsp-001"></a>
### OC-LSP-001 [MEDIUM] LSP Command Without Extensions
**Requirement**: LSP entries with `command` SHOULD also define `extensions`
**Detection**: Parse JSON, flag LSP entries with command but no extensions
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-lsp-002"></a>
### OC-LSP-002 [HIGH] Invalid LSP Extensions
**Requirement**: LSP `extensions` MUST be a non-empty array of strings
**Detection**: Parse JSON, validate extensions field in LSP entries
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-tui-001"></a>
### OC-TUI-001 [MEDIUM] Unknown TUI Key
**Requirement**: TUI configuration keys SHOULD be from the known set
**Detection**: Parse JSON, compare TUI keys against known set
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-tui-002"></a>
### OC-TUI-002 [HIGH] Invalid scroll_speed
**Requirement**: The `tui.scroll_speed` field MUST be a number >= 0.001
**Detection**: Parse JSON, validate scroll_speed value
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-tui-003"></a>
### OC-TUI-003 [HIGH] Invalid diff_style
**Requirement**: The `tui.diff_style` field MUST be `auto` or `stacked`
**Detection**: Parse JSON, validate diff_style value
**Fix**: [AUTO-FIX] Replace with closest valid diff style (unsafe)
**Source**: opencode.ai/docs/config

<a id="oc-agm-001"></a>
### OC-AGM-001 [HIGH] Empty AGENTS.md
**Requirement**: `AGENTS.md` MUST NOT be empty
**Detection**: Check `AGENTS.md` file size/content
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

<a id="oc-agm-002"></a>
### OC-AGM-002 [HIGH] Secrets in AGENTS.md
**Requirement**: `AGENTS.md` MUST NOT contain hardcoded secrets
**Detection**: Scan `AGENTS.md` for secret patterns
**Fix**: No auto-fix
**Source**: opencode.ai/docs/config

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

---

## CODEX CLI RULES

<a id="cdx-000"></a>
### CDX-000 [HIGH] TOML Parse Error
**Requirement**: Codex config.toml files MUST have valid TOML syntax
**Detection**: Attempt to parse as TOML; report parse errors with line/column
**Fix**: Correct the TOML syntax
**Source**: developers.openai.com/codex/


<a id="cdx-001"></a>
### CDX-001 [HIGH] Invalid Approval Mode
**Requirement**: The `approvalMode` field in `.codex/config.toml` MUST be `"suggest"`, `"auto-edit"`, or `"full-auto"`
**Detection**: Parse TOML, validate `approvalMode` value against allowed set
**Fix**: Auto-fix (unsafe) -- replace with closest valid approval mode
**Source**: developers.openai.com/codex/

<a id="cdx-002"></a>
### CDX-002 [HIGH] Invalid Full Auto Error Mode
**Requirement**: The `fullAutoErrorMode` field in `.codex/config.toml` MUST be `"ask-user"` or `"ignore-and-continue"`
**Detection**: Parse TOML, validate `fullAutoErrorMode` value against allowed set
**Fix**: Auto-fix (unsafe) -- replace with closest valid full auto error mode
**Source**: developers.openai.com/codex/

<a id="cdx-003"></a>
### CDX-003 [MEDIUM] AGENTS.override.md in Version Control
**Requirement**: `AGENTS.override.md` SHOULD NOT be committed to version control (contains user-specific overrides)
**Detection**: Check if file name is `AGENTS.override.md`
**Fix**: Add `AGENTS.override.md` to `.gitignore`
**Source**: developers.openai.com/codex/

<a id="cdx-004"></a>
### CDX-004 [MEDIUM] Unknown Config Key
**Requirement**: Top-level keys in `.codex/config.toml` SHOULD be from the known configuration schema
**Detection**: Parse TOML, compare top-level keys against known key and table allowlists
**Fix**: [AUTO-FIX] Remove unrecognized keys
**Source**: developers.openai.com/codex/

<a id="cdx-005"></a>
### CDX-005 [HIGH] project_doc_max_bytes Exceeds Limit
**Requirement**: `project_doc_max_bytes` in `.codex/config.toml` MUST be a positive integer <= 65536
**Detection**: Parse TOML, validate `project_doc_max_bytes` value is an integer within the allowed range
**Fix**: Reduce value to 65536 or less (default: 32768)
**Source**: developers.openai.com/codex/

<a id="cdx-006"></a>
### CDX-006 [HIGH] Invalid project_doc_fallback_filenames
**Requirement**: `project_doc_fallback_filenames` in `.codex/config.toml` MUST be an array of unique, non-empty filename strings
**Detection**: Parse TOML, validate array type, ensure all entries are non-empty strings, flag duplicate and path-like entries
**Fix**: Use a unique array of bare filenames (e.g., `["AGENTS.md", "README.md"]`)
**Source**: developers.openai.com/codex/guides/agents-md/

<a id="cdx-cfg-001"></a>
### CDX-CFG-001 [HIGH] Invalid approval_policy Value
**Requirement**: `approval_policy` in Codex config MUST be `untrusted`, `on-request`, `never`, or `on-failure`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `approval_policy` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-002"></a>
### CDX-CFG-002 [HIGH] Invalid sandbox_mode Value
**Requirement**: `sandbox_mode` in Codex config MUST be `read-only`, `workspace-write`, or `danger-full-access`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `sandbox_mode` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-003"></a>
### CDX-CFG-003 [HIGH] Invalid model_reasoning_effort Value
**Requirement**: `model_reasoning_effort` in Codex config MUST be one of `none|minimal|low|medium|high|xhigh`
**Detection**: Parse `.codex/config.toml|json|yaml` and validate `model_reasoning_effort` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

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
**Requirement**: Codex config keys SHOULD match the official schema at top-level and known nested sections (`features`, `tui`, `shell_environment_policy`, `mcp_servers`, `apps`)
**Detection**: Parse `.codex/config.toml|json|yaml`, compare observed keys against allowlists, and report unknown keys
**Fix**: No auto-fix (remove or rename unsupported fields)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-007"></a>
### CDX-CFG-007 [HIGH] Danger Full Access Without Acknowledgment
**Requirement**: `sandbox_mode = "danger-full-access"` MUST include explicit acknowledgement via `notice.hide_full_access_warning = true`
**Detection**: Parse config and flag danger-full-access when explicit warning acknowledgment is absent
**Fix**: No auto-fix (explicitly acknowledge risk or reduce sandbox mode)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-008"></a>
### CDX-CFG-008 [HIGH] Invalid shell_environment_policy.inherit Value
**Requirement**: `shell_environment_policy.inherit` MUST be one of `core|all|none`
**Detection**: Parse config and validate `shell_environment_policy.inherit` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

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
### CDX-CFG-011 [MEDIUM] Invalid Feature Flag Name
**Requirement**: Keys under `[features]` SHOULD use known Codex feature flag names
**Detection**: Parse config and report unknown keys under `features`
**Fix**: No auto-fix (remove unsupported flags or rename)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-012"></a>
### CDX-CFG-012 [HIGH] Invalid cli_auth_credentials_store Value
**Requirement**: `cli_auth_credentials_store` MUST be one of `file|keyring|auto|ephemeral`
**Detection**: Parse config and validate `cli_auth_credentials_store` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-ag-001"></a>
### CDX-AG-001 [HIGH] Empty AGENTS.md for Codex
**Requirement**: AGENTS.md used by Codex MUST contain actionable project guidance
**Detection**: For `AGENTS.md` variants, flag files where `content.trim().is_empty()`
**Fix**: No auto-fix (add repository-specific instructions)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-ag-002"></a>
### CDX-AG-002 [HIGH] Secrets in AGENTS.md for Codex
**Requirement**: AGENTS.md MUST NOT include hardcoded credentials or tokens
**Detection**: Scan AGENTS.md lines for secret markers/prefixes and credential-like assignments
**Fix**: No auto-fix (remove secrets and use environment variables)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-ag-003"></a>
### CDX-AG-003 [MEDIUM] Generic AGENTS.md Guidance for Codex
**Requirement**: AGENTS.md SHOULD provide specific, actionable repo guidance instead of generic boilerplate
**Detection**: Detect generic-only instruction content lacking concrete commands/paths/constraints
**Fix**: No auto-fix (replace generic text with concrete repository guidance)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-app-001"></a>
### CDX-APP-001 [HIGH] Invalid default_tools_approval_mode Value
**Requirement**: `apps.<app_id>.default_tools_approval_mode` MUST be one of `auto|prompt|approve`
**Detection**: Parse config and validate app-level `default_tools_approval_mode` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-cfg-013"></a>
### CDX-CFG-013 [HIGH] Invalid sandbox_workspace_write Mode
**Requirement**: `sandbox_workspace_write.mode` MUST be one of `allowlist|denylist|all`
**Detection**: Parse config and validate `sandbox_workspace_write.mode` enum values
**Fix**: No auto-fix (set to a supported value)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

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
**Requirement**: Granular approval_policy sub-fields SHOULD be from known set
**Detection**: Check object keys against: sandbox_approval, rules, mcp_elicitations, request_permissions, skill_approval
**Fix**: Manual
**Source**: developers.openai.com/codex/

<a id="cdx-cfg-024"></a>
### CDX-CFG-024 [MEDIUM] Invalid Approvals Reviewer Value
**Requirement**: approvals_reviewer SHOULD be "user" or "guardian_subagent"
**Detection**: Check enum value
**Fix**: Manual
**Source**: developers.openai.com/codex/

<a id="cdx-cfg-025"></a>
### CDX-CFG-025 [MEDIUM] Invalid Service Tier Value
**Requirement**: service_tier SHOULD be "flex" or "fast"
**Detection**: Check enum value
**Fix**: Manual
**Source**: developers.openai.com/codex/

<a id="cdx-cfg-026"></a>
### CDX-CFG-026 [LOW] Invalid Network Permission Field
**Requirement**: permissions.network sub-fields MAY be validated
**Detection**: Check keys against known network permission fields
**Fix**: Manual
**Source**: developers.openai.com/codex/

<a id="cdx-cfg-027"></a>
### CDX-CFG-027 [LOW] Invalid Windows Sandbox Value
**Requirement**: windows.sandbox SHOULD be "elevated" or "unelevated"
**Detection**: Check enum value
**Fix**: Manual
**Source**: developers.openai.com/codex/

<a id="cdx-ag-004"></a>
### CDX-AG-004 [MEDIUM] AGENTS.md Exceeds Size Limit
**Requirement**: AGENTS.md SHOULD not exceed 100,000 bytes
**Detection**: Check file size against the limit
**Fix**: No auto-fix (reduce content length)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-ag-005"></a>
### CDX-AG-005 [MEDIUM] AGENTS.md References Missing File
**Requirement**: File references in AGENTS.md SHOULD point to existing files
**Detection**: Extract backtick-quoted file paths and check existence
**Fix**: No auto-fix (fix reference or create missing file)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-ag-006"></a>
### CDX-AG-006 [LOW] AGENTS.md Missing Project Context
**Requirement**: AGENTS.md SHOULD include project-specific structure like headings, commands, and paths
**Detection**: Check for presence of headings, backtick commands, and file paths
**Fix**: No auto-fix (add project-specific content)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-ag-007"></a>
### CDX-AG-007 [MEDIUM] AGENTS.md Contradicts config.toml
**Requirement**: AGENTS.md instructions SHOULD be consistent with config.toml values
**Detection**: Cross-file analysis (project-level check)
**Fix**: No auto-fix (align instructions with configuration)
**Source**: developers.openai.com/codex/guides/agents-md

<a id="cdx-app-002"></a>
### CDX-APP-002 [MEDIUM] Invalid skills Configuration
**Requirement**: `skills` SHOULD be a TOML table (object)
**Detection**: Parse config and verify `skills` is an object type
**Fix**: No auto-fix (configure as a TOML table)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

<a id="cdx-app-003"></a>
### CDX-APP-003 [MEDIUM] Invalid profile Configuration
**Requirement**: `profile` SHOULD be a string
**Detection**: Parse config and verify `profile` is a string type
**Fix**: No auto-fix (set to a valid profile name string)
**Source**: developers.openai.com/codex/config-reference, developers.openai.com/codex/config-schema.json

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
**Requirement**: Kiro steering files MUST use a valid inclusion mode
**Detection**: Frontmatter `inclusion` field is not one of: always, fileMatch, manual, auto
**Fix**: [AUTO-FIX] Use one of: always, fileMatch, manual, auto
**Source**: kiro.dev/docs/steering/

<a id="kiro-002"></a>
### KIRO-002 [HIGH] Missing Required Fields for Inclusion Mode
**Requirement**: Steering files MUST include required fields for their inclusion mode
**Detection**: `inclusion: auto` without `name` and `description` fields, or `inclusion: fileMatch` without `fileMatchPattern` field
**Fix**: Add the missing required fields for the specified inclusion mode
**Source**: kiro.dev/docs/steering/

<a id="kiro-003"></a>
### KIRO-003 [MEDIUM] Invalid fileMatchPattern Glob
**Requirement**: The `fileMatchPattern` field SHOULD contain a valid glob pattern
**Detection**: Glob pattern fails to parse
**Fix**: Fix the glob pattern syntax
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
**Detection**: Content includes likely secret assignment patterns (API keys, tokens, passwords, secrets)
**Fix**: Remove plaintext secrets and use environment variable expansion
**Source**: kiro.dev/docs/steering/

<a id="kiro-007"></a>
### KIRO-007 [MEDIUM] fileMatchPattern Without fileMatch Inclusion
**Requirement**: `fileMatchPattern` SHOULD only be used with `inclusion: fileMatch`
**Detection**: `fileMatchPattern` exists while inclusion mode is missing or not `fileMatch`
**Fix**: Set `inclusion: fileMatch` or remove `fileMatchPattern`
**Source**: kiro.dev/docs/steering/

<a id="kiro-008"></a>
### KIRO-008 [MEDIUM] Unknown Steering Frontmatter Field
**Requirement**: Steering frontmatter SHOULD use only documented keys
**Detection**: Frontmatter includes key outside `inclusion`, `name`, `description`, `fileMatchPattern`
**Fix**: Rename or remove unknown frontmatter keys
**Source**: kiro.dev/docs/steering/

<a id="kiro-009"></a>
### KIRO-009 [MEDIUM] Broken Inline File Reference
**Requirement**: Inline `#[[file:...]]` references SHOULD resolve to existing files
**Detection**: Inline file reference points to a path that does not exist
**Fix**: Correct the file reference path or create the missing file
**Source**: kiro.dev/docs/steering/

<a id="kiro-010"></a>
### KIRO-010 [MEDIUM] Missing Inclusion Mode
**Requirement**: Kiro steering frontmatter SHOULD include an `inclusion` field
**Detection**: Check if `inclusion` field is absent from frontmatter
**Fix**: No auto-fix (add inclusion field)
**Source**: kiro.dev/docs/steering

<a id="kiro-011"></a>
### KIRO-011 [LOW] Steering Doc Excessively Long
**Requirement**: Kiro steering documents SHOULD be concise (under 50,000 bytes)
**Detection**: Check document byte length against threshold
**Fix**: No auto-fix (split into smaller files)
**Source**: kiro.dev/docs/steering

<a id="kiro-012"></a>
### KIRO-012 [MEDIUM] Duplicate Steering Name
**Requirement**: Kiro steering file names SHOULD be unique across the project
**Detection**: Cross-file analysis (project-level check)
**Fix**: No auto-fix (rename steering files)
**Source**: kiro.dev/docs/steering

<a id="kiro-013"></a>
### KIRO-013 [MEDIUM] Conflicting Inclusion Modes
**Requirement**: Kiro steering frontmatter SHOULD have exactly one `inclusion` key
**Detection**: Count `inclusion:` entries in frontmatter
**Fix**: No auto-fix (remove duplicate entries)
**Source**: kiro.dev/docs/steering

<a id="kiro-014"></a>
### KIRO-014 [LOW] Markdown Structure Issues
**Requirement**: Kiro steering body SHOULD have Markdown heading structure
**Detection**: Check if body content contains at least one heading
**Fix**: No auto-fix (add headings for structure)
**Source**: kiro.dev/docs/steering

---

## KIRO POWERS RULES

<a id="kr-pw-001"></a>
### KR-PW-001 [HIGH] Missing Required POWER.md Frontmatter Fields
**Requirement**: `POWER.md` MUST define required frontmatter fields `name`, `description`, and `keywords`
**Detection**: Missing frontmatter, invalid frontmatter, or missing required fields
**Fix**: Add required frontmatter fields and valid YAML
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-002"></a>
### KR-PW-002 [MEDIUM] Empty POWER.md Keywords Array
**Requirement**: `keywords` SHOULD contain one or more activation keywords
**Detection**: `keywords` exists but is an empty array
**Fix**: Add one or more keywords used for power activation
**Source**: kiro.dev/docs/powers/

<a id="kr-pw-003"></a>
### KR-PW-003 [MEDIUM] Empty POWER.md Body
**Requirement**: `POWER.md` SHOULD include onboarding/workflow/reference content
**Detection**: Non-empty frontmatter with empty markdown body
**Fix**: Add body content describing usage and behavior
**Source**: kiro.dev/docs/powers/create

<a id="kr-pw-004"></a>
### KR-PW-004 [MEDIUM] Invalid Adjacent Power mcp.json Structure
**Requirement**: Power-local `mcp.json` SHOULD define a valid `mcpServers` object
**Detection**: Adjacent `mcp.json` is malformed, missing `mcpServers`, or uses invalid `mcpServers` shape
**Fix**: Update `mcp.json` to use valid `mcpServers` object structure
**Source**: kiro.dev/docs/powers/

<a id="kr-pw-005"></a>
### KR-PW-005 [HIGH] Step Missing Description
**Requirement**: Each step heading in POWER.md MUST have description content below it
**Detection**: Check for empty sections after `## Step` headings
**Fix**: No auto-fix (add step description)
**Source**: kiro.dev/docs/powers

<a id="kr-pw-006"></a>
### KR-PW-006 [LOW] Duplicate Keywords
**Requirement**: POWER.md keywords SHOULD be unique
**Detection**: Check for duplicate entries in keywords array (case-insensitive)
**Fix**: No auto-fix (remove duplicate keywords)
**Source**: kiro.dev/docs/powers

<a id="kr-pw-007"></a>
### KR-PW-007 [MEDIUM] Name Invalid Characters
**Requirement**: POWER.md name SHOULD use lowercase alphanumeric characters and hyphens only
**Detection**: Validate name against `^[a-z0-9][a-z0-9_-]*$` pattern
**Fix**: No auto-fix (rename to valid format)
**Source**: kiro.dev/docs/powers

<a id="kr-pw-008"></a>
### KR-PW-008 [HIGH] Secrets in Power Body
**Requirement**: POWER.md body MUST NOT contain hardcoded credentials
**Detection**: Scan body for secret patterns (API keys, tokens, passwords)
**Fix**: No auto-fix (use environment variables)
**Source**: kiro.dev/docs/powers

---

## KIRO IDE HOOKS RULES

<a id="kr-hk-001"></a>
### KR-HK-001 [HIGH] Invalid Kiro IDE Hook Event Type
**Requirement**: IDE hook `event` MUST be one of the documented Kiro IDE events
**Detection**: `event` is missing or outside `fileEdited`, `fileCreate`, `fileDelete`, `promptSubmit`, `agentStop`, `preToolUse`, `postToolUse`, `manual`
**Fix**: Set `event` to a valid IDE hook event
**Source**: kiro.dev/docs/hooks/types

<a id="kr-hk-002"></a>
### KR-HK-002 [HIGH] File Event Hook Missing Patterns
**Requirement**: File-based IDE hooks MUST include `patterns`
**Detection**: `event` is `fileEdited`, `fileCreate`, or `fileDelete` and `patterns` is missing/empty
**Fix**: Add one or more glob patterns in `patterns`
**Source**: kiro.dev/docs/hooks/types

<a id="kr-hk-003"></a>
### KR-HK-003 [HIGH] IDE Hook Missing Action
**Requirement**: IDE hooks MUST define at least one action (`runCommand` or `askAgent`)
**Detection**: Both top-level and nested `then` action fields are missing
**Fix**: Add `runCommand` or `askAgent`
**Source**: kiro.dev/docs/hooks/actions

<a id="kr-hk-004"></a>
### KR-HK-004 [MEDIUM] Tool Hook Missing toolTypes Filter
**Requirement**: `preToolUse`/`postToolUse` hooks SHOULD specify `toolTypes`
**Detection**: Tool hook event has no `toolTypes` filter
**Fix**: Add `toolTypes` to narrow hook scope
**Source**: kiro.dev/docs/hooks/types

<a id="kr-hk-007"></a>
### KR-HK-007 [MEDIUM] Hook Timeout Out of Range
**Requirement**: Hook timeout SHOULD be between 1 and 300,000 milliseconds
**Detection**: Check timeout value is within valid range
**Fix**: No auto-fix (set to a valid timeout)
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-008"></a>
### KR-HK-008 [MEDIUM] Duplicate Event Handlers
**Requirement**: Hook files SHOULD NOT duplicate the same event+pattern combination
**Detection**: Cross-file analysis (project-level check)
**Fix**: No auto-fix (remove duplicate handlers)
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-009"></a>
### KR-HK-009 [MEDIUM] Command Uses Absolute Path
**Requirement**: Hook commands SHOULD use relative paths for portability
**Detection**: Check if command starts with `/` or drive letter
**Fix**: No auto-fix (use relative paths)
**Source**: kiro.dev/docs/hooks

<a id="kr-hk-010"></a>
### KR-HK-010 [HIGH] Secrets in Hook Command
**Requirement**: Hook commands MUST NOT contain hardcoded credentials
**Detection**: Scan command for secret patterns (API keys, tokens, passwords)
**Fix**: No auto-fix (use environment variables)
**Source**: kiro.dev/docs/hooks

---

## KIRO MCP RULES

<a id="kr-mcp-001"></a>
### KR-MCP-001 [HIGH] Kiro MCP Server Missing command and url
**Requirement**: Each Kiro MCP server MUST define `command` (local) or `url` (remote)
**Detection**: Server entry has neither `command` nor `url`, or top-level `mcpServers` is invalid
**Fix**: Define `command` or `url` for each MCP server
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-002"></a>
### KR-MCP-002 [MEDIUM] Hardcoded Secrets in Kiro MCP env
**Requirement**: Sensitive MCP `env` values SHOULD use variable expansion instead of plaintext
**Detection**: Sensitive env keys (API_KEY/SECRET/TOKEN/PASSWORD) contain hardcoded values
**Fix**: Use `${VAR_NAME}` expansion for sensitive env values
**Source**: kiro.dev/docs/mcp/configuration

<a id="kr-mcp-003"></a>
### KR-MCP-003 [MEDIUM] Missing Required Args
**Requirement**: Command-based MCP servers SHOULD include an `args` array
**Detection**: Server has `command` but no `args` or empty `args`
**Fix**: No auto-fix (add args array)
**Source**: kiro.dev/docs/mcp

<a id="kr-mcp-004"></a>
### KR-MCP-004 [HIGH] Invalid MCP URL
**Requirement**: Remote MCP server `url` MUST use a valid scheme (http/https/ws/wss/sse)
**Detection**: Validate URL scheme
**Fix**: No auto-fix (fix URL format)
**Source**: kiro.dev/docs/mcp

<a id="kr-mcp-005"></a>
### KR-MCP-005 [MEDIUM] Duplicate MCP Server Names
**Requirement**: MCP server names SHOULD be unique across configurations
**Detection**: Cross-file analysis (project-level check)
**Fix**: No auto-fix (rename servers)
**Source**: kiro.dev/docs/mcp

---

## UNIVERSAL RULES (XML)

<a id="xml-001"></a>
### XML-001 [HIGH] Unclosed XML Tag
**Requirement**: All XML tags MUST be properly closed
**Detection**: Parse tags, check balance with stack
**Fix**: [AUTO-FIX] Automatically insert matching closing XML tag
**Source**: platform.claude.com/docs prompt engineering

<a id="xml-002"></a>
### XML-002 [HIGH] Mismatched Closing Tag
**Requirement**: Closing tag MUST match opening tag
**Detection**: `stack.last().name != closing_tag.name`
**Fix**: Replace with correct closing tag
**Source**: XML parsing standard

<a id="xml-003"></a>
### XML-003 [HIGH] Unmatched Closing Tag
**Requirement**: Closing tag MUST have corresponding opening tag
**Detection**: `stack.is_empty() && found_closing_tag`
**Fix**: Remove or add opening tag
**Source**: XML parsing standard

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

<a id="ref-004"></a>
### REF-004 [MEDIUM] Non-Markdown Import
**Requirement**: @imports SHOULD reference .md files only
**Detection**: Extract @imports, check file extension, flag non-`.md` extensions
**Fix**: Convert referenced content to markdown or remove the import
**Source**: Claude Code memory docs

---

## PROMPT ENGINEERING RULES

<a id="pe-001"></a>
### PE-001 [MEDIUM] Lost in the Middle
**Requirement**: Critical content SHOULD NOT be in middle 40-60%
**Detection**: Find "critical|important|must" positions, check if in middle
**Fix**: Move to start or end
**Source**: Liu et al. (2023), "Lost in the Middle: How Language Models Use Long Contexts", TACL

<a id="pe-002"></a>
### PE-002 [MEDIUM] Chain-of-Thought on Simple Task
**Requirement**: SHOULD NOT use "think step by step" for simple operations
**Detection**: Check for CoT phrases in simple skills (file reads, basic commands)
**Fix**: Remove CoT instructions
**Source**: Wei et al. (2022), research shows CoT hurts simple tasks

<a id="pe-003"></a>
### PE-003 [MEDIUM] Weak Imperative Language
**Requirement**: Use strong language (must/always/never) for critical rules
**Detection**: Critical section with `should|could|try|consider|maybe`
**Fix**: [AUTO-FIX] Replace with must/always/required
**Source**: Multiple prompt engineering studies

<a id="pe-004"></a>
### PE-004 [MEDIUM] Ambiguous Instructions
**Requirement**: Instructions SHOULD be specific and measurable
**Detection**: Check for vague terms without concrete criteria
**Fix**: Add specific criteria or examples
**Source**: Anthropic prompt engineering guide

<a id="pe-005"></a>
### PE-005 [MEDIUM] Redundant Generic Instructions
**Requirement**: Instructions SHOULD NOT include generic directives that LLMs already follow by default
**Detection**: Check for phrases like "be helpful", "be accurate", "be concise", "follow instructions", etc.
**Fix**: [AUTO-FIX] Remove generic instructions and focus on project-specific behavior
**Source**: Anthropic prompt engineering guide

<a id="pe-006"></a>
### PE-006 [MEDIUM] Negative-Only Instructions
**Requirement**: Negative instructions SHOULD include a positive alternative
**Detection**: Check for "don't/never/avoid" without "instead/rather/prefer" within 3-line window
**Fix**: Add positive alternative (e.g., "Instead, use...")
**Source**: Anthropic prompt engineering guide

---

## CROSS-PLATFORM RULES

<a id="xp-001"></a>
### XP-001 [HIGH] Platform-Specific Feature in Generic Config
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
**Source**: developers.openai.com/codex/guides/agents-md, opencode.ai/docs/rules, docs.cursor.com/en/context, docs.cline.bot/features/custom-instructions, github.com/github/docs/changelog/2025-06-17-github-copilot-coding-agent-now-supports-agents-md-custom-instructions

<a id="xp-003"></a>
### XP-003 [MEDIUM] Hard-Coded Platform Paths
**Requirement**: Paths SHOULD use environment variables
**Detection**: Check for `.claude/`, `.opencode/` in configs
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
**Requirement**: AGENTS.md SHOULD stay under Codex CLI's 32768-byte default limit
**Detection**: Check byte length of AGENTS.md content against the 32768-byte threshold
**Fix**: Reduce content or split into multiple files using @import
**Source**: developers.openai.com/codex/guides/agents-md

<a id="xp-008"></a>
### XP-008 [MEDIUM] Claude-specific Features in CLAUDE.md for Cursor
**Requirement**: CLAUDE.md SHOULD guard Claude-specific features under a `## Claude Code` section when targeting Cursor
**Detection**: When target tool is Cursor, check CLAUDE.md and CLAUDE.local.md for Claude-specific directives (context:fork, agent fields, allowed-tools, hooks, @import) outside guarded sections
**Fix**: No auto-fix - move Cursor-compatible instructions to .cursor/rules/ or guard Claude-specific content under a `## Claude Code` section header
**Source**: docs.cursor.com/context/rules-for-ai

<a id="xp-sk-001"></a>
### XP-SK-001 [LOW] Skill Uses Client-Specific Features
**Requirement**: Skills SHOULD avoid client-specific frontmatter fields for maximum portability
**Detection**: Skill frontmatter uses extension fields (model, context, agent, hooks, etc.) that are not part of the universal Agent Skills spec
**Fix**: No auto-fix -- review whether the field is needed or can be removed for portability
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
- CC-MEM-001, CC-MEM-005 (Memory critical)
- XML-001 through XML-003 (XML balance)
- REF-001 through REF-004 (Import/reference validation)

### P1 (Week 4)
Add these 15 rules:
- AS-010 through AS-015 (Skills best practices)
- CC-MEM-006 through CC-MEM-010 (Memory quality)
- CC-AG-001 through CC-AG-013 (Agents)
- CC-PL-001 through CC-PL-010 (Plugins)

### P2 (Week 5-6)
Complete coverage:
- MCP-001 through MCP-006 (MCP protocol)
- PE-001 through PE-006 (Prompt engineering)
- XP-001 through XP-008, XP-SK-001 (Cross-platform)
- CR-SK-001, CL-SK-001, CP-SK-001, CX-SK-001, OC-SK-001, WS-SK-001, KR-SK-001, KR-AG-001 through KR-AG-007, KR-HK-001 through KR-HK-006, KR-PW-001 through KR-PW-004, KR-MCP-001 through KR-MCP-002, AMP-SK-001, RC-SK-001 (Per-client and Kiro rules)
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
    let name_re = Regex::new(r"^[a-z0-9]+(-[a-z0-9]+)*$").unwrap();
    if !name_re.is_match(&frontmatter.name) || frontmatter.name.len() > 64 {
        diagnostics.push(Diagnostic::error(
            path, 2, 0, "AS-004",
            format!("Invalid name format: {}", frontmatter.name)
        ).with_suggestion("Use lowercase letters, numbers, hyphens only"));
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
| AS-010 | Prepend "Use when user wants to " | unsafe |
| AS-014 | Normalize Windows path separators | safe |
| CC-SK-001 | Default invalid model to sonnet | unsafe |
| CC-SK-002 | Normalize context to fork | unsafe |
| CC-SK-003 | Add default agent for fork context | unsafe |
| CC-SK-004 | Insert context: fork before agent key | unsafe |
| CC-SK-007 | Suggest Bash(git:*) matcher | unsafe |
| CC-SK-011 | Remove disable-model-invocation line | unsafe |
| CC-SK-014 | Convert string to boolean | safe |
| CC-SK-015 | Convert string to boolean | safe |
| CC-HK-001 | Correct event name casing/typo | safe/unsafe |
| CC-HK-004 | Clamp timeout to valid range | safe |
| CC-HK-011 | Remove redundant wildcard matcher | unsafe |
| CC-HK-013 | Remove async field | safe |
| CC-HK-015 | Remove model field | safe |
| CC-HK-018 | Remove matcher field | safe |
| CC-HK-019 | Replace Setup with SessionStart | unsafe |
| CC-AG-003 | Default invalid model to sonnet | unsafe |
| CC-AG-004 | Default invalid permission mode | unsafe |
| CC-AG-008 | Replace with closest memory scope | unsafe |
| CC-MEM-005 | Remove generic instruction line | safe |
| CC-MEM-007 | Replace weak language with strong | safe/unsafe |
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
| CC-AG-013 | Replace skill name with kebab-case version | unsafe |
| CC-SK-006 | Insert disable-model-invocation: true | unsafe |
| CC-SK-012 | Append $ARGUMENTS to body | unsafe |
| CC-PL-003 | Normalize partial semver | unsafe |
| AGM-001 | Append closing code fence for unclosed blocks | unsafe |
| GM-001 | Append closing code fence for unclosed blocks | unsafe |
| GM-008 | Strip directory prefix from contextFileName | unsafe |
| PE-003 | Replace weak language with stronger alternative | unsafe |
| PE-005 | Delete redundant instruction line | unsafe |
| REF-003 | Delete duplicate import line | unsafe |
| CUR-011 | Replace invalid cursor hook event with closest match | unsafe |
| CUR-013 | Replace invalid cursor hook type with closest match | unsafe |
| KIRO-001 | Replace invalid inclusion mode with closest match | unsafe |
| OC-008 | Replace invalid permission mode with closest match | unsafe |
| OC-DEP-001 | Rename `mode` key to `agent` | safe |
| OC-DEP-002 | Rename `tools` key to `permission` | safe |
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
| Agent Skills | 19 | 15 | 4 | 0 | 9 |
| Claude Skills | 17 | 11 | 6 | 0 | 11 |
| Claude Hooks | 19 | 12 | 5 | 2 | 12 |
| Claude Agents | 13 | 12 | 1 | 0 | 7 |
| Claude Memory | 12 | 8 | 4 | 0 | 3 |
| AGENTS.md | 6 | 1 | 5 | 0 | 1 |
| Claude Plugins | 10 | 8 | 2 | 0 | 3 |
| GitHub Copilot | 17 | 11 | 6 | 0 | 8 |
| Cursor | 16 | 9 | 7 | 0 | 6 |
| Cline | 4 | 3 | 1 | 0 | 2 |
| OpenCode | 41 | 26 | 14 | 1 | 8 |
| Gemini CLI | 9 | 3 | 4 | 2 | 3 |
| Codex CLI | 39 | 21 | 17 | 1 | 3 |
| Windsurf | 4 | 1 | 2 | 1 | 0 |
| MCP | 24 | 19 | 5 | 0 | 7 |
| XML | 3 | 3 | 0 | 0 | 3 |
| References | 4 | 2 | 2 | 0 | 1 |
| Prompt Eng | 6 | 0 | 6 | 0 | 2 |
| Cross-Platform | 9 | 2 | 6 | 1 | 0 |
| Cursor Skills | 1 | 0 | 1 | 0 | 1 |
| Cline Skills | 1 | 0 | 1 | 0 | 1 |
| Copilot Skills | 1 | 0 | 1 | 0 | 1 |
| Codex Skills | 1 | 0 | 1 | 0 | 1 |
| OpenCode Skills | 1 | 0 | 1 | 0 | 1 |
| Windsurf Skills | 1 | 0 | 1 | 0 | 1 |
| Kiro Skills | 1 | 0 | 1 | 0 | 1 |
| Kiro Agents | 13 | 4 | 7 | 2 | 0 |
| Kiro Hooks | 10 | 6 | 4 | 0 | 0 |
| Kiro MCP | 5 | 2 | 3 | 0 | 0 |
| Kiro Powers | 8 | 3 | 4 | 1 | 0 |
| Kiro Steering | 14 | 3 | 9 | 2 | 1 |
| Amp Skills | 1 | 0 | 1 | 0 | 1 |
| Amp Checks | 4 | 2 | 2 | 0 | 3 |
| Roo Code Skills | 1 | 0 | 1 | 0 | 1 |
| Roo Code | 6 | 3 | 3 | 0 | 0 |
| Version Awareness | 1 | 0 | 0 | 1 | 0 |
| **TOTAL** | **342** | **190** | **138** | **14** | **102** |


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
- Liu et al. (2023) - Lost in the middle (TACL)
- Wei et al. (2022) - Chain-of-Thought
- Zhao et al. (2021) - Few-shot calibration

### Production Code
- agentsys/plugins/enhance/* (70 patterns, tested on 1000+ files)

### Community
- 15+ platforms researched
- GitHub repos and documentation
- Community conventions and patterns

---

**Total Coverage**: 342 validation rules across 36 categories

**Knowledge Base**: 11,036 lines, 320KB, 75+ sources
**Certainty**: 190 HIGH, 138 MEDIUM, 14 LOW
**Auto-Fixable**: 102 rules (34%)
