//! Claude Code plugin marketplace validation (CC-MP-001 to CC-MP-005) for
//! `.claude-plugin/marketplace.json`.
//!
//! Mirrors the `claude plugin validate` messages in the marketplace reference
//! ("Validation messages") and the load-time failures it lists:
//!
//! - CC-MP-001: the file doesn't parse, or `name`, `owner.name`, `plugins`,
//!   or an entry's `name` or `source` is missing
//! - CC-MP-002: a marketplace or plugin name Claude Code refuses or reserves
//! - CC-MP-003: a plugin `source` that fails validation, a duplicate entry
//!   name, or a `renames` target that doesn't resolve
//! - CC-MP-004: entry fields that fail to load: `hooks` as a path or array,
//!   `headersHelper` without `"strict": false`, and `"strict": false` with
//!   component fields for a plugin that has its own `plugin.json`
//! - CC-MP-005: warnings: unknown keys (ignored), no description, no plugins,
//!   an entry version that differs from `plugin.json`, and headers that only
//!   apply to archive sources

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::claude_json_spans::{JsonLocator, Seg},
    rules::plugin::{ReservedPluginName, is_anthropic_party, reserved_plugin_name},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use serde_json::Value;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CC-MP-001",
    "CC-MP-002",
    "CC-MP-003",
    "CC-MP-004",
    "CC-MP-005",
];

/// Official, community and directory names, "Reserved unless the marketplace
/// comes from a `github` or `git` marketplace source under
/// `github.com/anthropics/`".
const OFFICIAL_MARKETPLACE_NAMES: &[&str] = &[
    "claude-code-marketplace",
    "claude-code-plugins",
    "claude-plugins-official",
    "anthropic-marketplace",
    "anthropic-plugins",
    "agent-skills",
    "anthropic-agent-skills",
    "life-sciences",
    "knowledge-work-plugins",
    "claude-for-legal",
    "claude-for-financial-services",
    "financial-services-plugins",
    "first-party-plugins",
    "claude-tag-plugins",
    "claude-community",
    "claude-plugins-community",
    "healthcare",
    "anthropic-plugin-directory",
    "claude-plugin-directory",
];

/// Names Claude Code uses itself.
const INTERNAL_MARKETPLACE_NAMES: &[&str] = &[
    "inline",
    "builtin",
    "skills-dir",
    "synced",
    "claude-plugin-test",
];

/// "reserved in any casing".
const PACKAGE_MANAGER_NAMES: &[&str] = &["npm", "pip", "uv", "cargo", "github", "gh"];

/// "Member names that every JavaScript object has" (v2.1.296+).
const JS_MEMBER_NAMES: &[&str] = &[
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
];

const TOP_LEVEL_KEYS: &[&str] = &[
    "$schema",
    "name",
    "owner",
    "plugins",
    "description",
    "version",
    "metadata",
    "forceRemoveDeletedPlugins",
    "allowCrossMarketplaceDependenciesOn",
    "renames",
];

const METADATA_KEYS: &[&str] = &["description", "version", "pluginRoot"];

/// Entry fields: the entry's own fields plus every `plugin.json` field except
/// the directory listing fields (`icon`, `documentationUrl`, `supportUrl`,
/// `privacyPolicyUrl`, `termsOfServiceUrl`), which an entry reports as
/// unknown.
const ENTRY_KEYS: &[&str] = &[
    "name",
    "source",
    "description",
    "version",
    "category",
    "tags",
    "strict",
    "relevance",
    "dependencies",
    "defaultEnabled",
    "displayName",
    "metadata",
    "headers",
    "headersHelper",
    "$schema",
    "author",
    "homepage",
    "repository",
    "license",
    "keywords",
    "settings",
    "userConfig",
    "types",
    "channels",
    "skills",
    "commands",
    "agents",
    "hooks",
    "mcpServers",
    "lspServers",
    "outputStyles",
    "workflows",
    "themes",
    "monitors",
    "experimental",
];

/// Entry fields that `strict: false` turns into a conflict.
const COMPONENT_FIELDS: &[&str] = &[
    "commands",
    "agents",
    "skills",
    "hooks",
    "outputStyles",
    "themes",
];

/// Plugin source types and their required fields.
const SOURCE_TYPES: &[(&str, &[&str])] = &[
    ("github", &["repo"]),
    ("url", &["url"]),
    ("git-subdir", &["url", "path"]),
    ("npm", &["package"]),
    ("archive", &["url"]),
    ("command", &["command"]),
];

