# Project Memory: agnix

> Linter for agent configurations. Validates Skills, Hooks, MCP, Memory, Plugins.

**Repository**: https://github.com/avifenesh/agnix

## Project Instruction Files

- `CLAUDE.md` is the project memory entrypoint for Claude Code.
- `AGENTS.md` is a byte-for-byte copy of `CLAUDE.md` for tools that read `AGENTS.md` (Codex CLI, OpenCode, Cursor, Cline, Copilot).
- Keep them identical (tests enforce this).

## Critical Rules

1. **Rust workspace** - agnix-rules (data), agnix-core (lib), agnix-cli/agnix-lsp/agnix-mcp (binaries), agnix-wasm (WASM bindings)
2. **rules.json is source of truth** - `knowledge-base/rules.json` is the machine-readable source of truth. When adding a new rule, add it to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests enforce this.
3. **Plain text output** - No emojis, no ASCII art
4. **Certainty filtering** - HIGH (>95%), MEDIUM (75-95%), LOW (<75%)
5. **Release binaries** - Compile with LTO, strip symbols
6. **Track work in GitHub issues** - All tasks tracked there
7. **Task is not done until tests added** - Every feature/fix must have quality tests
8. **Documentation** - Keep long-form docs in `README.md`, `SPEC.md`, and `knowledge-base/` (especially `knowledge-base/VALIDATION-RULES.md`). Keep `CLAUDE.md`/`AGENTS.md` for agent instructions only.
9. **Always follow the skill/command flow as instructed** - No deviations
10. **No unnecessary files** - Don't create summary files, plan files, or temp docs unless specifically required
11. **Never merge without waiting for claude workflow to end successfully** - It might take time, but this is the major quality gate, and most thorough review.
12. **You MUST follow the flow phases one by one** - If they state to use subagents, tools, or any specific method, you must follow it exactly as described.
13. **You MUST address all comments and reviews** - If reviewers leave comments, even minor ones, and even if not a requested change, you must address them all before merging. If you disagree, respond in the review comments. Minor comments must still be addressed.
14. **Use single dash for em-dashes** - In prose, use ` - ` (single dash with spaces), never ` -- ` (double dash). This does not apply to CLI flags like `--help` or `--fix`.

## Architecture

### Crate Dependency Graph

```
agnix-rules (data-only, generated from rules.json)
    ↓
agnix-core (validation engine)
    ↓
├── agnix-cli (command-line interface)
├── agnix-lsp (language server protocol)
├── agnix-mcp (MCP server)
└── agnix-wasm (WebAssembly bindings)
```

### Project Layout

```
crates/
├── agnix-rules/    # Rule definitions (build-time generated)
├── agnix-core/     # Core: parsers, schemas, validators, diagnostics
├── agnix-cli/      # CLI binary (clap)
├── agnix-lsp/      # LSP server (tower-lsp, tokio)
├── agnix-mcp/      # MCP server (rmcp)
└── agnix-wasm/     # WASM bindings for browser/runtime integrations
editors/
├── neovim/         # Neovim plugin
├── vscode/         # VS Code extension
├── jetbrains/      # JetBrains IDE plugin
└── zed/            # Zed extension
knowledge-base/     # 230 rules, 75+ sources, rules.json

tests/fixtures/     # Test cases by category
```

### Core Modules (agnix-core)

- `parsers/` - Frontmatter, JSON, Markdown parsing
- `schemas/` - Type definitions (13 schemas: skill, hooks, agent, mcp, cline, roo, etc.)
- `rules/` - Validators implementing Validator trait (25 validators)
- `config.rs` - LintConfig, LintConfigBuilder, ConfigError, ToolVersions, SpecRevisions
- `diagnostics.rs` - Diagnostic, Fix, DiagnosticLevel, ValidationOutcome, LintError (= CoreError), LintResult
- `eval.rs` - Rule efficacy evaluation (precision/recall/F1)
- `file_types/` - FileType enum, detect_file_type(), FileTypeDetector trait, FileTypeDetectorChain
- `file_utils.rs` - Safe file I/O (symlink rejection, size limits)
- `fixes.rs` - Auto-fix application engine
- `fs.rs` - FileSystem trait abstraction (RealFileSystem, MockFileSystem)
- `pipeline.rs` - ValidationResult, validate_project(), validate_file() -> LintResult<ValidationOutcome>
- `registry.rs` - ValidatorRegistry, ValidatorRegistryBuilder, ValidatorProvider, factory functions

### Key Abstractions

