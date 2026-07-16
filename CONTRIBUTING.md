# Contributing to agnix

Thank you for contributing to agnix.

## Development Setup

```bash
git clone https://github.com/agent-sh/agnix
cd agnix
cargo build
cargo test
```

## Code Style

Before committing:

```bash
cargo fmt
cargo clippy --all-targets
```

## Adding a New Rule

1. **Add to rules.json** - `knowledge-base/rules.json` is the source of truth
2. **Add to VALIDATION-RULES.md** - `knowledge-base/VALIDATION-RULES.md` for human docs
3. **Implement validator** - `crates/agnix-core/src/rules/`
4. **Add test fixtures** - `tests/fixtures/`
5. **Run parity tests** - CI enforces rules.json and VALIDATION-RULES.md stay in sync

When editing project memory instructions, keep `CLAUDE.md` and `AGENTS.md` byte-identical.

Each rule in `rules.json` must include complete `evidence` metadata. See [Rule Evidence Requirements](#rule-evidence-requirements) below for field details.

Security-facing rules should also include `security` metadata. See [Security Rule Metadata](#security-rule-metadata).

## Rule Evidence Requirements

Each rule in `knowledge-base/rules.json` must include an `evidence` object documenting its authoritative source. The evidence fields are:

| Field | Type | Description |
|-------|------|-------------|
| `source_type` | enum | Classification of the source: `spec` (official specification), `vendor_docs` (vendor documentation), `vendor_code` (vendor source code), `paper` (academic research), `community` (community research such as agentsys) |
| `source_urls` | string[] | One or more URLs pointing to the authoritative documentation, specification, or research paper that supports this rule |
| `verified_on` | string | ISO 8601 date (YYYY-MM-DD) when the source was last verified to be current |
| `applies_to` | object | Applicability constraints: `tool` (specific tool name, e.g., "claude-code"), `version_range` (semver range, e.g., ">=1.0.0"), `spec_revision` (spec version date). Empty object `{}` means the rule applies universally |
| `normative_level` | enum | RFC 2119 level indicating rule strength: `MUST` (spec violation), `SHOULD` (strong recommendation), `BEST_PRACTICE` (advisory) |
| `tests` | object | Test coverage tracking: `{ "unit": true/false, "fixtures": true/false, "e2e": true/false }` |

See `knowledge-base/VALIDATION-RULES.md` for the full evidence schema reference with examples.

## Security Rule Metadata

Rules that detect or prevent security-relevant misconfiguration must include a `security` object in `knowledge-base/rules.json`:

| Field | Type | Description |
|-------|------|-------------|
| `cwe` | string[] | CWE IDs such as CWE ID 798 |
| `owasp` | string[] | OWASP Top 10 IDs such as `A07:2021` |
| `vulnerability_class` | string | Short class such as `hardcoded-secret`, `command-injection`, or `insecure-permissions` |
| `subcategory` | enum | `vuln`, `audit`, or `secure-default` |
| `confidence` | enum | `HIGH`, `MEDIUM`, or `LOW` |
| `likelihood` | enum | `HIGH`, `MEDIUM`, or `LOW` |
| `impact` | enum | `HIGH`, `MEDIUM`, or `LOW` |

This metadata is exported through `agnix-rules` and SARIF taxonomies, so keep it in `rules.json` rather than adding SARIF-only tags.

## Rule Lifecycle

Rules should prefer additive evolution over removal. Use these states:

| Status | Meaning |
|--------|---------|
| `active` | Default when no lifecycle fields are present |
| `deprecated` | Still emitted, but planned for removal or replacement |
| `removed` | No longer emitted; keep a redirect in `knowledge-base/removed-rules.json` |

When deprecating a rule, add `status`, `deprecated_since`, `replaced_by`, and `reason` fields to the rule entry in `knowledge-base/rules.json`, then document the change in `knowledge-base/VALIDATION-RULES.md` and `CHANGELOG.md`.

When removing a rule, remove the active rule entry, add an entry to `knowledge-base/removed-rules.json`, update docs/changelog, and make sure `.agnix.toml` validation warns for stale `disabled_rules` or `[rules.severity]` references. If a replacement exists, list it in `replaced_by`.

## Rule ID Conventions

Rule IDs follow the format `[PREFIX]-[NUMBER]` where the prefix indicates the rule family:

| Prefix | Rules | Current IDs |
|--------|-------|-------------|
| `AGM-` | 6 | AGM-001 through AGM-006 |
| `AMP-` | 4 | AMP-001 through AMP-004 |
| `AMP-SK-` | 1 | AMP-SK-001 |
| `AS-` | 14 | AS-001 through AS-006, AS-008 through AS-009, AS-011 through AS-013, AS-015 through AS-017 |
| `CC-AG-` | 17 | CC-AG-001 through CC-AG-015, CC-AG-017, CC-AG-019 |
| `CC-HK-` | 28 | CC-HK-001 through CC-HK-028 |
| `CC-MEM-` | 13 | CC-MEM-001 through CC-MEM-012, CC-MEM-014 |
| `CC-OS-` | 6 | CC-OS-001 through CC-OS-006 |
| `CC-PL-` | 15 | CC-PL-001 through CC-PL-015 |
| `CC-SET-` | 16 | CC-SET-001 through CC-SET-016 |
| `CC-SK-` | 21 | CC-SK-001 through CC-SK-021 |
| `CDX-` | 7 | CDX-000 through CDX-006 |
| `CDX-AG-` | 7 | CDX-AG-001 through CDX-AG-007 |
| `CDX-APP-` | 3 | CDX-APP-001 through CDX-APP-003 |
| `CDX-CFG-` | 30 | CDX-CFG-001 through CDX-CFG-030 |
| `CDX-PL-` | 16 | CDX-PL-001 through CDX-PL-016 |
| `CDX-REQ-` | 2 | CDX-REQ-000 through CDX-REQ-001 |
| `CL-SK-` | 3 | CL-SK-001 through CL-SK-003 |
| `CLN-` | 7 | CLN-001 through CLN-006, CLN-009 |
| `COP-` | 25 | COP-001 through COP-015, COP-017 through COP-020, COP-022 through COP-027 |
| `CP-SK-` | 1 | CP-SK-001 |
| `CR-SK-` | 1 | CR-SK-001 |
| `CUR-` | 19 | CUR-001 through CUR-019 |
| `CX-SK-` | 1 | CX-SK-001 |
| `GM-` | 10 | GM-001 through GM-010 |
| `GM-AG-` | 1 | GM-AG-001 |
| `KIRO-` | 14 | KIRO-001 through KIRO-014 |
| `KR-AG-` | 13 | KR-AG-001 through KR-AG-013 |
| `KR-HK-` | 10 | KR-HK-001 through KR-HK-010 |
| `KR-MCP-` | 6 | KR-MCP-001 through KR-MCP-006 |
| `KR-PW-` | 8 | KR-PW-001 through KR-PW-008 |
| `KR-SET-` | 3 | KR-SET-001 through KR-SET-003 |
| `KR-SK-` | 1 | KR-SK-001 |
| `MCP-` | 26 | MCP-001 through MCP-026 |
| `OC-` | 8 | OC-001 through OC-004, OC-006 through OC-009 |
| `OC-AG-` | 9 | OC-AG-001 through OC-AG-009 |
| `OC-AGM-` | 2 | OC-AGM-001 through OC-AGM-002 |
| `OC-CFG-` | 14 | OC-CFG-001 through OC-CFG-014 |
| `OC-DEP-` | 6 | OC-DEP-001 through OC-DEP-006 |
| `OC-LSP-` | 2 | OC-LSP-001 through OC-LSP-002 |
| `OC-PM-` | 2 | OC-PM-001 through OC-PM-002 |
| `OC-SK-` | 1 | OC-SK-001 |
| `OC-TUI-` | 3 | OC-TUI-001 through OC-TUI-003 |
| `PE-` | 6 | PE-001 through PE-006 |
| `RC-SK-` | 1 | RC-SK-001 |
| `REF-` | 4 | REF-001 through REF-004 |
| `ROO-` | 6 | ROO-001 through ROO-006 |
| `VER-` | 1 | VER-001 |
| `WS-` | 4 | WS-001 through WS-004 |
| `WS-SK-` | 1 | WS-SK-001 |
| `XML-` | 3 | XML-001 through XML-003 |
| `XP-` | 8 | XP-001 through XP-008 |
| `XP-SK-` | 1 | XP-SK-001 |

To find the next available number for a prefix, check `knowledge-base/rules.json` for the highest existing number in that prefix group and increment by one.

## Implementing a Validator

Step-by-step process for adding a new validation rule:

1. **Add the rule to `knowledge-base/rules.json`** - Include all required fields: `id`, `name`, `severity`, `category`, `message`, `detection`, `fix`, and complete `evidence` metadata. The `crates/agnix-rules/rules.json` file is automatically synchronized during the build process.

2. **Add documentation to `knowledge-base/VALIDATION-RULES.md`** - Document the rule following the existing format with detection logic, fix description, and source citation. CI parity tests will fail if the rule exists in one file but not the other.

3. **Implement the `Validator` trait** - Add validation logic in `crates/agnix-core/src/rules/`. Look at existing validators for patterns:
   - `xml_balance.rs` - simple single-file validator
   - `agents_md.rs` - project-level validator with cross-file analysis
   - `skill/mod.rs` and `hooks/mod.rs` - complex validators split into focused `helpers.rs` and `tests.rs` modules

4. **Register in `ValidatorRegistry`** - Add the validator factory to the appropriate category `ValidatorProvider` struct in `crates/agnix-core/src/registry.rs`. It will be included automatically via `ValidatorRegistry::with_defaults()`. External validators can use the `ValidatorProvider` trait instead.

5. **Add test fixtures** - Create test files in `tests/fixtures/` matching the validator's expected file type detection patterns. Fixtures should cover both valid and invalid configs.

6. **Run tests** - Verify everything passes:
   ```bash
   cargo test                              # Full test suite
   cargo test -p agnix-rules --test parity # Parity check
   cargo test -p agnix-core                # Core validator tests
   ```

## Testing Requirements

All new rules must include:

- **Unit tests** in the validator module (test individual rule detection and edge cases)
- **Integration tests** via test fixtures in `tests/fixtures/` (test end-to-end validation)
- **Parity tests** pass (rules.json matches VALIDATION-RULES.md; rules.json matches crates/agnix-rules/rules.json)
- **Full test suite** passes before submitting a PR (`cargo test`)

## Tool Tier System

agnix organizes tool support into tiers based on community adoption and maintenance commitment:

| Tier | Policy | Testing Requirement |
|------|--------|---------------------|
| **S** | Test always | Every CI run validates against these tools |
| **A** | Test on major changes | Tested when changes affect tool-specific rules |
| **B** | Test on significant changes if time permits | Spot-tested on large changes |
| **C** | Community reports fixes only | Fixes accepted via community issues |
| **D** | No active support, nice to have | Can try once in a while, mainly if users request |
| **E** | No support, community contributions only | Full community support and contributions |

Current tier assignments are documented in [`knowledge-base/RESEARCH-TRACKING.md`](./knowledge-base/RESEARCH-TRACKING.md). When proposing a tier change, open a GitHub issue with adoption data to support the change.

## Community Feedback

I welcome community input through several channels:

- **GitHub Issues** - Use the issue templates for structured feedback:
  - [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md) - Report validation errors
  - [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md) - Suggest new capabilities
  - [Rule Contribution](.github/ISSUE_TEMPLATE/rule_contribution.md) - Propose new validation rules
  - [Tool Support Request](.github/ISSUE_TEMPLATE/tool_support_request.md) - Request support for new tools
- **GitHub Discussions** - General questions, ideas, and community discussion

## Pull Request Process

1. **Update CHANGELOG.md** - Required for all PRs (skip with `[skip changelog]` in title)
2. **Add tests** - Every feature/fix must have tests
3. **Wait for CI** - The claude workflow is the major quality gate
4. **Get review approval** - At least one approval required

## Backward-Compatibility Policy

agnix follows a stability policy to protect downstream consumers (CLI, LSP, MCP, editor extensions) from accidental API breakage.

### Stability Tiers

| Tier | Scope | Contract |
|------|-------|----------|
| **Public/Stable** | Re-exported types at `agnix_core` root (`LintConfig`, `Diagnostic`, `DiagnosticLevel`, `Fix`, `LintError`, `LintResult`, `ValidationResult`, `FileType`, `ValidatorRegistry`, `ValidatorFactory`, `Validator` trait, `FileSystem` trait, `MockFileSystem`, `RealFileSystem`, `FixResult`, `ConfigWarning`, `FilesConfig`, `generate_schema`) and all `agnix_rules` public items (`RULES_DATA`, `VALID_TOOLS`, `TOOL_RULE_PREFIXES`, `rule_count`, `get_rule_name`, `valid_tools`, `normalize_tool_name`, etc.) | Breaking changes require a minor version bump (pre-1.0) or major version bump (post-1.0) with advance notice in CHANGELOG.md |
| **Public/Unstable** | Accessible public modules that may change between minor versions: `authoring`, `eval`, `i18n`, `validation` | May change with a minor version bump; consumers should pin exact versions if depending on these |
| **Internal** | Private modules not accessible outside the crate: `parsers`, `rules`, `schemas`, `file_utils`, `regex_util`, `span_utils` | May change freely in any release |

### What Constitutes a Breaking Change

The following changes to **Public/Stable** items are considered breaking:

- Removing a public type, function, or constant
- Changing a public function signature (parameter types, return type)
- Changing a struct field type or removing a field
- Removing an enum variant
- Changing a trait method signature or adding a required method without a default

### What is Non-Breaking

These changes are safe to make in any release:

- Adding new enum variants (this may break exhaustive matches; consumers should use wildcard `_` arms to stay forward-compatible)
- Adding new optional struct fields with `#[serde(default)]`
- Adding new public functions, types, or modules
- Adding new validators to `ValidatorRegistry::with_defaults()`
- Adding new rules to `agnix_rules`
- Adding new trait methods with default implementations

### Feature Flags

agnix-core intentionally has no feature flags. It is a focused validation library with no optional heavyweight dependencies. Only the CLI crate (`agnix-cli`) has a `telemetry` feature flag for opt-in analytics.

When to add feature flags in the future:
- If a new dependency adds significant compile time or binary size
- If a feature is truly optional and not needed by most consumers

### Pre-1.0 Caveat

While agnix is below version 1.0, this policy is followed in good faith. Minor versions may occasionally contain breaking changes when necessary, but these will always be documented in CHANGELOG.md with migration instructions.

## Commit Messages

Use conventional commits:

- `feat:` - New feature
- `fix:` - Bug fix
- `docs:` - Documentation
- `refactor:` - Code refactoring
- `test:` - Tests
- `chore:` - Maintenance

Reference issues when applicable: `fix: resolve timeout issue (#123)`

## Contributing Translations

agnix supports multiple languages. See [docs/TRANSLATING.md](docs/TRANSLATING.md) for:
- Adding new locales
- Translation guidelines
- Testing translations

## Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p agnix-core

# With output
cargo test -- --nocapture
```

### Security Tests

```bash
# Security integration tests
cargo test --test security_integration

# Fuzz testing (requires nightly)
cd crates/agnix-core
cargo +nightly fuzz run fuzz_markdown -- -max_total_time=300
cargo +nightly fuzz run fuzz_frontmatter -- -max_total_time=300
cargo +nightly fuzz run fuzz_json -- -max_total_time=300

# Dependency audit
cargo audit
cargo deny check
```

## Project Structure

```
crates/
  agnix-rules/    # Rule definitions (generated)
  agnix-core/     # Validation engine
  agnix-cli/      # CLI binary
  agnix-lsp/      # Language server
  agnix-mcp/      # MCP server
  agnix-wasm/     # WebAssembly bindings
editors/
  neovim/         # Neovim extension
  vscode/         # VS Code extension
  jetbrains/      # JetBrains extension scaffold
knowledge-base/   # Rules documentation
scripts/          # Development automation scripts
website/          # Docusaurus documentation website
tests/fixtures/   # Test cases
```

## Questions?

Open an issue or start a discussion.
