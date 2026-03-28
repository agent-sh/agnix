# Rules Reference

This section contains all `385` validation rules generated from `knowledge-base/rules.json`.
`124` rules have automatic fixes.

| Rule | Name | Severity | Category | Auto-Fix |
|------|------|----------|----------|----------|
| [AGM-001](./generated/agm-001.md) | Valid Markdown Structure | HIGH | AGENTS.md | Yes (safe) |
| [AGM-002](./generated/agm-002.md) | Missing Section Headers | MEDIUM | AGENTS.md | No |
| [AGM-003](./generated/agm-003.md) | Character Limit (Windsurf) | MEDIUM | AGENTS.md | No |
| [AGM-004](./generated/agm-004.md) | Missing Project Context | MEDIUM | AGENTS.md | No |
| [AGM-005](./generated/agm-005.md) | Platform-Specific Features Without Guard | MEDIUM | AGENTS.md | No |
| [AGM-006](./generated/agm-006.md) | Nested AGENTS.md Hierarchy | MEDIUM | AGENTS.md | No |
| [AMP-001](./generated/amp-001.md) | Invalid Amp Check Frontmatter | HIGH | Amp Checks | Yes (safe) |
| [AMP-002](./generated/amp-002.md) | Invalid Amp severity-default | MEDIUM | Amp Checks | Yes (safe) |
| [AMP-003](./generated/amp-003.md) | Invalid AGENTS.md globs Frontmatter for Amp | MEDIUM | Amp Checks | No |
| [AMP-004](./generated/amp-004.md) | Invalid Amp Settings Configuration | HIGH | Amp Checks | Yes (safe) |
| [AMP-SK-001](./generated/amp-sk-001.md) | Amp Skill Uses Unsupported Field | MEDIUM | Amp Skills | Yes (safe/unsafe) |
| [AS-001](./generated/as-001.md) | Missing Frontmatter | HIGH | Agent Skills | Yes (safe) |
| [AS-002](./generated/as-002.md) | Missing Required Field: name | HIGH | Agent Skills | Yes (safe) |
| [AS-003](./generated/as-003.md) | Missing Required Field: description | HIGH | Agent Skills | Yes (safe) |
| [AS-004](./generated/as-004.md) | Invalid Name Format | HIGH | Agent Skills | Yes (safe/unsafe) |
| [AS-005](./generated/as-005.md) | Name Starts/Ends with Hyphen | HIGH | Agent Skills | Yes (safe) |
| [AS-006](./generated/as-006.md) | Consecutive Hyphens in Name | HIGH | Agent Skills | Yes (safe) |
| [AS-007](./generated/as-007.md) | Reserved Name | HIGH | Agent Skills | No |
| [AS-008](./generated/as-008.md) | Description Too Short | HIGH | Agent Skills | No |
| [AS-009](./generated/as-009.md) | Description Contains XML | HIGH | Agent Skills | Yes (safe) |
| [AS-010](./generated/as-010.md) | Missing Trigger Phrase | MEDIUM | Agent Skills | Yes (unsafe) |
| [AS-011](./generated/as-011.md) | Compatibility Too Long | HIGH | Agent Skills | No |
| [AS-012](./generated/as-012.md) | Content Exceeds 500 Lines | MEDIUM | Agent Skills | No |
| [AS-013](./generated/as-013.md) | File Reference Too Deep | HIGH | Agent Skills | No |
| [AS-014](./generated/as-014.md) | Windows Path Separator | HIGH | Agent Skills | Yes (safe) |
| [AS-015](./generated/as-015.md) | Upload Size Exceeds 8MB | HIGH | Agent Skills | No |
| [AS-016](./generated/as-016.md) | Skill Parse Error | HIGH | Agent Skills | No |
| [AS-017](./generated/as-017.md) | Name Must Match Parent Directory | HIGH | Agent Skills | No |
| [AS-018](./generated/as-018.md) | Description Uses First or Second Person | MEDIUM | Agent Skills | No |
| [AS-019](./generated/as-019.md) | Vague Skill Name | MEDIUM | Agent Skills | No |
| [CC-AG-001](./generated/cc-ag-001.md) | Missing Name Field | HIGH | Claude Agents | Yes (safe) |
| [CC-AG-002](./generated/cc-ag-002.md) | Missing Description Field | HIGH | Claude Agents | Yes (safe) |
| [CC-AG-003](./generated/cc-ag-003.md) | Invalid Model Value | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-004](./generated/cc-ag-004.md) | Invalid Permission Mode | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-005](./generated/cc-ag-005.md) | Referenced Skill Not Found | HIGH | Claude Agents | No |
| [CC-AG-006](./generated/cc-ag-006.md) | Tool/Disallowed Conflict | HIGH | Claude Agents | No |
| [CC-AG-007](./generated/cc-ag-007.md) | Agent Parse Error | HIGH | Claude Agents | No |
| [CC-AG-008](./generated/cc-ag-008.md) | Invalid Memory Scope | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-009](./generated/cc-ag-009.md) | Invalid Tool Name in Tools List | HIGH | Claude Agents | No |
| [CC-AG-010](./generated/cc-ag-010.md) | Invalid Tool Name in DisallowedTools | HIGH | Claude Agents | No |
| [CC-AG-011](./generated/cc-ag-011.md) | Invalid Hooks in Agent Frontmatter | HIGH | Claude Agents | No |
| [CC-AG-012](./generated/cc-ag-012.md) | Bypass Permissions Warning | HIGH | Claude Agents | Yes (unsafe) |
| [CC-AG-013](./generated/cc-ag-013.md) | Invalid Skill Name Format | MEDIUM | Claude Agents | Yes (unsafe) |
| [CC-AG-014](./generated/cc-ag-014.md) | Invalid Effort Value | MEDIUM | Claude Agents | Yes (unsafe) |
| [CC-AG-015](./generated/cc-ag-015.md) | Invalid Isolation Value | MEDIUM | Claude Agents | Yes (unsafe) |
| [CC-AG-017](./generated/cc-ag-017.md) | Invalid MaxTurns Value | MEDIUM | Claude Agents | No |
| [CC-AG-019](./generated/cc-ag-019.md) | Unknown Agent Frontmatter Field | LOW | Claude Agents | Yes (unsafe) |
| [CC-HK-001](./generated/cc-hk-001.md) | Invalid Hook Event | HIGH | Claude Hooks | Yes (safe/unsafe) |
| [CC-HK-002](./generated/cc-hk-002.md) | Prompt Hook on Wrong Event | HIGH | Claude Hooks | No |
| [CC-HK-003](./generated/cc-hk-003.md) | Matcher Hint for Tool Events | LOW | Claude Hooks | No |
| [CC-HK-004](./generated/cc-hk-004.md) | Matcher on Non-Tool Event | HIGH | Claude Hooks | Yes (safe) |
| [CC-HK-005](./generated/cc-hk-005.md) | Missing Type Field | HIGH | Claude Hooks | Yes (safe) |
| [CC-HK-006](./generated/cc-hk-006.md) | Missing Command Field | HIGH | Claude Hooks | No |
| [CC-HK-007](./generated/cc-hk-007.md) | Missing Prompt Field | HIGH | Claude Hooks | No |
| [CC-HK-008](./generated/cc-hk-008.md) | Script File Not Found | HIGH | Claude Hooks | No |
| [CC-HK-009](./generated/cc-hk-009.md) | Dangerous Command Pattern | HIGH | Claude Hooks | No |
| [CC-HK-010](./generated/cc-hk-010.md) | Timeout Policy | MEDIUM | Claude Hooks | Yes (safe) |
| [CC-HK-011](./generated/cc-hk-011.md) | Invalid Timeout Value | HIGH | Claude Hooks | Yes (unsafe) |
| [CC-HK-012](./generated/cc-hk-012.md) | Hooks Parse Error | HIGH | Claude Hooks | No |
| [CC-HK-013](./generated/cc-hk-013.md) | Async on Non-Command Hook | HIGH | Claude Hooks | Yes (safe) |
| [CC-HK-014](./generated/cc-hk-014.md) | Once Outside Skill/Agent Frontmatter | MEDIUM | Claude Hooks | Yes (safe) |
| [CC-HK-015](./generated/cc-hk-015.md) | Model on Command Hook | MEDIUM | Claude Hooks | Yes (safe) |
| [CC-HK-016](./generated/cc-hk-016.md) | Validate Hook Type Agent | HIGH | Claude Hooks | Yes (unsafe) |
| [CC-HK-017](./generated/cc-hk-017.md) | Prompt/Agent Hook Missing $ARGUMENTS | MEDIUM | Claude Hooks | Yes (safe) |
| [CC-HK-018](./generated/cc-hk-018.md) | Matcher on UserPromptSubmit/Stop | LOW | Claude Hooks | Yes (safe) |
| [CC-HK-019](./generated/cc-hk-019.md) | Deprecated Setup Event | MEDIUM | Claude Hooks | Yes (unsafe) |
| [CC-HK-020](./generated/cc-hk-020.md) | HTTP Hook Missing URL | HIGH | Claude Hooks | No |
| [CC-HK-021](./generated/cc-hk-021.md) | Invalid If Field | MEDIUM | Claude Hooks | No |
| [CC-HK-022](./generated/cc-hk-022.md) | Invalid Shell Value | MEDIUM | Claude Hooks | Yes (unsafe) |
| [CC-HK-023](./generated/cc-hk-023.md) | Once Field Not Boolean | LOW | Claude Hooks | Yes (unsafe) |
| [CC-HK-024](./generated/cc-hk-024.md) | Headers Missing AllowedEnvVars | MEDIUM | Claude Hooks | Yes (safe) |
| [CC-HK-025](./generated/cc-hk-025.md) | Invalid Matcher Value | LOW | Claude Hooks | Yes (unsafe) |
| [CC-MEM-001](./generated/cc-mem-001.md) | Invalid Import Path | HIGH | Claude Memory | No |
| [CC-MEM-002](./generated/cc-mem-002.md) | Circular Import | HIGH | Claude Memory | No |
| [CC-MEM-003](./generated/cc-mem-003.md) | Import Depth Exceeds 5 | HIGH | Claude Memory | No |
| [CC-MEM-004](./generated/cc-mem-004.md) | Invalid Command Reference | MEDIUM | Claude Memory | No |
| [CC-MEM-005](./generated/cc-mem-005.md) | Generic Instruction | HIGH | Claude Memory | Yes (safe) |
| [CC-MEM-006](./generated/cc-mem-006.md) | Negative Without Positive | HIGH | Claude Memory | No |
| [CC-MEM-007](./generated/cc-mem-007.md) | Weak Constraint Language | HIGH | Claude Memory | Yes (safe/unsafe) |
| [CC-MEM-008](./generated/cc-mem-008.md) | Critical Content in Middle | HIGH | Claude Memory | No |
| [CC-MEM-009](./generated/cc-mem-009.md) | Token Count Exceeded | MEDIUM | Claude Memory | No |
| [CC-MEM-010](./generated/cc-mem-010.md) | README Duplication | MEDIUM | Claude Memory | No |
| [CC-MEM-011](./generated/cc-mem-011.md) | Invalid Paths Glob in Rules | HIGH | Claude Memory | No |
| [CC-MEM-012](./generated/cc-mem-012.md) | Rules File Unknown Frontmatter Key | MEDIUM | Claude Memory | Yes (unsafe) |
| [CC-MEM-014](./generated/cc-mem-014.md) | CLAUDE.md Exceeds Line Limit | MEDIUM | Claude Memory | No |
| [CC-PL-001](./generated/cc-pl-001.md) | Plugin Manifest Not in .claude-plugin/ | HIGH | Claude Plugins | No |
| [CC-PL-002](./generated/cc-pl-002.md) | Components in .claude-plugin/ | HIGH | Claude Plugins | No |
| [CC-PL-003](./generated/cc-pl-003.md) | Invalid Semver | HIGH | Claude Plugins | Yes (safe) |
| [CC-PL-004](./generated/cc-pl-004.md) | Missing Required/Recommended Plugin Field | HIGH | Claude Plugins | No |
| [CC-PL-005](./generated/cc-pl-005.md) | Empty Plugin Name | HIGH | Claude Plugins | Yes (unsafe) |
| [CC-PL-006](./generated/cc-pl-006.md) | Plugin Parse Error | HIGH | Claude Plugins | No |
| [CC-PL-007](./generated/cc-pl-007.md) | Invalid Component Path | HIGH | Claude Plugins | Yes (safe) |
| [CC-PL-008](./generated/cc-pl-008.md) | Component Inside .claude-plugin | HIGH | Claude Plugins | No |
| [CC-PL-009](./generated/cc-pl-009.md) | Invalid Author Object | MEDIUM | Claude Plugins | No |
| [CC-PL-010](./generated/cc-pl-010.md) | Invalid Homepage URL | MEDIUM | Claude Plugins | No |
| [CC-PL-011](./generated/cc-pl-011.md) | LSP Server Missing Required Fields | HIGH | Claude Plugins | No |
| [CC-PL-012](./generated/cc-pl-012.md) | Invalid UserConfig Key | MEDIUM | Claude Plugins | No |
| [CC-PL-013](./generated/cc-pl-013.md) | Channel Missing Server Reference | MEDIUM | Claude Plugins | No |
| [CC-PL-014](./generated/cc-pl-014.md) | Plugin Agent Unsupported Field | MEDIUM | Claude Plugins | Yes (safe) |
| [CC-SK-001](./generated/cc-sk-001.md) | Invalid Model Value | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-002](./generated/cc-sk-002.md) | Invalid Context Value | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-003](./generated/cc-sk-003.md) | Context Without Agent | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-004](./generated/cc-sk-004.md) | Agent Without Context | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-005](./generated/cc-sk-005.md) | Invalid Agent Type | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-006](./generated/cc-sk-006.md) | Dangerous Auto-Invocation | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-007](./generated/cc-sk-007.md) | Unrestricted Bash | MEDIUM | Claude Skills | Yes (unsafe) |
| [CC-SK-008](./generated/cc-sk-008.md) | Unknown Tool Name | HIGH | Claude Skills | No |
| [CC-SK-009](./generated/cc-sk-009.md) | Too Many Injections | MEDIUM | Claude Skills | No |
| [CC-SK-010](./generated/cc-sk-010.md) | Invalid Hooks in Skill Frontmatter | HIGH | Claude Skills | No |
| [CC-SK-011](./generated/cc-sk-011.md) | Unreachable Skill | HIGH | Claude Skills | Yes (unsafe) |
| [CC-SK-012](./generated/cc-sk-012.md) | Argument Hint Without $ARGUMENTS | MEDIUM | Claude Skills | Yes (unsafe) |
| [CC-SK-013](./generated/cc-sk-013.md) | Fork Context Without Actionable Instructions | MEDIUM | Claude Skills | No |
| [CC-SK-014](./generated/cc-sk-014.md) | Invalid disable-model-invocation Type | HIGH | Claude Skills | Yes (safe) |
| [CC-SK-015](./generated/cc-sk-015.md) | Invalid user-invocable Type | HIGH | Claude Skills | Yes (safe) |
| [CC-SK-016](./generated/cc-sk-016.md) | Indexed $ARGUMENTS Without argument-hint | MEDIUM | Claude Skills | No |
| [CC-SK-017](./generated/cc-sk-017.md) | Unknown Frontmatter Field | MEDIUM | Claude Skills | No |
| [CC-SK-018](./generated/cc-sk-018.md) | Invalid Effort Value | MEDIUM | Claude Skills | Yes (unsafe) |
| [CC-SK-019](./generated/cc-sk-019.md) | Invalid Paths Format | LOW | Claude Skills | No |
| [CC-SK-020](./generated/cc-sk-020.md) | Invalid Shell Value | MEDIUM | Claude Skills | Yes (unsafe) |
| [CDX-000](./generated/cdx-000.md) | TOML Parse Error | HIGH | Codex CLI | No |
| [CDX-001](./generated/cdx-001.md) | Invalid Approval Mode | HIGH | Codex CLI | Yes (unsafe) |
| [CDX-002](./generated/cdx-002.md) | Invalid Full Auto Error Mode | HIGH | Codex CLI | Yes (unsafe) |
| [CDX-003](./generated/cdx-003.md) | AGENTS.override.md in Version Control | MEDIUM | Codex CLI | No |
| [CDX-004](./generated/cdx-004.md) | Unknown Config Key | MEDIUM | Codex CLI | Yes (safe) |
| [CDX-005](./generated/cdx-005.md) | project_doc_max_bytes Exceeds Limit | HIGH | Codex CLI | No |
| [CDX-006](./generated/cdx-006.md) | Invalid project_doc_fallback_filenames | HIGH | Codex CLI | No |
| [CDX-CFG-001](./generated/cdx-cfg-001.md) | Invalid approval_policy Value | HIGH | Codex CLI | No |
| [CDX-CFG-002](./generated/cdx-cfg-002.md) | Invalid sandbox_mode Value | HIGH | Codex CLI | No |
| [CDX-CFG-003](./generated/cdx-cfg-003.md) | Invalid model_reasoning_effort Value | HIGH | Codex CLI | No |
| [CDX-CFG-004](./generated/cdx-cfg-004.md) | Invalid model_verbosity Value | HIGH | Codex CLI | No |
| [CDX-CFG-005](./generated/cdx-cfg-005.md) | Invalid personality Value | HIGH | Codex CLI | No |
| [CDX-CFG-006](./generated/cdx-cfg-006.md) | Unknown Codex Config Field | MEDIUM | Codex CLI | No |
| [CDX-CFG-007](./generated/cdx-cfg-007.md) | Danger Full Access Without Acknowledgment | HIGH | Codex CLI | No |
| [CDX-CFG-008](./generated/cdx-cfg-008.md) | Invalid shell_environment_policy.inherit Value | HIGH | Codex CLI | No |
| [CDX-CFG-009](./generated/cdx-cfg-009.md) | Invalid MCP Server Structure in Codex Config | HIGH | Codex CLI | No |
| [CDX-CFG-010](./generated/cdx-cfg-010.md) | Hardcoded Secret in Codex Config | HIGH | Codex CLI | No |
| [CDX-CFG-011](./generated/cdx-cfg-011.md) | Invalid Feature Flag Name | MEDIUM | Codex CLI | No |
| [CDX-CFG-012](./generated/cdx-cfg-012.md) | Invalid cli_auth_credentials_store Value | HIGH | Codex CLI | No |
| [CDX-CFG-013](./generated/cdx-cfg-013.md) | Invalid sandbox_workspace_write Mode | HIGH | Codex CLI | No |
| [CDX-CFG-014](./generated/cdx-cfg-014.md) | Invalid model Value | MEDIUM | Codex CLI | No |
| [CDX-CFG-015](./generated/cdx-cfg-015.md) | Invalid model_provider Value | HIGH | Codex CLI | No |
| [CDX-CFG-016](./generated/cdx-cfg-016.md) | Invalid model_reasoning_summary Value | MEDIUM | Codex CLI | No |
| [CDX-CFG-017](./generated/cdx-cfg-017.md) | Invalid history Configuration | MEDIUM | Codex CLI | No |
| [CDX-CFG-018](./generated/cdx-cfg-018.md) | Invalid tui Configuration | MEDIUM | Codex CLI | No |
| [CDX-CFG-019](./generated/cdx-cfg-019.md) | Invalid file_opener Value | MEDIUM | Codex CLI | No |
| [CDX-CFG-020](./generated/cdx-cfg-020.md) | Invalid MCP OAuth Config | HIGH | Codex CLI | No |
| [CDX-CFG-021](./generated/cdx-cfg-021.md) | Invalid model_context_window Value | MEDIUM | Codex CLI | No |
| [CDX-CFG-022](./generated/cdx-cfg-022.md) | Invalid model_auto_compact_token_limit Value | MEDIUM | Codex CLI | No |
| [CDX-AG-001](./generated/cdx-ag-001.md) | Empty AGENTS.md for Codex | HIGH | Codex CLI | No |
| [CDX-AG-002](./generated/cdx-ag-002.md) | Secrets in AGENTS.md for Codex | HIGH | Codex CLI | No |
| [CDX-AG-003](./generated/cdx-ag-003.md) | Generic AGENTS.md Guidance for Codex | MEDIUM | Codex CLI | No |
| [CDX-AG-004](./generated/cdx-ag-004.md) | AGENTS.md Exceeds Size Limit | MEDIUM | Codex CLI | No |
| [CDX-AG-005](./generated/cdx-ag-005.md) | AGENTS.md References Missing File | MEDIUM | Codex CLI | No |
| [CDX-AG-006](./generated/cdx-ag-006.md) | AGENTS.md Missing Project Context | LOW | Codex CLI | No |
| [CDX-AG-007](./generated/cdx-ag-007.md) | AGENTS.md Contradicts config.toml | MEDIUM | Codex CLI | No |
| [CDX-APP-001](./generated/cdx-app-001.md) | Invalid default_tools_approval_mode Value | HIGH | Codex CLI | No |
| [CDX-APP-002](./generated/cdx-app-002.md) | Invalid skills Configuration | MEDIUM | Codex CLI | No |
| [CDX-APP-003](./generated/cdx-app-003.md) | Invalid profile Configuration | MEDIUM | Codex CLI | No |
| [CDX-CFG-023](./generated/cdx-cfg-023.md) | Invalid Approval Policy Sub-field | MEDIUM | Codex CLI | Yes (safe) |
| [CDX-CFG-024](./generated/cdx-cfg-024.md) | Invalid Approvals Reviewer Value | MEDIUM | Codex CLI | Yes (unsafe) |
| [CDX-CFG-025](./generated/cdx-cfg-025.md) | Invalid Service Tier Value | MEDIUM | Codex CLI | Yes (unsafe) |
| [CDX-CFG-026](./generated/cdx-cfg-026.md) | Invalid Network Permission Field | LOW | Codex CLI | Yes (safe) |
| [CDX-CFG-027](./generated/cdx-cfg-027.md) | Invalid Windows Sandbox Value | LOW | Codex CLI | Yes (unsafe) |
| [CL-SK-001](./generated/cl-sk-001.md) | Cline Skill Uses Unsupported Field | MEDIUM | Cline Skills | Yes (safe/unsafe) |
| [CL-SK-002](./generated/cl-sk-002.md) | Missing Skill Name | HIGH | Cline Skills | Yes (safe) |
| [CL-SK-003](./generated/cl-sk-003.md) | Missing Skill Description | HIGH | Cline Skills | No |
| [CLN-001](./generated/cln-001.md) | Empty Cline Rules File | HIGH | Cline | No |
| [CLN-002](./generated/cln-002.md) | Invalid Paths Glob in Cline Rules | HIGH | Cline | No |
| [CLN-003](./generated/cln-003.md) | Unknown Frontmatter Key in Cline Rules | MEDIUM | Cline | Yes (unsafe) |
| [CLN-004](./generated/cln-004.md) | Scalar Paths in Cline Rules | HIGH | Cline | Yes (safe) |
| [CLN-005](./generated/cln-005.md) | Empty Workflow File | HIGH | Cline | No |
| [CLN-006](./generated/cln-006.md) | Workflow With Frontmatter | MEDIUM | Cline | Yes (unsafe) |
| [CLN-009](./generated/cln-009.md) | Unknown Hook Event Name | MEDIUM | Cline | No |
| [COP-001](./generated/cop-001.md) | Empty Copilot Instruction File | HIGH | GitHub Copilot | No |
| [COP-002](./generated/cop-002.md) | Invalid Frontmatter in Scoped Instructions | HIGH | GitHub Copilot | Yes (unsafe) |
| [COP-003](./generated/cop-003.md) | Invalid Glob Pattern in applyTo | HIGH | GitHub Copilot | No |
| [COP-004](./generated/cop-004.md) | Unknown Frontmatter Keys | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-005](./generated/cop-005.md) | Invalid excludeAgent Value | HIGH | GitHub Copilot | Yes (unsafe) |
| [COP-006](./generated/cop-006.md) | File Length Limit | MEDIUM | GitHub Copilot | No |
| [COP-007](./generated/cop-007.md) | Custom Agent Missing Description | HIGH | GitHub Copilot | No |
| [COP-008](./generated/cop-008.md) | Custom Agent Unknown or Invalid Frontmatter Field | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-009](./generated/cop-009.md) | Custom Agent Invalid Target | HIGH | GitHub Copilot | Yes (unsafe) |
| [COP-010](./generated/cop-010.md) | Custom Agent infer Field Must Be Boolean | MEDIUM | GitHub Copilot | No |
| [COP-011](./generated/cop-011.md) | Custom Agent Prompt Body Exceeds Length Limit | HIGH | GitHub Copilot | No |
| [COP-012](./generated/cop-012.md) | Custom Agent Uses GitHub.com Unsupported Fields | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-013](./generated/cop-013.md) | Prompt File Empty Body | HIGH | GitHub Copilot | No |
| [COP-014](./generated/cop-014.md) | Prompt File Unknown Frontmatter Field | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-015](./generated/cop-015.md) | Prompt File Invalid Agent Mode | HIGH | GitHub Copilot | Yes (safe) |
| [COP-017](./generated/cop-017.md) | Copilot Hooks Schema Validation | HIGH | GitHub Copilot | No |
| [COP-018](./generated/cop-018.md) | Copilot Setup Steps Missing or Invalid copilot-setup-steps Job | HIGH | GitHub Copilot | No |
| [COP-019](./generated/cop-019.md) | Plugin Missing Required Fields | HIGH | GitHub Copilot | No |
| [COP-020](./generated/cop-020.md) | Plugin Invalid Field Types | MEDIUM | GitHub Copilot | No |
| [COP-022](./generated/cop-022.md) | CLI Skill Missing Frontmatter | HIGH | GitHub Copilot | No |
| [COP-023](./generated/cop-023.md) | CLI Skill Name Format | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-024](./generated/cop-024.md) | CLI Skill Unknown Field | MEDIUM | GitHub Copilot | Yes (safe) |
| [COP-025](./generated/cop-025.md) | Agent File Wrong Location | LOW | GitHub Copilot | No |
| [COP-026](./generated/cop-026.md) | Deprecated SSE Transport | LOW | GitHub Copilot | Yes (unsafe) |
| [COP-027](./generated/cop-027.md) | Deprecated Infer Field | LOW | GitHub Copilot | No |
| [CP-SK-001](./generated/cp-sk-001.md) | Copilot Skill Uses Unsupported Field | MEDIUM | Copilot Skills | Yes (safe/unsafe) |
| [CR-SK-001](./generated/cr-sk-001.md) | Cursor Skill Uses Unsupported Field | MEDIUM | Cursor Skills | Yes (safe/unsafe) |
| [CUR-001](./generated/cur-001.md) | Empty Cursor Rule File | HIGH | Cursor | No |
| [CUR-002](./generated/cur-002.md) | Missing Frontmatter in .mdc File | MEDIUM | Cursor | Yes (unsafe) |
| [CUR-003](./generated/cur-003.md) | Invalid YAML Frontmatter | HIGH | Cursor | No |
| [CUR-004](./generated/cur-004.md) | Invalid Glob Pattern in globs Field | HIGH | Cursor | No |
| [CUR-005](./generated/cur-005.md) | Unknown Frontmatter Keys | MEDIUM | Cursor | Yes (safe) |
| [CUR-006](./generated/cur-006.md) | Legacy .cursorrules File Detected | MEDIUM | Cursor | No |
| [CUR-007](./generated/cur-007.md) | alwaysApply with Redundant globs | MEDIUM | Cursor | Yes (safe) |
| [CUR-008](./generated/cur-008.md) | Invalid alwaysApply Type | HIGH | Cursor | Yes (safe) |
| [CUR-009](./generated/cur-009.md) | Missing Description for Agent-Requested Rule | MEDIUM | Cursor | No |
| [CUR-010](./generated/cur-010.md) | Invalid Cursor Hooks Schema | HIGH | Cursor | No |
| [CUR-011](./generated/cur-011.md) | Unknown Cursor Hook Event Name | MEDIUM | Cursor | Yes (safe) |
| [CUR-012](./generated/cur-012.md) | Hook Entry Missing Required Command Field | HIGH | Cursor | No |
| [CUR-013](./generated/cur-013.md) | Invalid Cursor Hook Type Value | HIGH | Cursor | Yes (safe) |
| [CUR-014](./generated/cur-014.md) | Invalid Cursor Subagent Frontmatter | HIGH | Cursor | No |
| [CUR-015](./generated/cur-015.md) | Empty Cursor Subagent Body | MEDIUM | Cursor | No |
| [CUR-016](./generated/cur-016.md) | Invalid Cursor Environment Schema | HIGH | Cursor | No |
| [CUR-017](./generated/cur-017.md) | Invalid Hook Entry Field Types | MEDIUM | Cursor | No |
| [CUR-018](./generated/cur-018.md) | Prompt Hook Missing Prompt Field | MEDIUM | Cursor | No |
| [CUR-019](./generated/cur-019.md) | Invalid Prompt Hook Model Type | LOW | Cursor | No |
| [CX-SK-001](./generated/cx-sk-001.md) | Codex Skill Uses Unsupported Field | MEDIUM | Codex Skills | Yes (safe/unsafe) |
| [GM-001](./generated/gm-001.md) | Invalid Markdown Structure in GEMINI.md | HIGH | Gemini CLI | Yes (safe) |
| [GM-002](./generated/gm-002.md) | Missing Section Headers in GEMINI.md | MEDIUM | Gemini CLI | No |
| [GM-003](./generated/gm-003.md) | Missing Project Context in GEMINI.md | MEDIUM | Gemini CLI | No |
| [GM-004](./generated/gm-004.md) | Invalid Hooks Configuration in Gemini Settings | MEDIUM | Gemini CLI | No |
| [GM-005](./generated/gm-005.md) | Invalid Extension Manifest | HIGH | Gemini CLI | No |
| [GM-006](./generated/gm-006.md) | Invalid .geminiignore File | LOW | Gemini CLI | No |
| [GM-007](./generated/gm-007.md) | @import File Not Found in GEMINI.md | MEDIUM | Gemini CLI | No |
| [GM-008](./generated/gm-008.md) | Invalid Context File Name Configuration | LOW | Gemini CLI | Yes (safe) |
| [GM-009](./generated/gm-009.md) | Settings.json Parse Error | HIGH | Gemini CLI | Yes (safe) |
| [KIRO-001](./generated/kiro-001.md) | Invalid Steering File Inclusion Mode | HIGH | Kiro Steering | Yes (safe) |
| [KIRO-002](./generated/kiro-002.md) | Missing Required Fields for Inclusion Mode | HIGH | Kiro Steering | No |
| [KIRO-003](./generated/kiro-003.md) | Invalid fileMatchPattern Glob | MEDIUM | Kiro Steering | No |
| [KIRO-004](./generated/kiro-004.md) | Empty Kiro Steering File | MEDIUM | Kiro Steering | No |
| [KIRO-005](./generated/kiro-005.md) | Empty Steering Body After Frontmatter | MEDIUM | Kiro Steering | No |
| [KIRO-006](./generated/kiro-006.md) | Secrets Detected in Steering File | HIGH | Kiro Steering | No |
| [KIRO-007](./generated/kiro-007.md) | fileMatchPattern Without fileMatch Inclusion | MEDIUM | Kiro Steering | No |
| [KIRO-008](./generated/kiro-008.md) | Unknown Kiro Steering Frontmatter Field | MEDIUM | Kiro Steering | No |
| [KIRO-009](./generated/kiro-009.md) | Broken Inline File Reference in Steering | MEDIUM | Kiro Steering | No |
| [KIRO-010](./generated/kiro-010.md) | Missing Inclusion Mode | MEDIUM | Kiro Steering | No |
| [KIRO-011](./generated/kiro-011.md) | Steering Doc Excessively Long | LOW | Kiro Steering | No |
| [KIRO-012](./generated/kiro-012.md) | Duplicate Steering Name | MEDIUM | Kiro Steering | No |
| [KIRO-013](./generated/kiro-013.md) | Conflicting Inclusion Modes | MEDIUM | Kiro Steering | No |
| [KIRO-014](./generated/kiro-014.md) | Markdown Structure Issues | LOW | Kiro Steering | No |
| [KR-SK-001](./generated/kr-sk-001.md) | Kiro Skill Uses Unsupported Field | MEDIUM | Kiro Skills | Yes (safe/unsafe) |
| [KR-AG-001](./generated/kr-ag-001.md) | Unknown Field in Kiro Agent JSON | MEDIUM | Kiro Agents | No |
| [KR-AG-002](./generated/kr-ag-002.md) | Invalid Kiro Agent Resource Protocol | HIGH | Kiro Agents | No |
| [KR-AG-003](./generated/kr-ag-003.md) | allowedTools Not Subset of tools | MEDIUM | Kiro Agents | No |
| [KR-AG-004](./generated/kr-ag-004.md) | Invalid Kiro Agent Model Value | MEDIUM | Kiro Agents | No |
| [KR-AG-005](./generated/kr-ag-005.md) | Kiro Agent Has No MCP Access | LOW | Kiro Agents | No |
| [KR-AG-006](./generated/kr-ag-006.md) | Kiro Agent References Unknown Subagent | MEDIUM | Kiro Agents | No |
| [KR-AG-007](./generated/kr-ag-007.md) | Kiro Agent Tool Scope Broader Than Referenced Subagent | MEDIUM | Kiro Agents | No |
| [KR-AG-008](./generated/kr-ag-008.md) | Agent Missing Name | HIGH | Kiro Agents | No |
| [KR-AG-009](./generated/kr-ag-009.md) | Agent Missing Prompt | HIGH | Kiro Agents | No |
| [KR-AG-010](./generated/kr-ag-010.md) | Duplicate Tool Entries | MEDIUM | Kiro Agents | No |
| [KR-AG-011](./generated/kr-ag-011.md) | Empty Tools Array | LOW | Kiro Agents | No |
| [KR-AG-012](./generated/kr-ag-012.md) | toolAliases References Unknown Tool | MEDIUM | Kiro Agents | No |
| [KR-AG-013](./generated/kr-ag-013.md) | Secrets in Agent Prompt | HIGH | Kiro Agents | No |
| [KR-HK-001](./generated/kr-hk-001.md) | Invalid Kiro IDE Hook Event Type | HIGH | Kiro Hooks | No |
| [KR-HK-002](./generated/kr-hk-002.md) | Kiro File Hook Missing Patterns | HIGH | Kiro Hooks | No |
| [KR-HK-003](./generated/kr-hk-003.md) | Kiro IDE Hook Missing Action | HIGH | Kiro Hooks | No |
| [KR-HK-004](./generated/kr-hk-004.md) | Kiro Tool Hook Missing toolTypes Filter | MEDIUM | Kiro Hooks | No |
| [KR-HK-005](./generated/kr-hk-005.md) | Invalid Kiro CLI Hook Event Key | HIGH | Kiro Hooks | No |
| [KR-HK-006](./generated/kr-hk-006.md) | Kiro CLI Hook Missing Command | HIGH | Kiro Hooks | No |
| [KR-HK-007](./generated/kr-hk-007.md) | Hook Timeout Out of Range | MEDIUM | Kiro Hooks | No |
| [KR-HK-008](./generated/kr-hk-008.md) | Duplicate Event Handlers | MEDIUM | Kiro Hooks | No |
| [KR-HK-009](./generated/kr-hk-009.md) | Command Uses Absolute Path | MEDIUM | Kiro Hooks | No |
| [KR-HK-010](./generated/kr-hk-010.md) | Secrets in Hook Command | HIGH | Kiro Hooks | No |
| [KR-MCP-001](./generated/kr-mcp-001.md) | Kiro MCP Server Missing command and url | HIGH | Kiro MCP | No |
| [KR-MCP-002](./generated/kr-mcp-002.md) | Hardcoded Secrets in Kiro MCP env | MEDIUM | Kiro MCP | No |
| [KR-MCP-003](./generated/kr-mcp-003.md) | Missing Required Args | MEDIUM | Kiro MCP | No |
| [KR-MCP-004](./generated/kr-mcp-004.md) | Invalid MCP URL | HIGH | Kiro MCP | No |
| [KR-MCP-005](./generated/kr-mcp-005.md) | Duplicate MCP Server Names | MEDIUM | Kiro MCP | No |
| [KR-PW-001](./generated/kr-pw-001.md) | Missing Required POWER.md Frontmatter Fields | HIGH | Kiro Powers | No |
| [KR-PW-002](./generated/kr-pw-002.md) | Empty POWER.md Keywords Array | MEDIUM | Kiro Powers | No |
| [KR-PW-003](./generated/kr-pw-003.md) | Empty POWER.md Body | MEDIUM | Kiro Powers | No |
| [KR-PW-004](./generated/kr-pw-004.md) | Invalid Adjacent Power mcp.json Structure | MEDIUM | Kiro Powers | No |
| [KR-PW-005](./generated/kr-pw-005.md) | Step Missing Description | HIGH | Kiro Powers | No |
| [KR-PW-006](./generated/kr-pw-006.md) | Duplicate Keywords | LOW | Kiro Powers | No |
| [KR-PW-007](./generated/kr-pw-007.md) | Name Invalid Characters | MEDIUM | Kiro Powers | No |
| [KR-PW-008](./generated/kr-pw-008.md) | Secrets in Power Body | HIGH | Kiro Powers | No |
| [MCP-001](./generated/mcp-001.md) | Invalid JSON-RPC Version | HIGH | MCP | Yes (safe) |
| [MCP-002](./generated/mcp-002.md) | Missing Required Tool Field | HIGH | MCP | No |
| [MCP-003](./generated/mcp-003.md) | Invalid JSON Schema | HIGH | MCP | No |
| [MCP-004](./generated/mcp-004.md) | Missing Tool Description | HIGH | MCP | No |
| [MCP-005](./generated/mcp-005.md) | Tool Without User Consent | HIGH | MCP | No |
| [MCP-006](./generated/mcp-006.md) | Untrusted Annotations | HIGH | MCP | No |
| [MCP-007](./generated/mcp-007.md) | MCP Parse Error | HIGH | MCP | No |
| [MCP-008](./generated/mcp-008.md) | Protocol Version Mismatch | MEDIUM | MCP | Yes (unsafe) |
| [MCP-009](./generated/mcp-009.md) | Missing command for stdio server | HIGH | MCP | No |
| [MCP-010](./generated/mcp-010.md) | Missing url for http/sse server | HIGH | MCP | No |
| [MCP-011](./generated/mcp-011.md) | Invalid MCP server type | HIGH | MCP | Yes (unsafe) |
| [MCP-012](./generated/mcp-012.md) | Deprecated SSE transport | HIGH | MCP | Yes (unsafe) |
| [MCP-013](./generated/mcp-013.md) | Invalid Tool Name Format | HIGH | MCP | Yes (safe) |
| [MCP-014](./generated/mcp-014.md) | Invalid outputSchema Definition | HIGH | MCP | No |
| [MCP-015](./generated/mcp-015.md) | Missing Resource Required Fields | HIGH | MCP | No |
| [MCP-016](./generated/mcp-016.md) | Missing Prompt Required Name | HIGH | MCP | No |
| [MCP-017](./generated/mcp-017.md) | Non-HTTPS Remote HTTP Server URL | HIGH | MCP | Yes (safe) |
| [MCP-018](./generated/mcp-018.md) | Potential Plaintext Secret in MCP Env | MEDIUM | MCP | No |
| [MCP-019](./generated/mcp-019.md) | Potentially Dangerous Stdio Command | MEDIUM | MCP | No |
| [MCP-020](./generated/mcp-020.md) | Unknown Capability Declaration Key | MEDIUM | MCP | No |
| [MCP-021](./generated/mcp-021.md) | Wildcard HTTP Interface Binding | MEDIUM | MCP | Yes (safe) |
| [MCP-022](./generated/mcp-022.md) | Invalid args Array Type | HIGH | MCP | No |
| [MCP-023](./generated/mcp-023.md) | Duplicate MCP Server Names | HIGH | MCP | No |
| [MCP-024](./generated/mcp-024.md) | Empty MCP Server Configuration | HIGH | MCP | No |
| [OC-001](./generated/oc-001.md) | Invalid Share Mode | HIGH | OpenCode | Yes (unsafe) |
| [OC-002](./generated/oc-002.md) | Invalid Instruction Path | HIGH | OpenCode | No |
| [OC-003](./generated/oc-003.md) | opencode.json Parse Error | HIGH | OpenCode | No |
| [OC-004](./generated/oc-004.md) | Unknown Config Key | MEDIUM | OpenCode | No |
| [OC-006](./generated/oc-006.md) | Remote URL in Instructions | LOW | OpenCode | No |
| [OC-007](./generated/oc-007.md) | Invalid Agent Definition | MEDIUM | OpenCode | No |
| [OC-008](./generated/oc-008.md) | Invalid Permission Config | HIGH | OpenCode | Yes (unsafe) |
| [OC-009](./generated/oc-009.md) | Invalid Variable Substitution | MEDIUM | OpenCode | No |
| [OC-SK-001](./generated/oc-sk-001.md) | OpenCode Skill Uses Unsupported Field | MEDIUM | OpenCode Skills | Yes (safe/unsafe) |
| [PE-001](./generated/pe-001.md) | Lost in the Middle | MEDIUM | Prompt Engineering | No |
| [PE-002](./generated/pe-002.md) | Chain-of-Thought on Simple Task | MEDIUM | Prompt Engineering | No |
| [PE-003](./generated/pe-003.md) | Weak Imperative Language | MEDIUM | Prompt Engineering | Yes (unsafe) |
| [PE-004](./generated/pe-004.md) | Ambiguous Instructions | MEDIUM | Prompt Engineering | No |
| [PE-005](./generated/pe-005.md) | Redundant Generic Instructions | MEDIUM | Prompt Engineering | Yes (safe) |
| [PE-006](./generated/pe-006.md) | Negative-Only Instructions | MEDIUM | Prompt Engineering | No |
| [RC-SK-001](./generated/rc-sk-001.md) | Roo Code Skill Uses Unsupported Field | MEDIUM | Roo Code Skills | Yes (safe/unsafe) |
| [REF-001](./generated/ref-001.md) | Import File Not Found | HIGH | References | No |
| [REF-002](./generated/ref-002.md) | Broken Markdown Link | HIGH | References | No |
| [REF-003](./generated/ref-003.md) | Duplicate Import | MEDIUM | References | Yes (safe) |
| [REF-004](./generated/ref-004.md) | Non-Markdown Import | MEDIUM | References | No |
| [ROO-001](./generated/roo-001.md) | Empty Roo Code Rule File | HIGH | Roo Code | No |
| [ROO-002](./generated/roo-002.md) | Invalid .roomodes Configuration | HIGH | Roo Code | No |
| [ROO-003](./generated/roo-003.md) | Invalid .rooignore File | MEDIUM | Roo Code | No |
| [ROO-004](./generated/roo-004.md) | Invalid Mode Slug in Rule Directory | MEDIUM | Roo Code | No |
| [ROO-005](./generated/roo-005.md) | Invalid .roo/mcp.json Configuration | HIGH | Roo Code | No |
| [ROO-006](./generated/roo-006.md) | Mode Slug Not Recognized | MEDIUM | Roo Code | No |
| [VER-001](./generated/ver-001.md) | No Tool/Spec Versions Pinned | LOW | Version Awareness | No |
| [WS-001](./generated/ws-001.md) | Empty Windsurf Rule File | MEDIUM | windsurf | No |
| [WS-002](./generated/ws-002.md) | Windsurf Rule File Exceeds Character Limit | HIGH | windsurf | No |
| [WS-003](./generated/ws-003.md) | Empty or Oversized Windsurf Workflow File | MEDIUM | windsurf | No |
| [WS-004](./generated/ws-004.md) | Legacy .windsurfrules File Detected | LOW | windsurf | No |
| [WS-SK-001](./generated/ws-sk-001.md) | Windsurf Skill Uses Unsupported Field | MEDIUM | Windsurf Skills | Yes (safe/unsafe) |
| [XML-001](./generated/xml-001.md) | Unclosed XML Tag | HIGH | XML | Yes (unsafe) |
| [XML-002](./generated/xml-002.md) | Mismatched Closing Tag | HIGH | XML | Yes (unsafe) |
| [XML-003](./generated/xml-003.md) | Unmatched Closing Tag | HIGH | XML | Yes (unsafe) |
| [XP-001](./generated/xp-001.md) | Platform-Specific Feature in Generic Config | HIGH | Cross-Platform | No |
| [XP-002](./generated/xp-002.md) | AGENTS.md Platform Compatibility | MEDIUM | Cross-Platform | No |
| [XP-003](./generated/xp-003.md) | Hard-Coded Platform Paths | MEDIUM | Cross-Platform | No |
| [XP-004](./generated/xp-004.md) | Conflicting Build/Test Commands | MEDIUM | Cross-Platform | No |
| [XP-005](./generated/xp-005.md) | Conflicting Tool Constraints | HIGH | Cross-Platform | No |
| [XP-006](./generated/xp-006.md) | Multiple Layers Without Documented Precedence | MEDIUM | Cross-Platform | No |
| [XP-007](./generated/xp-007.md) | AGENTS.md Exceeds Codex Byte Limit | MEDIUM | Cross-Platform | No |
| [XP-008](./generated/xp-008.md) | Claude-specific Features in CLAUDE.md for Cursor | MEDIUM | Cross-Platform | No |
| [XP-SK-001](./generated/xp-sk-001.md) | Skill Uses Client-Specific Features | LOW | Cross-Platform | No |
| [OC-CFG-001](./generated/oc-cfg-001.md) | Invalid Model Format | HIGH | OpenCode | No |
| [OC-CFG-002](./generated/oc-cfg-002.md) | Invalid autoupdate value | HIGH | OpenCode | No |
| [OC-CFG-003](./generated/oc-cfg-003.md) | Unknown Top-level Config Field | MEDIUM | OpenCode | No |
| [OC-CFG-004](./generated/oc-cfg-004.md) | Invalid Default Agent | MEDIUM | OpenCode | No |
| [OC-CFG-005](./generated/oc-cfg-005.md) | Hardcoded API Key | HIGH | OpenCode | No |
| [OC-CFG-006](./generated/oc-cfg-006.md) | Invalid MCP Server Structure | HIGH | OpenCode | No |
| [OC-CFG-007](./generated/oc-cfg-007.md) | MCP Server Missing Command or URL | HIGH | OpenCode | No |
| [OC-AG-001](./generated/oc-ag-001.md) | Invalid Agent Mode Value | HIGH | OpenCode | No |
| [OC-AG-002](./generated/oc-ag-002.md) | Invalid Color Format | HIGH | OpenCode | No |
| [OC-AG-003](./generated/oc-ag-003.md) | Temperature Out of Range | HIGH | OpenCode | No |
| [OC-AG-004](./generated/oc-ag-004.md) | Steps Not a Positive Integer | HIGH | OpenCode | No |
| [OC-PM-001](./generated/oc-pm-001.md) | Invalid Permission Action | HIGH | OpenCode | No |
| [OC-PM-002](./generated/oc-pm-002.md) | Unknown Permission Key | MEDIUM | OpenCode | No |
| [OC-AGM-001](./generated/oc-agm-001.md) | Empty AGENTS.md | HIGH | OpenCode | No |
| [OC-AGM-002](./generated/oc-agm-002.md) | Secrets in AGENTS.md | HIGH | OpenCode | No |
| [OC-DEP-001](./generated/oc-dep-001.md) | Deprecated mode Field | MEDIUM | OpenCode | Yes (safe) |
| [OC-DEP-002](./generated/oc-dep-002.md) | Deprecated tools Field | MEDIUM | OpenCode | Yes (safe) |
| [OC-DEP-003](./generated/oc-dep-003.md) | Deprecated autoshare Field | MEDIUM | OpenCode | Yes (safe) |
| [OC-DEP-004](./generated/oc-dep-004.md) | Deprecated CONTEXT.md Filename | MEDIUM | OpenCode | No |
| [OC-DEP-005](./generated/oc-dep-005.md) | Deprecated TUI Keys | MEDIUM | OpenCode | No |
| [OC-DEP-006](./generated/oc-dep-006.md) | Deprecated MaxSteps Field | MEDIUM | OpenCode | Yes (safe) |
| [OC-CFG-008](./generated/oc-cfg-008.md) | Invalid Log Level | HIGH | OpenCode | Yes (unsafe) |
| [OC-CFG-009](./generated/oc-cfg-009.md) | Invalid Compaction Reserved | HIGH | OpenCode | No |
| [OC-CFG-010](./generated/oc-cfg-010.md) | Invalid Skills URL | HIGH | OpenCode | No |
| [OC-CFG-011](./generated/oc-cfg-011.md) | Invalid MCP Timeout | HIGH | OpenCode | No |
| [OC-CFG-012](./generated/oc-cfg-012.md) | Invalid MCP OAuth Config | HIGH | OpenCode | No |
| [OC-CFG-013](./generated/oc-cfg-013.md) | Invalid Server Config | HIGH | OpenCode | No |
| [OC-AG-005](./generated/oc-ag-005.md) | top_p Out of Range | HIGH | OpenCode | No |
| [OC-AG-006](./generated/oc-ag-006.md) | Invalid Named Color | MEDIUM | OpenCode | Yes (unsafe) |
| [OC-AG-007](./generated/oc-ag-007.md) | Redundant steps and maxSteps | MEDIUM | OpenCode | No |
| [OC-AG-008](./generated/oc-ag-008.md) | Invalid hidden Type | HIGH | OpenCode | No |
| [OC-AG-009](./generated/oc-ag-009.md) | Invalid Agent Disable Type | HIGH | OpenCode | Yes (unsafe) |
| [OC-LSP-001](./generated/oc-lsp-001.md) | LSP Command Without Extensions | MEDIUM | OpenCode | No |
| [OC-LSP-002](./generated/oc-lsp-002.md) | Invalid LSP Extensions | HIGH | OpenCode | No |
| [OC-TUI-001](./generated/oc-tui-001.md) | Unknown TUI Key | MEDIUM | OpenCode | No |
| [OC-TUI-002](./generated/oc-tui-002.md) | Invalid scroll_speed | HIGH | OpenCode | No |
| [OC-TUI-003](./generated/oc-tui-003.md) | Invalid diff_style | HIGH | OpenCode | Yes (unsafe) |
