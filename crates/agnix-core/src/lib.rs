// The if-let pattern `if config.is_rule_enabled("X") { if condition { ... } }`
// is used intentionally throughout validators for readability.
#![allow(clippy::collapsible_if)]

//! # agnix-core
//!
//! Core validation engine for agent configurations.
//!
//! Validates:
//! - Agent Skills (SKILL.md)
//! - Agent definitions (.md files with frontmatter)
//! - MCP tool configurations
//! - Claude Code hooks
//! - CLAUDE.md memory files
//! - Plugin manifests
//!
//! This crate requires `std`. The `filesystem` Cargo feature (enabled by
//! default) adds file I/O dependencies; disabling it does not enable `no_std`.
//!
//! ## Stability Tiers
//!
//! Public modules are classified into stability tiers:
//!
//! - **Stable** -- `config`, `diagnostics`, `fixes`, `fs`.
//!   These modules follow semver: breaking changes require a major version bump.
//! - **Unstable** -- `authoring`, `eval`, `i18n`, `validation`.
//!   Interfaces may change on minor releases. Use with care in downstream crates.
//! - **Internal** -- `parsers` (pub(crate)).
//!   Not part of the public API. Some types are re-exported at the crate root
//!   with `#[doc(hidden)]` for fuzz/bench/test use only.

// Allow common test patterns that clippy flags but are intentional in tests
#![cfg_attr(
    test,
    allow(
        clippy::field_reassign_with_default,
        clippy::len_zero,
        clippy::useless_vec
    )
)]

rust_i18n::i18n!("locales", fallback = "en");

/// Skill authoring and scaffolding utilities.
///
/// **Stability: unstable** -- interface may change on minor releases.
pub mod authoring;
/// Lint configuration types and schema generation.
///
/// **Stability: stable** -- breaking changes require a major version bump.
pub mod config;
/// Diagnostic, severity, fix, and error types.
///
/// **Stability: stable** -- breaking changes require a major version bump.
pub mod diagnostics;
/// Rule efficacy evaluation (precision/recall/F1).
///
/// **Stability: unstable** -- interface may change on minor releases.
#[cfg(feature = "filesystem")]
pub mod eval;
/// File type detection and extensible detector chain.
///
/// **Stability: unstable** -- interface may change on minor releases.
pub mod file_types;
mod file_utils;
/// Auto-fix application engine.
///
/// **Stability: stable** -- breaking changes require a major version bump.
pub mod fixes;
/// Filesystem abstraction (real and mock).
///
/// **Stability: stable** -- breaking changes require a major version bump.
pub mod fs;
/// Internationalization helpers.
///
/// **Stability: unstable** -- interface may change on minor releases.
pub mod i18n;
/// Internal parsers (frontmatter, JSON, Markdown).
///
/// **Stability: internal** -- not part of the public API.
pub(crate) mod parsers;
mod pipeline;
mod regex_util;
mod registry;
mod rules;
mod schemas;
pub(crate) mod span_utils;
/// Validation registry and file-type detection.
///
/// **Stability: unstable** -- interface may change on minor releases.
pub mod validation;

pub use config::{ConfigWarning, FilesConfig, LintConfig, generate_schema};
pub use diagnostics::{
    ConfigError, CoreError, Diagnostic, DiagnosticLevel, FileError, Fix, FixConfidenceTier,
    LintError, LintResult, RuleMetadata, ValidationError, ValidationOutcome,
};
pub use file_types::{FileType, detect_file_type};
pub use file_types::{FileTypeDetector, FileTypeDetectorChain};
pub use file_utils::MAX_LSP_DOCUMENT_BYTES;
pub use fixes::{
    FixApplyMode, FixApplyOptions, FixResult, apply_fixes, apply_fixes_with_fs,
    apply_fixes_with_fs_options, apply_fixes_with_options,
};
pub use fs::{FileSystem, MockFileSystem, RealFileSystem};
pub use pipeline::{ValidationResult, resolve_file_type, validate_content};
#[cfg(feature = "filesystem")]
pub use pipeline::{
    validate_file, validate_file_with_registry, validate_project, validate_project_rules,
    validate_project_with_registry,
};
pub use registry::{
    ValidatorFactory, ValidatorProvider, ValidatorRegistry, ValidatorRegistryBuilder,
};
pub use rules::{Validator, ValidatorMetadata};

