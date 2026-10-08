# Project Memory: agnix

> Linter for agent configurations. Validates Skills, Hooks, MCP, Memory, Plugins.

**Repository**: https://github.com/agent-sh/agnix

## Rules

- `knowledge-base/rules.json` is the source of truth for rules. A new rule goes into both `rules.json` and `knowledge-base/VALIDATION-RULES.md`; CI parity tests fail when they drift. See "Adding a rule" below.
- Output is plain text: no emojis, no ASCII art.
- Certainty levels: HIGH (>95%), MEDIUM (75-95%), LOW (<75%).
- Release binaries are compiled with LTO and stripped.
- Track work in GitHub issues.
- A feature or fix is done when a test covers it.
- Long-form docs live in `README.md`, `SPEC.md` and `knowledge-base/` (especially `knowledge-base/VALIDATION-RULES.md`). This file holds agent instructions only. Create no summary, plan or scratch docs unless the task needs them.
- Wait for the `revuto-review` check to pass before merging: it is the most thorough review. If revuto is capped or unavailable, do not wait; a self-review is enough, noted in the PR body.
- Address every review comment before merging, minor ones included. If you disagree, reply in the review thread.
- In prose, write a spaced single dash (` - `), not ` -- `. CLI flags like `--help` or `--fix` are fine.

## Architecture

### Crate dependency graph

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

### Project layout

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
knowledge-base/     # 455 rules, 75+ sources, rules.json

tests/fixtures/     # Test cases by category
```

### agnix-core modules

- `parsers/`: frontmatter, JSON and Markdown parsing.
- `schemas/`: types for skill, hooks, agent, MCP, Cline, Roo and other tool configs.
- `rules/`: validators implementing the `Validator` trait (`rules/mod.rs`).
- `config.rs`, `config/builder.rs`: `LintConfig` and `LintConfigBuilder` (fields are private; build with `LintConfig::builder()...build()?`), `ConfigError`, `ToolVersions`, `SpecRevisions`.
- `diagnostics.rs`: `Diagnostic`, `Fix`, `DiagnosticLevel`, `ValidationOutcome`, `LintError` (= `CoreError`), `LintResult`.
- `file_types/`: `FileType`, `detect_file_type()`, and the `FileTypeDetector` trait with `FileTypeDetectorChain` (chain of responsibility; `with_builtin()`, `prepend`, `push`).
- `registry.rs`: `ValidatorRegistry` and its builder, `ValidatorProvider` for external validators. Validators are `Send + Sync + 'static` because the registry caches one instance each and shares them across threads.
- `pipeline.rs`: `validate_project()`, `validate_file()` returning `LintResult<ValidationOutcome>`.
- `fixes.rs` (auto-fix engine), `eval.rs` (rule precision/recall/F1), `file_utils.rs` (safe I/O: symlink rejection, size limits), `fs.rs` (`FileSystem` trait with `RealFileSystem` and `MockFileSystem`).

`build_unchecked()` (`cfg(test)` or the `__internal_unchecked` feature) and the `__internal` module are for tests, fuzz targets and benches only; `agnix_core::normalize_line_endings` is stable at the crate root. The public API surface and its stability rules are in `CONTRIBUTING.md`.

### Validation flow

```
CLI args → LintConfig → validate_project()
    → Directory walk (ignore crate, respects .gitignore)
    → detect_file_type() per file (path-based, no I/O)
    → Parallel validation (rayon)
    → Validators from registry run sequentially per file
    → Project-level checks (AGM-006, XP-004/005/006, VER-001) via rules/project_level
    → Output (text/JSON/SARIF)
```

### LSP

The backend holds `Arc<ArcSwap<LintConfig>>` for lock-free config reads, an immutable `Arc<ValidatorRegistry>` and a document cache. Validation runs in `spawn_blocking()` because it is CPU-bound and sync. Handled events: `did_open`, `did_change`, `did_save`, `did_close`, `did_change_configuration`, `codeAction`, `hover`.

## Commands

```bash
cargo check                    # Compile check
cargo test                     # Tests and doc tests
cargo nextest run --workspace  # Optional process-per-test runner
cargo build --release          # Build binaries
cargo run --bin agnix -- .     # Run CLI
cargo run --bin agnix-lsp      # Run LSP server
cargo run --bin agnix-mcp      # Run MCP server
bash scripts/preflight.sh      # Quick local checks; --full before pushing
```

Run the tests a change touches, scoped to a crate or integration binary, for example `cargo test -p agnix-core --test fix_integration`. Cargo is the local and CI default. Nextest is optional, uses two test processes by default (raise `--test-threads` only within the machine's CPU budget), and skips doc tests, so pair it with `cargo test --workspace --doc`. Dev and test builds keep `line-tables-only` debug info for file and line backtraces.

CI runs lint and the workspace merge contracts on owner PRs and pushes to main; owner PRs run scoped tests locally before pushing. External contributors' PRs, the daily run, manual runs, PRs labeled `full-suite` and release validation run the full suite.

## Rules reference

455 rules defined in `knowledge-base/rules.json` (source of truth), 455 validation rules across 40 validators. Human-readable docs: `knowledge-base/VALIDATION-RULES.md`. IDs are `[CATEGORY]-[NUMBER]` (AS-004, CC-HK-001).

### Adding a rule

1. Add it to both `rules.json` and `VALIDATION-RULES.md`. Each rule in `rules.json` needs complete `evidence` metadata (`source_type`, `source_urls`, `verified_on`, `applies_to`, `normative_level`, `tests`); the schema is in `VALIDATION-RULES.md`.
2. Run `node scripts/sync-rule-bookkeeping.js` (add `--validators=N` if you registered a new validator). It updates `total_rules` and `last_updated` in `rules.json`, the count phrases in AGENTS.md and README.md, the `crates/agnix-rules/rules.json` mirror and the website docs. CI runs it with `--check`.

## Tool support tiers

agnix validates 11 tools, those with a per-tool validator in `crates/agnix-core/src/rules/`. A higher tier gets stricter testing and release tracking.

**Validated** (have a validator in agnix):

- **S** (test always): Claude Code, Codex CLI, OpenCode, Kiro CLI
- **A** (test on major changes): GitHub Copilot, Cline, Cursor
- **B** (test if time permits): Roo Code, amp
- **C** (community reports only): Gemini CLI
- **D** (nice to have): Windsurf

Release tracking is automated where the upstream publishes to GitHub: `.github/tool-release-baselines.json` and `.github/workflows/tool-release-watch.yml`.

**Watchlist** (no validator yet; tracked by hand in `knowledge-base/RESEARCH-TRACKING.md`): continue, Antigravity, Tabnine, Codeium, Amazon Q, Aider, SourceGraph Cody, pi.

**E** (no support): everything else. Community contributions are welcome through the Tool Support Request issue template.

## Validation scope

agnix is CPU-only Rust and WASM tooling. Pick the checks the change touches from `scripts/preflight.sh`: format, lint, tests, rule bookkeeping and parity, packaging. A change that adds GPU, runtime or model behavior, or claims about it, needs native qualification on that hardware before any support claim.

## References

- `SPEC.md`: technical reference
- `knowledge-base/INDEX.md`: knowledge navigation
- https://agentskills.io
- https://modelcontextprotocol.io