pub struct ClaudeMarketplaceValidator;

struct Ctx<'a> {
    path: &'a Path,
    locator: JsonLocator<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl Ctx<'_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        segs: &[Seg<'_>],
        on_key: bool,
        message: String,
        suggestion: String,
    ) {
        let (line, col) = if on_key {
            self.locator.key_pos(segs)
        } else {
            self.locator.value_pos(segs)
        };
        let make = match level {
            DiagnosticLevel::Error => Diagnostic::error,
            DiagnosticLevel::Warning => Diagnostic::warning,
            DiagnosticLevel::Info => Diagnostic::info,
        };
        self.diagnostics.push(
            make(self.path.to_path_buf(), line, col, rule, message).with_suggestion(suggestion),
        );
    }
}

/// Letters, digits, `.`, `_`, `-`, starting with a letter or digit.
fn is_valid_id(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|c| c.is_ascii_alphanumeric())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

fn has_control_or_bidi(name: &str) -> bool {
    name.chars().any(|c| {
        c.is_control()
            || matches!(c, '\u{200E}' | '\u{200F}' | '\u{061C}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
    })
}

/// The reserved name `name` spells, if any: exact names, "a name that
/// differs from a reserved name only by a trailing dot, or by a symbol other
/// than an underscore in place of a hyphen", and the case-insensitive
/// package-manager names.
fn reserved_marketplace_name(name: &str) -> Option<String> {
    if PACKAGE_MANAGER_NAMES
        .iter()
        .any(|n| n.eq_ignore_ascii_case(name))
    {
        return Some(name.to_ascii_lowercase());
    }
    if JS_MEMBER_NAMES.contains(&name) || INTERNAL_MARKETPLACE_NAMES.contains(&name) {
        return Some(name.to_string());
    }
    let spelled: String = name
        .trim_end_matches('.')
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    OFFICIAL_MARKETPLACE_NAMES
        .iter()
        .chain(INTERNAL_MARKETPLACE_NAMES)
        .find(|reserved| **reserved == spelled)
        .map(|reserved| reserved.to_string())
}

impl Validator for ClaudeMarketplaceValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        if !config.rules().plugins {
            return Vec::new();
        }
        let mut cx = Ctx {
            path,
            locator: JsonLocator::new(content),
            diagnostics: Vec::new(),
        };
        let on = |id: &str| config.is_rule_enabled(id);

        let root: Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if on("CC-MP-001") {
                    let line = e.line().max(1);
                    cx.diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            e.column(),
                            "CC-MP-001",
                            t!("rules.cc_mp_001.parse", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.cc_mp_001.parse_suggestion")),
                    );
                }
                return cx.diagnostics;
            }
        };
        let Some(top) = root.as_object() else {
            if on("CC-MP-001") {
                cx.push(
                    DiagnosticLevel::Error,
                    "CC-MP-001",
                    &[],
                    false,
                    t!("rules.cc_mp_001.not_object").to_string(),
                    t!("rules.cc_mp_001.not_object_suggestion").to_string(),
                );
            }
            return cx.diagnostics;
        };
        // Anthropic's own marketplaces use the reserved names legitimately
        // (they come from github.com/anthropics/).
        let anthropic = is_anthropic_party(top.get("owner"));
        let plugin_root = top
            .get("metadata")
            .and_then(|m| m.get("pluginRoot"))
            .and_then(Value::as_str);
        let marketplace_root = path
            .parent()
            .and_then(Path::parent)
            .unwrap_or_else(|| Path::new("."));

        // ---- CC-MP-001: required top-level fields ----
        let name = top.get("name").and_then(Value::as_str).map(str::trim);
        if on("CC-MP-001") {
            if name.is_none_or(str::is_empty) {
                cx.push(
                    DiagnosticLevel::Error,
                    "CC-MP-001",
                    &[Seg::Key("name")],
                    true,
                    t!("rules.cc_mp_001.missing", field = "name").to_string(),
                    t!("rules.cc_mp_001.missing_suggestion", field = "name").to_string(),
                );
            }
            let owner_name = top
                .get("owner")
                .and_then(|o| o.get("name"))
                .and_then(Value::as_str)
                .map(str::trim);
            if owner_name.is_none_or(str::is_empty) {
                cx.push(
                    DiagnosticLevel::Error,
                    "CC-MP-001",
                    &[Seg::Key("owner")],
                    true,
                    t!("rules.cc_mp_001.missing", field = "owner.name").to_string(),
                    t!("rules.cc_mp_001.missing_suggestion", field = "owner.name").to_string(),
                );
            }
            if !top.get("plugins").is_some_and(Value::is_array) {
                cx.push(
                    DiagnosticLevel::Error,
                    "CC-MP-001",
                    &[Seg::Key("plugins")],
                    true,
                    t!("rules.cc_mp_001.missing", field = "plugins").to_string(),
                    t!("rules.cc_mp_001.missing_suggestion", field = "plugins").to_string(),
                );
            }
        }

        // ---- CC-MP-002: marketplace name ----
        if let Some(name) = name.filter(|n| !n.is_empty())
            && on("CC-MP-002")
        {
            let problem = if has_control_or_bidi(name) {
                Some(t!("rules.cc_mp_002.control", name = name))
            } else if !name.is_ascii() {
                Some(t!("rules.cc_mp_002.impersonates", name = name))
            } else if name.contains(char::is_whitespace) {
                Some(t!("rules.cc_mp_002.spaces", name = name))
            } else if name == "." || name.contains("..") || name.contains(['/', '\\']) {
                Some(t!("rules.cc_mp_002.path", name = name))
            } else if !is_valid_id(name) {
                Some(t!("rules.cc_mp_002.charset", name = name))
            } else if name.to_ascii_lowercase().starts_with("claudeai-") {
                Some(t!("rules.cc_mp_002.claudeai", name = name))
            } else if let Some(reserved) = reserved_marketplace_name(name)
                && !(anthropic && OFFICIAL_MARKETPLACE_NAMES.contains(&reserved.as_str()))
            {
                Some(t!(
                    "rules.cc_mp_002.reserved",
                    name = name,
                    reserved = reserved.as_str()
                ))
            } else {
                None
            };
            if let Some(message) = problem {
                cx.push(
                    DiagnosticLevel::Error,
                    "CC-MP-002",
                    &[Seg::Key("name")],
                    false,
                    message.to_string(),
                    t!("rules.cc_mp_002.suggestion").to_string(),
                );
            }
        }

        // ---- CC-MP-005: top-level warnings ----
        if on("CC-MP-005") {
            for key in top.keys() {
                if !TOP_LEVEL_KEYS.contains(&key.as_str()) {
                    cx.push(
                        DiagnosticLevel::Warning,
                        "CC-MP-005",
                        &[Seg::Key(key.as_str())],
                        true,
                        t!("rules.cc_mp_005.unknown_key", key = key.as_str()).to_string(),
                        t!("rules.cc_mp_005.unknown_key_suggestion").to_string(),
                    );
                }
            }
            if let Some(metadata) = top.get("metadata").and_then(Value::as_object) {
                for key in metadata.keys() {
                    if !METADATA_KEYS.contains(&key.as_str()) {
                        cx.push(
                            DiagnosticLevel::Warning,
                            "CC-MP-005",
                            &[Seg::Key("metadata"), Seg::Key(key.as_str())],
                            true,
                            t!(
                                "rules.cc_mp_005.unknown_key",
                                key = format!("metadata.{key}")
                            )
                            .to_string(),
                            t!("rules.cc_mp_005.unknown_key_suggestion").to_string(),
                        );
                    }
                }
            }
            let described = top.get("description").and_then(Value::as_str).is_some()
                || top
                    .get("metadata")
                    .and_then(|m| m.get("description"))
                    .and_then(Value::as_str)
                    .is_some();
            if !described {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CC-MP-005",
                    &[],
                    false,
                    t!("rules.cc_mp_005.no_description").to_string(),
                    t!("rules.cc_mp_005.no_description_suggestion").to_string(),
                );
            }
            if top
                .get("plugins")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CC-MP-005",
                    &[Seg::Key("plugins")],
                    false,
                    t!("rules.cc_mp_005.no_plugins").to_string(),
                    t!("rules.cc_mp_005.no_plugins_suggestion").to_string(),
                );
            }
        }

        let entries: &[Value] = top
            .get("plugins")
            .and_then(Value::as_array)
            .map_or(&[], Vec::as_slice);
        let mut seen: Vec<&str> = Vec::new();
        for (i, entry) in entries.iter().enumerate() {
            let base = [Seg::Key("plugins"), Seg::Index(i)];
            let at = |key: &'static str| [Seg::Key("plugins"), Seg::Index(i), Seg::Key(key)];
            let Some(obj) = entry.as_object() else {
                if on("CC-MP-001") {
                    cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-001",
                        &base,
                        false,
                        t!("rules.cc_mp_001.entry_not_object", index = i).to_string(),
                        t!("rules.cc_mp_001.entry_suggestion").to_string(),
                    );
                }
                continue;
            };
            let entry_name = obj.get("name").and_then(Value::as_str).map(str::trim);
            if on("CC-MP-001") {
                for field in ["name", "source"] {
                    let missing = match obj.get(field) {
                        None | Some(Value::Null) => true,
                        Some(Value::String(s)) => s.trim().is_empty(),
                        Some(_) => false,
                    };
                    if missing {
                        cx.push(
                            DiagnosticLevel::Error,
                            "CC-MP-001",
                            &base,
                            false,
                            t!("rules.cc_mp_001.entry_missing", index = i, field = field)
                                .to_string(),
                            t!("rules.cc_mp_001.entry_suggestion").to_string(),
                        );
                    }
                }
            }

            // ---- CC-MP-002: entry names ----
            if let Some(entry_name) = entry_name.filter(|n| !n.is_empty())
                && on("CC-MP-002")
            {
                let invalid = if has_control_or_bidi(entry_name) {
                    Some(t!("rules.cc_mp_002.plugin_control", name = entry_name))
                } else if entry_name.contains(char::is_whitespace) {
                    Some(t!("rules.cc_mp_002.plugin_spaces", name = entry_name))
                } else if !is_valid_id(entry_name) {
                    Some(t!("rules.cc_mp_002.plugin_charset", name = entry_name))
                } else {
                    None
                };
                if let Some(message) = invalid {
                    cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-002",
                        &at("name"),
                        false,
                        message.to_string(),
                        t!("rules.cc_mp_002.plugin_suggestion").to_string(),
                    );
                } else if !anthropic && !is_anthropic_party(obj.get("author")) {
                    match reserved_plugin_name(entry_name) {
                        Some(ReservedPluginName::Error) => cx.push(
                            DiagnosticLevel::Error,
                            "CC-MP-002",
                            &at("name"),
                            false,
                            t!("rules.cc_mp_002.plugin_reserved", name = entry_name).to_string(),
                            t!("rules.cc_mp_002.plugin_suggestion").to_string(),
                        ),
                        Some(ReservedPluginName::Warning) => cx.push(
                            DiagnosticLevel::Warning,
                            "CC-MP-002",
                            &at("name"),
                            false,
                            t!("rules.cc_mp_002.plugin_reads_as", name = entry_name).to_string(),
                            t!("rules.cc_mp_002.plugin_suggestion").to_string(),
                        ),
                        None => {}
                    }
                }
            }

            // ---- CC-MP-003: duplicate names and sources ----
            if on("CC-MP-003") {
                if let Some(entry_name) = entry_name.filter(|n| !n.is_empty()) {
                    if seen.contains(&entry_name) {
                        cx.push(
                            DiagnosticLevel::Error,
                            "CC-MP-003",
                            &at("name"),
                            false,
                            t!("rules.cc_mp_003.duplicate", name = entry_name).to_string(),
                            t!("rules.cc_mp_003.duplicate_suggestion").to_string(),
                        );
                    } else {
                        seen.push(entry_name);
                    }
                }
                match obj.get("source") {
                    Some(Value::String(source)) => {
                        let message = relative_source_problem(source, plugin_root);
                        if let Some(message) = message {
                            cx.push(
                                DiagnosticLevel::Error,
                                "CC-MP-003",
                                &at("source"),
                                false,
                                message,
                                t!("rules.cc_mp_003.source_suggestion").to_string(),
                            );
                        }
                    }
                    Some(Value::Object(source)) => {
                        let kind = source.get("source").and_then(Value::as_str);
                        let required = kind.and_then(|k| {
                            SOURCE_TYPES
                                .iter()
                                .find(|(name, _)| *name == k)
                                .map(|(_, fields)| *fields)
                        });
                        let problem = match (kind, required) {
                            (Some("unsupported"), _) => {
                                Some(t!("rules.cc_mp_003.placeholder").to_string())
                            }
                            (_, None) => Some(
                                t!(
                                    "rules.cc_mp_003.unknown_type",
                                    kind = kind.unwrap_or("(missing)")
                                )
                                .to_string(),
                            ),
                            (Some(k), Some(fields)) => fields
                                .iter()
                                .find(|f| !source.get(**f).is_some_and(Value::is_string))
                                .map(|f| {
                                    t!("rules.cc_mp_003.missing_field", kind = k, field = *f)
                                        .to_string()
                                }),
                            (None, Some(_)) => None,
                        };
                        if let Some(message) = problem {
                            cx.push(
                                DiagnosticLevel::Error,
                                "CC-MP-003",
                                &at("source"),
                                false,
                                message,
                                t!("rules.cc_mp_003.source_suggestion").to_string(),
                            );
                        }
                    }
                    Some(other) if !other.is_null() => cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-003",
                        &at("source"),
                        false,
                        t!(
                            "rules.cc_mp_003.unknown_type",
                            kind = crate::rules::json_type_name(other)
                        )
                        .to_string(),
                        t!("rules.cc_mp_003.source_suggestion").to_string(),
                    ),
                    _ => {}
                }
            }

            let source_kind = match obj.get("source") {
                Some(Value::String(_)) => Some("relative"),
                Some(Value::Object(s)) => s.get("source").and_then(Value::as_str),
                _ => None,
            };
            let relative_dir = obj
                .get("source")
                .and_then(Value::as_str)
                .map(|s| resolve_relative_source(marketplace_root, s, plugin_root));
            let local_manifest = relative_dir
                .as_ref()
                .map(|dir| dir.join(".claude-plugin").join("plugin.json"))
                .filter(|p| config.fs().is_file(p));

            // ---- CC-MP-004: entry fields that fail to load ----
            if on("CC-MP-004") {
                if let Some(hooks) = obj.get("hooks")
                    && (hooks.is_string() || hooks.is_array())
                {
                    cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-004",
                        &at("hooks"),
                        true,
                        t!("rules.cc_mp_004.hooks", index = i).to_string(),
                        t!("rules.cc_mp_004.hooks_suggestion").to_string(),
                    );
                }
                let strict_false = obj.get("strict").and_then(Value::as_bool) == Some(false);
                if obj.contains_key("headersHelper") && !strict_false {
                    cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-004",
                        &at("headersHelper"),
                        true,
                        t!(
                            "rules.cc_mp_004.headers_helper",
                            name = entry_name.unwrap_or("?")
                        )
                        .to_string(),
                        t!("rules.cc_mp_004.headers_helper_suggestion").to_string(),
                    );
                }
                if strict_false && local_manifest.is_some() {
                    let components: Vec<&str> = COMPONENT_FIELDS
                        .iter()
                        .copied()
                        .filter(|f| obj.contains_key(*f))
                        .collect();
                    if !components.is_empty() {
                        cx.push(
                            DiagnosticLevel::Error,
                            "CC-MP-004",
                            &at("strict"),
                            true,
                            t!(
                                "rules.cc_mp_004.strict_conflict",
                                name = entry_name.unwrap_or("?"),
                                fields = components.join(", ")
                            )
                            .to_string(),
                            t!("rules.cc_mp_004.strict_conflict_suggestion").to_string(),
                        );
                    }
                }
            }

            // ---- CC-MP-005: entry warnings ----
            if on("CC-MP-005") {
                for key in obj.keys() {
                    if !ENTRY_KEYS.contains(&key.as_str()) {
                        cx.push(
                            DiagnosticLevel::Warning,
                            "CC-MP-005",
                            &[Seg::Key("plugins"), Seg::Index(i), Seg::Key(key.as_str())],
                            true,
                            t!(
                                "rules.cc_mp_005.unknown_key",
                                key = format!("plugins[{i}].{key}")
                            )
                            .to_string(),
                            t!("rules.cc_mp_005.unknown_key_suggestion").to_string(),
                        );
                    }
                }
                if source_kind != Some("archive") {
                    for key in ["headers", "headersHelper"] {
                        if obj.contains_key(key) {
                            cx.push(
                                DiagnosticLevel::Warning,
                                "CC-MP-005",
                                &at(if key == "headers" {
                                    "headers"
                                } else {
                                    "headersHelper"
                                }),
                                true,
                                t!(
                                    "rules.cc_mp_005.headers_no_effect",
                                    name = entry_name.unwrap_or("?"),
                                    key = key
                                )
                                .to_string(),
                                t!("rules.cc_mp_005.headers_no_effect_suggestion").to_string(),
                            );
                        }
                    }
                }
                if let (Some(entry_version), Some(manifest_path)) =
                    (obj.get("version").and_then(Value::as_str), &local_manifest)
                    && let Ok(text) = config.fs().read_to_string(manifest_path)
                    && let Ok(manifest) = serde_json::from_str::<Value>(&text)
                    && let Some(manifest_version) = manifest.get("version").and_then(Value::as_str)
                    && manifest_version != entry_version
                {
                    cx.push(
                        DiagnosticLevel::Warning,
                        "CC-MP-005",
                        &at("version"),
                        false,
                        t!(
                            "rules.cc_mp_005.version_mismatch",
                            entry = entry_version,
                            manifest = manifest_version
                        )
                        .to_string(),
                        t!("rules.cc_mp_005.version_mismatch_suggestion").to_string(),
                    );
                }
            }
        }

        // ---- CC-MP-003: renames ----
        if on("CC-MP-003")
            && let Some(renames) = top.get("renames").and_then(Value::as_object)
        {
            for (old, target) in renames {
                let ok = match target {
                    Value::Null => true,
                    Value::String(t) => seen.contains(&t.as_str()) || renames.contains_key(t),
                    _ => false,
                };
                if !ok {
                    cx.push(
                        DiagnosticLevel::Error,
                        "CC-MP-003",
                        &[Seg::Key("renames"), Seg::Key(old.as_str())],
                        false,
                        t!("rules.cc_mp_003.rename", old = old.as_str()).to_string(),
                        t!("rules.cc_mp_003.rename_suggestion").to_string(),
                    );
                }
            }
        }

        cx.diagnostics
    }
}