/// Normalize CRLF (`\r\n`) and lone CR (`\r`) line endings to LF (`\n`).
///
/// Returns `Cow::Borrowed` (zero allocation) when no `\r` is present.
///
/// **Stability: stable** - breaking changes require a major version bump.
pub use parsers::frontmatter::normalize_line_endings;

// Internal re-exports (not part of the stable API).
// These types are needed by fuzz/bench/test targets or leak through LintConfig.
// They are hidden from rustdoc and namespaced to discourage external use.
// NOTE: normalize_line_endings is intentionally NOT re-exported here;
// it is a stable crate-root export (see above).
#[doc(hidden)]
#[cfg(any(test, feature = "__internal"))]
pub mod __internal {
    pub use crate::parsers::ImportCache;
    pub use crate::parsers::frontmatter::{FrontmatterParts, split_frontmatter};
    pub use crate::parsers::json::parse_json_config;
    pub use crate::parsers::markdown::Import;
    pub use crate::parsers::markdown::{
        MAX_REGEX_INPUT_SIZE, MarkdownLink, XmlTag, check_xml_balance,
        check_xml_balance_with_content_end, extract_imports, extract_markdown_links,
        extract_xml_tags, sanitize_for_pulldown_cmark,
    };
    pub use crate::schemas::cross_platform::is_instruction_file;
}

#[cfg(test)]
mod i18n_tests {
    use rust_i18n::t;
    use std::sync::Mutex;

    // Mutex to serialize i18n tests since set_locale is global state
    static LOCALE_MUTEX: Mutex<()> = Mutex::new(());

    /// Verify that English translations load correctly and are not raw keys.
    #[test]
    fn test_english_translations_load() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("en");

        // Sample translations from each section
        let xml_msg = t!("rules.xml_001.message", tag = "test");
        assert!(
            xml_msg.contains("Unclosed XML tag"),
            "Expected English translation, got: {}",
            xml_msg
        );

        let cli_validating = t!("cli.validating");
        assert_eq!(cli_validating, "Validating:");

