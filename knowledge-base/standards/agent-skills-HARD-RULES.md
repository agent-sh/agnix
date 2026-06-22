# Agent Skills Standard - Hard Rules (Non-Negotiable)

> Definitive requirements extracted from official specifications. These are MUST/MUST NOT statements that will break compatibility if violated.

**Last Updated**: 2026-01-31
**Standard Version**: Agent Skills Specification (agentskills.io)
**Sources**: 10+ authoritative sources including official specs, SDK docs, API docs, and reference implementations

---

## Table of Contents

1. [Directory Structure Requirements](#directory-structure-requirements)
2. [SKILL.md File Requirements](#skillmd-file-requirements)
3. [YAML Frontmatter Requirements](#yaml-frontmatter-requirements)
4. [Field Specifications](#field-specifications)
5. [File Path Requirements](#file-path-requirements)
6. [Size and Character Limits](#size-and-character-limits)
7. [API Integration Requirements](#api-integration-requirements)
8. [Anti-Patterns (Will Break)](#anti-patterns-will-break)

---

## Directory Structure Requirements

### Required Structure
**Source**: https://agentskills.io/specification

A skill MUST be a directory containing at minimum a `SKILL.md` file:

```
skill-name/
└── SKILL.md          # Required
```

**HARD RULE**: The skill directory MUST contain a file named exactly `SKILL.md` (uppercase, with .md extension).

**Source**: https://agentskills.io/specification

---

## SKILL.md File Requirements

### File Format
**Source**: https://agentskills.io/specification

The `SKILL.md` file MUST contain:
1. YAML frontmatter at the beginning
2. Three-dash delimiters (`---`) before and after the frontmatter
3. Markdown content after the frontmatter

**HARD RULE**: YAML frontmatter is REQUIRED. The file MUST start with `---` followed by valid YAML, then another `---`, then Markdown content.

```markdown
---
name: skill-name
description: A description of what this skill does.
---

# Skill instructions here
```

**Source**: https://agentskills.io/specification

---

## YAML Frontmatter Requirements

### Required Fields

| Field | Required | Source |
|-------|----------|--------|
| `name` | **YES** | https://agentskills.io/specification |
| `description` | **YES** | https://agentskills.io/specification |
| `license` | NO | https://agentskills.io/specification |
| `compatibility` | NO | https://agentskills.io/specification |
| `metadata` | NO | https://agentskills.io/specification |
| `allowed-tools` | NO | https://agentskills.io/specification |

**HARD RULE**: The `name` and `description` fields are REQUIRED. A skill without these fields is invalid.

**Source**: https://agentskills.io/specification

---

## Field Specifications

### `name` Field Requirements

**Source**: https://agentskills.io/specification

The `name` field MUST satisfy ALL of the following constraints. For plugin-provided skills named as `<plugin>:<skill-name>`, apply these constraints to each segment and match the parent directory against only `<skill-name>`.

#### Character Length
- **MUST** be 1-64 characters
- Minimum: 1 character
- Maximum: 64 characters

#### Character Set
- **MUST** only contain Unicode lowercase alphanumeric characters and hyphens
- Valid characters: `a-z` (lowercase letters), `0-9` (numbers), `-` (hyphen)
- **MUST NOT** contain:
  - Uppercase letters (A-Z)
  - Spaces
  - Underscores (_)
  - Special characters except hyphen

#### Naming Rules
- **MUST NOT** start with a hyphen (`-`)
- **MUST NOT** end with a hyphen (`-`)
- **MUST NOT** contain consecutive hyphens (`--`)

#### Reserved Words
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

- **MUST NOT** contain reserved words: `"anthropic"`, `"claude"`
- **MUST NOT** contain XML tags

#### Directory Matching
**Source**: https://agentskills.io/specification

- **MUST** match the parent directory name

**Valid examples**:
```yaml
name: pdf-processing
name: data-analysis
name: code-review
```

**Invalid examples** (WILL BREAK):
```yaml
name: PDF-Processing  # INVALID: uppercase not allowed
name: -pdf            # INVALID: cannot start with hyphen
name: pdf--processing # INVALID: consecutive hyphens not allowed
name: pdf_processing  # INVALID: underscores not allowed
name: pdf processing  # INVALID: spaces not allowed
name: anthropic-tool  # INVALID: reserved word
```

---

### `description` Field Requirements

**Source**: https://agentskills.io/specification

The `description` field MUST satisfy ALL of the following constraints:

#### Character Length
- **MUST** be 1-1024 characters
- Minimum: 1 character (non-empty)
- Maximum: 1024 characters

#### Content Rules
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

- **MUST NOT** contain XML tags
- **MUST** be written in third person (not first or second person)

**Valid example**:
```yaml
description: Extracts text and tables from PDF files, fills forms, and merges multiple PDFs. Use when working with PDF documents or when the user mentions PDFs, forms, or document extraction.
```

**Invalid examples** (WILL BREAK):
```yaml
description: ""                              # INVALID: empty string
description: I can help you process PDFs     # INVALID: first person
description: You can use this to process PDFs # INVALID: second person
description: <tool>Process PDFs</tool>       # INVALID: contains XML tags
```

**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

---

### `license` Field Requirements

**Source**: https://agentskills.io/specification

If provided:
- MUST be a string
- No character limit specified in spec
- Should be a license name or reference to a bundled license file

---

### `compatibility` Field Requirements

**Source**: https://agentskills.io/specification

If provided:
- **MUST** be 1-500 characters
- Minimum: 1 character
- Maximum: 500 characters

---

### `metadata` Field Requirements

**Source**: https://agentskills.io/specification

If provided:
- **MUST** be a map from string keys to string values
- Keys: MUST be strings
- Values: MUST be strings

**Valid example**:
```yaml
metadata:
  author: example-org
  version: "1.0"
```

---

### `allowed-tools` Field Requirements

**Source**: https://agentskills.io/specification

If provided:
- **MUST** be a space-delimited list
- Experimental feature - support may vary

**IMPORTANT CAVEAT**:
**Source**: https://platform.claude.com/docs/en/agent-sdk/skills

The `allowed-tools` frontmatter field is ONLY supported when using Claude Code CLI directly. It does NOT apply when using Skills through the SDK.

---

## File Path Requirements

### Path Separator
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

**HARD RULE**: MUST use forward slashes (`/`) for file paths, even on Windows.

- **Valid**: `scripts/helper.py`, `reference/guide.md`
- **Invalid**: `scripts\helper.py`, `reference\guide.md`

**Anti-Pattern**: Windows-style paths (`\`) will cause errors on Unix systems.

### Reference Depth
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

**HARD RULE**: File references MUST be one level deep from SKILL.md.

**Reason**: Claude may partially read files when they're referenced from other referenced files, resulting in incomplete information.

**Valid**:
```markdown
# SKILL.md
See [advanced.md](advanced.md)
See [reference.md](reference.md)
```

**Invalid** (WILL BREAK):
```markdown
# SKILL.md → advanced.md → details.md
```

---

## Size and Character Limits

### Upload Size Limits
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

- **Total upload size**: MUST be under 8MB (all files combined)

### Line Count Recommendations
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

While not a hard breaking constraint, the specification strongly recommends:
- **SKILL.md body**: Keep under 500 lines
- **Token target**: < 5000 tokens recommended

**Note**: This is listed as a recommendation in the spec, but exceeding it may impact performance.

---

## API Integration Requirements

### Beta Headers
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

When using Skills with the Claude API, the following beta headers are REQUIRED:

```
anthropic-beta: code-execution-2025-08-25,skills-2025-10-02
```

Additional header for Files API operations:
```
anthropic-beta: files-api-2025-04-14
```

**HARD RULE**: Skills integration REQUIRES the `code-execution-2025-08-25` beta header. Skills will NOT work without code execution enabled.

### Container Parameter Structure
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

The `container` parameter MUST have the following structure:

```json
{
  "container": {
    "skills": [
      {
        "type": "anthropic" | "custom",
        "skill_id": "string",
        "version": "string" | "latest"
      }
    ]
  }
}
```

**Required fields in each skill object**:
- `type`: MUST be either `"anthropic"` or `"custom"`
- `skill_id`: MUST be a valid skill ID string

**Optional fields**:
- `version`: Can be specific version or `"latest"`

### Maximum Skills Per Request
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

**HARD RULE**: Maximum 8 skills per request.

---

## Anti-Patterns (Will Break)

### 1. Invalid Name Formats
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Uppercase letters in name
```yaml
name: PDF-Processing  # WILL FAIL VALIDATION
```

❌ **BREAKS**: Starting or ending with hyphen
```yaml
name: -pdf-tool  # WILL FAIL VALIDATION
name: pdf-tool-  # WILL FAIL VALIDATION
```

❌ **BREAKS**: Consecutive hyphens
```yaml
name: pdf--processing  # WILL FAIL VALIDATION
```

❌ **BREAKS**: Reserved words
```yaml
name: anthropic-helper  # WILL FAIL VALIDATION
name: claude-tool       # WILL FAIL VALIDATION
```

---

### 2. Missing Required Fields
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Missing `name` or `description`
```yaml
---
description: Does something
# Missing 'name' field - WILL FAIL
---
```

```yaml
---
name: my-skill
# Missing 'description' field - WILL FAIL
---
```

---

### 3. Empty Description
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Empty description string
```yaml
description: ""  # WILL FAIL VALIDATION
```

---

### 4. Invalid Character Length
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Name exceeds 64 characters
```yaml
name: this-is-a-very-long-skill-name-that-exceeds-the-maximum-allowed-length-of-sixty-four-characters  # WILL FAIL
```

❌ **BREAKS**: Description exceeds 1024 characters
```yaml
description: "..." # String longer than 1024 characters - WILL FAIL
```

---

### 5. XML Tags in Frontmatter
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

❌ **BREAKS**: XML tags in name or description
```yaml
name: <skill>pdf</skill>  # WILL FAIL VALIDATION
description: <tool>Process PDFs</tool>  # WILL FAIL VALIDATION
```

---

### 6. Wrong Point of View in Description
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

❌ **BREAKS DISCOVERY**: First or second person in description
```yaml
description: I can help you process PDFs        # BREAKS DISCOVERY
description: You can use this to process PDFs   # BREAKS DISCOVERY
```

✅ **CORRECT**: Third person
```yaml
description: Processes Excel files and generates reports  # CORRECT
```

---

### 7. Deeply Nested File References
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

❌ **BREAKS**: References deeper than one level
```
SKILL.md → advanced.md → details.md → specifics.md
```

**Why it breaks**: Claude may partially read files in nested chains, resulting in incomplete information.

---

### 8. Windows-Style Path Separators
**Source**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices

❌ **BREAKS ON UNIX**: Backslashes in paths
```markdown
See [reference](reference\guide.md)  # BREAKS ON UNIX SYSTEMS
```

✅ **CORRECT**: Forward slashes
```markdown
See [reference](reference/guide.md)  # WORKS EVERYWHERE
```

---

### 9. Missing SKILL.md File
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Skill directory without SKILL.md
```
my-skill/
├── README.md
└── script.py
# Missing SKILL.md - WILL FAIL
```

---

### 10. Missing YAML Frontmatter Delimiters
**Source**: https://agentskills.io/specification

❌ **BREAKS**: Missing or incorrect frontmatter delimiters
```markdown
name: my-skill
description: Does something
---

# Instructions
```

```markdown
---
name: my-skill
description: Does something

# Instructions (missing closing ---)
```

✅ **CORRECT**: Proper delimiters
```markdown
---
name: my-skill
description: Does something
---

# Instructions
```

---

### 11. Upload Size Exceeds 8MB
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

❌ **BREAKS**: Total skill files exceed 8MB
```
my-skill/
├── SKILL.md
├── large-data.csv  (5 MB)
└── model.pkl       (4 MB)
# Total: 9 MB - WILL FAIL UPLOAD
```

---

### 12. More Than 8 Skills in Container
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

❌ **BREAKS**: Requesting more than 8 skills
```json
{
  "container": {
    "skills": [
      {"type": "anthropic", "skill_id": "skill1"},
      {"type": "anthropic", "skill_id": "skill2"},
      {"type": "anthropic", "skill_id": "skill3"},
      {"type": "anthropic", "skill_id": "skill4"},
      {"type": "anthropic", "skill_id": "skill5"},
      {"type": "anthropic", "skill_id": "skill6"},
      {"type": "anthropic", "skill_id": "skill7"},
      {"type": "anthropic", "skill_id": "skill8"},
      {"type": "anthropic", "skill_id": "skill9"}
      // 9 skills - WILL FAIL
    ]
  }
}
```

---

### 13. Skills Without Code Execution
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

❌ **BREAKS**: Using skills without code execution tool
```json
{
  "model": "claude-sonnet-4-5-20250929",
  "container": {
    "skills": [{"type": "anthropic", "skill_id": "xlsx"}]
  },
  "messages": [{"role": "user", "content": "Process spreadsheet"}]
  // Missing tools array with code_execution - WILL FAIL
}
```

✅ **CORRECT**: Code execution tool must be enabled
```json
{
  "model": "claude-sonnet-4-5-20250929",
  "container": {
    "skills": [{"type": "anthropic", "skill_id": "xlsx"}]
  },
  "messages": [{"role": "user", "content": "Process spreadsheet"}],
  "tools": [{"type": "code_execution_20250825", "name": "code_execution"}]
}
```

---

## Validation

### Official Validation Tool
**Source**: https://agentskills.io/specification

Use the official `skills-ref` library to validate:

```bash
skills-ref validate ./my-skill
```

This checks:
- YAML frontmatter validity
- All naming conventions
- Required fields presence
- Character constraints

**Repository**: https://github.com/agentskills/agentskills/tree/main/skills-ref

---

## SDK Integration Requirements

### settingSources/setting_sources Configuration
**Source**: https://platform.claude.com/docs/en/agent-sdk/skills

**HARD RULE**: Skills are NOT loaded by default in the SDK. You MUST explicitly configure `settingSources` (TypeScript) or `setting_sources` (Python) to load Skills from the filesystem.

❌ **BREAKS**: Skills won't be loaded
```python
options = ClaudeAgentOptions(
    allowed_tools=["Skill"]  # Missing setting_sources - SKILLS WON'T LOAD
)
```

✅ **CORRECT**: Explicit configuration required
```python
options = ClaudeAgentOptions(
    setting_sources=["user", "project"],  # REQUIRED
    allowed_tools=["Skill"]
)
```

### Filesystem Artifact Requirement
**Source**: https://platform.claude.com/docs/en/agent-sdk/skills

**HARD RULE**: Skills MUST be created as filesystem artifacts. The SDK does NOT provide a programmatic API for registering Skills.

Unlike subagents (which can be defined programmatically), Skills must exist as SKILL.md files in configured directories.

---

## Environment Constraints

### Code Execution Environment
**Source**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide

Skills run in the code execution container with these HARD limitations:

1. **No network access** - Cannot make external API calls
2. **No runtime package installation** - Only pre-installed packages available
3. **Isolated environment** - Each request gets a fresh container

**Anti-Pattern**: Assuming network access or package installation will BREAK execution.

---

## Summary of Breaking Changes

| Violation | Impact | Source |
|-----------|--------|--------|
| Missing `name` or `description` | Validation failure | agentskills.io/specification |
| Invalid `name` format | Validation failure | agentskills.io/specification |
| Name/description exceeds limits | Validation failure | agentskills.io/specification |
| Reserved words in name | Validation failure | best-practices |
| XML tags in frontmatter | Validation failure | best-practices |
| Empty description | Validation failure | agentskills.io/specification |
| Missing SKILL.md | Discovery failure | agentskills.io/specification |
| Missing frontmatter delimiters | Parse failure | agentskills.io/specification |
| Windows paths on Unix | Execution failure | best-practices |
| Upload exceeds 8MB | Upload failure | skills-guide |
| More than 8 skills | Request failure | skills-guide |
| Missing code execution | Integration failure | skills-guide |
| Missing setting_sources in SDK | Skills not loaded | agent-sdk/skills |
| Deep file references | Incomplete loading | best-practices |

---

## Version History

- **2026-01-31**: Initial comprehensive extraction from 10+ sources
  - Official specification (agentskills.io)
  - Claude API documentation
  - Agent SDK documentation
  - Best practices guide
  - GitHub issues and discussions
  - Community implementations

---

## References

### Primary Sources
1. **Specification**: https://agentskills.io/specification
2. **What Are Skills**: https://agentskills.io/what-are-skills
3. **Integration Guide**: https://agentskills.io/integrate-skills
4. **Best Practices**: https://platform.claude.com/docs/en/agents-and-tools/agent-skills/best-practices
5. **API Skills Guide**: https://platform.claude.com/docs/en/docs/build-with-claude/skills-guide
6. **Agent SDK Skills**: https://platform.claude.com/docs/en/agent-sdk/skills
7. **GitHub Repository**: https://github.com/agentskills/agentskills
8. **Examples Repository**: https://github.com/anthropics/skills
9. **GitHub Issues**: https://github.com/agentskills/agentskills/issues
10. **Community Implementations**: Various GitHub repositories implementing the standard

### Additional Context
- Code execution tool documentation
- Files API documentation
- Prompt caching documentation
- MCP (Model Context Protocol) integration patterns

---

**Note**: This document contains ONLY hard rules that will break compatibility. For recommendations and best practices, see `agent-skills-OPINIONS.md`.