/// Why a relative-path plugin source fails validation, if it does.
fn relative_source_problem(source: &str, plugin_root: Option<&str>) -> Option<String> {
    if source.split(['/', '\\']).any(|seg| seg == "..") {
        return Some(t!("rules.cc_mp_003.traversal", source = source).to_string());
    }
    if source == "." {
        return None;
    }
    if let Some(rest) = source.strip_prefix("./") {
        if rest.contains('\\') {
            return Some(t!("rules.cc_mp_003.backslash", source = source).to_string());
        }
        return None;
    }
    // "A bare name is a single directory name with no `/`" under
    // metadata.pluginRoot.
    if plugin_root.is_some() && !source.contains(['/', '\\']) {
        return None;
    }
    Some(t!("rules.cc_mp_003.dot_slash", source = source).to_string())
}

fn resolve_relative_source(
    marketplace_root: &Path,
    source: &str,
    plugin_root: Option<&str>,
) -> std::path::PathBuf {
    match source.strip_prefix("./") {
        Some(rest) => marketplace_root.join(rest),
        None if source == "." => marketplace_root.to_path_buf(),
        None => match plugin_root {
            Some(root) => marketplace_root
                .join(root.trim_start_matches("./"))
                .join(source),
            None => marketplace_root.join(source),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;
    use tempfile::TempDir;

    fn validate(content: &str) -> Vec<Diagnostic> {
        ClaudeMarketplaceValidator.validate(
            Path::new("market/.claude-plugin/marketplace.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn rules<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    const VALID: &str = r#"{
  "name": "acme-tools",
  "description": "Acme plugins",
  "owner": { "name": "Acme" },
  "plugins": [
    { "name": "formatter", "source": "./plugins/formatter", "description": "Formats code" },
    { "name": "deployer", "source": { "source": "github", "repo": "acme/deployer" } }
  ]
}"#;

    #[test]
    fn valid_marketplace_is_clean() {
        let diagnostics = validate(VALID);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn cc_mp_001_required_fields() {
        let diagnostics = validate(r#"{"owner": {}, "plugins": [{"name": "x"}, 3]}"#);
        let hits = rules(&diagnostics, "CC-MP-001");
        // name, owner.name, entry 0 source, entry 1 not an object
        assert_eq!(hits.len(), 4, "{diagnostics:?}");
        let diagnostics = validate("{ not json");
        assert_eq!(rules(&diagnostics, "CC-MP-001").len(), 1);
    }

    #[test]
    fn cc_mp_002_names() {
        for (name, fires) in [
            ("my marketplace", true),
            ("claude-plugins-official", true),
            ("claude.code.plugins", true),
            ("NPM", true),
            ("claudeai-team", true),
            ("toString", true),
            ("-leading", true),
            ("acme-tools", false),
            ("acme.tools_v2", false),
        ] {
            let content = VALID.replace("acme-tools", name);
            let diagnostics = validate(&content);
            assert_eq!(
                !rules(&diagnostics, "CC-MP-002").is_empty(),
                fires,
                "{name}: {diagnostics:?}"
            );
        }
        // Anthropic's own marketplace uses an official name.
        let content = VALID
            .replace("acme-tools", "claude-plugins-official")
            .replace("\"Acme\"", "\"Anthropic\"");
        assert!(rules(&validate(&content), "CC-MP-002").is_empty());
    }

    #[test]
    fn cc_mp_002_reserved_plugin_entry_names() {
        let content = VALID
            .replace("\"formatter\"", "\"claude-formatter\"")
            .replace("\"deployer\"", "\"mcp-for-claude\"");
        let diagnostics = validate(&content);
        let hits = rules(&diagnostics, "CC-MP-002");
        assert_eq!(hits.len(), 2, "{diagnostics:?}");
        assert_eq!(hits[0].level, DiagnosticLevel::Error);
        assert_eq!(hits[1].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn cc_mp_003_sources_duplicates_renames() {
        let content = r#"{
  "name": "m", "description": "d", "owner": {"name": "o"},
  "metadata": {"pluginRoot": "./plugins"},
  "renames": {"old": "formatter", "gone": null, "bad": "nowhere"},
  "plugins": [
    {"name": "formatter", "source": "formatter"},
    {"name": "formatter", "source": "./a"},
    {"name": "b", "source": "plugins/b"},
    {"name": "c", "source": "./../c"},
    {"name": "d", "source": {"source": "github"}},
    {"name": "e", "source": {"source": "svn", "url": "x"}},
    {"name": "f", "source": "."}
  ]
}"#;
        let diagnostics = validate(content);
        let hits = rules(&diagnostics, "CC-MP-003");
        // duplicate, plugins/b, ./../c, github without repo, svn, renames.bad
        assert_eq!(hits.len(), 6, "{diagnostics:?}");
    }

    #[test]
    fn cc_mp_004_entry_hooks_and_strict() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join(".claude-plugin")).unwrap();
        fs::create_dir_all(root.join("plugins/p/.claude-plugin")).unwrap();
        fs::write(
            root.join("plugins/p/.claude-plugin/plugin.json"),
            r#"{"name": "p", "version": "2.0.0"}"#,
        )
        .unwrap();
        let content = r#"{
  "name": "m", "description": "d", "owner": {"name": "o"},
  "plugins": [
    {"name": "p", "source": "./plugins/p", "strict": false, "skills": ["./x"], "version": "1.0.0"},
    {"name": "q", "source": {"source": "archive", "url": "https://x/q.zip"}, "headersHelper": "./h.sh", "hooks": "./hooks.json"},
    {"name": "r", "source": "./plugins/r", "headers": {"A": "b"}}
  ]
}"#;
        let path = root.join(".claude-plugin/marketplace.json");
        fs::write(&path, content).unwrap();
        let diagnostics =
            ClaudeMarketplaceValidator.validate(&path, content, &LintConfig::default());
        let hits = rules(&diagnostics, "CC-MP-004");
        assert_eq!(hits.len(), 3, "{diagnostics:?}");
        let warnings = rules(&diagnostics, "CC-MP-005");
        // version mismatch, headers on a non-archive entry
        assert_eq!(warnings.len(), 2, "{diagnostics:?}");
    }

    #[test]
    fn cc_mp_005_unknown_keys_and_description() {
        let content = r#"{"name": "m", "owner": {"name": "o"}, "plugin": [], "plugins": [{"name": "a", "source": "./a", "icon": "x.png"}]}"#;
        let diagnostics = validate(content);
        let hits = rules(&diagnostics, "CC-MP-005");
        // `plugin` (typo), `icon` (directory listing field), no description
        assert_eq!(hits.len(), 3, "{diagnostics:?}");
        assert!(hits.iter().all(|d| d.level == DiagnosticLevel::Warning));
    }
}