        let lsp_label = t!("lsp.suggestion_label");
        assert_eq!(lsp_label, "Suggestion:");
    }

    /// Verify that Spanish translations load correctly.
    #[test]
    fn test_spanish_translations_load() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("es");

        let xml_msg = t!("rules.xml_001.message", tag = "test");
        assert!(
            xml_msg.contains("Etiqueta XML sin cerrar"),
            "Expected Spanish translation, got: {}",
            xml_msg
        );

        let cli_validating = t!("cli.validating");
        assert_eq!(cli_validating, "Validando:");

        rust_i18n::set_locale("en");
    }

    /// Verify that Chinese (Simplified) translations load correctly.
    #[test]
    fn test_chinese_translations_load() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("zh-CN");

        let xml_msg = t!("rules.xml_001.message", tag = "test");
        assert!(
            xml_msg.contains("\u{672A}\u{5173}\u{95ED}"),
            "Expected Chinese translation, got: {}",
            xml_msg
        );

        let cli_validating = t!("cli.validating");
        assert!(
            cli_validating.contains("\u{6B63}\u{5728}\u{9A8C}\u{8BC1}"),
            "Expected Chinese translation, got: {}",
            cli_validating
        );

        rust_i18n::set_locale("en");
    }

    /// Verify that unsupported locale falls back to English.
    #[test]
    fn test_fallback_to_english() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("fr"); // French not supported

        let msg = t!("cli.validating");
        assert_eq!(
            msg, "Validating:",
            "Should fall back to English, got: {}",
            msg
        );

        rust_i18n::set_locale("en");
    }

    /// Verify that translation keys with parameters resolve correctly.
    #[test]
    fn test_parameterized_translations() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("en");

        let msg = t!("rules.as_004.message", name = "TestName");
        assert!(
            msg.contains("TestName"),
            "Parameter should be interpolated, got: {}",
            msg
        );
        assert!(
            msg.contains("must be 1-64 characters"),
            "Message template should be filled, got: {}",
            msg
        );
    }

    /// Verify that all supported locales have key sections.
    #[test]
    fn test_available_locales() {
        let locales = rust_i18n::available_locales!();
        assert!(
            locales.contains(&"en"),
            "English locale must be available, found: {:?}",
            locales
        );
        assert!(
            locales.contains(&"es"),
            "Spanish locale must be available, found: {:?}",
            locales
        );
        assert!(
            locales.contains(&"zh-CN"),
            "Chinese locale must be available, found: {:?}",
            locales
        );
    }

    /// Verify that rule IDs are never translated (they stay as-is in diagnostics).
    #[test]
    fn test_rule_ids_not_translated() {
        use super::*;
        use std::path::Path;

        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("es"); // Spanish

        let config = config::LintConfig::default();
        let content = "---\nname: test\n---\nSome content";
        let path = Path::new("test/.claude/skills/test/SKILL.md");

        let validator = rules::skill::SkillValidator;
        let diagnostics = validator.validate(path, content, &config);

        // Rule IDs should always be in English format
        for diag in &diagnostics {
            assert!(
                diag.rule.is_ascii(),
                "Rule ID should be ASCII: {}",
                diag.rule
            );
        }

        rust_i18n::set_locale("en");
    }

    /// Verify that Spanish locale produces localized diagnostic messages.
    #[test]
    fn test_spanish_diagnostics() {
        use super::*;
        use std::path::Path;

        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("es");

        let config = config::LintConfig::default();
        let content = "<unclosed>";
        let path = Path::new("test/CLAUDE.md");

        let validator = rules::xml::XmlValidator;
        let diagnostics = validator.validate(path, content, &config);

        assert!(!diagnostics.is_empty(), "Should produce diagnostics");
        let xml_diag = diagnostics.iter().find(|d| d.rule == "XML-001").unwrap();
        assert!(
            xml_diag.message.contains("Etiqueta XML sin cerrar"),
            "Message should be in Spanish, got: {}",
            xml_diag.message
        );

        rust_i18n::set_locale("en");
    }

    /// Verify that new suggestion locale keys from #323 resolve to real text.
    #[test]
    fn test_new_suggestion_keys_resolve() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("en");

        // Parse error suggestions added in #323
        macro_rules! assert_key_resolves {
            ($key:expr) => {
                let value = t!($key);
                assert!(
                    !value.starts_with("rules."),
                    "Locale key '{}' should resolve to text, not raw key path: {}",
                    $key,
                    value
                );
            };
        }
        assert_key_resolves!("rules.as_016.suggestion");
        assert_key_resolves!("rules.cc_hk_012.suggestion");
        assert_key_resolves!("rules.mcp_007.suggestion");
        assert_key_resolves!("rules.cc_pl_006.suggestion");
        assert_key_resolves!("rules.cc_ag_007.parse_error_suggestion");
        assert_key_resolves!("rules.cdx_000.suggestion");
        assert_key_resolves!("rules.file_read_error_suggestion");
        assert_key_resolves!("rules.xp_004_read_error_suggestion");

        // CDX-000 message (migrated from format!() to t!())
        let cdx_msg = t!("rules.cdx_000.message", error = "test error");
        assert!(
            cdx_msg.contains("test error"),
            "CDX-000 message should interpolate error param, got: {}",
            cdx_msg
        );

        // file::read and XP-004 read error messages
        let file_msg = t!("rules.file_read_error", error = "permission denied");
        assert!(
            file_msg.contains("permission denied"),
            "file_read_error should interpolate error param, got: {}",
            file_msg
        );
        let xp_msg = t!("rules.xp_004_read_error", error = "not found");
        assert!(
            xp_msg.contains("not found"),
            "xp_004_read_error should interpolate error param, got: {}",
            xp_msg
        );
    }

    /// Verify that keys across all sections resolve to human-readable text,
    /// not raw key paths. This catches the bug where i18n!() path resolution
    /// fails and t!() silently returns the key itself.
    #[test]
    fn test_keys_resolve_to_text_not_raw_paths() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("en");

        // Helper: assert a key resolves to real text (not the key path itself)
        macro_rules! assert_not_raw_key {
            ($key:expr) => {
                let value = t!($key);
                assert!(
                    (!value.starts_with("rules."))
                        && (!value.starts_with("cli."))
                        && (!value.starts_with("lsp."))
                        && (!value.starts_with("core.")),
                    "Key '{}' returned raw path instead of translated text: {}",
                    $key,
                    value
                );
            };
            ($key:expr, $($param:ident = $val:expr),+) => {
                let value = t!($key, $($param = $val),+);
                assert!(
                    (!value.starts_with("rules."))
                        && (!value.starts_with("cli."))
                        && (!value.starts_with("lsp."))
                        && (!value.starts_with("core.")),
                    "Key '{}' returned raw path instead of translated text: {}",
                    $key,
                    value
                );
            };
        }

        // Rules section - sample messages from different validators
        assert_not_raw_key!("rules.as_001.message");
        assert_not_raw_key!("rules.as_004.message", name = "test");
        assert_not_raw_key!("rules.cc_ag_009.message", tool = "x", known = "y");
        assert_not_raw_key!("rules.xml_001.message", tag = "div");
        assert_not_raw_key!("rules.cc_hk_001.message");
        assert_not_raw_key!("rules.pe_003.message");
        assert_not_raw_key!("rules.cc_mem_009.message");

        // Rules section - suggestions
        assert_not_raw_key!("rules.as_001.suggestion");
        assert_not_raw_key!("rules.as_004.suggestion");
        assert_not_raw_key!("rules.cc_ag_009.suggestion");

        // Rules section - assumptions
        assert_not_raw_key!("rules.cc_hk_010.assumption");
        assert_not_raw_key!("rules.mcp_008.assumption");

        // CLI section
        assert_not_raw_key!("cli.validating");
        assert_not_raw_key!("cli.no_issues_found");
        assert_not_raw_key!(
            "cli.found_errors_warnings",
            errors = "1",
            error_word = "error",
            warnings = "0",
            warning_word = "warnings"
        );

        // LSP section
        assert_not_raw_key!("lsp.suggestion_label");

        // Core section
        assert_not_raw_key!("core.error.file_read", path = "/tmp/test");
    }

    /// Regression guard for v0.20.0 #799. Walks `src/**/*.rs`,
    /// extracts every literal `t!("rules.X.Y")` reference, and asserts
    /// the key resolves to real text (not the key path). Catches new
    /// rules added to code without matching locale entries before they
    /// reach a release.
    ///
    /// Runs at test time only; reads `CARGO_MANIFEST_DIR` so it works
    /// from any worktree. If Windows path handling ever breaks this,
    /// it's also runnable via `cargo test` from CI which uses Linux.
    #[test]
    fn test_every_rule_locale_key_referenced_in_source_resolves() {
        let _lock = LOCALE_MUTEX.lock().unwrap();
        rust_i18n::set_locale("en");

        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR set by cargo");
        let src_dir = std::path::Path::new(&manifest_dir).join("src");

        // Walk src/** and collect t!("rules.X.Y") refs.
        let mut keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        fn walk(dir: &std::path::Path, keys: &mut std::collections::BTreeSet<String>) {
            let Ok(rd) = std::fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, keys);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    if let Ok(text) = std::fs::read_to_string(&p) {
                        // Match t!( with a word boundary (preceding
                        // byte must NOT be an identifier char) so we
                        // don't capture `format!(` or `assert!(` as
                        // "t!(". Then scan the immediate call for a
                        // `"rules.X.Y"` literal. We bound the search
                        // to the same line group to avoid picking up
                        // a `rules.*` string from an unrelated later
                        // macro call.
                        let mut i = 0;
                        let bytes = text.as_bytes();
                        while i + 10 < bytes.len() {
                            if &bytes[i..i + 3] == b"t!(" {
                                let prev_is_ident = i > 0
                                    && (bytes[i - 1].is_ascii_alphanumeric()
                                        || bytes[i - 1] == b'_');
                                if !prev_is_ident {
                                    // Search only within this macro
                                    // call — stop at the matching `)`
                                    // (approximate: stop at the next
                                    // unescaped `)` after the first
                                    // literal). For our purposes —
                                    // literal keys, no nested calls —
                                    // scanning up to 300 chars ahead
                                    // is plenty and avoids grabbing
                                    // strings from a later call.
                                    let end_pos = bytes
                                        .get(i..(i + 300).min(bytes.len()))
                                        .and_then(|s| s.iter().position(|&b| b == b')'))
                                        .map(|p| p + i)
                                        .unwrap_or(bytes.len().min(i + 300));
                                    let rest = &text[i..end_pos];
                                    if let Some(q) = rest.find(r#""rules."#)
                                        && let Some(endq) = rest[q + 1..].find('"')
                                    {
                                        let key = &rest[q + 1..q + 1 + endq];
                                        // Only accept strict rules.a_b.c shape
                                        if key.matches('.').count() == 2
                                            && key.chars().all(|c| {
                                                c.is_ascii_lowercase()
                                                    || c.is_ascii_digit()
                                                    || c == '.'
                                                    || c == '_'
                                            })
                                        {
                                            keys.insert(key.to_string());
                                        }
                                    }
                                }
                            }
                            i += 1;
                        }
                    }
                }
            }
        }
        walk(&src_dir, &mut keys);

        assert!(
            !keys.is_empty(),
            "scanner found no t!(\"rules.*.*\") calls under {}; check the walker",
            src_dir.display()
        );

        // Load the compiled en.yml at test time and check membership.
        // Using the YAML directly (rather than rust_i18n::t!, which
        // requires static string literals) lets us iterate over the
        // collected keys at runtime.
        let locale_path = std::path::Path::new(&manifest_dir)
            .join("locales")
            .join("en.yml");
        let locale_text = std::fs::read_to_string(&locale_path).unwrap_or_else(|e| {
            panic!("read {}: {}", locale_path.display(), e);
        });
        // Parse — we only need key membership, not interpolation.
        // Extracting via a simple manual parse avoids adding a test-
        // only serde_yaml dependency: we just ask "does the indented
        // structure contain a node for rules.X.Y.message?". Since
        // locale keys are snake_case with no colons inside them,
        // scanning for a `X:\n    Y:\n        Z:` pattern is enough.
        // Simpler still: since every key we produce lives under
        // `rules: / <rule_id>: / <field>:`, we can just split on
        // "\n  <rule_id>:\n" and "\n    <field>:" marker pairs.

        // Build a flat set of "rules.X.Y" keys from the YAML text.
        let mut available: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let mut current_rule: Option<String> = None;
        let mut inside_rules = false;
        for line in locale_text.lines() {
            if line.starts_with("rules:") {
                inside_rules = true;
                continue;
            }
            // A 1-indented root key (no leading space) after `rules:`
            // ends the block.
            if inside_rules
                && !line.is_empty()
                && !line.starts_with(' ')
                && !line.starts_with('#')
                && !line.starts_with("rules:")
            {
                inside_rules = false;
            }
            if !inside_rules {
                continue;
            }
            // `  rule_id:` at 2-space indent
            if let Some(tail) = line.strip_prefix("  ")
                && !tail.starts_with(' ')
                && !tail.starts_with('#')
                && let Some(name) = tail.strip_suffix(':')
                && name
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
            {
                current_rule = Some(name.to_string());
                continue;
            }
            // `    field:` or `    field: "…"` at 4-space indent
            if let Some(tail) = line.strip_prefix("    ")
                && !tail.starts_with(' ')
                && !tail.starts_with('#')
                && let Some(colon) = tail.find(':')
                && let Some(rule) = &current_rule
            {
                let field = &tail[..colon];
                if field
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
                {
                    available.insert(format!("rules.{rule}.{field}"));
                }
            }
        }

        let mut missing: Vec<String> = Vec::new();
        for k in &keys {
            if !available.contains(k) {
                missing.push(k.clone());
            }
        }

        if !missing.is_empty() {
            let mut msg = format!(
                "{} rule-locale keys are referenced in source but missing from locales/en.yml:\n",
                missing.len()
            );
            for k in &missing {
                msg.push_str(&format!("  {}\n", k));
            }
            panic!("{}", msg);
        }
    }
}