```rust
// Primary extension point
// Implementors must be Send + Sync + 'static (cached in registry, shared across threads)
pub trait Validator: Send + Sync + 'static {
    fn validate(&self, path: &Path, content: &str, config: &LintConfig) -> Vec<Diagnostic>;
    fn name(&self) -> &'static str { /* default: short type name */ }
    fn metadata(&self) -> ValidatorMetadata { /* default: empty rule_ids */ }
}

// Plugin architecture for extensibility
pub trait ValidatorProvider: Send + Sync {
    fn name(&self) -> &str { /* default: short type name */ }
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)>;
    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> { /* default: wraps validators() with None names */ }
}

// Registry with builder pattern and runtime filtering
// Stores cached Box<dyn Validator> instances; no per-file re-instantiation
pub struct ValidatorRegistry { /* ... */ }

impl ValidatorRegistry {
    pub fn builder() -> ValidatorRegistryBuilder;
    pub fn with_defaults() -> Self;
    pub fn validators_for(&self, file_type: FileType) -> &[Box<dyn Validator>];
    pub fn total_validator_count(&self) -> usize;
    pub fn disable_validator(&mut self, name: &'static str);
    pub fn disable_validator_owned(&mut self, name: &str);
}

// Extensible file type detection (chain-of-responsibility)
pub trait FileTypeDetector: Send + Sync {
    fn detect(&self, path: &Path) -> Option<FileType>;
    fn name(&self) -> &str { /* default: short type name */ }
}

pub struct FileTypeDetectorChain { /* ... */ }

impl FileTypeDetectorChain {
    pub fn new() -> Self;
    pub fn with_builtin() -> Self;
    pub fn prepend(self, detector: impl FileTypeDetector + 'static) -> Self;
    pub fn push(self, detector: impl FileTypeDetector + 'static) -> Self;
    pub fn detect(&self, path: &Path) -> Option<FileType>;
}

// Validated config construction (fields are private)
// Usage: LintConfig::builder().severity(Error).tools(vec![...]).build()?
pub struct LintConfigBuilder { /* ... */ }

impl LintConfigBuilder {
    pub fn severity(&mut self, s: SeverityLevel) -> &mut Self;
    pub fn target(&mut self, t: TargetTool) -> &mut Self;
    pub fn tools(&mut self, t: Vec<String>) -> &mut Self;
    pub fn exclude(&mut self, e: Vec<String>) -> &mut Self;
    pub fn disable_rule(&mut self, id: impl Into<String>) -> &mut Self;
    pub fn disable_validator(&mut self, name: impl Into<String>) -> &mut Self;
    pub fn build(&mut self) -> Result<LintConfig, ConfigError>;
    pub fn build_lenient(&mut self) -> Result<LintConfig, ConfigError>;
    // build_unchecked() exists but is #[cfg(any(test, feature = "__internal_unchecked"))]
    // __internal module exists but is #[cfg(any(test, feature = "__internal"))]
    // normalize_line_endings is stable at crate root: agnix_core::normalize_line_endings
}

impl LintConfig {
    pub fn builder() -> LintConfigBuilder;
}
```

### Validation Flow

```
CLI args → LintConfig → validate_project()
    → Directory walk (ignore crate, respects .gitignore)
    → detect_file_type() per file (path-based, no I/O)
    → Parallel validation (rayon)
    → Validators from registry run sequentially per file
    → Project-level checks (AGM-006, XP-004/005/006, VER-001) via rules/project_level
    → Output (text/JSON/SARIF)
```

### LSP Architecture

- Backend holds `Arc<ArcSwap<LintConfig>>` for lock-free config reads, immutable `Arc<ValidatorRegistry>`, document cache
- Validation runs in `spawn_blocking()` (CPU-bound, sync)
- Events: `did_open`, `did_change`, `did_save`, `did_close`, `did_change_configuration`, `codeAction`, `hover`

## Commands

```bash
cargo check                 # Compile check
cargo test                  # Run tests
cargo build --release       # Build binaries
cargo run --bin agnix -- .  # Run CLI
cargo run --bin agnix-lsp   # Run LSP server
cargo run --bin agnix-mcp   # Run MCP server
```

## Rules Reference

230 rules defined in `knowledge-base/rules.json` (source of truth)


Human-readable docs: `knowledge-base/VALIDATION-RULES.md`

Format: `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001, etc.)

**Adding a new rule**: Add to BOTH `rules.json` AND `VALIDATION-RULES.md`. CI parity tests will fail if they drift. Each rule in `rules.json` must include complete `evidence` metadata (source_type, source_urls, verified_on, applies_to, normative_level, tests). See VALIDATION-RULES.md for the evidence schema reference.

## Current State

- v0.13.0 - Production-ready with full validation pipeline
- 230 validation rules across 25 validators

- 3400+ passing tests
- LSP + MCP servers with VS Code extension
- See GitHub issues for roadmap

## Tool Support Tiers

- **S** (test always): Claude Code, Codex CLI, OpenCode
- **A** (test on major changes): GitHub Copilot, Cline, Cursor
- **B** (test if time permits): Roo Code, Kiro CLI, amp, pi
- **C** (community reports only): gemini cli, continue, Antigravity
- **D** (nice to have): Tabnine, Codeium, Amazon Q, Windsurf, Aider, SourceGraph Cody
- **E** (no support, community contributions): Everything else

## References

- SPEC.md - Technical reference
- knowledge-base/INDEX.md - Knowledge navigation
- https://agentskills.io
- https://modelcontextprotocol.io
