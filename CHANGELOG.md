# Changelog

All notable changes to agnix will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.22.1] - 2026-04-26

Security-only patch release shipped via PR #826. No user-visible feature changes.

### Security
- **VS Code extension archive extraction uses argv-only spawn** (#826). Replaced shell-string `execAsync` with a `child_process.spawn`-based wrapper using PowerShell `-LiteralPath` on Windows and `tar` argv on POSIX. Closes the audit finding that a single quote in a user's home directory path could break command quoting.
- **LSP caps `textDocument/didOpen` + `didChange` content at 5 MiB** (#826). Previously any editor could push arbitrary-size documents that were cached in `self.documents`. Oversized docs are now rejected and dropped from cache. The reject is surfaced to the user as a visible WARNING diagnostic so the editor shows *why* validation was skipped instead of silently presenting an empty diagnostic set that looks identical to "no issues".
- **YAML frontmatter rejects pathological nesting > 32 levels** (#826). `serde_yaml` (unmaintained upstream) is still used but guarded by a pre-parse depth check to prevent YAML-bomb memory blowup within the 1 MiB file cap. The depth counter uses raw column positions (not `spaces/2 + tabs`) so 1-space-indented YAML bombs cannot bypass the cap by widening the file rather than deepening it.

## [0.22.0] - 2026-04-26

Shipped via PRs #820-#823 (`agnix tools check/detect`, `agnix schema --fix`, LLM changelog triage CI).

### Added
- **`agnix tools check` + `agnix tools detect` for version-pin drift (#717)**. Two new subcommands for keeping `.tool_versions` in `.agnix.toml` honest against what's actually installed on PATH:
  - **`agnix tools check`** walks every pinned/installed pair, classifies the outcome (`[ok]`, `[drift]`, `[unpinned]`, `[missing]`), prints a report, and exits 0 by default. With `--strict` it exits 1 on any `[drift]` or `[missing]` - fits pre-commit and CI.
  - **`agnix tools detect`** runs `<cli> --version` for each supported tool found on PATH and prints a suggested `[tool_versions]` TOML snippet. With `--write` it merges the detected versions into `.agnix.toml`'s `[tool_versions]` section in place, preserving comments, blank lines, and untouched keys via a pure string-level transform (no TOML round-trip that would lose comments).
  - Resolves petemounce's #717 thread ending where you agreed to "auto-detect and warn. If the user pinned it, it's legit. If not, I detect and warn." Warn-by-default in `check`, `--strict` gives fail-by-default for his pre-commit case. Exact-match comparison (range matching deferred).
  - Semver extractor accepts `N.N.N` with optional pre-release / build-metadata (`beta.3`, `+1234`), skips two-segment versions like `v20.11`, and parses the first match in stdout+stderr combined so CLIs logging to either stream are covered.
  - Scoped to the 4 tools ToolVersions already covers (claude_code, codex, cursor, copilot). Expanding ToolVersions to cover all 11 validated tools is a separate refactor the commands will inherit for free.
  - 19 unit tests (version extraction edge cases + classify-outcome + TOML section round-trip including comment preservation) plus 5 CLI integration tests covering subcommand help discoverability, `check` exit-0 path with nothing pinned, `check --strict` exit-1 path via a pinned-but-missing-CLI fixture (guarded to degrade gracefully when the CLI is unexpectedly present), and `detect --write` exit-0 regardless of whether tools are on the runner's PATH. Locale entries in en/es/zh-CN. Closes #717.
- **`agnix schema --fix` for source-controlled schema drift detection (#716)**. Pete source-controls `schemas/agnix.json` and wanted a pre-commit hook that regenerates it when the agnix binary is bumped. New `--fix` flag reads the current target file, compares against the binary's live schema output, and overwrites only when they differ. Defaults the target path to `schemas/agnix.json` when `--output` is absent. Silent on clean runs (pre-commit stays quiet on no-op), prints `Schema created: <path>` or `Schema updated: <path>` when it writes. Creates parent directories as needed. `.pre-commit-hooks.yaml` ships a new `agnix-schema` hook entry that runs `agnix schema --fix` with `always_run: true` + `pass_filenames: false`. 5 new integration tests cover create-missing, silent-on-clean, update-on-drift, custom path with parent creation, and --help discoverability. Closes #716.

### Changed
- **Schema output now ends with a trailing newline**. Affects both legacy modes (`agnix schema` to stdout and `agnix schema --output <path>`) and the new `--fix` mode. Matches what most editors and formatters produce, so source-controlled schema files round-trip cleanly without spurious "no newline at EOF" diffs. Consumers that grep or diff schema output byte-for-byte may need a one-time refresh; schema parsers are unaffected (trailing whitespace in JSON is legal).
- **LLM-assisted changelog triage in tool-release-watch (#802)**. When `GLM_API_KEY` is set and a tool declares `changes_of_interest` in `.github/tool-release-baselines.json`, the watcher now runs `scripts/glm-extract.js --mode=agnix-triage` after fetching the release notes. The LLM filters the notes down to validator-relevant items + rule candidates using the per-tool descriptor (`config_surfaces`, `relevant`, `irrelevant`). The filtered summary is posted as `## Agnix Triage (auto-filtered)` at the top of the issue body, with the full upstream changelog preserved in a `<details>` block below. Gracefully falls back to raw changelog on any LLM failure (missing key, empty response, HTTP error, node unavailable) - zero regressions vs. prior behavior. `changes_of_interest` descriptors authored for 10 of 11 tracked tools (all except amp, which is already filtered via RSS CDATA). Closes #802.

## [0.21.0] - 2026-04-26

Shipped via PRs #810-#819 (v0.21 rule-candidates sprint: 9 new rules, 3 new validators, rule-bookkeeping automation, safe auto-fixes for Kiro toolSearch).

### Changed
- **Auto-fix for KR-SET-001/002/003 type coercion**. The three Kiro settings rules now ship safe auto-fixes for the common case of "user wrote a string when the docs say bool/number". KR-SET-001 strips quotes + normalizes case on `"true"` / `"False"` / `"TRUE"` → `true`/`false`; KR-SET-002/003 strip quotes on numeric strings (KR-SET-003 only accepts integer strings since minTokens must be whole). Fixes are marked `safe: true` - Kiro rejects quoted booleans/numbers anyway, so unquoting preserves user intent exactly. Other paths (negative values, fractional tokens, non-bool-ambiguous strings like "yes") stay manual. Audit of all 9 rules from the sprint found only these 3 to be mechanically fixable - the others require user-specific values (API tokens, server names, URL templates) that can't be guessed. 19 new unit tests covering autofix paths + `find_value_span` JSON-value-slicing helper.
- **Rule bookkeeping: extend automation to VALIDATION-RULES.md footer stats**. `scripts/sync-rule-bookkeeping.js` now also syncs the three derived lines at the end of `knowledge-base/VALIDATION-RULES.md`: **Total Coverage** (N rules across M categories), **Certainty** (HIGH/MEDIUM/LOW counts), and **Auto-Fixable** (count + percentage). Previously these drifted whenever a rule was added or a rule's `autofix` flag flipped - now they're derived from the rules.json source of truth on every sync run.
- **Rule bookkeeping automation (#129)**. Adding a rule previously meant hand-updating eight or so derived locations - `total_rules` and `last_updated` in `knowledge-base/rules.json`, the `crates/agnix-rules/rules.json` mirror, the "N rules" / "N validators" phrases in CLAUDE.md/AGENTS.md/README.md, and the website docs regen. New `scripts/sync-rule-bookkeeping.js` handles them all from the single source of truth: run it after editing `rules.json` (optionally with `--validators=N` if a new validator was registered) and it syncs every derived file. CI now runs `node scripts/sync-rule-bookkeeping.js --check --skip-docs` in the `test` job so drift fails the build.

### Added
- **GM-AG-001: Validate `auth` block in Gemini CLI agent MCP config (#809)**. Gemini CLI v0.39.0 added an `auth` block to `mcp_servers.*` entries in local agent markdown frontmatter (`.gemini/agents/*.md`) - see google-gemini/gemini-cli#24770. Two variants: `type: "google-credentials"` (only `scopes` allowed) and `type: "oauth"` (`client_id`, `client_secret`, `scopes`, `authorization_url`, `token_url`). GM-AG-001 enforces the discriminator, rejects unknown fields per variant, type-checks string/array values, and validates URL shape. Introduces `FileType::GeminiAgent` for `.gemini/agents/*.md` detection + a new `GeminiAgentValidator` with its own frontmatter extraction + auth-block line scanner. 24 unit tests + valid/invalid fixtures. New `gemini-agents` rule category. Closes #809.
- **KR-SET-001 + KR-SET-002 + KR-SET-003: Validate Kiro Tool Search settings (#808)**. Kiro CLI 2.1 added the Tool Search feature configured via flat-key JSON in `.kiro/settings.json` (or `~/.kiro/settings.json`): `toolSearch.enabled` (boolean master toggle, default false), `toolSearch.minPct` (number, default 5), and `toolSearch.minTokens` (integer, default 50000). Docs at kiro.dev/docs/cli/mcp/tool-search/. Introduces a new `FileType::KiroSettings` variant + detection for files named `settings.json` with a `.kiro` parent directory, a new `KiroSettingsValidator`, and three rules: KR-SET-001 (HIGH, non-boolean `toolSearch.enabled`), KR-SET-002 (MEDIUM, type/negative/>100 on `toolSearch.minPct`), KR-SET-003 (MEDIUM, type/negative/fractional on `toolSearch.minTokens`). 23 unit tests + valid/invalid fixtures. New `kiro-settings` rule category. Closes #808.
- **CC-HK-026 + CC-HK-027: Validate required fields on `type: "mcp_tool"` hooks (#804)**. Claude Code v2.1.118 added the `mcp_tool` hook action type; the docs at code.claude.com/docs/en/hooks#mcp-tool-hook-fields specify `server` (required string, names an already-connected MCP server) and `tool` (required string, names the tool to invoke), plus optional `input` (object for tool arguments with `${path}` substitution). CC-HK-026 flags missing/empty/non-string `server`; CC-HK-027 flags the same for `tool`. Both are HIGH severity (hook will fail at runtime), independently disableable. Also updated the typed `Hook::McpTool` schema in `schemas/hooks.rs` with the correct shape (was a stub with wrong fields). 8 new tests + valid/invalid fixtures. Closes #804.
- **CC-SET-001: Validate `prUrlTemplate` in `.claude/settings.json` (#803)**. Claude Code v2.1.119 added `prUrlTemplate` as a top-level settings key that points the footer PR badge at a custom code-review URL instead of github.com. CC-SET-001 flags three misconfigurations: (a) non-string values (ERROR), (b) empty strings (ERROR, badge would never render), (c) strings with none of the documented placeholders `{host}`, `{owner}`, `{repo}`, `{number}`, `{url}` (WARNING, every PR would resolve to the same static URL). Ships with a new `ClaudeSettingsValidator` scoped to `.claude/settings.json`, `.claude/settings.local.json`, and `.claude/managed-settings.json` so sibling tools (`.amp/settings.json`, etc.) that share the Hooks FileType aren't false-positived. 19 unit tests + valid/invalid fixtures. Introduces a new `claude-settings` rule category to distinguish top-level settings rules from hooks, memory, skills, etc. Closes #803.
- **CDX-CFG-029: agents.max_threads incompatible with multi_agent_v2 (#806)**. In rust-v0.125.0 Codex rejects the config at load time with "agents.max_threads cannot be set when multi_agent_v2 is enabled" - the v2 agent lifecycle manages threading internally and accepting the legacy limit alongside creates conflicting semantics (openai/codex#19129). CDX-CFG-029 detects the feature in either shape Codex accepts (flat `[features] multi_agent_v2 = true` or table `[features.multi_agent_v2] enabled = true`) and errors when `[agents] max_threads` is also set. Closes #806.
- **CDX-CFG-028: Reject unsupported inline MCP `bearer_token` field (#805)**. In rust-v0.125.0 Codex runtime rejects `mcp_servers.<name>.bearer_token` and requires `bearer_token_env_var` instead; schemars generation also dropped the field (openai/codex#19294). agnix now emits a HIGH-severity diagnostic pointing users at the correct replacement, with a specific suggestion that keeps secrets out of the config file. Removed `bearer_token` from `KNOWN_MCP_SERVER_KEYS` and suppressed it from the generic CDX-CFG-006 (nested unknown-key) path when CDX-CFG-028 is enabled, so users get one specific diagnostic rather than two. Disabling CDX-CFG-028 intentionally falls through to CDX-CFG-006 so the field is never silently accepted. Closes #805.

### Changed
- **MCP validator now accepts flat top-level server-map shape (#807)**. Codex rust-v0.123.0 made plugin MCP loading accept both `{ "mcpServers": {...} }` (the MCP spec / Claude Code / Cursor shape) and a flat `{ "serverName": {...}, ... }` top-level map. Previously agnix's `extract_mcp_servers` silently returned empty when `mcpServers` was absent, so per-server rules (MCP-009..012, MCP-024) didn't fire on flat-form files. Now the extractor falls back to reading the root object as a server map when every top-level value looks like a server config (has `command` or `url`). Guardrails: when `mcpServers` is present it still wins (flat shape is fallback-only), and generic JSON-RPC payloads + partial typos are rejected conservatively so the heuristic doesn't misread non-server JSON as servers. 8 new unit tests cover both shapes plus the rejection guardrails. Closes #807.
- **Tool-release watcher triage (2026-04-25)**: full changelog review of 8 tracked tools against their validators. Bumped baselines in `.github/tool-release-baselines.json` and `Last Reviewed` in `knowledge-base/RESEARCH-TRACKING.md` for Claude Code (v2.1.117 → v2.1.119), Codex CLI (rust-v0.122.0 → rust-v0.125.0), OpenCode (v1.14.20 → v1.14.25), Kiro CLI (2.0.1 → 2.1.1), Cline (v3.80.0 → v3.81.0), Cursor (3.1.17 → 3.2.11), Roo Code (v3.52.1 → v3.53.0), Gemini CLI (v0.38.2 → v0.39.1). No validator code changes — rule candidates surfaced (Claude Code `prUrlTemplate`, `type:"mcp_tool"` hook action; Codex `.mcp.json` dual-shape, `agents.max_threads`+`multi_agent_v2` conflict, `bearer_token` rejection; Kiro `toolSearch.enabled`; Gemini agent MCP `auth` block) are filed as separate follow-up issues so each gets a proper design pass against upstream docs. Closes #778, #779, #780, #781, #782, #795, #796, #797.

## [0.20.1] - 2026-04-25

Shipped via PR #800.

### Fixed
- **#799 + full i18n audit: 84 missing rule-locale keys across 37 rules** - a comprehensive audit of every `t!("rules.X.Y")` call against `locales/en.yml` surfaced 84 missing entries, spanning Codex (CDX-CFG-013..022, CDX-APP-002/003), Kiro (KIRO-010/011/013/014, KR-AG-008..013, KR-HK-007/009/010, KR-MCP-003/004, KR-PW-005..008), and OpenCode (OC-AG-001/002, OC-CFG-001/005/006/007). Every one would have rendered the lookup key to users instead of the diagnostic text — same regression class as #799 but much wider. Added complete entries (message + suggestion, plus type_error/local_missing/etc where the code expects them). New automated regression guard `i18n_tests::test_every_rule_locale_key_referenced_in_source_resolves` walks `src/**/*.rs` at test time, extracts every literal `t!("rules.X.Y")` reference (word-boundary match so `format!(...t!(...)...)` calls within other macros are handled correctly), and fails CI if any key is missing from `locales/en.yml` — so this class of bug can never ship silently again.
- **#798 XML-balance false positive on `list<string>`** - bare lowercase primitive-type names in `<...>` inside Markdown tables or prose (`list<string>`, `map<string,int>`, `Vec<&str>`) were flagged as unclosed XML tags. Extended `is_likely_type_parameter` with a lowercase-primitive allowlist (`str`, `string`, `int`, `bool`, `float`, `char`, `byte`, `list`, `map`, `vec`, `slice`, etc) plus Rust sized int/float pattern (`i32`, `u64`, `f32`). Guardrail test ensures non-primitive lowercase tags (e.g. `<custom>`) still flag.

## [0.20.0] - 2026-04-24

### Changed
- **Dependency bumps (dependabot queue drain)**: consolidated 8 open dependabot PRs (#783-#790) into a single verified branch:
  - Rust crates: `rayon 1.10 -> 1.12`, `toml 0.8 -> 1.x` (the clean `cargo update` resolved to the latest 1.x release at merge time; coexists with `rust-i18n-support`'s `0.8.x`), `rmcp 1.4 -> 1.5`, `similar 3.0 -> 3.1`, `uuid 1.23.0 -> 1.23.1`.
  - CI actions: `actions/setup-node` in `tool-release-watch.yml` bumped `v4.4.0 -> v6.4.0`; other `setup-node` callers bumped `v6.3.0 -> v6.4.0`; `taiki-e/install-action 2.75.17 -> 2.75.21`; `anthropics/claude-code-action@v1` SHA updated to `e58dfa5` (v1.0.101 -> v1.0.105).
  - `deny.toml` skip list updated: the post-bump dep tree dropped `0.48.5` from several `windows_*` arch crates and added a new `0.53.1` family (transitive via newer tokio/notify).
  - `cargo audit` ignores two new transitive advisories from `rust-i18n-macro 3.1.2` → `libyml` (RUSTSEC-2025-0067) and `serde_yml` (RUSTSEC-2025-0068); both tracked in `docs/RUSTSEC-ADVISORIES.md` for removal when rust-i18n migrates off `serde_yml`.
  - Full workspace build + `cargo test` + `cargo clippy --workspace --all-targets -- -D warnings` pass after the bump.

## [0.19.0] - 2026-04-23

### Added
- **Output-style validator (CC-OS-001..006)** - new validator for `.claude/output-styles/*.md` files, surfaced during the Claude Code v2.1.117 triage (#745). The output-style frontmatter spec was added in v2.1.94 with the `keep-coding-instructions` field. New rules: CC-OS-001 missing description (LOW), CC-OS-002 invalid `keep-coding-instructions` type (HIGH), CC-OS-003 unknown frontmatter key (MEDIUM), CC-OS-004 empty body (MEDIUM), CC-OS-005 name exceeds 64 chars (LOW), CC-OS-006 invalid frontmatter syntax (HIGH). All non-autofix. New `FileType::ClaudeOutputStyle`, `OutputStyleSchema`, `OutputStyleValidator`. Total rules: 399 -> 405.
- **Daily tool-release watcher** (`.github/workflows/tool-release-watch.yml`, `scripts/check-tool-releases.sh`, `scripts/glm-extract.js`) - polls every supported tool's release feed daily at 7am UTC and opens a per-tool issue when a new release is detected. Supports four source types via `.github/tool-release-baselines.json`: GitHub releases (claude-code, codex, opencode, copilot via `microsoft/vscode-copilot-chat`, cline, roo-code, gemini-cli), JSON update endpoint (cursor's `api2.cursor.sh` stable channel), HTML scrape with regex (kiro, windsurf), and RSS slug (amp). Per-tool `notes_extractor` selects between LLM-extracted markdown via GLM (`glm-5` by default; activates only when `GLM_API_KEY` repo secret is set), `<item><description>` CDATA parse, or stub link. Issues are deduped by `tool-release:<id>` label so re-runs comment on existing open issues. `workflow_dispatch` accepts `tool` filter and `update_baselines: true` toggle.
- **Playground state in URL** - playground configs are now shareable by link via `?file=`, `?tool=`, and `?code=` querystring params (URL-safe base64 for UTF-8 content, debounced writes, capped at 6 KB to stay URL-safe).

### Changed
- **Clippy gate now covers tests and benches** - `cargo clippy` in `.github/workflows/ci.yml` and `scripts/pre-push-rust` now runs with `--all-targets --all-features`, so warnings in test, bench, and example code fail the build instead of slipping through. Surfaced and fixed 8 latent warnings: `items_after_test_module` in `agnix-cli/src/main.rs`, `unnecessary_literal_unwrap` and `Ok(...).unwrap()` in `agnix-cli/src/sarif.rs` and `agnix-core/tests/api_contract.rs` (both intentional contract tests, marked `#[allow]`), needless `<'a>` lifetime in `agnix-cli/tests/kiro_fixture_inventory.rs`, deprecated `criterion::black_box` -> `std::hint::black_box` in `agnix-core/benches/validation.rs`, `needless_borrows_for_generic_args` in `agnix-core/src/rules/cross_platform.rs` and `agnix-core/src/rules/project_level.rs` (3 sites), `unnecessary_cast` in `agnix-lsp/src/backend/tests.rs`, and unused `DiagnosticLevel` / `LintConfig` imports in `agnix-lsp/tests/lsp_integration.rs`.
- **Tool-tier list reorganised** in `CLAUDE.md` / `AGENTS.md` - split into "Validated" (the 11 tools with a per-tool validator in `crates/agnix-core/src/rules/`) and "Watchlist" (8 aspirational entries with no validator yet) so a reader can tell at a glance what agnix actually supports today. `knowledge-base/RESEARCH-TRACKING.md` aligned: Kiro CLI moved B-tier -> S-tier (matches the canonical tier list), Roo Code repo URL updated (`RooVetGit/Roo-Code` -> `RooCodeInc/Roo-Code` after upstream rename), and watchlist entries removed from the per-tier tables since they have no rules to track.
- **Search UX** - the top-right search now shows 8 inline preview hits as you type (via `searchResultLimits` on `docusaurus-search-local`) instead of only a "See all results" link.
- **Docs drift** - updated validation rule count from 385 to 399 across 20+ user-facing locations: `CLAUDE.md`, `AGENTS.md`, `README.md`, `SPEC.md`, `npm/README.md`, `crates/agnix-lsp/README.md`, `docs/EDITOR-SETUP.md`, Neovim and VS Code editor READMEs, `editors/vscode/package.json`, `editors/zed/README.md`, `knowledge-base/INDEX.md` (5 places), `knowledge-base/README.md`, `plugin/.claude-plugin/plugin.json`, `plugin/commands/agnix.md`, `plugin/skills/agnix/SKILL.md`, `skills/agnix/SKILL.md`, `crates/agnix-mcp/src/main.rs` server instructions (3 places), and website docs (intro, getting-started, contributing). `rules.json` is the source of truth; these human-written references had drifted.
- **CONTRIBUTING tone** - `CONTRIBUTING.md` and `website/docs/contributing.md` use first-person singular ("I welcome", "I want to know") since agnix is single-maintainer. Previously used "we" which implied a team.
- **Docusaurus** - bumped `@docusaurus/core` and `@docusaurus/preset-classic` from 3.9.2 to 3.10.0.
- **Cursor 3.0.0 -> 3.1.17 doc refresh + URL migration** (#749) - re-verified all 19 CUR-* rules against the upstream changelog at `cursor.com/changelog` (3.0 "New Cursor Interface" 2026-04-02, two patch posts 2026-04-08 / 2026-04-15, 3.1 "Tiled Layout and Upgraded Voice Input" 2026-04-13). The 3.0/3.1 features are all UX-side (Agents Window, /worktree, /best-of-n, Design Mode, tiled layout, voice input, Canvases) or cloud-side (Bugbot learned rules, Bugbot MCP) - none touch any on-disk file agnix validates. MDC frontmatter (`description`, `globs`, `alwaysApply`), Cursor agent fields (`name`, `description`, `model`, `readonly`, `is_background`), and the 20 hook events all match agnix's allow-lists exactly (verified against the new docs). No code changes required. Cursor's docs URL structure migrated upstream: `/docs/context/rules` -> `/docs/rules` (CUR-001..009), `/docs/agent/hooks` -> `/docs/hooks` (CUR-010..013, CUR-017..019), `/docs/context/subagents` -> `/docs/subagents` (CUR-014/015). Updated 18 source URL references across 18 rule entries (CUR-016 cloud-agent URL is unchanged). Bumped `verified_on` for all 19 CUR-* rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Cursor from 2026-02-26 to 2026-04-22; expanded the Config Format column to list all 5 file types Cursor validates (was only listing `.cursor/rules/*.mdc` and `.cursorrules`; added `.cursor/hooks.json`, `.cursor/agents/*.md`, `.cursor/environment.json`); updated the per-doc-source breakdown rows to use the new URLs.
- **Cline v3.77.0 -> v3.80.0 doc refresh** (#746) - re-verified all 10 Cline rules (CLN-001..006, CLN-009 + CL-SK-001..003; CLN-007/008 are reserved IDs not currently defined) against the upstream `https://docs.cline.bot/features/cline-rules/overview` and `https://docs.cline.bot/features/hooks` docs. The 3-version bump introduced `globalSkills` enterprise remote config with `alwaysEnabled` enforcement (v3.79.0/v3.80.0), removed foreground terminal mode (v3.80.0), added quota-exceeded UI, and fixed long-conversation OOM crashes - none of these touch any on-disk file agnix validates. `globalSkills`/`alwaysEnabled` live exclusively in Cline's `RemoteConfigSchema` (delivered over the network via the enterprise remote-config API), not in `.cline/skills/*/SKILL.md` or `.clinerules/skills/*/SKILL.md` frontmatter. Foreground terminal removal is VS Code extension internals; agnix has no validator for Cline settings files. Verified the v3.77.0 changelog mention "Polish Notification hook functionality" against the official hooks docs - `Notification` is NOT a valid Cline hook event (the 8 valid events match agnix's `VALID_HOOK_EVENTS` exactly: TaskStart, TaskResume, TaskCancel, TaskComplete, PreToolUse, PostToolUse, UserPromptSubmit, PreCompact). No code changes required. Bumped `verified_on` for all 10 Cline rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Cline from 2026-02-05 to 2026-04-22; expanded the Config Format column to list all 5 file types Cline validates (was only listing 2) and filled in the Rules column (was `--`).
- **Kiro CLI 1.28.3 -> 2.0.1 doc refresh** (#751) - re-verified all 51 Kiro rules (KIRO-* steering, KR-AG-* agents, KR-HK-* hooks, KR-MCP-* MCP, KR-PW-* powers, KR-SK-* skills) against the upstream `https://kiro.dev/changelog/cli/` HTML changelog and the relevant pages under `https://kiro.dev/docs/`. The 1.28 -> 2.0 jump was platform expansion (Windows native, headless mode `KIRO_API_KEY`, TUI graduation, granular tool trust, session settings tool, simplified agent creation) - none of those add fields to any of the 5 Kiro file types agnix validates. Two ambiguous changelog mentions (`availableAgents`/`trustedAgents` from v1.25, `knowledgeIndex` resource type from v1.23/v1.24) were verified against `kiro.dev/docs/cli/custom-agents/configuration-reference` and confirmed NOT to appear in `.kiro/agents/*.json` field lists - they're either global settings or were never shipped as documented config. No code changes required. Bumped `verified_on` for all 51 Kiro rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Kiro CLI from 2026-02-05 to 2026-04-22 (also added `.kiro/powers/*/POWER.md` to the Config Format column - it was missing). Discovered: agnix has FIVE Kiro validators, not four - `kiro_power.rs` (KR-PW-001..008) for `.kiro/powers/*/POWER.md` was undocumented in earlier triage briefings.
- **OpenCode v1.14.20 -> v1.14.21 doc refresh** (#766) - re-verified all 46 OC-* rules against the upstream changelog. v1.14.21 is a runtime/UI-only patch (LSP pull diagnostics support, bare Git repo project caching, session compaction improvements, UTF-8 BOM preservation, Roslyn for C#, Mistral high reasoning variant, TUI/Desktop fixes) - zero on-disk file schema changes. No code changes required. Bumped `verified_on` for all 46 OC-* rules to 2026-04-23; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for OpenCode 2026-04-22 -> 2026-04-23 and added OC to the Rules column (was just `AGM, XP`).
- **Roo Code v3.51.1 -> v3.52.1 doc refresh** (#753) - re-verified all 6 ROO-* rules against the upstream `CHANGELOG.md`. v3.52.0 added Poe AI provider, MiniMax model fixes, xAI Grok-4.20 + GPT-5.4 model catalog updates; v3.52.1 added the IDE-side JSON schema for `.roomodes` autocomplete + UI cleanup. None of these touch any on-disk file agnix validates: provider/model lists are not allowlisted, and agnix's `parse_roomodes` is permissive (only reads `slug`, `name`, `roleDefinition`, `groups` — ignores `whenToUse`, `description`, `customInstructions`, `source`, etc.). No code changes required. Bumped `verified_on` for all 6 ROO-* rules to 2026-04-23; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Roo Code 2026-02-05 -> 2026-04-23; expanded the Config Format column to list all 6 file types Roo validates (was only listing `.roo/rules/*.md`); filled in Rules column (was `--`, now `ROO`).
- **amp (Sourcegraph) `amp-free-is-ad-free` doc refresh** (#744) - the news post is a pricing/business announcement (Amp Free no longer shows ads). Zero on-disk file schema impact. No code changes. Bumped `verified_on` to 2026-04-23 for all 5 amp rules (AMP-001..004 + AMP-SK-001); bumped `RESEARCH-TRACKING.md` "Last Reviewed" for amp 2026-02-05 -> 2026-04-23 and filled in the Rules column (was `--`, now `AMP, AMP-SK`).
- **Gemini CLI v0.36.0 -> v0.38.2 doc refresh** (#750) - re-verified all 9 GM-* rules. Many runtime/UX/sandbox/browser-agent changes in this cumulative window; only schema-relevant addition was `experimental.adk.agentSessionNoninteractiveEnabled` in v0.37.0. agnix's GM-* rules are permissive on settings.json (GM-004 validates hooks structure only, GM-009 catches parse errors only), so no false-positive risk. No code changes. Bumped `verified_on` for all 9 GM-* rules to 2026-04-23; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for gemini cli 2026-02-05 -> 2026-04-23, normalized Rules column from `GM-` to `GM`.
- **Windsurf Wave 13 -> Wave 14 doc refresh** (#754) - Wave 14 added Arena Mode (run two Cascade agents side-by-side), Plan Mode (`megaplan`), team-admin default-model setting. All runtime UI / cloud features. Zero on-disk schema impact. No code changes. Bumped `verified_on` for all 5 WS-* rules to 2026-04-23; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Windsurf 2026-02-05 -> 2026-04-23, filled in Rules column (was `--`, now `WS, WS-SK`).

### Fixed
- **Codex CLI rust-v0.122.0 -> rust-v0.123.0 catch-up** (#765) - upstream `config-schema.json` (verified 2026-04-23) gained 1 new top-level key: `experimental_thread_store_endpoint` (per #18714 in v0.123.0). Without this fix, agnix would false-positive **CDX-004** (unknown TOML top-level key) for any v0.123.0+ config. Added to both `KNOWN_TOP_LEVEL_KEYS` (schemas/codex.rs, used by CDX-004) and `KNOWN_CONFIG_TOP_LEVEL_KEYS` (rules/codex.rs, used by CDX-CFG-006) plus regression tests for both the TOML and JSON paths. Bumped `verified_on` for all 58 CDX-* rules to 2026-04-23. Other v0.123.0 changes (amazon-bedrock model provider #18744, /mcp verbose #18610, plugin MCP loading dual format #18780, realtime handoffs #18597, `remote_sandbox_config` #18763, model metadata refresh) need no agnix changes — `remote_sandbox_config` does not appear as a top-level key in the published schema; model providers are not allowlisted by agnix; the rest are runtime/CLI/UI.
- **Claude Code v2.1.117 -> v2.1.118 catch-up** (#764) - upstream v2.1.118 added `type: "mcp_tool"` so hooks can invoke MCP tools directly. Without this fix, agnix would false-positive **CC-HK-016** (unknown hook type) on every v2.1.118+ user that follows the release notes. Added `"mcp_tool"` to both `valid_types` arrays AND `known_non_command` arrays in `crates/agnix-core/src/rules/hooks/helpers.rs` (4 sites total — two for the strict-fields pass, two for the async-on-non-command pass) plus a regression test asserting `mcp_tool` doesn't trigger CC-HK-016. Note: as of 2026-04-23 the docs at `code.claude.com/docs/en/hooks` only list 4 types; the v2.1.118 release notes are authoritative for the new `mcp_tool` type until the docs catch up. Bumped `verified_on` for all 25 CC-HK-* rules to 2026-04-23. Other v2.1.118 changes (vim visual mode, `/cost`+`/stats`->`/usage`, custom themes, plugin `themes/` directory, `DISABLE_UPDATES`, `wslInheritsWindowsSettings` policy key, `autoMode.*` `"$defaults"` token, `claude plugin tag`, OAuth and credential bug fixes) need no agnix changes — they're runtime/UI/CLI behavior or settings.json fields agnix doesn't validate directly.
- **GitHub Copilot v0.42.2 -> v0.43.0 catch-up** (#748) - upstream PR `microsoft/vscode-copilot-chat#4964` (in v0.43.0) makes the `description` frontmatter field on `.instructions.md` files a user-visible feature in the VS Code Chat Customizations UI. Without this fix, agnix would false-positive **COP-004** (unknown frontmatter key) and offer an auto-fix to delete the line on every v0.43.0+ user that follows the docs. Added `"description"` to `KNOWN_KEYS` and a typed `description: Option<String>` field on `CopilotScopedSchema` in `crates/agnix-core/src/schemas/copilot.rs` plus two regression tests (parse + unknown-key check). Documentation: bumped `verified_on` for all 25 COP-* rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for GitHub Copilot from 2026-02-05 to 2026-04-22. Verified other v0.43.0 changes (hooks/plugins UI wiring, AGENTS.md/CLAUDE.md multi-root discovery fix, internal NES/session/telemetry refactors) need no agnix changes - they're UI/runtime only with no schema impact.
- **OpenCode v1.3.13 -> v1.14.20 catch-up** (#752) - upstream PR `anomalyco/opencode#13748` (in v1.3.16) added the `tui.mouse: bool` config key to disable terminal mouse capture. Without this fix, agnix would false-positive **OC-TUI-001** (unknown TUI key) on any current OpenCode config that includes `{"tui": {"mouse": false}}`. Added `"mouse"` to `KNOWN_TUI_KEYS` in `crates/agnix-core/src/schemas/opencode.rs` plus two regression tests (`mouse: false` and `mouse: true`). Documentation: bumped `verified_on` for all 46 OC-* rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for OpenCode from 2026-02-05 to 2026-04-22. Verified other schema additions in the bump window (MCP `oauth_redirect_uri` in v1.4.3, `compaction.autocontinue` in v1.4.4, `compaction.preserve_recent_tokens` in v1.14.19, LLM Gateway provider in v1.4.9) need no agnix changes - they extend optional sub-fields agnix doesn't enforce. Note: as of v1.14.x, upstream `config.ts` deprecates the in-config `tui` block in favour of a separate `tui.json` file; agnix's TUI validator continues to help users on v1.3-v1.4 and users who keep the deprecated form. Migrating to validate `tui.json` is a future enhancement, not a blocker for this triage.
- **Codex CLI rust-v0.118.0 -> rust-v0.122.0 catch-up** (#747) - upstream `config-schema.json` (verified 2026-04-22) gained 8 new top-level keys (`experimental_realtime_start_instructions`, `experimental_realtime_ws_startup_context`, `include_apps_instructions`, `include_environment_context`, `include_permissions_instructions`, `marketplaces`, `realtime`, `tool_suggest`). Without this fix, agnix would false-positive **CDX-004** (unknown TOML top-level key, primary path for `.codex/config.toml`) and CDX-CFG-006 (its JSON/YAML equivalent) on any current Codex config that uses them. Added all 8 to `KNOWN_TOP_LEVEL_KEYS` in `crates/agnix-core/src/schemas/codex.rs` (the TOML path used by CDX-004) AND `KNOWN_CONFIG_TOP_LEVEL_KEYS` in `crates/agnix-core/src/rules/codex.rs` (the JSON/YAML path). Also added `realtime` and `marketplaces` to `KNOWN_TABLE_KEYS` since both can appear as TOML tables (`[realtime]`, `[[marketplaces]]`). Plus a regression test that runs an inline-table config and a section-table config and asserts zero CDX-004 diagnostics. Documentation: bumped `verified_on` for all 58 CDX-* rules to 2026-04-22; bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Codex CLI from 2026-02-05 to 2026-04-22. Verified MCP server config (`env`, `env_vars`), AGENTS.md discovery refactor (#18035), filesystem deny-read globs (#15979), and PermissionRequest hooks (#17563) require no agnix changes - their upstream effects are runtime-only, additive, or already covered by existing validators.
- **Claude Code v2.1.90 -> v2.1.117 catch-up** (#745) - `Monitor` (the new built-in tool added in v2.1.98 for streaming events from background scripts) is now in `KNOWN_AGENT_TOOLS`, so `tools: [Monitor]` in agent frontmatter no longer false-positives CC-AG-009/CC-AG-010. `xhigh` (the new effort level for Opus 4.7, added in v2.1.111) is now in `schemas::skill::VALID_EFFORT_LEVELS` (the single source of truth shared by both agent CC-AG-014 and skill CC-SK-018 validation), so `effort: xhigh` no longer false-positives in either rule. The agent `color` field (display color for the task list and transcript) is now a typed field on `AgentSchema` and listed in `KNOWN_AGENT_FIELDS`, so `color: blue` no longer false-positives CC-AG-019. Documentation: bumped `verified_on` for CC-AG-009/010/011/014/019 and CC-SK-018 to 2026-04-22; added a single scope note at the top of the `CLAUDE CODE RULES (SUBAGENTS)` section in `VALIDATION-RULES.md` that agent-frontmatter `hooks` and `mcpServers` are loaded for both subagent spawning and `--agent` main-thread sessions (v2.1.116/v2.1.117); bumped `RESEARCH-TRACKING.md` "Last Reviewed" for Claude Code from 2026-02-05 to 2026-04-22.
- **Clippy errors under Rust 1.95** - collapsed nested `if` blocks into match guards in `kiro_steering.rs`, `roo.rs`, and `windsurf.rs`, and switched `fixes.rs` to `sort_by_key` with `Reverse`. Resolves `collapsible_match` and `unnecessary_sort_by` lints newly enforced on stable 1.95, unblocking CI on `main` and all open PRs.
- **`[files].exclude` silently half-worked** (#722) - previously this filter ran at `resolve_file_type`, which skipped per-file validators but left project-level rules (AGM-006, XP-004/005/006) collecting vendored paths by filename during the walk. A vendored `AGENTS.md` would still fire "Nested AGENTS.md" even when excluded. `[files].exclude` now joins the walker filter alongside top-level `exclude`, so both filters share one "don't look at this path" semantic and cross-file rules honour it too.
- **Pre-commit hook rescanned the whole repo** (#723) - `agnix` now accepts multiple positional paths, and `.pre-commit-hooks.yaml` no longer forces `pass_filenames: false` + trailing `.`. pre-commit's built-in optimisation works again: only the changed files get checked, not every eligible file in the repo.
- **Docs site versioning** - `/docs/` now serves the latest released version (0.18.0) instead of unreleased dev content, and dev docs moved to `/docs/next/` with an unreleased banner. Snapshotted `docs/` as `version-0.18.0` (lossless - no commits touched `docs/` after the v0.18.0 tag).
- **Release automation** - the `version-docs` job in `release.yml` now bumps `lastVersion` in `docusaurus.config.js` and includes the config change in the auto-opened docs PR, so `/docs/` automatically flips to point at the newly released version. Previously the snapshot was created but `lastVersion` stayed stale, which caused the v0.18.0 drift.

## [0.18.0] - 2026-04-02

### Added
- **Codex CLI plugin manifest validation** (CDX-PL-001 to CDX-PL-014) - 14 new rules for `.codex-plugin/plugin.json` manifests introduced in Codex CLI v0.117.0, covering manifest location, name validation, component paths, defaultPrompt constraints, interface URLs, asset paths, and unsupported fields
- **New FileType::CodexPlugin** variant with case-insensitive parent directory detection

## [0.17.0] - 2026-03-28

### Added
- **44 new validation rules** from March 2026 monthly spec drift review covering Claude Code hooks (CC-HK-020..025), skills (CC-SK-018..020), agents (CC-AG-014..017,019), plugins (CC-PL-011..014), memory (CC-MEM-014), Codex CLI (CDX-CFG-023..027), Cursor (CUR-017..019), Cline workflows/hooks/skills (CLN-005,006,009, CL-SK-002,003), Copilot CLI plugins/skills (COP-019..027), and OpenCode (OC-CFG-013, OC-AG-009, OC-DEP-005,006)
- **HTTP hook type support** for Claude Code hooks (`type: "http"` with url, headers, allowedEnvVars, timeout fields)
- **Full model ID support** - accept `claude-opus-4-6`, `claude-sonnet-4-6` etc. alongside short aliases in skill and agent frontmatter
- **Cline workflow/hook/skill detection** - file type detection now routes `.clinerules/workflows/`, `.clinerules/hooks/`, and `.clinerules/skills/` to the Cline validator
- **Copilot CLI plugin validation** - new rules for `plugin.json` manifests, CLI SKILL.md, and deprecated `infer` field

### Fixed
- **Spec-drift sentinel workflow** broken since Feb 25 (10+ consecutive failures) - multiline JSON output now uses heredoc syntax
- **CC-HK-001 false positives** - 11 new hook events added to allowlist (PostCompact, InstructionsLoaded, ConfigChange, CwdChanged, FileChanged, TaskCreated, WorktreeCreate, WorktreeRemove, Elicitation, ElicitationResult, StopFailure)
- **CC-HK-004 false positives** - matcher support expanded from 4 to 17 events (SessionStart, SessionEnd, Notification, SubagentStart/Stop, PreCompact, PostCompact, ConfigChange, FileChanged, StopFailure, InstructionsLoaded, Elicitation, ElicitationResult)
- **CC-SK-017 false positives** - `effort`, `paths`, `shell` added to known skill frontmatter fields
- **CC-AG false positives** - `maxTurns`, `effort`, `background`, `isolation`, `initialPrompt`, `mcpServers` added to agent schema
- **CDX-004/CDX-CFG-006 false positives** - 4 config keys added (approvals_reviewer, default_permissions, service_tier, openai_base_url)
- **CDX-CFG-011 false positives** - 3 feature flags added (fast_mode, codex_hooks, smart_approvals)
- **CUR-014 false positive** - subagent `name` and `description` changed from required to optional per Cursor docs
- **OpenCode false positive** - `codesearch` added to known permissions
- **Stale tracking data** - RESEARCH-TRACKING.md updated with current 386-rule inventory, fixed broken URLs (amp.dev, windsurf.com/docs), fixed rule prefix mismatches in spec-baselines.json

### Security
- Bump `picomatch` from 2.3.1 to 2.3.2 in editors/vscode and website (CVE-2026-33671, CVE-2026-33672) (#677).
- Override `serialize-javascript` to ^7.0.5 in website and vscode extension to fix CPU exhaustion DoS (GHSA-qj8w-gfj5-8c6v) (#693).

### Fixed
- Remove non-standard `version` field from SKILL.md frontmatter (XP-SK-001, code scanning alert #1062) (#693).
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **Download URLs**: Updated hardcoded repository path from `avifenesh/agnix` to `agent-sh/agnix` across download scripts, editor extensions, and CI workflows (#676).

## [0.16.4] - 2026-03-23

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **Release workflow**: Create GitHub release as draft before uploading assets, then publish - fixes immutable release protection blocking asset uploads (caused v0.16.3 to ship with zero binaries)

## [0.16.3] - 2026-03-19

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **CC-MEM-006 false positive on bold positive before dash**: Fix `.trim()` stripping trailing space needed for ` - ` separator detection in `has_positive_before` check. Pattern `**Positive action** - Never negative` is now correctly recognized (#661).
- **XP-006 false positive for identical CLAUDE.md and AGENTS.md**: Skip "multiple instruction layers" warning when instruction files have identical content, since they are intentional copies for different tools (#660).
- **CDX-AG-002 false positive on prose words**: Require sensitive keywords like `token`, `secret`, `password` to appear in an assignment context (`=` or `:`) before flagging as a potential secret. Prose like "Token efficiency" no longer triggers (#659).

## [0.16.2] - 2026-03-15

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **XML-001 false positives on Rust type references**: Wrap Rust type references in backticks to prevent XML tag misdetection (#646).
- **Self-validation build**: Use build-from-source for self-validation in CI to ensure agnix validates itself correctly (#646).

### Changed
- **CI self-validation**: Added self-validation step to the CI pipeline so agnix lints its own agent configuration files on every PR/push (#646).

## [0.16.1] - 2026-03-07

### Changed
- **MCP server compatibility**: Updated rmcp dependency from 0.16.0 to 1.1.0 and adapted code for non-exhaustive struct construction using `Default::default()` pattern (#636).

## [0.16.0] - 2026-03-06

### Added
- **Kiro and Codex S-tier alignment (38 new rules)**: Added 22 Kiro rules: `KIRO-010` through `KIRO-014` (missing inclusion mode, steering doc length, duplicate names, conflicting inclusion modes, markdown structure issues), `KR-AG-008` through `KR-AG-013` (agent missing name, agent missing prompt, duplicate tool entries, empty tools array, `toolAliases` references unknown tool, secrets in agent prompt), `KR-HK-007` through `KR-HK-010` (hook timeout out of range, duplicate event handlers, command uses absolute path, secrets in hook command), `KR-MCP-003` through `KR-MCP-005` (missing required args, invalid MCP URL, duplicate MCP server names), and `KR-PW-005` through `KR-PW-008` (step missing description, duplicate keywords, name invalid characters, secrets in power body). Added 16 Codex rules: `CDX-AG-004` through `CDX-AG-007` (AGENTS.md size limit, missing file references, missing project context, contradicts config.toml), `CDX-APP-002` through `CDX-APP-003` (invalid skills configuration, invalid profile configuration), and `CDX-CFG-013` through `CDX-CFG-022` (sandbox_workspace_write mode, model value, model_provider value, model_reasoning_summary value, history configuration, tui configuration, file_opener value, MCP OAuth config, model_context_window value, model_auto_compact_token_limit value). Brings total rule count to 342 (#603).
- **OpenCode expanded coverage (18 new rules)**: Added `OC-DEP-001` through `OC-DEP-004` (deprecation warnings for the `mode`, `tools`, `autoshare`, and `CONTEXT.md` fields), `OC-CFG-008` through `OC-CFG-012` (`logLevel` enum, `compaction.reserved` minimum, `skills` URL array, MCP server timeout, MCP OAuth config), `OC-AG-005` through `OC-AG-008` (`top_p` range, named color enum, redundant `steps`/`maxSteps` pair, `hidden` boolean type), `OC-LSP-001` through `OC-LSP-002` (LSP command requires `extensions` array, `extensions` must be non-empty), and `OC-TUI-001` through `OC-TUI-003` (unknown TUI keys, `scroll_speed` minimum value, `diff_style` enum). Brings total OpenCode rules to 43 and total rule count to 304 (#630).

## [0.15.0] - 2026-03-03

### Added
- **OpenCode validation rules**: Added validation rules for OpenCode configuration (#601).
- **OpenCode coverage follow-up**: Added `OC-CFG-002` (autoupdate value), `OC-CFG-003` (unknown top-level key), and `OC-PM-001` (invalid permission action), plus stricter MCP/agent type checks and localization keys for OpenCode diagnostics (#601).
- **Kiro target support**: Added `Kiro` to `TargetTool` and CLI `--target kiro` handling across CLI, core config, MCP, and LSP target parsing.
- **Kiro config validation parity**: `.agnix.toml` validation now accepts `tools = ["kiro"]` and Kiro-specific disabled rule prefixes (`KIRO-*`, `KR-SK-*`).
- **README supported tools update**: Added a Kiro row to the Supported Tools table with current rule coverage (`KIRO-*`, `KR-SK-*`) and current file surface (`.kiro/steering/**/*.md` and `.kiro/skills/**/SKILL.md`) (#596).
- **Kiro S-tier CI gate**: Added an explicit `Kiro S-tier Gate` step in the main CI test job that executes dedicated Kiro gate checks for target behavior, docs/rule parity integrity, and real-world manifest coverage on every PR/push (#602).
- **Kiro fixture expansion**: Added fixture packs for Kiro powers, agents, hooks, and MCP settings plus integration tests to keep the corpus present and CLI-runnable (#599).
- **Kiro real-world repo coverage**: Added/updated explicit Kiro-tagged real-world repos (`awsdataarchitect/kiro-best-practices`, `dereknguyen269/derek-power`, `cremich/promptz`, `Theadd/kiro-agents`) and tightened CI gating to require the expanded baseline in `tests/real-world/repos.yaml` (#598).
- **Kiro schema foundations**: Added dedicated schema modules for `.kiro/agents/*.json`, `.kiro/hooks/*.kiro.hook`, `POWER.md`, and Kiro MCP configs to support structured parsing with explicit error metadata (#595).
- **Kiro S-tier promotion**: Moved Kiro CLI from B tier to S tier in project memory docs (`CLAUDE.md`/`AGENTS.md`) and added Kiro CI gate coverage to prevent regression (#592).
- **Kiro file type detection expansion**: Added dedicated `KiroPower`, `KiroAgent`, `KiroHook`, and `KiroMcp` file type detection with registry coverage to route these surfaces through the validation pipeline (#594).

### Changed
- **Kiro docs and tests**: Expanded integration/contract tests for Kiro target paths and updated usage docs to clarify legacy `--target` behavior versus `tools = [...]` filtering.
- **Kiro authoring parity follow-up**: Kiro agent completions now emit JSON-formatted insert text (quoted values and JSON key syntax) and fixture docs now explicitly clarify that `KiroMcp` detection currently targets `.kiro/settings/mcp.json` only (#612, #613).

## [0.14.0] - 2026-02-27

### Added
- **`.clinerules/*.txt` file support**: agnix now detects and validates `.clinerules/*.txt` files as `ClineRulesFolder` file type. Rules CLN-001 through CLN-004 now apply to `.txt` files in addition to `.md` files, matching Cline's actual behavior.
- **CDX-006 rule**: Validates `project_doc_fallback_filenames` semantics in Codex CLI configuration (#569).

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **OC-004 config key allowlist expanded**: Added 9 missing top-level keys (`autoshare`, `enterprise`, `layout`, `logLevel`, `lsp`, `mode`, `skills`, `snapshot`, `username`) to `KNOWN_TOP_LEVEL_KEYS` in the OpenCode schema, eliminating false-positive OC-004 warnings for valid opencode.json fields. All 9 OC rules re-verified against current OpenCode source on 2026-02-27.
- **CUR-016 environment.json validation rewritten**: Schema now matches the current Cursor Cloud Agent spec - `install` is required, `terminals` is optional (was previously required), and `build` (with `dockerfile` and `context`) and `update` fields are now validated. Snapshot-based approach replaced with field-level structural validation. Includes 12 new unit tests.
- **CUR-001 to CUR-016 verified_on dates updated**: All 16 Cursor rules re-verified against current Cursor documentation on 2026-02-26.
- **spec-baselines.json expanded**: Baseline entries added for CUR-007 through CUR-016 to enable cross-version regression detection.
- **Zed extension fixes**: Removed mdc language definition per upstream review (#559), added license files (#560), bumped extension version (#562).
- **Updated yanked wasm-bindgen and js-sys dependencies** (#561).

### Changed
- **Unified design system**: Import shared design tokens from agent-sh/design-system. Switch from Outfit to Inter font. Add font preconnect hints to Docusaurus config. Keeps teal accent and light/dark mode.
- **GitHub Copilot rule revalidation (COP-001 to COP-018)**: Refreshed evidence links and guidance for current custom-agent docs, added strict type checks for `infer` (boolean only), and aligned COP schema coverage for custom-agent keys (`name`, `disable-model-invocation`, `user-invocable`, `metadata`) with updated fixtures and generated rule docs (#567).
- **CI**: Added shared CI workflows, Claude Code review, and git hooks (#557).
- **Docs**: Updated Zed extension links to marketplace (#563).

## [0.13.0] - 2026-02-21

### Added
- **XP-008 rule**: New MEDIUM-severity cross-platform rule that warns when CLAUDE.md contains Claude-specific directives (context:fork, agent fields, allowed-tools, hooks, @import) outside a guarded `## Claude Code` section, helping users targeting Cursor avoid silently-ignored configuration

## [0.12.4] - 2026-02-21

### Security
- **Fix minimatch ReDoS vulnerability**: Added npm `overrides` in `website/package.json` to force `minimatch@^10.2.1`, resolving Dependabot alert #75 (ReDoS via repeated wildcards in `serve-handler`'s transitive `minimatch@3.1.2` dependency)
- **Update wasm-bindgen**: Bumped `wasm-bindgen` from 0.2.109 (yanked) to 0.2.110 along with related crates (`js-sys`, `web-sys`, `wasm-bindgen-test`, etc.)

### Performance
- **WASM conversion optimization**: Refactored `WasmDiagnostic::from_diagnostic` to take ownership of `Diagnostic` and its fields, eliminating unnecessary string cloning when converting diagnostics to the WASM-compatible representation in `agnix-wasm`

### Added
- **LSP `pub(crate)` testability refactor**: Made internal modules (`backend`, `code_actions`, `completion_provider`, `diagnostic_mapper`, `hover_provider`, `position`, `vscode_config`) and `Backend` struct fields/methods `pub(crate)` to enable crate-internal test access. Added `Backend::new_test()` constructor (gated behind `#[cfg(test)]`) and 18 new tests in `testability_tests.rs` verifying that all promoted `pub(crate)` items are accessible from the crate root. No public API changes.
- **Improved frontmatter parsing test coverage**: Added exhaustive unit tests for `frontmatter_value_byte_range`, `frontmatter_key_offset`, and `frontmatter_key_line_byte_range` in `agnix-core`. Covers unquoted/quoted values, comments, CRLF endings, indented keys, and malformed input.
- **`build_lenient()` on `LintConfigBuilder`**: New builder terminal that runs security-critical glob pattern validation (syntax, path traversal, absolute paths) while skipping semantic warnings such as unknown tool names and deprecated field warnings. Intended for embedders that accept future or unknown tool names without rebuilding. `ConfigError::AbsolutePathPattern` variant added for absolute-path glob patterns (#475)
- **Expanded autofix coverage**: Added `with_fix()` autofix support to 38 additional validation rules across AGM, AMP, AS, CC-AG, CC-HK, CC-PL, CC-SK, CDX, COP, CUR, GM, KIRO, MCP, OC, PE, and REF categories, bringing total fixable rules from 59 to 97 (42% of all rules)
- **Kiro steering file validation**: 4 new validation rules (KIRO-001 through KIRO-004) for `.kiro/steering/*.md` files - validates inclusion modes (`always`, `fileMatch`, `manual`, `auto`), required companion fields, glob pattern syntax, and empty file detection
- **Cross-platform and reference validation expansion**: 5 new rules - XP-007 (AGENTS.md exceeds Codex CLI 32KB byte limit), REF-003 (duplicate @import detection), REF-004 (non-markdown @import warning), PE-005 (redundant LLM instructions), PE-006 (negative instructions without positive alternatives)
- **Roo Code support**: 6 new validation rules (ROO-001 through ROO-006) for `.roorules`, `.roomodes`, `.rooignore`, `.roo/rules/*.md`, `.roo/rules-{slug}/*.md`, and `.roo/mcp.json` configuration files
- **Cursor expanded coverage**: Added 7 new validation rules (CUR-010 through CUR-016) for `.cursor/hooks.json`, `.cursor/agents/**/*.md`, and `.cursor/environment.json`, including stricter field validation and case-insensitive path detection.
- **Windsurf support**: Added 4 validation rules (WS-001 through WS-004) for `.windsurf/rules/*.md` and `.windsurf/workflows/*.md` directories, plus legacy `.windsurfrules` detection. Includes file type detection, character limit enforcement (12,000), and empty file warnings.
- **Gemini CLI expanded coverage**: Added 6 new validation rules (GM-004 through GM-009) for .gemini/settings.json hooks configuration, gemini-extension.json manifests, and .geminiignore files. Added 3 new file type detectors and validators.
- **Codex CLI expanded validation**: CDX-004 (unknown config keys), CDX-005 (`project_doc_max_bytes` exceeds 65536 limit); updated CDX source_urls to official docs
- **OpenCode expanded validation**: OC-004 (unknown config keys), OC-006 (remote instruction URL timeout warning), OC-007 (invalid agent definition), OC-008 (invalid permission configuration), OC-009 (variable substitution syntax validation)
- **`agnix-wasm` crate**: New WebAssembly bindings for the validation engine, enabling browser-based validation without a server
- **`validate_content()` API**: New pure function in `agnix-core` for validating content strings without filesystem I/O
- **`filesystem` feature flag**: `agnix-core` now gates filesystem-dependent code (`rayon`, `ignore`, `dirs`) behind a `filesystem` feature (enabled by default), allowing WASM compilation with `default-features = false`
- **`agnix-core` std requirement documentation**: Added crate-level documentation in `lib.rs`, `Cargo.toml`, and `README.md` clarifying that `agnix-core` requires `std` unconditionally and that the `filesystem` feature flag does not enable `no_std` support. Resolves downstream confusion for WASM consumers using `default-features = false` (#485)
- **Web playground UI polish**: Teal gradient background, staggered animations, panel shadows, focus glow, SVG icons, active preset state, empty state with checkmark, loading spinner, `prefers-reduced-motion` support
- **Inline editor diagnostics**: Red/yellow/teal wavy underlines via `@codemirror/lint`, gutter markers, hover tooltips with rule ID and message
- **Auto-fix in playground**: WASM now exposes `Fix` data; per-diagnostic "Fix" buttons and "Fix all" button apply replacements directly in the editor
- **New playground presets**: AGENTS.md, `.claude/agents/reviewer.md`, `plugin.json`; enriched `.claude/settings.json` hooks preset
- **Backend revalidation regression tests**: Added coverage for `did_save` project-trigger revalidation and stale generation guard behavior in `agnix-lsp` backend tests
- **Confidence-tiered autofix engine**: `Fix` metadata now supports confidence, alternative groups, and dependencies; CLI adds `--fix-unsafe` and `--show-fixes`; core exposes confidence-based `FixApplyMode`/`FixApplyOptions`
- **CI crate graph parity test**: New workspace-level test validates that all `Cargo.toml` workspace members are documented in CLAUDE.md, AGENTS.md, README.md, SPEC.md, and CONTRIBUTING.md - prevents architecture-doc drift
- **`resolve_validation_root` file-input tests**: 7 integration tests covering single-file validation mode - validates file-input path behavior, unknown file type handling, project-level rule scoping, and nonexistent file edge case (#450)
- **`ImportsValidator` concurrency and multi-file cycle tests**: 11 new tests covering thread-safety under concurrent validation, multi-file import cycles (3- and 4-file chains), depth boundary conditions at and below `MAX_IMPORT_DEPTH` (complementing existing above-boundary coverage), diamond dependency graphs, and mixed valid/invalid import scenarios (#456)
- **UTF-8 boundary `_checked` Fix constructors**: Added 6 new `Fix` constructor variants (`replace_checked`, `replace_with_confidence_checked`, `insert_checked`, `insert_with_confidence_checked`, `delete_checked`, `delete_with_confidence_checked`) that accept `content: &str` and validate UTF-8 char boundary alignment via `debug_assert!` in debug builds - no-ops in release builds (#463)
- **LSP concurrent revalidation stress tests**: 8 new stress tests covering concurrent document open/close cycles, rapid config changes dropping stale batches, concurrent changes to the same document, config change during active validation, concurrent project and per-file validation, high document count revalidation after a single config change, concurrent hover requests during active validation, and rapid project validation generation guard behavior (#458)
- **`MAX_REGEX_INPUT_SIZE` precise boundary tests**: 27 tests covering the exact 65536-byte limit for all 12 guarded regex functions across `markdown.rs`, `prompt.rs`, and `cross_platform.rs` - each function gets an at-limit (processed) and one-byte-over (rejected) test; also confirms `extract_imports` and `extract_markdown_links` are unrestricted (byte-scan/pulldown-cmark, not regex) (#457)

### Changed
- **API**: Removed `#[non_exhaustive]` from `ValidationResult` struct - all fields are public and the attribute was unnecessarily preventing struct literal construction and exhaustive destructuring outside the crate (#487)
- **`CoreResult` type alias removed** (breaking): `CoreResult<T>` has been removed from the public API. Use `LintResult<T>` (i.e., `Result<T, LintError>`) instead. `LintError` is a public alias for `CoreError`; both remain exported. (#477)
- **`__internal` module feature-gated**: The `__internal` module in `agnix-core` is now behind the `__internal` Cargo feature; it was previously unconditionally public which created semver obligations for internal items (#472)
- **`normalize_line_endings` promoted to stable public API**: Accessible at the crate root (`agnix_core::normalize_line_endings`) without requiring the `__internal` feature (#472)
- **Project-level validation extracted to `rules/project_level.rs`**: Extracted `run_project_level_checks`, `join_paths`, and associated unit tests from `pipeline.rs` into a new `rules/project_level.rs` module; adds 7 new unit tests for AGM-006, XP-004/005/006, and VER-001 behaviors (#474)
- **`build_unchecked()` scoped to test/internal use**: `LintConfigBuilder::build_unchecked()` is now gated behind `#[cfg(any(test, feature = "__internal_unchecked"))]` and marked `#[doc(hidden)]`. External embedders should migrate to `build_lenient()`. The `__internal_unchecked` feature in `agnix-core` is available for integration tests that construct intentionally-invalid configs (#475)
- **Core refactor**: Replaced the `DEFAULTS` const array in `registry.rs` with 8 private category `ValidatorProvider` structs. Public API (`ValidatorRegistry`, `ValidatorRegistryBuilder`, `with_defaults()`) is unchanged; this is an internal reorganization only.
- **`validate_file` / `validate_file_with_registry` return type** (breaking): Both functions now return `LintResult<ValidationOutcome>` instead of `LintResult<Vec<Diagnostic>>`. `ValidationOutcome` is a `#[non_exhaustive]` enum with three variants: `Success(Vec<Diagnostic>)` (validation ran), `IoError(FileError)` (`filesystem` feature only - file could not be read), and `Skipped` (unknown file type, no validation performed). The `Err` path is now reserved exclusively for config-level errors. Use `into_diagnostics()` for a quick migration path that matches the old flat `Vec<Diagnostic>` behavior (#466)
- **Docs**: Updated architecture references in README.md, SPEC.md, CLAUDE.md, and AGENTS.md to explicitly include the `agnix-wasm` workspace crate
- **Core refactor**: Split oversized `crates/agnix-core/src/config.rs` into focused submodules (`builder`, `rule_filter`, `schema`, `tests`) while preserving the stable `config` API
- **LSP refactor**: Split oversized `crates/agnix-lsp/src/backend.rs` into focused submodules (`events`, `helpers`, `revalidation`, `tests`) while preserving `Backend` behavior and public exports
- **`named_validators()` invariant documentation and debug guard**: Expanded `ValidatorProvider::named_validators()` doc comment to document the name/factory invariant - each `Some(name)` must equal `factory().name()` or the disabled-validator mechanism silently misbehaves. Added `debug_assert_eq!` inside `register_named()` to catch mismatches early in debug builds. Added 4 tests covering the debug panic, silent-skip, and slip-through failure modes (#501)
- **Targeted `#[allow(dead_code)]` in parsers and schemas**: Replaced blanket `#![allow(dead_code)]` module attributes in `agnix-core` parsers and schemas modules with per-item allows on the specific fields and variants that require them. Narrows lint suppression scope, making future dead-code regressions visible at the item level. No public API changes (#484)

### Performance
- **ValidatorRegistry instance caching**: Registry now stores pre-constructed `Box<dyn Validator>` instances instead of factories, eliminating per-file validator re-instantiation. `validators_for()` returns `&[Box<dyn Validator>]` (borrowed slice) instead of `Vec<Box<dyn Validator>>`. Added `total_validator_count()` method; `total_factory_count()` is deprecated and will be removed in a future release. The `Validator` trait now requires `Send + Sync + 'static` bounds to allow safe sharing via `Arc<ValidatorRegistry>` (#460)
- **REF-002 link validation**: Hoisted loop-invariant `canonicalize()` call out of per-link loop in `validate_markdown_links()` - eliminates N-1 redundant filesystem syscalls when validating N markdown links
- **ValidatorRegistry memory efficiency**: Replaced `String` with `&'static str` for validator names, eliminating per-validator heap allocations during registry construction. Added `disable_validator_owned()` variants for runtime string disabling with duplicate detection to prevent unnecessary memory leaks
- **Instruction file detection**: Rewrote `is_instruction_file()` to use allocation-free path component iteration and `eq_ignore_ascii_case`, eliminating 2 heap allocations per file during project validation walks
- **Parallel validation fold**: Eliminated PathBuf clone on error path in parallel fold by moving the owned value into the diagnostic
- **LSP lock-free config reads**: Replaced `Arc<RwLock<Arc<LintConfig>>>` with `Arc<ArcSwap<LintConfig>>` in LSP backend, eliminating read lock contention on every `did_change`/`did_open`/`did_save` event (#468)
- **Disabled-validator fast path**: Added `named_validators()` to `ValidatorProvider` trait (default impl wraps `validators()` with `None` names). Providers that override it with `Some(name)` allow `ValidatorRegistryBuilder` to skip the factory call entirely for disabled validators, avoiding the allocation. Built-in validators use the fast path automatically (#461)
- **`LintConfig` cheap cloning**: Introduced `Arc<ConfigData>` inner struct to hold all serializable fields. Cloning a `LintConfig` (e.g., in `validate_project` / `validate_project_with_registry` parallel dispatch) now bumps an `Arc` refcount instead of deep-copying `Vec<String>` fields and nested structs. Mutations use `Arc::make_mut` for copy-on-write semantics, so the allocation only occurs when the `Arc` is actually shared (#467)

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **`resolve_validation_root` silent fallback removed**: Passing a nonexistent path to `validate_project()` or `validate_project_with_registry()` now returns `Err(CoreError::Validation(ValidationError::RootNotFound { path }))` immediately instead of silently falling back to the current working directory. The CLI exits with code 1 and prints `"Validation root not found: <path>"` to stderr. Added `ValidationError::RootNotFound` variant and extended `CoreError::path()` to cover it (#483)
- **LSP document version tracking**: The LSP backend now tracks document versions reported by the client (`did_open`, `did_change`) and includes them in all `publish_diagnostics` calls. Editors that inspect diagnostic version tags (e.g., for stale-result suppression) now receive accurate version numbers instead of `None`. Version and content updates are atomized under a single lock acquisition so readers never observe a state where content and version are out of sync. Empty `did_change` notifications (no content changes) also correctly advance the tracked version per the LSP spec (#478)
- **Frontmatter leading newline stripped**: `split_frontmatter()` no longer includes the newline that follows the opening `---` delimiter in the extracted frontmatter string. Downstream validators (`AgentValidator`, `AmpValidator`, `KiroSteeringValidator`) have been updated to compute correct 1-based line numbers; diagnostic line numbers for AMP-001, CC-AG-007, and KIRO-001 through KIRO-004 are now accurate (#482)
- **Empty-frontmatter panic guard**: `split_frontmatter()` now uses `str::get()` instead of direct slice indexing when extracting frontmatter content, preventing an index-out-of-bounds panic on files with an opening `---` delimiter but no content (#482)
- **Predictable UUID Generation for Telemetry**: Replaced the custom, insecure random number generator with a cryptographically secure pseudo-random number generator (CSPRNG) using the `uuid` crate. Ensures telemetry installation IDs are unpredictable and unique.
- **`ImportsValidator` poisoned-lock recovery**: `ImportsValidator` now emits a `lint::cache-poison` `Warning` diagnostic (with i18n message and suggestion in en/es/zh-CN) when the shared `ImportCache` `RwLock` is poisoned by a prior validator panic, rather than panicking or silently dropping data. Validation continues with the recovered cache state. Deduplicated with `push_unique_diagnostic` to avoid one diagnostic per import. Includes 4 new tests covering detection, deduplication, continued import validation, and recursive-tree deduplication (#481)
- **`Fix` constructor range assertions**: Added `debug_assert!(start <= end)` to `Fix::replace`, `Fix::replace_with_confidence`, `Fix::delete`, and `Fix::delete_with_confidence` to catch inverted byte ranges in debug builds (#463)
- **CRLF line ending normalization**: `normalize_line_endings()` is now applied at all pipeline entry points (`validate_file_with_type`, `validate_content`, `run_project_level_checks`) and in the fix engine (`apply_fixes_with_fs_options`). Windows files with CRLF endings produce identical diagnostics and byte-accurate auto-fixes as their LF equivalents. Files written by `--fix` use LF endings (#480)
- **`validate_file_with_registry` disabled-validator gap**: `config.rules().disabled_validators` was silently ignored in the `validate_file_with_type` path (used by `validate_file_with_registry` and `validate_project_with_registry`). Validators now respect `disabled_validators` at runtime in all code paths, consistent with `validate_content()` (#469)
- **REF-001**: Corrected metadata to reflect universal applicability across all tools (not claude-code specific), changed source_type to community, and added agentskills.io reference
- **CC-HK-001**: Added `TeammateIdle` and `TaskCompleted` as valid hook event names
- **CC-AG-004**: Added `delegate` as a valid permission mode for Claude Code agents
- **CC-HK-002**: Expanded PROMPT_EVENTS to include all 8 officially supported events (Stop, SubagentStop, PreToolUse, PostToolUse, PostToolUseFailure, PermissionRequest, UserPromptSubmit, TaskCompleted) per Claude Code documentation, fixing false positives for prompt/agent hooks on previously-valid events
- **Playground editor not initializing**: `loading` state was missing from CodeMirror `useEffect` dependency array, so the editor never mounted after WASM loaded
- **Blue flash on playground load**: Changed editor pane background from `--ag-code-bg` to neutral `--ag-surface-raised`
- **Autofix dependency/group edge cases**: Dependency checks now consider only structurally applicable fixes, and grouped alternatives now fall back correctly when an earlier candidate is eliminated
- **MCP-008**: Updated default MCP protocol version from `2025-06-18` to `2025-11-25` to align with the latest specification
- **CC-HK-003**: Downgraded from Error to Info level - matcher field is optional for tool events, not required; omitting it matches all tools (best practice hint, not an error)
- **SARIF artifact URIs**: Now uses git repository root as base path instead of current working directory, ensuring correct IDE file navigation for SARIF output. Falls back to CWD when scan path is not inside a git repository (#488)
- **CI**: Added `defaults.run.shell: bash` and `set -euo pipefail` to all 9 workflow files for consistent shell behavior and early error detection; `GITHUB_OUTPUT` redirects in `release.yml` are now consistently quoted (#465)
- **CI**: Moved `VSCE_PAT` from CLI argument to environment variable in VS Code extension publish step, preventing secret exposure in process list (#464)
- **MCP server error codes**: `validate_file` and `validate_project` tools now return `invalid_params` (JSON-RPC -32602) instead of `internal_error` (-32603) for user-supplied path validation failures, correctly distinguishing client errors from server faults. Renamed internal `make_error` helper to `make_internal_error` for clarity (#462)
- **MCP `tools` input schema**: `ToolsInput` now uses a manual `JsonSchema` impl that emits `anyOf` with the array variant first and `inline_schema = true`, so MCP clients see the array-preferred `anyOf` directly at each property site instead of a `$ref` to `$defs`. Removed the standalone `schemars` 0.8 dependency; tests now use `rmcp::schemars` (v1) directly (#479)
- **Invalid glob pattern diagnostics**: Invalid `[files]` glob patterns in `.agnix.toml` are now surfaced as `Warning` diagnostics (rule `config::glob`) in the validation output instead of writing to stderr. `markdown.rs` panic recovery paths also no longer write to stderr; they return empty results silently with a source comment (#459)

## [0.11.1] - 2026-02-11

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- **CI**: Release workflow now explicitly builds binary crates (`-p agnix-cli -p agnix-lsp -p agnix-mcp`) to prevent cache-related build skips
- **CI**: Release version check now reads from `[workspace.package]` instead of root `[package]` section

## [0.11.0] - 2026-02-11

### Added
- **Builder pattern for LintConfig**: `LintConfig::builder()` with validation on `build()`. All serializable fields are now private with getter/setter methods. `ConfigError` enum for build-time validation failures. Runtime state (`root_dir`, `import_cache`) moved into `RuntimeContext`
- **RUSTSEC advisory tracking** - Documented process for reviewing ignored security advisories with `docs/RUSTSEC-ADVISORIES.md` tracking document, monthly review checklist in `MONTHLY-REVIEW.md`, and pre-release checks in `RELEASING.md` (closes #346)
- **Structured rule metadata in diagnostics** - All diagnostic outputs (JSON, SARIF, MCP, LSP, CLI) now include optional metadata fields: category, rule_severity, and applies_to_tool. Metadata is automatically populated from rules.json at build time
- **Plugin architecture**: `ValidatorProvider` trait enables external validator registration
- **Builder pattern**: `ValidatorRegistry::builder()` for ergonomic registry construction with `with_defaults()`, `with_provider()`, `without_validator()`
- **Validator disabling**: `disabled_validators` config field in `[rules]` section to disable validators by name at runtime
- **Validator naming**: `Validator::name()` method for programmatic identification of validators
- **Validator introspection**: `Validator::metadata()` method returns rule IDs and descriptions for runtime validator inspection
- **Hierarchical error types** - New `CoreError` enum with `File(FileError)`, `Validation(ValidationError)`, `Config(ConfigError)` variants provides structured error information. Helper methods `path()` and `source_diagnostics()` enable better error introspection. `LintError` remains as type alias for backward compatibility
- **Backward-compatibility policy** documenting public vs. internal API surface with three stability tiers (CONTRIBUTING.md)
- **Cross-crate API contract tests** ensuring stable interfaces between agnix-core, agnix-rules, and downstream crates (CLI, LSP, MCP)
- **Feature flags policy** documenting when and how to use feature flags
- **Clickable rule links in IDEs** - LSP diagnostics now include `code_description` so rule codes (e.g. AS-001) link to per-rule website docs
- **Explicit code action kinds** - LSP advertises QUICKFIX capability for more reliable quick-fix surfacing
- **Per-rule examples for all 155 rules** - Each rule now has specific good/bad examples in `rules.json` and on the website, replacing generic category-level stubs
- **LSP project-level validation** - `validate_project_rules()` public API for workspace-wide rules (AGM-006, XP-004/005/006, VER-001)
- **LSP lifecycle integration** - project-level diagnostics on workspace open, file save, config change
- **VS Code `validateWorkspace`** - now triggers `agnix.validateProjectRules` executeCommand
- **Dependabot** config for automated cargo and GitHub Actions dependency updates
- **MSRV** defined as Rust 1.91 (latest stable), tested in CI matrix
- **70+ new tests** covering diagnostics, config versions, LSP backend, MCP errors, parsers, schemas, span_utils, eval edge cases

### Changed
- **Refactoring**: Extracted `file_types.rs` into extensible `file_types/` module directory with `FileTypeDetector` trait, `FileTypeDetectorChain`, named constants, `Display` impl, and `is_validatable()` method (#349)
- **Refactoring**: Split `crates/agnix-core/src/lib.rs` into focused modules: `file_types.rs`, `registry.rs`, `pipeline.rs`
- **Error handling**: Replaced flat `LintError` enum with hierarchical `CoreError` structure, preserving error context through conversion layers. Binary crates (CLI, LSP, MCP) gain automatic `anyhow::Error` conversion via thiserror
- All rule documentation links now point to website (`avifenesh.github.io/agnix`) instead of GitHub `VALIDATION-RULES.md`
- README overhauled to focused landing page with punchy value prop and website links
- **API (BREAKING)**: Made `parsers` module internal and moved `#[doc(hidden)]` re-exports to `__internal` module (closes #350)
- **API (BREAKING)**: Marked `ValidationResult` as `#[non_exhaustive]` - use `ValidationResult::new()` or `..` in patterns
- **API (BREAKING)**: Renamed `ValidationResult.rules_checked` to `validator_factories_registered` for accuracy
- **API**: Added stability tier documentation (Stable/Unstable/Internal) to all public modules
- **API**: Added metadata fields to `ValidationResult`: `validation_time_ms` and `validator_factories_registered`
- **API**: Use saturating cast for validation timing (prevents u128 truncation to u64)

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation
- i18n diagnostic messages now display properly translated text instead of raw key paths when installed via `cargo install` (fixes #341)
- CI locale-sync check prevents locale files from drifting across crates
- CC-AG-009, CC-AG-010, CC-SK-008 false positives for `Skill`, `StatusBarMessageTool`, `TaskOutput` tools and MCP server tools with `mcp__<server>__<tool>` format (fixes #342)
- **Performance**: Replaced Mutex-based path collection with rayon fold/reduce in parallel validation, eliminating lock contention
- **Performance**: Reduced string allocations in `normalize_rel_path`, `detect_file_type`, and project-level checks
- **Code quality**: Merged duplicate `resolve_config_path` functions in CLI
- **Code quality**: Improved regex error messages in hooks validator
- **Code quality**: Added panic-safe `EnvGuard` for telemetry test isolation
- **Code quality**: Added panic logging in markdown parser instead of silent failure
- **CI**: Pinned `huacnlee/zed-extension-action` to SHA, pinned cargo tool versions
- **CI**: Moved `CARGO_REGISTRY_TOKEN` from CLI args to env vars in release workflow

## [0.10.2] - 2026-02-08

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- VS Code extension version was out of sync with release binaries, causing download failures for agnix-lsp

## [0.10.1] - 2026-02-07

### Added

- **Per-client skill validation** - 10 new rules detect when SKILL.md files in client-specific directories use unsupported frontmatter fields: CR-SK-001 (Cursor), CL-SK-001 (Cline), CP-SK-001 (Copilot), CX-SK-001 (Codex), OC-SK-001 (OpenCode), WS-SK-001 (Windsurf), KR-SK-001 (Kiro), AMP-SK-001 (Amp), RC-SK-001 (Roo Code), XP-SK-001 (cross-platform portability)

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- Markdown structure validation now skips headers inside fenced code blocks
- Flaky telemetry env-dependent tests serialized with mutex
- Clippy warnings in span_utils test assertions

## [0.10.0] - 2026-02-07

### Performance

- **Auto-fix span finding** - Replaced 8 dynamic `Regex::new()` calls with byte-level scanning in auto-fix helpers, eliminating regex compilation overhead entirely (closes #325)

### Added

- **Website automation** - `generate-docs-rules.py` now generates `website/src/data/siteData.json` with dynamic stats (rule count, category count, autofix count, tool list); landing page and JSON-LD import generated data instead of hardcoding; `release.yml` `version-docs` job auto-cuts versioned docs on release
- **GEMINI.md categorization** - `categorize_layer()` now recognizes `GEMINI.md` and `GEMINI.local.md` files as `LayerType::GeminiMd` for accurate XP-006 layer categorization
- **Codex CLI support** - 3 new validation rules (CDX-001, CDX-002, CDX-003) for `.codex/config.toml` configuration files
- **Cline support** - 3 new validation rules (CLN-001, CLN-002, CLN-003) for `.clinerules` configuration
- **OpenCode support** - 3 new validation rules (OC-001, OC-002, OC-003) for `opencode.json` configuration
- **GEMINI.md support** - 3 new validation rules (GM-001, GM-002, GM-003) for `GEMINI.md` files
- CC-HK-013: `async` field only valid on command hooks (error)
- CC-HK-014: `once` field only meaningful in skill/agent frontmatter (warning)
- CC-HK-015: `model` field only valid on prompt/agent hooks (warning)
- CC-HK-016: Unknown hook type validation, recognizes `agent` type (error)
- CC-HK-017: Prompt/agent hooks missing `$ARGUMENTS` reference (warning)
- CC-HK-018: Matcher on `UserPromptSubmit`/`Stop` events silently ignored (info)
- CC-AG-008: Validate `memory` scope is `user`, `project`, or `local`
- CC-AG-009: Validate tool names in agent `tools` list
- CC-AG-010: Validate tool names in agent `disallowedTools` list
- CC-AG-011: Validate `hooks` object schema in agent frontmatter
- CC-AG-012: Warn on `permissionMode: bypassPermissions` usage
- CC-AG-013: Validate skill name format in agent `skills` array
- MCP-009: Validate `command` is present for stdio MCP servers (HIGH)
- MCP-010: Validate `url` is present for http/sse MCP servers (HIGH)
- MCP-011: Validate MCP server `type` is one of stdio, http, sse (HIGH)
- MCP-012: Warn when SSE transport is used (deprecated in favor of Streamable HTTP) with auto-fix (MEDIUM)
- CC-SK-010: Validate hooks in skill frontmatter follow settings.json schema
- CC-SK-011: Detect unreachable skills (user-invocable=false + disable-model-invocation=true)
- CC-SK-012: Warn when argument-hint is set but body never references $ARGUMENTS
- CC-SK-013: Warn when context=fork is used with reference-only content (no imperative verbs)
- CC-SK-014: Validate disable-model-invocation is boolean, not string "true"
- CC-SK-015: Validate user-invocable is boolean, not string "true"/"false"
- CC-PL-007: Validate component paths are relative (no absolute paths or `..` traversal) with safe auto-fix (HIGH)
- CC-PL-008: Detect component paths pointing inside `.claude-plugin/` directory (HIGH)
- CC-PL-009: Validate `author.name` is non-empty when author object present (MEDIUM)
- CC-PL-010: Validate `homepage` is a valid http/https URL when present (MEDIUM)
- COP-005: Validate `excludeAgent` field contains valid agent identifiers
- COP-006: Warn when global Copilot instruction file exceeds ~4000 characters
- CUR-007: Warn when `alwaysApply: true` is set alongside `globs` (redundant) with safe auto-fix
- CUR-008: Detect `alwaysApply` as quoted string instead of boolean (HIGH)
- CUR-009: Warn when agent-requested rule has no description
- CC-MEM-011: Validate `.claude/rules` frontmatter `description` field
- CC-MEM-012: Validate `.claude/rules` frontmatter `globs` field format
- Fix metadata (`autofix`, `fix_safety`) for all rules in rules.json
- Fix metadata schema validation parity test
- Autofix count parity test (rules.json vs VALIDATION-RULES.md)
- Context-aware completions documented in all editor READMEs
- `--fix-safe` flag documented in README.md usage section
- `[files]` configuration section for custom file inclusion/exclusion patterns
  - `include_as_memory` glob patterns validate files as CLAUDE.md-like instruction files
  - `include_as_generic` glob patterns validate files as generic markdown
  - `exclude` glob patterns skip files from validation entirely
  - Priority: exclude > include_as_memory > include_as_generic > built-in detection

### Changed

- **Actionable diagnostic suggestions** - All parse error diagnostics now include actionable suggestions (AS-016, CC-HK-012, MCP-007, CC-AG-007, CC-PL-006, CDX-000, file-level errors); improved 4 generic suggestions with concrete guidance (MCP-003 lists valid JSON Schema types, MCP-006 warns about self-reported annotations, AGM-001/GM-001 specify common markdown issues)
- **Website landing page** - Updated stats (145 rules, 2400+ tests, 10+ tools), added Cline/OpenCode/Gemini CLI/Roo Code/Kiro CLI to tools grid, visual redesign with Outfit font, syntax-highlighted terminal, scroll reveal animations, and open-ended "And many more" tool card
- Auto-fix implementations added for 8 rules: CC-SK-011 (unsafe), CC-HK-013 (safe), CC-HK-015 (safe), CC-HK-018 (safe), CUR-008 (safe), COP-005 (unsafe), CC-AG-008 (unsafe), MCP-011 (unsafe)
- Auto-fix pack 2: 8 additional rules with unsafe auto-fixes: CC-SK-005, CC-AG-012, CUR-002, COP-002, CDX-001, CDX-002, OC-001, CC-HK-016
- Auto-fix table in VALIDATION-RULES.md expanded from 7 to 48 rules with safety classification
- Auto-fixable count updated to 48 rules (33%)
- Generated website rule pages now include Auto-Fix metadata
- Website rules index table includes Auto-Fix column
- `generate-docs-rules.py` renders fix metadata with strict validation
- Collapsed nested `if` patterns using Rust let-chains (stable since 1.87), removing stale `#[allow(clippy::collapsible_if)]` annotations
- Moved `#[allow(dead_code)]` from module-level to method-level in telemetry stub for precision

## [0.9.3] - 2026-02-06

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- VS Code extension now probes PATH binaries with `--version` and prefers up-to-date downloaded binary over outdated system installations
- Version check handles pre-0.9.2 agnix-lsp binaries without `--version` support
- Reordered `findLspBinary()` to prefer the downloaded binary when its version marker matches, skipping the `--version` probe on subsequent restarts

## [0.9.2] - 2026-02-06

### Added

- `agnix-lsp --version`/`-V` flag for debugging

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- VS Code and JetBrains plugins now auto-update LSP binary when plugin version changes
- Plugin writes `.agnix-lsp-version` marker file to detect version mismatches
- GitHub release URLs use versioned paths instead of `/latest/` for reliable downloads

## [0.9.1] - 2026-02-06

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- CC-MEM-006: Detect positive alternatives after negatives ("NEVER X - always Y" no longer false positive)
- PE-004: Skip ambiguous terms inside parentheses (descriptive text no longer flagged)
- CC-AG-007: Humanize YAML parse errors ("expected a YAML list" instead of "expected a sequence")
- MCP-002: Suggest `parameters` -> `inputSchema` when field exists under wrong name
- VS Code marketplace image now bundled in extension package
- Exclude DEVELOPER.md and 11 other developer-focused files from validation

### Added

- JetBrains plugin auto-publish in release workflow
- Zed extension auto-publish via zed-extension-action
- All editor extension versions now auto-synced from Cargo.toml on release

## [0.9.0] - 2026-02-06

### Changed

- Validated against 1,200+ real-world repositories with 71 rules triggered
- Exclude non-agent markdown files (README.md, docs/, wiki/) from validation
- Restrict REF-002 broken link detection to agent config files only
- Skip HTML5 void elements and markdown-safe elements in XML balance checking
- Resolve @imports relative to project root when file-relative fails
- Apply prompt quality rules (CC-MEM-005/006, PE-\*) to Cursor rule files
- Detect .cursorrules.md as Cursor rules variant
- Flag `|| true` and `2>/dev/null` as error suppression in hooks (CC-HK-009)
- Broaden persona detection in CC-MEM-005 ("You're a senior...")
- Add PCRE assertions to AS-014 regex escape detection
- Fix %% formatting in diagnostic messages across all locales
- Reduce false positive rate from ~30% to <3% across XML, REF, and XP rules
- Skip type parameters and path template placeholders in XML validation
- Filter email domains, Java annotations, and social handles from @import detection

### Added

- `docs/RELEASING.md` - Release process guide with install target verification
- `docs/REAL-WORLD-TESTING.md` - Real-world validation and manual inspection guide
- `scripts/real-world-validate.py` - Batch validation harness
- `tests/real-world/repos.yaml` - Curated manifest of 1,236 repos
- Regression test fixtures for HTML5 void elements, type parameters, and absolute paths

## [0.8.1] - 2026-02-06

### Added

- Authoring metadata and completion system (`authoring` module) with context-aware suggestions and hover docs for all config file types
- LSP completion provider with intelligent key/value/snippet suggestions
- Auto-fix support across validators: skills (AS-005, AS-006, CC-SK-001, CC-SK-003, CC-SK-005), agents (CC-AG-003, CC-AG-004), hooks (CC-HK-011), plugins (CC-PL-005), MCP (MCP-001)
- Safety tagging for all auto-fixes (safe vs unsafe)

### Changed

- LSP hover provider simplified by delegating to `agnix_core::authoring` module
- Agent and skill validators now use `split_frontmatter()` directly for better error location and fix generation

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- CC-AG-007 parse error diagnostics now report the actual error line/column instead of always line 1

## [0.8.0] - 2026-02-06

### Added

- Real-world validation harness (`scripts/real-world-validate.py`) with 121 curated repos (`tests/real-world/repos.yaml`) (#184)
- XP-001: detect `@import` syntax in AGENTS.md files (Claude Code specific)
- XP-003: detect OS-specific absolute paths (`/Users/`, `/home/`, `~/Library/`, `~/.config/`)
- CC-MEM-005: detect role-play preambles and generic programming principles

### Changed

- Exclude non-agent markdown files from validation (README.md, CONTRIBUTING.md, docs/, wiki/, etc.) to reduce false positives by 57%
- Agent directory files (`agents/*.md`) take precedence over filename exclusions

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- Operator precedence bug in `@import` email filtering that incorrectly matched email addresses
- Zed editor extension with automatic LSP binary download and MDC file type support (#198)
- Documentation website pipeline (#195)
  - Added Docusaurus website under `website/` with versioned docs and local search
  - Added rule-doc generation from `knowledge-base/rules.json` via `scripts/generate-docs-rules.py`
  - Added docs parity test (`crates/agnix-cli/tests/docs_website_parity.rs`) and CI workflow (`.github/workflows/docs-site.yml`)
- CI: code coverage reporting with cargo-llvm-cov and Codecov integration (#238)
- JetBrains plugin: archive extraction tests for AgnixBinaryDownloader (#255)
  - 19 tests covering TAR.GZ/ZIP extraction, binary selection, path traversal protection
  - Refactored extraction methods to companion object for testability
  - Switched path verification to `java.nio.file.Path` API for robustness
- Internationalization (i18n) support with rust-i18n (#207)
  - Support for multiple languages: English (en), Spanish (es), Chinese Simplified (zh-CN)
  - CLI flag `--locale` to set output language
  - CLI flag `--list-locales` to display available locales
  - Environment variable `AGNIX_LOCALE` for system-wide locale setting
  - Config field `locale` in `.agnix.toml` for project-specific locale
  - Automatic locale detection from system settings (LANG/LC_ALL)
  - LSP server locale initialization for editor integration
  - JSON and SARIF output always in English for CI/CD consistency
  - Translation guide in docs/TRANSLATING.md for contributors
  - Comprehensive test suite for locale detection and fallback behavior
  - IDE locale setting: VS Code (`agnix.locale`), Neovim plugin, and LSP config bridge
    - Supports explicit null to revert to auto-detection

### Changed

- Documentation and website navigation now include direct install links for VS Code and JetBrains extensions, plus a prominent website link in the README.
- Core: introduce `static_regex!` macro for validated regex initialization (#246)
  - Replaces bare `.unwrap()` on `Regex::new()` with descriptive `.expect()` messages
  - Migrates 36 `OnceLock<Regex>` patterns across 7 files to use the macro
  - Converts `hooks.rs` from `once_cell::sync::Lazy` to `std::sync::OnceLock`
  - Removes `once_cell` direct dependency from agnix-core
  - Adds per-module `test_regex_patterns_compile` tests for all static patterns

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- CLI: harden telemetry queue timestamp parsing against malformed data (#231)
  - Replace panic-prone byte-index slicing with safe `str::get()` calls
  - Add ASCII guard, separator validation, and range checks (year, month-aware day bounds, hour, minute, second)
  - Use `checked_sub` for day arithmetic to prevent u32 underflow
- Config validation: accept VER-\* prefix in disabled_rules (#233)
- VS Code extension: harden `downloadFile()` cleanup for stream and HTTP failure paths (#240)
  - Closes file/request handles on failure
  - Removes temporary download artifacts on failed downloads
  - Adds regression tests for non-200, stream-error, and success branches
- CLI: gate telemetry module wiring behind `telemetry` feature while preserving command UX via a non-feature stub (#245)
  - `telemetry` module compiles only when feature-enabled
  - Non-feature builds route telemetry calls through `telemetry_stub` no-op facade
  - Added stub-path unit tests and validated both feature and non-feature builds
- LSP backend now uses shared `Arc<String>` document cache entries to avoid full-text cloning on `did_change`, `did_save`, `codeAction`, and `hover` paths (#244)
- LSP now revalidates open documents with bounded concurrency on config changes and drops stale diagnostics from outdated config/content snapshots (#243)

### Security

- ReDoS protection via regex input size limits (MAX_REGEX_INPUT_SIZE = 64KB)
  - Markdown XML tag extraction skips oversized content
  - Cross-platform and prompt engineering validators protected
- File count limits to prevent DoS attacks
  - Default limit of 10,000 files (configurable via max_files_to_validate)
  - CLI flag --max-files to override or disable (--max-files 0)
- Fuzz testing infrastructure with cargo-fuzz
  - Three fuzz targets: fuzz_frontmatter, fuzz_markdown, fuzz_json
  - CI runs 5-minute fuzzing on PRs, 30-minute weekly fuzzing
  - UTF-8 boundary validation for markdown parsing
- Enhanced symlink handling documentation and tests
  - Comprehensive tests for Unix and Windows symlink behavior
  - MAX_SYMLINK_DEPTH = 40 to prevent circular symlink loops
- Security integration test suite (crates/agnix-core/tests/security_integration.rs)
  - Symlink rejection, file size limits, path traversal, file count limits
  - ReDoS protection validation, concurrent validation safety
- Hardened dependency management
  - cargo-audit integration (pinned to v0.21.0) in CI
  - cargo-deny policy with multiple-versions = deny
  - audit.toml and deny.toml configuration files
- Security documentation
  - SECURITY.md with reporting policy and security configuration
  - knowledge-base/SECURITY-MODEL.md with threat model and implementation details
  - Audit history tracking and incident response procedures
- LSP workspace boundary check hardened (#232)
  - Added normalize_path() fallback when canonicalize() fails
  - Prevents path traversal via .. components in non-canonical paths

### Added

- Neovim plugin at `editors/neovim/` with full LSP integration (#187)
  - Automatic LSP attachment to agnix-relevant files
  - Commands: `:AgnixStart`, `:AgnixStop`, `:AgnixRestart`, `:AgnixInfo`, `:AgnixValidateFile`, `:AgnixShowRules`, `:AgnixFixAll`, `:AgnixFixSafe`, `:AgnixIgnoreRule`, `:AgnixShowRuleDoc`
  - Optional Telescope integration for rule browsing
  - `:checkhealth agnix` support
  - Installation via lazy.nvim, packer.nvim, vim-plug, or manual
- Research tracking document (`knowledge-base/RESEARCH-TRACKING.md`) with AI tool inventory and monitoring process (#191)
- Monthly review checklist (`knowledge-base/MONTHLY-REVIEW.md`) with February 2026 review completed (#191)
- Rule contribution and tool support request issue templates (#191)
- Expanded CONTRIBUTING.md with rule authoring guide, evidence requirements, and tier system (#191)
- JetBrains IDE plugin with LSP integration (#196)
  - Supports IntelliJ IDEA, WebStorm, PyCharm, and all JetBrains IDEs (2023.3+)
  - Real-time validation, quick fixes, hover documentation
  - Auto-download of agnix-lsp binary from GitHub releases
  - Settings UI with LSP path configuration, auto-download toggle, trace level
  - Context menu actions: Validate File, Restart Server, Settings
  - Uses LSP4IJ for standard LSP client support
- `agnix schema` command for JSON Schema generation (#206)
  - Outputs JSON Schema for `.agnix.toml` to stdout or file
  - Generated from Rust types using schemars
- Config validation with helpful warnings (#206)
  - Validates `disabled_rules` against known rule ID patterns
  - Validates `tools` array contains recognized tool names
  - Warns on deprecated fields (`mcp_protocol_version`)
- VS Code schema association for `.agnix.toml` autocomplete (#206)
- Opt-in telemetry module with privacy-first design (#209)
  - Disabled by default, requires explicit `agnix telemetry enable`
  - Tracks aggregate metrics: rule trigger counts, error/warning counts, duration
  - Never collects: file paths, contents, user identity
  - Respects DO_NOT_TRACK, CI, GITHUB_ACTIONS environment variables
  - Feature-gated HTTP client for minimal binary size impact
  - Local event queue for offline storage with automatic retry
- `agnix telemetry` subcommand with status/enable/disable commands
- Comprehensive telemetry documentation in SECURITY.md
- Rule ID validation at collection point (defense-in-depth)
- VS Code extension settings UI for configuring all validation options (#225)
  - Settings page accessible via "Open Settings (UI)" command
  - Live preview of all rules with descriptions
  - Changes apply immediately without server restart
  - Built with Svelte for reactive UI

### Changed

- Refactored SkillValidator internal structure for better maintainability (#211)
  - Extracted monolithic 660-line validate() method into ValidationContext struct
  - Grouped validation logic into 11 focused methods by concern
  - Reduced main validate() from ~660 lines to ~78 lines
  - All 128 tests pass without modification (zero behavior changes)
- Refactored HooksValidator into standalone validation functions (#212)
  - Extracted 12 validation rules (CC-HK-001 through CC-HK-012) into standalone functions
  - Reduced main validate() method from ~480 to ~210 lines
  - Organized validation into clear phases with documentation
  - Improved maintainability and testability without changing validation behavior
- Split Hook and Skill validator modules into focused files (#242)
  - Replaced monolithic `rules/hooks.rs` and `rules/skill.rs` with `rules/hooks/{mod,helpers,tests}.rs` and `rules/skill/{mod,helpers,tests}.rs`
  - No validation behavior changes; refactor is layout-only for maintainability

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- CLI `--fix` now exits with status `0` when all diagnostics are resolved by auto-fixes (#230)
  - Exit status now reflects post-fix diagnostics for non-dry-run fix modes
  - Added integration regression test for `--fix` success after full auto-fix
- Imports validation now recovers from poisoned shared `ImportCache` locks during project validation (#239)
- Import traversal now revisits files discovered at shallower depth and avoids duplicate REF-001 diagnostics (#239)

### Performance

- Benchmark infrastructure with iai-callgrind for deterministic CI testing (#202)
  - Instruction count benchmarks immune to system load variance
  - Helper script (./scripts/bench.sh) for iai/criterion/bloat workflows
  - Scale testing with 100 and 1000 file projects
  - Memory usage tracking with tracking-allocator
  - CI job blocks merge on performance regressions
  - Cross-platform support (Linux/macOS with Valgrind, Windows uses Criterion only)

## [0.7.2] - 2026-02-05

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- npm package wrapper script now preserved during binary installation
  - Fixes "command not found" error when running `agnix` from npm install
  - Postinstall script backs up and restores wrapper script

## [0.7.1] - 2026-02-05

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- VS Code extension LSP installation - now downloads LSP-specific archives (`agnix-lsp-*.tar.gz`)
  - Fixes "chmod: No such file or directory" error on macOS ARM64 and Linux ARM64
  - Added binary existence check before chmod for better error messages
- CC-MEM-006 rule now correctly recognizes positive alternatives before negatives
  - Pattern "DO X, don't do Y" now accepted (previously incorrectly flagged)
  - Example: "Fetch web resources fresh, don't rely on cached data" ✓

### Changed

- Release workflow now builds separate LSP archives for VS Code auto-download

## [0.7.0] - 2026-02-05

### Changed

- Refactored LintConfig internal structure for better maintainability (#214)
  - Introduced RuntimeContext struct to group non-serialized state
  - Introduced RuleFilter trait to encapsulate rule filtering logic
  - Public API remains fully backward compatible

### Added

- FileSystem trait for abstracting file system operations (#213)
  - Enables unit testing validators with MockFileSystem instead of requiring real temp files
  - RealFileSystem delegates to std::fs and file_utils for production use
  - MockFileSystem provides HashMap-based in-memory storage with RwLock for thread safety
  - Support for symlink handling and circular symlink detection
  - Integrated into LintConfig via fs() accessor for dependency injection
- Comprehensive test suite for validation rule coverage (#221)
  - Added exhaustive tests for all valid values in enums and constants
  - Improved test coverage for edge cases and error conditions
  - Fixed test logic to properly reflect tool event requirements

### Performance

- Shared import cache at project validation level reduces redundant parsing (#216)

## [0.3.0] - 2026-02-05

### Added

- Comprehensive config file tests (30+ new tests)
- Performance benchmarks for validation pipeline
- Support for partial config files (only specify fields you need)

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- Config now allows partial files - users can specify only `disabled_rules` without all other fields
- Windows path false positives - regex patterns (`\n`, `\s`, `\d`) no longer flagged as Windows paths
- Comma-separated tool parsing - both `Read, Grep` and `Read Write` formats now work
- Git ref depth check - `refs/remotes/origin/HEAD` no longer flagged as deep file paths
- Template placeholder links - `{url}`, `{repoUrl}` placeholders skipped in link validation
- Wiki-style links - single-word links like `[[brackets]]` no longer flagged
- CHANGELOG.md excluded from validation (not an agent config file)
- @import/reference false positives - requires file extension for paths with `/`

### Changed

- README updated for v0.3.0 with accurate config examples and benchmark numbers
- Installation now uses `cargo install agnix-cli` from crates.io

## [0.2.0] - 2026-02-05

### Added

- crates.io publishing support (#20)
  - New `agnix-rules` crate for independent rule updates without CLI republish
  - LICENSE-MIT and LICENSE-APACHE files for dual licensing
  - Crate-level READMEs for crates.io pages
  - Automatic crates.io publish on release tags via CI workflow
  - Parity test ensures rules.json stays in sync between knowledge-base and crate
  - Input validation in build.rs for secure code generation
- Language Server Protocol (LSP) implementation for real-time editor validation (#18)
  - New `agnix-lsp` crate with tower-lsp backend
  - Real-time diagnostics on document changes (textDocument/didChange)
  - Real-time diagnostics on file open and save events
  - Quick-fix code actions from Fix objects
  - Hover documentation for frontmatter fields
  - Document content caching for performance
  - Supports all 342 agnix validation rules with severity mapping

  - Workspace boundary validation for security (prevents path traversal)
  - Config caching optimization for performance
  - Editor support for VS Code, Neovim, Helix, and other LSP-compatible editors
  - Comprehensive test coverage with 36 unit and integration tests
  - Installation: `cargo install --path crates/agnix-lsp`
  - LSP now loads `.agnix.toml` from workspace root (#174)

- Multi-tool support via `tools` array in config (#175)
  - Specify `tools = ["claude-code", "cursor"]` to enable only relevant rules
  - Tool-specific rules (CC-_, COP-_, CUR-\*) filtered based on tools list
  - Generic rules (AS-_, XP-_, AGM-_, MCP-_, PE-\*) always apply
  - Case-insensitive tool name matching
  - Takes precedence over legacy `target` field for flexibility
- VS Code extension with full LSP integration (#22)
  - Real-time diagnostics for all 342 validation rules

  - Status bar indicator showing agnix validation status
  - Syntax highlighting for SKILL.md YAML frontmatter
  - Commands: 'Restart Language Server' and 'Show Output Channel'
  - Configuration: agnix.lspPath, agnix.enable, agnix.trace.server
  - Safe LSP binary detection (prevents command injection)
  - Documentation in editors/vscode/README.md

- Spec Drift Sentinel workflow for automated upstream specification monitoring (#107)
  - Weekly checks for S-tier sources (Agent Skills, MCP, Claude Code, Codex CLI, OpenCode)
  - Monthly checks for A-tier sources (Cursor, GitHub Copilot, Cline)
  - SHA256 content hashing with whitespace normalization for drift detection
  - Baseline storage in `.github/spec-baselines.json`
  - Auto-creates GitHub issues when drift detected with actionable review steps
  - Manual workflow dispatch for on-demand checks and baseline updates
  - Security hardened: HTTPS-only URL validation, SHA-pinned actions, minimal permissions
- Version-aware validation with configurable tool and spec versions
  - New VER-001 rule: Warns when no tool/spec versions are pinned in .agnix.toml
  - Added [tool_versions] section for pinning tool versions (claude_code, codex, cursor, copilot)
  - Added [spec_revisions] section for pinning spec versions (mcp_protocol, agent_skills_spec, agents_md_spec)
  - CC-HK-010 and MCP-008 now add assumption notes when versions are not pinned
  - Diagnostics include assumption field explaining version-dependent behavior
  - Documentation in README.md and VALIDATION-RULES.md
- Cross-layer contradiction detection with 3 new validation rules (XP-004 to XP-006)
  - XP-004: Conflicting build/test commands detection (npm vs pnpm vs yarn vs bun)
  - XP-005: Conflicting tool constraints detection (allow vs disallow across files)
  - XP-006: Multiple instruction layers without documented precedence warning
  - Detects contradictions across CLAUDE.md, AGENTS.md, .cursor/rules, and Copilot files
  - HashMap-based O(n\*m) algorithms for efficient conflict detection
  - Word boundary matching to prevent false positives
  - Backup file exclusion (.bak, .old, .tmp, .swp, ~)
- Evidence metadata schema for all 100 validation rules
  - Added `evidence` field to each rule in `knowledge-base/rules.json` with:
    - `source_type`: Classification (spec, vendor_docs, vendor_code, paper, community)
    - `source_urls`: Links to authoritative documentation or specifications
    - `verified_on`: ISO 8601 date of last verification
    - `applies_to`: Tool/version/spec applicability constraints
    - `normative_level`: RFC 2119 level (MUST, SHOULD, BEST_PRACTICE)
    - `tests`: Coverage tracking (unit, fixtures, e2e)
  - Build-time SARIF rule generation from rules.json (replaces hardcoded registry)
  - CI validation tests for evidence metadata completeness and validity
  - Documentation in VALIDATION-RULES.md with schema reference and examples
- Cursor Project Rules support with 6 new validation rules (CUR-001 to CUR-006)
  - CUR-001: Empty .mdc rule file detection
  - CUR-002: Missing frontmatter warning
  - CUR-003: Invalid YAML frontmatter validation
  - CUR-004: Invalid glob pattern in globs field
  - CUR-005: Unknown frontmatter keys warning
  - CUR-006: Legacy .cursorrules migration warning
  - New file type detection for `.cursor/rules/*.mdc` and `.cursorrules`
  - Comprehensive test coverage with 8 fixtures

### Performance

- LSP server now caches ValidatorRegistry in Backend struct (#171)
  - Registry wrapped in Arc and shared across spawn_blocking validation tasks
  - Eliminates redundant HashMap allocations and validator factory lookups per validation
- AS-015 directory size validation now short-circuits when limit exceeded, improving performance on large skill directories (#84)
- Stream file walk to reduce memory usage on large repositories (#172)
  - Replaced collect-then-validate pattern with streaming par_bridge()
  - Eliminated intermediate Vec<PathBuf> storage (O(n) to O(1) memory for file paths)
  - Use AtomicUsize and Arc<Mutex<Vec>> for concurrent metadata collection
  - Small synchronization overhead traded for significant memory reduction on large repos

### Tests

- Added validation pipeline tests for AGENTS.md path collection and files_checked counter (#83)

### Changed

- Tool mappings derived from rules.json at compile time (#176)
  - VALID_TOOLS and TOOL_RULE_PREFIXES now extracted from rules.json evidence metadata
  - New helper functions in agnix-rules: valid_tools(), get_tool_for_prefix(), get_prefixes_for_tool()
  - Config tools array validation uses derived mappings instead of hardcoded list
  - Backward compatibility maintained with "copilot" alias for "github-copilot"
  - Zero runtime cost - all mappings resolved at compile time
- Narrowed agnix-core public API surface (#85)
  - Made `parsers`, `rules`, `schemas`, and `file_utils` modules private
  - Re-exported `Validator` trait for custom validator implementations
  - No breaking changes for agnix-cli or external consumers using documented API

### Removed

- Removed unused config flags `tool_names` and `required_fields` from `.agnix.toml`
  - These flags were never referenced in the codebase
  - Backward compatibility maintained - old configs with these fields still parse correctly

### Fixed
- **Repo references**: Updated remaining hardcoded `avifenesh/agnix` references to `agent-sh/agnix` across all editors (Zed, Neovim), website, metadata files, and documentation

- Mutex locks in streaming validation now use unwrap() for consistent fail-fast on poisoning (#172)
- CLAUDE/AGENTS parity test now resilient to different directory structures (worktrees, symlinks)
  - Replaced brittle `.ancestors().nth(2)` with dynamic workspace root detection
  - New `workspace_root()` helper searches for `[workspace]` in ancestor Cargo.toml files
- JSON output `files_checked` now correctly reports total validated files, not just files with diagnostics
- CLI `--target` flag now validates values instead of silently falling back to "generic"
  - Invalid values rejected with helpful error message showing valid options
  - Prevents configuration typos from going unnoticed
- GitHub Action: Windows binary extension handling (.exe)
- GitHub Action: Missing verbose flag in SARIF output re-run
- GitHub Action: Document jq dependency and fail-on-error input in README
- Config parse errors now display a warning instead of silently falling back to defaults
  - Invalid `.agnix.toml` files show clear error message with parse location
  - Validation continues with default config after displaying warning
  - Warning goes to stderr, preserving JSON/SARIF output validity
- Pinned `cargo-machete` to version `0.9.1` in CI workflow to prevent nondeterministic build failures
- Exclude patterns now prune directories during traversal to reduce IO on large repos
- CLI init command output replaced checkmark emoji with plain text prefix
- Reject `--fix`, `--dry-run`, and `--fix-safe` when using JSON or SARIF output formats
- Exclude glob patterns now match correctly when validate_project() is called with absolute paths (#67)
  - Patterns like `target/**` previously failed to match when walker yielded absolute paths
  - Added path normalization by stripping base path prefix before glob matching
- PE-001 through PE-004 rules now properly dispatch on CLAUDE.md and AGENTS.md files (PromptValidator was implemented but not registered in ValidatorRegistry)
- `is_mcp_revision_pinned()` now correctly returns false when neither `spec_revisions.mcp_protocol` nor `mcp_protocol_version` are explicitly set
  - Previously always returned true due to `serde(default)` on `mcp_protocol_version`
  - This allows MCP-008 assumption notes to appear when no version is configured

### Security

- GitHub Action: Validate version input format to prevent path traversal attacks
- GitHub Action: Sanitize diagnostic messages in workflow commands to prevent injection
- GitHub Action: Use authenticated GitHub API requests when token available (avoids rate limits)
- Blocked @import paths that resolve outside the project root to prevent traversal
- Hardened file reading with symlink rejection and size limits:
  - Added `FileSymlink` error to reject symlinks (prevents path traversal)
  - Added `FileTooBig` error for files exceeding 1 MiB (prevents DoS)
  - New `file_utils` module with `safe_read_file()` using `symlink_metadata()`
  - Applied to validation, imports, fixes, and config loading
  - Cross-platform tests for Unix and Windows symlink handling
- Hardened GitHub Actions workflows with security best practices:
  - Added explicit permissions blocks to all workflows (principle of least privilege)
  - SHA-pinned all third-party actions to prevent supply chain attacks
  - Restricted cache saves to main branch only (prevents cache poisoning from PRs)
  - Documented SHA pin reference in .github/workflows/README.md for maintainability

### Added

- Evaluation harness with `agnix eval` command for measuring rule efficacy
  - Load test cases from YAML manifests with expected rule IDs
  - Calculate precision, recall, and F1 scores per rule and overall
  - Output formats: markdown (default), JSON, CSV
  - Filter by rule prefix (`--filter`)
  - Verbose mode for per-case details (`--verbose`)
  - 39 test cases covering AS-_, CC-SK-_, MCP-_, AGM-_, XP-_, XML-_, REF-\* rules
  - Path traversal protection (relative paths only)
  - Documentation in knowledge-base/EVALUATION.md
- MCP-008 rule for protocol version validation with configurable `mcp_protocol_version` option
- 5 new parse error rules with normalized IDs (AS-016, CC-HK-012, CC-AG-007, CC-PL-006, MCP-007)
- Auto-fix support for CC-MEM-005 and CC-MEM-007 memory rules
  - CC-MEM-005: Delete lines containing generic instructions
  - CC-MEM-007: Replace weak constraint language with stronger alternatives
  - CRLF line ending support for correct byte offsets on Windows
- Auto-fix implementations for five additional rules:
  - AS-004: Convert invalid skill names to kebab-case (case-only fixes marked safe)
  - AS-010: Prepend "Use when user wants to " to descriptions missing trigger phrase
  - XML-001: Automatically insert closing XML tags for unclosed elements
  - CC-HK-001: Replace invalid hook event names with closest valid match
  - CC-SK-007: Replace unrestricted Bash access with scoped alternatives (e.g., `Bash(git:*)`)
- Reusable GitHub Action for CI/CD integration:
  - Composite action using pre-built release binaries
  - Inputs for path, strict, target, config, format, verbose, version
  - Outputs for result, errors, warnings, sarif-file
  - GitHub annotations from validation diagnostics
  - Cross-platform support (Linux, macOS, Windows)
  - Test workflow for action validation
- Release workflow for automated binary distribution on version tags:
  - Builds for 5 targets (linux-gnu, linux-musl, macos-x86, macos-arm, windows)
  - Creates archives with SHA256 checksums
  - Extracts release notes from CHANGELOG.md
  - Uploads artifacts to GitHub Releases
- 52 CLI integration tests for comprehensive coverage of all output formats and flags:
  - 12 rule family coverage tests (AS, CC-SK, CC-HK, CC-AG, MCP, XML, CC-PL, COP, AGM, CC-MEM, REF, XP)
  - 5 SARIF output validation tests (schema, tool info, rules, locations, help URIs)
  - 6 text output formatting tests (location, levels, summary, verbose mode)
  - 5 fix/dry-run flag tests (--fix, --fix-safe, --dry-run)
  - 5 flag combination tests (--strict, --verbose, --target, --validate)

- Support for instruction filename variants:
  - CLAUDE.local.md - Claude Code local instructions (not synced to cloud)
  - AGENTS.local.md - Codex CLI/OpenCode local instructions
  - AGENTS.override.md - Codex CLI override file for workspace-specific rules
  - All variants are validated with the same rules as their base files
- Rule parity CI check to ensure documented rules stay in sync with implementation:
  - Added `knowledge-base/rules.json` as machine-readable source of truth for all 84 rules
  - Added `crates/agnix-cli/tests/rule_parity.rs` integration test suite
  - CI fails if rules drift between documentation, SARIF registry, and implementation
  - CLAUDE.md/AGENTS.md updated to document rules.json workflow
- GitHub Copilot instruction files validation with 4 rules (COP-001 to COP-004)
  - COP-001: Empty/missing global copilot-instructions.md
  - COP-002: Invalid YAML frontmatter in scoped instruction files
  - COP-003: Invalid applyTo glob pattern
  - COP-004: Unknown frontmatter keys
  - Supports .github/copilot-instructions.md (global instructions)
  - Supports .github/instructions/\*.instructions.md (path-scoped instructions)
  - Config-based copilot category toggle (rules.copilot)
- ValidatorRegistry API for custom validator registration in agnix-core
- AGENTS.md validation rules (AGM-001 to AGM-006)
  - AGM-001: Valid markdown structure
  - AGM-002: Missing section headers
  - AGM-003: Character limit (12000 for Windsurf)
  - AGM-004: Missing project context
  - AGM-005: Unguarded platform features
  - AGM-006: Nested AGENTS.md hierarchy
- AGENTS.md validator now runs via the default registry, with project-level AGM-006 detection
- Explicit HTML anchors in VALIDATION-RULES.md for SARIF help_uri links (#88)
  - Added 80 anchors (one per rule) to fix GitHub anchor mismatch
  - Added tests to validate help_uri format and anchor correctness
- Prompt Engineering validation with 4 rules (PE-001 to PE-004)
  - PE-001: Detects critical content in middle of document (lost in the middle effect)
  - PE-002: Warns when chain-of-thought markers used on simple tasks
  - PE-003: Detects weak imperative language (should, try, consider) in critical sections
  - PE-004: Flags ambiguous instructions (e.g., "be helpful", "as needed")
- PromptValidator implementation in agnix-core
- Config-based prompt_engineering category toggle (rules.prompt_engineering)
- 8 test fixtures in tests/fixtures/prompt/ directory
- 48 comprehensive unit tests for prompt engineering validation
- MCP (Model Context Protocol) validation with 6 rules (MCP-001 to MCP-006)
  - MCP-001: Validates JSON-RPC version is "2.0"
  - MCP-002: Validates required tool fields (name, description, inputSchema)
  - MCP-003: Validates inputSchema is valid JSON Schema
  - MCP-004: Warns when tool description is too short (<10 chars)
  - MCP-005: Warns when tool lacks consent mechanism (requiresApproval/confirmation)
  - MCP-006: Warns about untrusted annotations that should be validated
- McpValidator and McpToolSchema in agnix-core
- Config-based MCP category toggle (rules.mcp)
- 8 test fixtures in tests/fixtures/mcp/ directory
- 48 comprehensive unit tests for MCP validation
- Cross-platform validation rules XP-001, XP-002, XP-003
  - XP-001: Detects Claude-specific features (hooks, context:fork, agent, allowed-tools) in AGENTS.md (error)
    - Supports section guards: Features inside Claude-specific sections (e.g., `## Claude Code Specific`) are allowed
  - XP-002: Validates AGENTS.md markdown structure for cross-platform compatibility (warning)
  - XP-003: Detects hard-coded platform paths (.claude/, .opencode/, .cursor/, etc.) in configs (warning)
- New `cross_platform` config category toggle for XP-\* rules
- 5 test fixtures in tests/fixtures/cross_platform/ directory
- 30 comprehensive unit tests for cross-platform validation
- Hook timeout validation rules CC-HK-010 and CC-HK-011
  - CC-HK-010: Warns when hooks lack timeout specification (MEDIUM)
  - CC-HK-011: Errors when timeout value is invalid (negative, zero, or non-integer) (HIGH)
  - Two new test fixtures: no-timeout.json, invalid-timeout.json
- Claude Memory validation rules CC-MEM-004, CC-MEM-006 through CC-MEM-010
  - CC-MEM-004: Validates npm scripts referenced in CLAUDE.md exist in package.json
  - CC-MEM-006: Detects negative instructions ("don't", "never") without positive alternatives
  - CC-MEM-007: Warns about weak constraint language ("should", "try") in critical sections
  - CC-MEM-008: Detects critical content in middle of document (lost in the middle effect)
  - CC-MEM-009: Warns when file exceeds ~1500 tokens, suggests using @imports
  - CC-MEM-010: Detects significant overlap (>40%) between CLAUDE.md and README.md
- SARIF 2.1.0 output format with `--format sarif` CLI option for CI/CD integration
  - Full SARIF 2.1.0 specification compliance with JSON schema validation
  - Includes all 80 validation rules in driver.rules with help URIs
  - Supports GitHub Code Scanning and other SARIF-compatible tools
  - Proper exit codes for CI workflows (errors exit 1)
  - Path normalization for cross-platform compatibility
  - 8 comprehensive integration tests for SARIF output
- SkillValidator Claude Code rules (CC-SK-001 to CC-SK-005, CC-SK-008 to CC-SK-009)
  - CC-SK-001: Validates model field values (sonnet, opus, haiku, inherit)
  - CC-SK-002: Validates context field must be 'fork' or omitted
  - CC-SK-003: Requires 'agent' field when context is 'fork'
  - CC-SK-004: Requires 'context: fork' when agent field is present
  - CC-SK-005: Validates agent type values (Explore, Plan, general-purpose, or custom kebab-case names 1-64 chars)
  - CC-SK-006: Dangerous skills must set 'disable-model-invocation: true'
  - CC-SK-007: Warns on unrestricted Bash access (suggests scoped versions)
  - CC-SK-008: Validates tool names in allowed-tools against known Claude Code tools
  - CC-SK-009: Warns when too many dynamic injections (!`) detected (>3)
- 27 comprehensive unit tests for skill validation (244 total tests)
- 9 test fixtures in tests/fixtures/skills/ directory for CC-SK rules
- JSON output format with `--format json` CLI option for programmatic consumption
  - Simple, human-readable structure for easy parsing and integration
  - Includes version, files_checked, diagnostics array, and summary counts
  - Cross-platform path normalization (forward slashes)
  - Proper exit codes for CI workflows (errors exit 1)
  - 14 comprehensive unit tests for JSON output
- Comprehensive CI workflow with format check, clippy, machete, and test matrix (3 OS x 2 Rust versions)
- Security scanning workflow with CodeQL analysis and cargo-audit (runs on push, PR, and weekly schedule)
- Changelog validation workflow to ensure CHANGELOG.md is updated in PRs
- PluginValidator implementation with 5 validation rules (CC-PL-001 to CC-PL-005)
  - CC-PL-001: Validates plugin.json is in .claude-plugin/ directory
  - CC-PL-002: Detects misplaced components (skills/agents/hooks) inside .claude-plugin/
  - CC-PL-003: Validates version uses semver format (X.Y.Z)
  - CC-PL-004: Validates required field (name) and recommended fields (description, version)
  - CC-PL-005: Validates name field is not empty
- Path traversal protection with MAX_TRAVERSAL_DEPTH limit
- 47 comprehensive tests for plugin validation (234 total tests)
- 4 test fixtures in tests/fixtures/plugins/ directory
- Auto-fix infrastructure with CLI flags:
  - `--fix`: Apply automatic fixes to detected issues
  - `--dry-run`: Preview fixes without modifying files
  - `--fix-safe`: Only apply high-certainty (safe) fixes
- `Fix` struct with `FixKind` enum (Replace, Insert, Delete) in diagnostics
- `apply_fixes()` function to process and apply fixes to files
- Diagnostics now include `[fixable]` marker in output for issues with available fixes
- Hint message in CLI output when fixable issues are detected
- Config-based rule filtering with category toggles (skills, hooks, agents, memory, plugins, xml, imports)
- Target tool filtering - CC-\* rules automatically disabled for non-Claude Code targets (Cursor, Codex)
- Individual rule disabling via `disabled_rules` config list
- `is_rule_enabled()` method with category and target awareness
- AgentValidator implementation with 6 validation rules (CC-AG-001 to CC-AG-006)
  - CC-AG-001: Validates required 'name' field in agent frontmatter
  - CC-AG-002: Validates required 'description' field in agent frontmatter
  - CC-AG-003: Validates model values (sonnet, opus, haiku, inherit)
  - CC-AG-004: Validates permissionMode values (default, acceptEdits, dontAsk, bypassPermissions, plan)
  - CC-AG-005: Validates referenced skills exist at .claude/skills/[name]/SKILL.md
  - CC-AG-006: Detects conflicts between 'tools' and 'disallowedTools' arrays
- Path traversal security protection for skill name validation
- 44 comprehensive tests for agent validation (152 total tests)
- 7 test fixtures in tests/fixtures/agents/ directory
- Parallel file validation using rayon for improved performance on large projects
- Deterministic diagnostic output with sorting by severity and file path
- Comprehensive tests for parallel validation edge cases
- Reference validator rules REF-001 and REF-002
  - REF-001: @import references must point to existing files (error)
  - REF-002: Markdown links [text](path) should point to existing files (error)
  - Both rules are in the "imports" category
  - Supports fragment stripping (file.md#section validates file.md)
  - Skips external URLs (http://, https://, mailto:, etc.)
  - 4 test fixtures in tests/fixtures/refs/ directory
  - 31 comprehensive unit tests for reference validation

### Changed

- Removed miette dependency from agnix-core to reduce binary size and compile times
  - agnix-core is now a pure library without terminal output dependencies
  - CLI continues to use colored for output formatting
  - Removed 8 unused LintError variants that used miette-specific features
- Downgraded 5 rules from ERROR to WARNING severity based on RFC 2119 audit:
  - PE-001 (Lost in the middle): Research-based recommendation, not spec violation
  - PE-002 (Chain-of-thought on simple task): Best practice advice, not requirement
  - CC-MEM-004 (Invalid command reference): Helpful validation, not breaking error
  - AGM-003 (Character limit): Uses SHOULD in documentation (Windsurf-specific)
  - AGM-005 (Platform-specific features): Uses SHOULD in documentation
- Imports validator now routes diagnostics by file type:
  - CLAUDE.md files emit CC-MEM-001/002/003 (Claude Code memory rules)
  - Non-CLAUDE markdown files emit REF-001 (generic reference validation)
  - Improved security with path traversal protection (rejects absolute paths)
  - Fixed critical bug: file type now determined per-file during recursion
- XML validator now emits specific rule IDs for each error type:
  - XML-001: Unclosed XML tag
  - XML-002: Mismatched closing tag
  - XML-003: Unmatched closing tag
- Individual XML rules can now be disabled via `disabled_rules` config
- Test fixtures restructured for improved validator integration:
  - Skills: Moved to subdirectory pattern (deep-reference/SKILL.md, missing-frontmatter/SKILL.md, windows-path/SKILL.md)
  - MCP: Renamed with .mcp.json suffix for proper FileType detection
  - Ensures validate_project() correctly identifies fixture types during integration tests
- `validate_project()` now processes files in parallel while maintaining deterministic output
- Directory walking remains sequential, only validation is parallelized
- All validators now respect config-based category toggles and disabled rules
- Config structure enhanced with category-based toggles (legacy flags still supported)
- Knowledge base docs refreshed (rule counts, AGENTS.md support tiers, Cursor rules)
- Fixture layout aligned with detector paths to ensure validators exercise fixtures directly
- CC-HK-010 timeout thresholds now align with official Claude Code documentation
  - Command hooks: warn when timeout > 600s (10-minute default)
  - Prompt hooks: warn when timeout > 30s (30-second default)

### Performance

- Significant speed improvements on projects with many files
- Maintains correctness with deterministic sorting of results
