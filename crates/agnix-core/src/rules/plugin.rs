//! Plugin manifest validation (CC-PL-001 to CC-PL-023).
//!
//! Validates `.claude-plugin/plugin.json` manifests.

use crate::regex_util::static_regex;
use crate::rules::claude_json_spans::{JsonLocator, Seg};
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::{Validator, ValidatorMetadata},
};
use regex::Regex;
use rust_i18n::t;

static_regex!(fn forbidden_plugin_name, r"[\p{Cc}\p{Cf}\s@:/\\]");
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CC-PL-001",
    "CC-PL-002",
    "CC-PL-003",
    "CC-PL-004",
    "CC-PL-005",
    "CC-PL-006",
    "CC-PL-007",
    "CC-PL-008",
    "CC-PL-009",
    "CC-PL-010",
    "CC-PL-011",
    "CC-PL-012",
    "CC-PL-013",
    // CC-PL-014 is checked on plugin agent Markdown files (AgentValidator)
    // and CC-PL-020 on plugin skill bodies (SkillValidator).
    "CC-PL-015",
    "CC-PL-016",
    "CC-PL-017",
    "CC-PL-018",
    "CC-PL-019",
    "CC-PL-021",
    "CC-PL-022",
    "CC-PL-023",
];

/// Whether a plugin `name` matches the documented kebab-case identifier form:
/// lowercase letters, digits, and hyphens, not starting or ending with a hyphen.
///
/// Control and invisible characters fail this by construction, which is the
/// case Claude Code v2.1.247 started rejecting on the marketplace side.
fn is_kebab_case_plugin_name(name: &str) -> bool {
    !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Who reads a `plugin.json` that is not in `.claude-plugin/`. Claude Code
/// reads its manifest only from `.claude-plugin/plugin.json`, while GitHub
/// Copilot CLI and VS Code read a `plugin.json` at the plugin root: "Agent
/// Plugins requires the manifest at the plugin root", and legacy Copilot
/// plugins look in "`.plugin/plugin.json`, `plugin.json`,
/// `.github/plugin/plugin.json`, or `.claude-plugin/plugin.json`" (Copilot
/// CLI plugin reference, "File locations"); VS Code lists the Copilot format
/// as a root `plugin.json` and uses it "as the default when no other format
/// marker is found".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutsideManifest {
    /// An Agent Plugins or GitHub Copilot manifest; no Claude Code rule applies.
    OtherTool,
    /// A Claude Code manifest in the wrong place: it uses fields only Claude
    /// Code documents, or the run selects Claude Code and not Copilot.
    Claude,
    /// No signal either way.
    Unknown,
}

/// Manifest fields Claude Code documents and the legacy Copilot manifest
/// does not (plugins manifest reference, "Fields"; Copilot CLI plugin
/// reference, "Legacy manifest fields").
const CLAUDE_ONLY_FIELDS: &[&str] = &[
    "displayName",
    "metadata",
    "icon",
    "documentationUrl",
    "supportUrl",
    "privacyPolicyUrl",
    "termsOfServiceUrl",
    "defaultEnabled",
    "dependencies",
    "settings",
    "userConfig",
    "types",
    "channels",
    "outputStyles",
    "workflows",
    "experimental",
    "themes",
    "monitors",
];

/// Manifest fields of GitHub Copilot's formats that Claude Code does not
/// document: `extensions` (Agent Plugins client data, or legacy extension
/// directories) and the legacy `category` and `tags` metadata.
const COPILOT_ONLY_FIELDS: &[&str] = &["extensions", "category", "tags"];

pub(crate) fn has_agent_plugins_schema(value: &serde_json::Value) -> bool {
    value
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|schema| {
            schema.starts_with(crate::rules::codex_plugin::AGENT_PLUGIN_SCHEMA_PREFIX)
        })
}

/// The Agent Plugins layout in the plugin directory `dir`, if any: "Copilot
/// CLI reads its client-specific components from the `com.github.copilot`
/// directory", and "The root `mcp.json` must declare a recognized Agent
/// Plugins `$schema`". Returns the marker found (`com.github.copilot/` or
/// `mcp.json`).
pub(crate) fn agent_plugins_layout(
    dir: &Path,
    fs: &dyn crate::fs::FileSystem,
) -> Option<&'static str> {
    if fs.is_dir(&dir.join("com.github.copilot")) {
        return Some("com.github.copilot/");
    }
    fs.read_to_string(&dir.join("mcp.json"))
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
        .is_some_and(|mcp| has_agent_plugins_schema(&mcp))
        .then_some("mcp.json")
}

/// Whether the `plugin.json` at `path`, outside `.claude-plugin/`, is an
/// Agent Plugins or GitHub Copilot manifest by its content and the files
/// beside it (the signals CC-PL-001 uses). The run's tool scope is left to
/// the caller.
pub(crate) fn is_copilot_root_manifest(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> bool {
    classify_outside_manifest(path, content, config) == OutsideManifest::OtherTool
}

/// Whether the run selects Claude Code and not GitHub Copilot.
fn selects_claude_without_copilot(config: &PerFileLintConfig<'_>) -> bool {
    let tools = config.tools();
    if tools.is_empty() {
        return config.target() == crate::config::TargetTool::ClaudeCode;
    }
    let selects = |names: &[&str]| {
        tools
            .iter()
            .any(|tool| names.iter().any(|name| tool.eq_ignore_ascii_case(name)))
    };
    selects(&["claude-code"]) && !selects(&["github-copilot", "copilot"])
}

/// Classify a `plugin.json` outside `.claude-plugin/` by the signals the
/// vendor docs give.
fn classify_outside_manifest(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> OutsideManifest {
    let value = serde_json::from_str::<serde_json::Value>(content).ok();
    let fields = value.as_ref().and_then(serde_json::Value::as_object);
    // "Copilot CLI recognizes the canonical `$schema` values for Agent
    // Plugins ... opting a plugin into Agent Plugins semantics."
    if value.as_ref().is_some_and(has_agent_plugins_schema) {
        return OutsideManifest::OtherTool;
    }
    if let Some(dir) = path.parent() {
        let fs = config.fs();
        // The plugin keeps its Claude Code manifest in `.claude-plugin/`, so
        // this one is for the tools that read a root manifest first.
        if fs.is_file(&dir.join(".claude-plugin").join("plugin.json")) {
            return OutsideManifest::OtherTool;
        }
        if agent_plugins_layout(dir, fs.as_ref()).is_some() {
            return OutsideManifest::OtherTool;
        }
    }
    let has_any = |names: &[&str]| {
        fields.is_some_and(|fields| names.iter().any(|name| fields.contains_key(*name)))
    };
    if has_any(CLAUDE_ONLY_FIELDS) {
        return OutsideManifest::Claude;
    }
    if has_any(COPILOT_ONLY_FIELDS) {
        return OutsideManifest::OtherTool;
    }
    if selects_claude_without_copilot(config) {
        return OutsideManifest::Claude;
    }
    OutsideManifest::Unknown
}

pub struct PluginValidator;

impl Validator for PluginValidator {
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
        let mut diagnostics = Vec::new();

        if !config.rules().plugins {
            return diagnostics;
        }

        let plugin_dir = path.parent();
        let is_in_claude_plugin = plugin_dir
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .map(|n| n.eq_ignore_ascii_case(".claude-plugin"))
            .unwrap_or(false);

        // Cursor plugin manifests (`.cursor-plugin/plugin.json`) follow Cursor's
        // schema, and Claude Code never reads them.
        let is_in_cursor_plugin = plugin_dir
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(".cursor-plugin"));
        if is_in_cursor_plugin {
            return diagnostics;
        }

        if !is_in_claude_plugin {
            match classify_outside_manifest(path, content, config) {
                // Not a Claude Code manifest: no Claude Code rule applies.
                OutsideManifest::OtherTool => return diagnostics,
                // Claude Code ignores the file, and GitHub Copilot may read
                // it with its own rules: report the location only.
                OutsideManifest::Unknown => {
                    if config.is_rule_enabled("CC-PL-001") {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-001",
                                t!("rules.cc_pl_001.shared_root"),
                            )
                            .with_suggestion(t!("rules.cc_pl_001.shared_root_suggestion")),
                        );
                    }
                    return diagnostics;
                }
                OutsideManifest::Claude => {
                    if config.is_rule_enabled("CC-PL-001") {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-001",
                                t!("rules.cc_pl_001.message"),
                            )
                            .with_suggestion(t!("rules.cc_pl_001.suggestion")),
                        );
                    }
                }
            }
        }

        #[allow(clippy::collapsible_if)]
        if config.is_rule_enabled("CC-PL-002") && is_in_claude_plugin {
            if let Some(plugin_dir) = plugin_dir {
                let fs = config.fs();
                // The full documented set: "All other directories (commands/,
                // agents/, skills/, workflows/, output-styles/, themes/,
                // monitors/, hooks/) must be at the plugin root, not inside
                // `.claude-plugin/`." Four were missing.
                let disallowed = [
                    "commands",
                    "agents",
                    "skills",
                    "workflows",
                    "output-styles",
                    "themes",
                    "monitors",
                    "hooks",
                ];
                for entry in disallowed {
                    if fs.exists(&plugin_dir.join(entry)) {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-002",
                                t!("rules.cc_pl_002.message", component = entry),
                            )
                            .with_suggestion(t!("rules.cc_pl_002.suggestion")),
                        );
                    }
                }
            }
        }

        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("CC-PL-006") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-006",
                            t!("rules.cc_pl_006.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.cc_pl_006.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        let locator = JsonLocator::new(content);

        if config.is_rule_enabled("CC-PL-017")
            && let Some(fields) = raw_value.as_object()
        {
            const KNOWN_FIELDS: &[&str] = &[
                "$schema",
                "displayName",
                "defaultEnabled",
                "dependencies",
                "termsOfServiceUrl",
                "types",
                "channels",
                "experimental",
                "name",
                "version",
                "description",
                "author",
                "homepage",
                "repository",
                "license",
                "keywords",
                "commands",
                "agents",
                "skills",
                "workflows",
                "outputStyles",
                "themes",
                "monitors",
                "hooks",
                "mcpServers",
                "lspServers",
                "settings",
                "userConfig",
                "metadata",
                "icon",
                "documentationUrl",
                "supportUrl",
                "privacyPolicyUrl",
            ];
            for key in fields
                .keys()
                .filter(|key| !KNOWN_FIELDS.contains(&key.as_str()))
            {
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-017",
                        t!("rules.cc_pl_017.message", field = key.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_017.suggestion")),
                );
            }
        }

        // A blank name is CC-PL-005's finding (it carries the fix); CC-PL-004
        // still reports it when CC-PL-005 is disabled.
        let blank_name = raw_value
            .get("name")
            .and_then(|v| v.as_str())
            .is_some_and(|name| name.trim().is_empty());
        if config.is_rule_enabled("CC-PL-004") {
            if !(blank_name && config.is_rule_enabled("CC-PL-005")) {
                check_required_field(&raw_value, "name", path, diagnostics.as_mut());
            }
            check_recommended_field(&raw_value, "description", path, diagnostics.as_mut());
            check_recommended_field(&raw_value, "version", path, diagnostics.as_mut());
        }

        if config.is_rule_enabled("CC-PL-005") {
            if let Some(name) = raw_value.get("name").and_then(|v| v.as_str()) {
                if name.trim().is_empty() {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-005",
                        t!("rules.cc_pl_005.message"),
                    )
                    .with_suggestion(t!("rules.cc_pl_005.suggestion"));

                    // Unsafe auto-fix: populate empty plugin name with a deterministic placeholder.
                    if let Some((start, end, _)) =
                        find_unique_json_string_value_range(content, "name")
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            "my-plugin",
                            "Set plugin name to 'my-plugin'",
                            false,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-PL-016: Plugin name is not kebab-case. The manifest reference
        // documents `name` as a kebab-case identifier with no spaces, and
        // Claude Code v2.1.247 hardened marketplace handling to reject names
        // containing control or invisible characters - which this catches as a
        // subset, before the plugin reaches a marketplace.
        if config.is_rule_enabled("CC-PL-016")
            && let Some(name) = raw_value.get("name").and_then(|v| v.as_str())
            && !name.trim().is_empty()
            && !is_kebab_case_plugin_name(name)
        {
            diagnostics.push(
                if forbidden_plugin_name().is_match(name) {
                    Diagnostic::error
                } else {
                    Diagnostic::warning
                }(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-PL-016",
                    t!("rules.cc_pl_016.message", name = name),
                )
                .with_suggestion(t!("rules.cc_pl_016.suggestion")),
            );
        }

        // CC-PL-007: Invalid component path / CC-PL-008: Component inside .claude-plugin
        let pl_007_enabled = config.is_rule_enabled("CC-PL-007");
        let pl_008_enabled = config.is_rule_enabled("CC-PL-008");
        if pl_007_enabled || pl_008_enabled {
            // Every documented path-bearing manifest key. The component-path
            // table lists ten; only four were checked, so six fields accepted
            // absolute paths, `..` traversal, and `.claude-plugin/` targets
            // without complaint.
            let path_fields = [
                "commands",
                "agents",
                "skills",
                "workflows",
                "hooks",
                "mcpServers",
                "outputStyles",
                "lspServers",
                "experimental.themes",
                "experimental.monitors",
            ];
            for field in path_fields {
                if pl_007_enabled {
                    check_component_paths(&raw_value, field, path, content, &mut diagnostics);
                }
                if pl_008_enabled {
                    check_component_inside_claude_plugin(&raw_value, field, path, &mut diagnostics);
                }
            }
        }

        // CC-PL-015: A manifest path override shadows the matching default folder.
        if config.is_rule_enabled("CC-PL-015") && is_in_claude_plugin {
            check_default_component_shadowing(&raw_value, path, config, &mut diagnostics);
        }

        // CC-PL-009: Invalid author object
        if config.is_rule_enabled("CC-PL-009") {
            if let Some(author) = raw_value.get("author") {
                if author.is_object() {
                    let name_empty = author
                        .get("name")
                        .and_then(|n| n.as_str())
                        .map(|n| n.trim().is_empty())
                        .unwrap_or(true);
                    if name_empty {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CC-PL-009",
                                t!("rules.cc_pl_009.message"),
                            )
                            .with_suggestion(t!("rules.cc_pl_009.suggestion")),
                        );
                    }
                } else {
                    // author is present but not an object
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-PL-009",
                            t!("rules.cc_pl_009.message"),
                        )
                        .with_suggestion(t!("rules.cc_pl_009.suggestion")),
                    );
                }
            }
        }

        // CC-PL-010: homepage "Must parse as a URL, or the plugin fails to
        // load". A URL that parses but isn't http(s) only gets a note.
        if config.is_rule_enabled("CC-PL-010") {
            if let Some(homepage_val) = raw_value.get("homepage") {
                let (line, col) = locator.value_pos(&[Seg::Key("homepage")]);
                let shown = homepage_val
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| homepage_val.to_string());
                match homepage_val.as_str().map(|h| (h, url::Url::parse(h))) {
                    // An empty homepage was never reported; whether Claude Code
                    // rejects it is not documented.
                    Some(("", _)) => {}
                    Some((_, Ok(url))) => {
                        if !matches!(url.scheme(), "http" | "https") {
                            diagnostics.push(
                                Diagnostic::info(
                                    path.to_path_buf(),
                                    line,
                                    col,
                                    "CC-PL-010",
                                    t!("rules.cc_pl_010.scheme", url = shown.as_str()),
                                )
                                .with_suggestion(t!("rules.cc_pl_010.scheme_suggestion")),
                            );
                        }
                    }
                    _ => {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                col,
                                "CC-PL-010",
                                t!("rules.cc_pl_010.message", url = shown.as_str()),
                            )
                            .with_suggestion(t!("rules.cc_pl_010.suggestion")),
                        );
                    }
                }
            }
        }

        if config.is_rule_enabled("CC-PL-003") {
            if let Some(version) = raw_value.get("version").and_then(|v| v.as_str()) {
                let trimmed = version.trim();
                if !trimmed.is_empty() && !is_valid_semver(trimmed) {
                    // Arbitrary version strings load. Semver is an optional
                    // interoperability recommendation for plugins that others
                    // consume through version-constrained dependencies.
                    let mut diagnostic = Diagnostic::info(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-003",
                        t!("rules.cc_pl_003.message", version = version),
                    )
                    .with_suggestion(t!("rules.cc_pl_003.suggestion"));

                    // Normalize partial semver: "1" -> "1.0.0", "1.2" -> "1.2.0"
                    let normalized = normalize_partial_semver(trimmed);
                    if let Some(norm) = &normalized {
                        if norm != trimmed {
                            if let Some((start, end)) =
                                crate::rules::find_unique_json_string_value_span(
                                    content, "version", version,
                                )
                            {
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    norm.as_str(),
                                    format!("Normalize version to '{}'", norm),
                                    false,
                                ));
                            }
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }

        // CC-PL-011: LSP server missing required fields
        if config.is_rule_enabled("CC-PL-011") {
            if let Some(lsp_servers) = raw_value.get("lspServers") {
                if let Some(obj) = lsp_servers.as_object() {
                    for (server_name, server_val) in obj {
                        if let Some(server_obj) = server_val.as_object() {
                            if !server_obj.contains_key("command") {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-PL-011",
                                        format!(
                                            "LSP server '{}' is missing required field 'command'",
                                            server_name
                                        ),
                                    )
                                    .with_suggestion(format!(
                                        "Add a 'command' string to LSP server '{}'",
                                        server_name
                                    )),
                                );
                            }
                            if !server_obj.contains_key("extensionToLanguage") {
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-PL-011",
                                        format!(
                                            "LSP server '{}' is missing required field 'extensionToLanguage'",
                                            server_name
                                        ),
                                    )
                                    .with_suggestion(format!(
                                        "Add an 'extensionToLanguage' mapping to LSP server '{}'",
                                        server_name
                                    )),
                                );
                            }
                        }
                    }
                }
            }
        }

        // CC-PL-012: Invalid userConfig key
        if config.is_rule_enabled("CC-PL-012") {
            if let Some(user_config) = raw_value.get("userConfig") {
                if let Some(obj) = user_config.as_object() {
                    for key in obj.keys() {
                        if !is_valid_identifier(key) {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-PL-012",
                                    format!(
                                        "Invalid userConfig key '{}': must be a valid identifier (alphanumeric and underscores, not starting with a number)",
                                        key
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Rename '{}' to a valid identifier (e.g., 'my_config_key')",
                                    key
                                )),
                            );
                        }
                    }
                }
            }
        }

        // CC-PL-013: channels entry missing server field
        if config.is_rule_enabled("CC-PL-013") {
            if let Some(channels) = raw_value.get("channels") {
                if let Some(arr) = channels.as_array() {
                    let mcp_server_keys: Vec<String> = raw_value
                        .get("mcpServers")
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.keys().cloned().collect())
                        .unwrap_or_default();

                    for (i, entry) in arr.iter().enumerate() {
                        match entry.get("server").and_then(|v| v.as_str()) {
                            None => {
                                diagnostics.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-PL-013",
                                        format!(
                                            "channels[{}] is missing required 'server' field",
                                            i
                                        ),
                                    )
                                    .with_suggestion(
                                        "Add a 'server' field referencing a key in 'mcpServers'"
                                            .to_string(),
                                    ),
                                );
                            }
                            Some(server_ref) => {
                                if !mcp_server_keys.is_empty()
                                    && !mcp_server_keys.contains(&server_ref.to_string())
                                {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-PL-013",
                                            format!(
                                                "channels[{}] references server '{}' which is not defined in 'mcpServers'",
                                                i, server_ref
                                            ),
                                        )
                                        .with_suggestion(format!(
                                            "Add '{}' to 'mcpServers' or fix the server reference",
                                            server_ref
                                        )),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        // CC-PL-014 moved to the plugin agent Markdown files: the manifest
        // `agents` key takes .md paths only (see CC-PL-007 below).

        if is_in_claude_plugin && let Some(plugin_root) = plugin_dir.and_then(Path::parent) {
            check_manifest_extras(
                path,
                &raw_value,
                plugin_root,
                &locator,
                config,
                &mut diagnostics,
            );
        }

        diagnostics
    }
}

/// How `claude plugin validate` treats a plugin name that passes as one of
/// Anthropic's own: "The check ignores case and treats any run of separators
/// as one".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReservedPluginName {
    /// "Starts with `claude-`, `anthropic-`, `anthropics-`, or `cc-plugin-`",
    /// "Is `claude`, `anthropic`, `anthropics`, `claude-code`, or
    /// `claude-mods`", or "Puts `official` beside `claude` or `anthropic`".
    Error,
    /// "Has `claude`, `anthropic`, or `anthropics` as a whole word anywhere
    /// else".
    Warning,
}

pub(crate) fn reserved_plugin_name(name: &str) -> Option<ReservedPluginName> {
    let words: Vec<String> = name
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect();
    if words.is_empty() {
        return None;
    }
    let joined = words.join("-");
    let brand = |w: &str| matches!(w, "claude" | "anthropic" | "anthropics");
    let error = ["claude-", "anthropic-", "anthropics-", "cc-plugin-"]
        .iter()
        .any(|prefix| joined.starts_with(prefix))
        || matches!(
            joined.as_str(),
            "claude" | "anthropic" | "anthropics" | "claude-code" | "claude-mods"
        )
        || words.windows(2).any(|pair| {
            (pair[0] == "official" && (pair[1] == "claude" || pair[1] == "anthropic"))
                || (pair[1] == "official" && (pair[0] == "claude" || pair[0] == "anthropic"))
        });
    if error {
        Some(ReservedPluginName::Error)
    } else if words.iter().any(|w| brand(w)) {
        Some(ReservedPluginName::Warning)
    } else {
        None
    }
}

/// Whether an `author` or `owner` object names Anthropic, whose own plugins
/// and marketplaces use the reserved names.
pub(crate) fn is_anthropic_party(value: Option<&serde_json::Value>) -> bool {
    value.is_some_and(|party| {
        party
            .get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.trim().eq_ignore_ascii_case("anthropic"))
            || party
                .get("email")
                .and_then(|e| e.as_str())
                .is_some_and(|e| e.to_ascii_lowercase().ends_with("@anthropic.com"))
    })
}

/// The plugin name and the keys of the MCP servers a Claude plugin at `root`
/// declares: `.mcp.json` at the plugin root, plus an inline `mcpServers` map
/// in `.claude-plugin/plugin.json`. The name is the manifest `name`, or the
/// plugin directory name when the manifest is missing or has none ("The
/// plugin name then comes from the marketplace entry, or from the directory
/// name").
pub(crate) fn plugin_mcp_servers(
    root: &Path,
    fs: &dyn crate::fs::FileSystem,
) -> crate::rules::hooks::PluginServers {
    let read_json = |path: &Path| {
        fs.read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    };
    let manifest = read_json(&root.join(".claude-plugin").join("plugin.json"));
    let plugin = manifest
        .as_ref()
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .or_else(|| {
            root.file_name()
                .and_then(|n| n.to_str())
                .map(str::to_string)
        })
        .unwrap_or_default();
    let mut servers: Vec<String> = Vec::new();
    if let Some(mcp) = read_json(&root.join(".mcp.json")) {
        let map = mcp
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .or_else(|| mcp.as_object().filter(|o| !o.contains_key("mcpServers")));
        if let Some(map) = map {
            servers.extend(
                map.iter()
                    .filter(|(_, v)| v.is_object())
                    .map(|(k, _)| k.clone()),
            );
        }
    }
    if let Some(inline) = manifest
        .as_ref()
        .and_then(|m| m.get("mcpServers"))
        .and_then(|v| v.as_object())
    {
        servers.extend(inline.keys().cloned());
    }
    servers.sort();
    servers.dedup();
    crate::rules::hooks::PluginServers { plugin, servers }
}

const USER_CONFIG_OPTION_KEYS: &[&str] = &[
    "type",
    "title",
    "description",
    "required",
    "default",
    "options",
    "multiple",
    "sensitive",
    "min",
    "max",
];
const CHANNEL_KEYS: &[&str] = &["server", "displayName", "userConfig"];
const LSP_SERVER_KEYS: &[&str] = &[
    "command",
    "extensionToLanguage",
    "args",
    "transport",
    "env",
    "initializationOptions",
    "settings",
    "workspaceFolder",
    "startupTimeout",
    "shutdownTimeout",
    "requestTimeout",
    "restartOnCrash",
    "maxRestarts",
    "diagnostics",
];
const MONITOR_KEYS: &[&str] = &["name", "command", "description", "when"];
/// "Only `agent` and `subagentStatusLine` take effect".
const PLUGIN_SETTINGS_KEYS: &[&str] = &["agent", "subagentStatusLine"];

/// Whether a node-semver range parses: `||` alternatives of space-separated
/// comparators (`>=1.4`, `~2.1.0`, `^2.0`, `=2.1.0`, `1.x`, `*`) or a hyphen
/// range (`1.2 - 2.3`).
pub(crate) fn is_semver_range(range: &str) -> bool {
    static_regex!(fn partial_re, r"^v?(?:x|X|\*|0|[1-9]\d*)(?:\.(?:x|X|\*|0|[1-9]\d*)(?:\.(?:x|X|\*|0|[1-9]\d*)(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?)?)?$");
    range.split("||").all(|set| {
        let set = set.trim();
        if set.is_empty() {
            return true;
        }
        if let Some((low, high)) = set.split_once(" - ") {
            return partial_re().is_match(low.trim()) && partial_re().is_match(high.trim());
        }
        let mut tokens = set.split_whitespace().peekable();
        while let Some(token) = tokens.next() {
            let mut token = token.to_string();
            if matches!(
                token.as_str(),
                ">=" | "<=" | ">" | "<" | "=" | "~" | "^" | "~>"
            ) {
                match tokens.next() {
                    Some(next) => token.push_str(next),
                    None => return false,
                }
            }
            let version = [">=", "<=", "~>", ">", "<", "=", "~", "^"]
                .iter()
                .find_map(|op| token.strip_prefix(op))
                .unwrap_or(&token);
            if !partial_re().is_match(version) {
                return false;
            }
        }
        true
    })
}

/// Whether the plugin at `plugin_root` is Anthropic's own: its manifest
/// author is Anthropic, or it sits in a marketplace Anthropic owns.
fn is_anthropic_plugin(
    raw_value: &serde_json::Value,
    plugin_root: &Path,
    fs: &dyn crate::fs::FileSystem,
) -> bool {
    if is_anthropic_party(raw_value.get("author")) {
        return true;
    }
    plugin_root.ancestors().take(6).any(|dir| {
        fs.read_to_string(&dir.join(".claude-plugin").join("marketplace.json"))
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
            .is_some_and(|m| is_anthropic_party(m.get("owner")))
    })
}

/// Read a JSON file the manifest points at (or a default file), relative to
/// the plugin root.
fn read_plugin_json(
    plugin_root: &Path,
    rel: &str,
    fs: &dyn crate::fs::FileSystem,
) -> Option<(std::path::PathBuf, String, serde_json::Value)> {
    if is_invalid_component_path(rel) {
        return None;
    }
    let file = plugin_root.join(rel.trim_start_matches("./"));
    let text = fs.read_to_string(&file).ok()?;
    let value = serde_json::from_str(&text).ok()?;
    Some((file, text, value))
}

/// CC-PL-007 (entry types, existence), CC-PL-018, CC-PL-019, CC-PL-021,
/// CC-PL-022 and CC-PL-023 on a `.claude-plugin/plugin.json`.
fn check_manifest_extras(
    path: &Path,
    raw_value: &serde_json::Value,
    plugin_root: &Path,
    locator: &JsonLocator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let fs = config.fs();
    let push = |diagnostics: &mut Vec<Diagnostic>,
                file: &Path,
                level: crate::diagnostics::DiagnosticLevel,
                (line, col): (usize, usize),
                rule: &str,
                message: String,
                suggestion: String| {
        let make = match level {
            crate::diagnostics::DiagnosticLevel::Error => Diagnostic::error,
            crate::diagnostics::DiagnosticLevel::Warning => Diagnostic::warning,
            crate::diagnostics::DiagnosticLevel::Info => Diagnostic::info,
        };
        diagnostics
            .push(make(file.to_path_buf(), line, col, rule, message).with_suggestion(suggestion));
    };
    use crate::diagnostics::DiagnosticLevel::{Error, Info, Warning};

    // ---- CC-PL-007: "`agents` entries must be `.md` files, and `skills`
    // entries must be directories", and "Every component path must resolve
    // inside the plugin root and must exist". ----
    if config.is_rule_enabled("CC-PL-007") {
        if let Some(agents) = raw_value.get("agents") {
            let entries: Vec<(Option<usize>, &serde_json::Value)> = match agents {
                serde_json::Value::Array(items) => items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| (Some(i), v))
                    .collect(),
                other => vec![(None, other)],
            };
            for (index, entry) in entries {
                let bad = match entry.as_str() {
                    Some(p) => !p.trim().to_ascii_lowercase().ends_with(".md"),
                    None => true,
                };
                if bad {
                    let segs: Vec<Seg<'_>> = match index {
                        Some(i) => vec![Seg::Key("agents"), Seg::Index(i)],
                        None => vec![Seg::Key("agents")],
                    };
                    let shown = entry
                        .as_str()
                        .map(|p| format!("'{p}'"))
                        .unwrap_or_else(|| crate::rules::json_type_name(entry).to_string());
                    push(
                        diagnostics,
                        path,
                        Error,
                        locator.value_pos(&segs),
                        "CC-PL-007",
                        t!("rules.cc_pl_007.agents_entry", value = shown.as_str()).to_string(),
                        t!("rules.cc_pl_007.entry_suggestion").to_string(),
                    );
                }
            }
        }
        for (index, skill) in
            extract_paths(raw_value.get("skills").unwrap_or(&serde_json::Value::Null))
                .iter()
                .enumerate()
        {
            if skill.trim().to_ascii_lowercase().ends_with(".md") {
                let segs = if raw_value.get("skills").is_some_and(|v| v.is_array()) {
                    vec![Seg::Key("skills"), Seg::Index(index)]
                } else {
                    vec![Seg::Key("skills")]
                };
                push(
                    diagnostics,
                    path,
                    Error,
                    locator.value_pos(&segs),
                    "CC-PL-007",
                    t!("rules.cc_pl_007.skills_entry", value = skill.as_str()).to_string(),
                    t!("rules.cc_pl_007.entry_suggestion").to_string(),
                );
            }
        }
        for field in [
            "commands",
            "agents",
            "skills",
            "workflows",
            "hooks",
            "mcpServers",
            "outputStyles",
            "lspServers",
            "experimental.themes",
            "experimental.monitors",
        ] {
            let Some(value) = manifest_field(raw_value, field) else {
                continue;
            };
            let mut paths = extract_paths(value);
            if field == "commands"
                && let Some(map) = value.as_object()
            {
                paths.extend(
                    map.values()
                        .filter_map(|c| c.get("source").and_then(|s| s.as_str()))
                        .map(str::to_string),
                );
            }
            for p in paths {
                let trimmed = p.trim();
                // Paths that already fail the `./` or containment checks are
                // reported once, above.
                if trimmed.is_empty()
                    || matches!(trimmed, "." | "./")
                    || is_autofixable_path(field, trimmed)
                    || is_invalid_component_path(trimmed)
                    || path_inside_claude_plugin(trimmed)
                    || trimmed.contains("://")
                    || trimmed.contains('$')
                {
                    continue;
                }
                if !fs.exists(&plugin_root.join(trimmed.trim_start_matches("./"))) {
                    let top = field.split('.').next().unwrap_or(field);
                    push(
                        diagnostics,
                        path,
                        Error,
                        locator.value_pos(&[Seg::Key(top)]),
                        "CC-PL-007",
                        t!("rules.cc_pl_007.missing", path = trimmed, field = field).to_string(),
                        t!("rules.cc_pl_007.missing_suggestion").to_string(),
                    );
                }
            }
        }
    }

    // ---- CC-PL-018: reserved plugin names ----
    if config.is_rule_enabled("CC-PL-018")
        && let Some(name) = raw_value.get("name").and_then(|n| n.as_str())
        && let Some(level) = reserved_plugin_name(name)
        && !is_anthropic_plugin(raw_value, plugin_root, fs.as_ref())
    {
        let (level, key) = match level {
            ReservedPluginName::Error => (Error, "rules.cc_pl_018.error"),
            ReservedPluginName::Warning => (Warning, "rules.cc_pl_018.warning"),
        };
        push(
            diagnostics,
            path,
            level,
            locator.value_pos(&[Seg::Key("name")]),
            "CC-PL-018",
            t!(key, name = name).to_string(),
            t!("rules.cc_pl_018.suggestion").to_string(),
        );
    }

    // ---- CC-PL-019: dependency version ranges ----
    if config.is_rule_enabled("CC-PL-019")
        && let Some(deps) = raw_value.get("dependencies").and_then(|d| d.as_array())
    {
        for (i, dep) in deps.iter().enumerate() {
            let Some(version) = dep.get("version").and_then(|v| v.as_str()) else {
                continue;
            };
            if !is_semver_range(version) {
                let name = dep.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                push(
                    diagnostics,
                    path,
                    Warning,
                    locator.value_pos(&[
                        Seg::Key("dependencies"),
                        Seg::Index(i),
                        Seg::Key("version"),
                    ]),
                    "CC-PL-019",
                    t!("rules.cc_pl_019.message", name = name, version = version).to_string(),
                    t!("rules.cc_pl_019.suggestion").to_string(),
                );
            }
        }
    }

    // ---- CC-PL-021: manifest `settings` keys that are dropped ----
    if config.is_rule_enabled("CC-PL-021")
        && let Some(settings) = raw_value.get("settings").and_then(|s| s.as_object())
    {
        for key in settings.keys() {
            if !PLUGIN_SETTINGS_KEYS.contains(&key.as_str()) {
                push(
                    diagnostics,
                    path,
                    Warning,
                    locator.key_pos(&[Seg::Key("settings"), Seg::Key(key.as_str())]),
                    "CC-PL-021",
                    t!(
                        "rules.cc_pl_021.message",
                        key = key.as_str(),
                        location = "plugin.json settings"
                    )
                    .to_string(),
                    t!("rules.cc_pl_021.suggestion").to_string(),
                );
            }
        }
    }

    // ---- CC-PL-022 and CC-PL-023: monitors (inline or file) ----
    let pl022 = config.is_rule_enabled("CC-PL-022");
    let pl023 = config.is_rule_enabled("CC-PL-023");
    let check_monitors = |diagnostics: &mut Vec<Diagnostic>,
                          file: &Path,
                          monitors: &serde_json::Value,
                          locate: &dyn Fn(&[Seg<'_>]) -> (usize, usize),
                          base: &[Seg<'_>]| {
        let Some(items) = monitors.as_array() else {
            return;
        };
        for (i, monitor) in items.iter().enumerate() {
            let Some(obj) = monitor.as_object() else {
                continue;
            };
            let mut at = base.to_vec();
            at.push(Seg::Index(i));
            let monitor_name = obj.get("name").and_then(|n| n.as_str()).unwrap_or("?");
            if pl022
                && obj
                    .get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|c| c.contains("${user_config."))
            {
                let mut segs = at.clone();
                segs.push(Seg::Key("command"));
                push(
                    diagnostics,
                    file,
                    Error,
                    locate(&segs),
                    "CC-PL-022",
                    t!("rules.cc_pl_022.message", monitor = monitor_name).to_string(),
                    t!("rules.cc_pl_022.suggestion").to_string(),
                );
            }
            if pl023 {
                for key in obj.keys() {
                    if !MONITOR_KEYS.contains(&key.as_str()) {
                        let mut segs = at.clone();
                        segs.push(Seg::Key(key.as_str()));
                        let location = format!("monitor '{monitor_name}'");
                        push(
                            diagnostics,
                            file,
                            Error,
                            locate(&segs),
                            "CC-PL-023",
                            t!(
                                "rules.cc_pl_023.unknown_key",
                                key = key.as_str(),
                                location = location.as_str()
                            )
                            .to_string(),
                            t!("rules.cc_pl_023.unknown_key_suggestion").to_string(),
                        );
                    }
                }
            }
        }
    };
    if pl022 || pl023 {
        let inline_locate = |segs: &[Seg<'_>]| locator.value_pos(segs);
        let experimental = raw_value
            .get("experimental")
            .and_then(|e| e.get("monitors"));
        match (experimental, raw_value.get("monitors")) {
            (Some(serde_json::Value::String(rel)), _) => {
                if let Some((file, text, value)) = read_plugin_json(plugin_root, rel, fs.as_ref()) {
                    let file_locator = JsonLocator::new(&text);
                    check_monitors(
                        diagnostics,
                        &file,
                        &value,
                        &|s| file_locator.value_pos(s),
                        &[],
                    );
                }
            }
            (Some(inline @ serde_json::Value::Array(_)), _) => check_monitors(
                diagnostics,
                path,
                inline,
                &inline_locate,
                &[Seg::Key("experimental"), Seg::Key("monitors")],
            ),
            (None, Some(inline @ serde_json::Value::Array(_))) => check_monitors(
                diagnostics,
                path,
                inline,
                &inline_locate,
                &[Seg::Key("monitors")],
            ),
            (None, None) => {
                if let Some((file, text, value)) =
                    read_plugin_json(plugin_root, "monitors/monitors.json", fs.as_ref())
                {
                    let file_locator = JsonLocator::new(&text);
                    check_monitors(
                        diagnostics,
                        &file,
                        &value,
                        &|s| file_locator.value_pos(s),
                        &[],
                    );
                }
            }
            _ => {}
        }
    }

    if !pl023 {
        return;
    }

    // ---- CC-PL-023: strict userConfig options, channels and LSP configs ----
    let check_user_config =
        |diagnostics: &mut Vec<Diagnostic>, user_config: &serde_json::Value, base: Vec<Seg<'_>>| {
            let Some(options) = user_config.as_object() else {
                return;
            };
            for (option, spec) in options {
                let Some(spec) = spec.as_object() else {
                    continue;
                };
                let mut at = base.clone();
                at.push(Seg::Key(option.as_str()));
                for key in spec.keys() {
                    if !USER_CONFIG_OPTION_KEYS.contains(&key.as_str()) {
                        let mut segs = at.clone();
                        segs.push(Seg::Key(key.as_str()));
                        let location = format!("userConfig option '{option}'");
                        push(
                            diagnostics,
                            path,
                            Error,
                            locator.key_pos(&segs),
                            "CC-PL-023",
                            t!(
                                "rules.cc_pl_023.unknown_key",
                                key = key.as_str(),
                                location = location.as_str()
                            )
                            .to_string(),
                            t!("rules.cc_pl_023.unknown_key_suggestion").to_string(),
                        );
                    }
                }
                let Some(list) = spec.get("options") else {
                    continue;
                };
                let mut segs = at.clone();
                segs.push(Seg::Key("options"));
                let pos = locator.value_pos(&segs);
                let is_true = |k: &str| spec.get(k).and_then(|v| v.as_bool()) == Some(true);
                let problem = if spec.get("type").and_then(|t| t.as_str()) != Some("string")
                    || is_true("multiple")
                    || is_true("sensitive")
                {
                    Some("rules.cc_pl_023.option_type")
                } else if !list.as_array().is_some_and(|items| {
                    !items.is_empty()
                        && items.iter().all(|item| {
                            item.as_str()
                                .is_some_and(|s| (1..=64).contains(&s.chars().count()))
                        })
                }) {
                    Some("rules.cc_pl_023.option_label")
                } else {
                    let labels: Vec<&str> = list
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|v| v.as_str())
                        .collect();
                    match spec.get("default") {
                        Some(serde_json::Value::String(d)) if labels.contains(&d.as_str()) => None,
                        None if is_true("required") => None,
                        _ => Some("rules.cc_pl_023.option_default"),
                    }
                };
                if let Some(key) = problem {
                    push(
                        diagnostics,
                        path,
                        Error,
                        pos,
                        "CC-PL-023",
                        t!(key, option = option.as_str()).to_string(),
                        t!("rules.cc_pl_023.option_suggestion").to_string(),
                    );
                }
            }
        };
    if let Some(user_config) = raw_value.get("userConfig") {
        check_user_config(diagnostics, user_config, vec![Seg::Key("userConfig")]);
    }
    if let Some(channels) = raw_value.get("channels").and_then(|c| c.as_array()) {
        for (i, channel) in channels.iter().enumerate() {
            let Some(obj) = channel.as_object() else {
                continue;
            };
            for key in obj.keys() {
                if !CHANNEL_KEYS.contains(&key.as_str()) {
                    let location = format!("channels[{i}]");
                    push(
                        diagnostics,
                        path,
                        Error,
                        locator.key_pos(&[
                            Seg::Key("channels"),
                            Seg::Index(i),
                            Seg::Key(key.as_str()),
                        ]),
                        "CC-PL-023",
                        t!(
                            "rules.cc_pl_023.unknown_key",
                            key = key.as_str(),
                            location = location.as_str()
                        )
                        .to_string(),
                        t!("rules.cc_pl_023.unknown_key_suggestion").to_string(),
                    );
                }
            }
            if let Some(user_config) = obj.get("userConfig") {
                check_user_config(
                    diagnostics,
                    user_config,
                    vec![Seg::Key("channels"), Seg::Index(i), Seg::Key("userConfig")],
                );
            }
        }
    }
    let check_lsp = |diagnostics: &mut Vec<Diagnostic>,
                     file: &Path,
                     servers: &serde_json::Value,
                     locate: &dyn Fn(&[Seg<'_>]) -> (usize, usize),
                     base: &[Seg<'_>]| {
        let Some(map) = servers.as_object() else {
            return;
        };
        for (server, config_value) in map {
            let Some(obj) = config_value.as_object() else {
                continue;
            };
            for key in obj.keys() {
                if !LSP_SERVER_KEYS.contains(&key.as_str()) {
                    let mut segs = base.to_vec();
                    segs.push(Seg::Key(server.as_str()));
                    segs.push(Seg::Key(key.as_str()));
                    let location = format!("LSP server '{server}'");
                    push(
                        diagnostics,
                        file,
                        Error,
                        locate(&segs),
                        "CC-PL-023",
                        t!(
                            "rules.cc_pl_023.unknown_key",
                            key = key.as_str(),
                            location = location.as_str()
                        )
                        .to_string(),
                        t!("rules.cc_pl_023.unknown_key_suggestion").to_string(),
                    );
                }
            }
        }
    };
    let mut lsp_files: Vec<String> = vec![".lsp.json".to_string()];
    match raw_value.get("lspServers") {
        Some(inline @ serde_json::Value::Object(_)) => check_lsp(
            diagnostics,
            path,
            inline,
            &|s| locator.key_pos(s),
            &[Seg::Key("lspServers")],
        ),
        Some(other) => lsp_files.extend(extract_paths(other)),
        None => {}
    }
    for rel in lsp_files {
        if let Some((file, text, value)) = read_plugin_json(plugin_root, &rel, fs.as_ref()) {
            let file_locator = JsonLocator::new(&text);
            let servers = value.get("lspServers").unwrap_or(&value);
            let base: Vec<Seg<'_>> = if value.get("lspServers").is_some() {
                vec![Seg::Key("lspServers")]
            } else {
                Vec::new()
            };
            check_lsp(
                diagnostics,
                &file,
                servers,
                &|s| file_locator.key_pos(s),
                &base,
            );
        }
    }
    let _ = Info;
}

fn is_field_missing(value: &serde_json::Value, field: &str) -> bool {
    match value.get(field) {
        Some(v) => !v.is_string() || v.as_str().map(|s| s.trim().is_empty()).unwrap_or(true),
        None => true,
    }
}

fn check_required_field(
    value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if is_field_missing(value, field) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CC-PL-004",
                t!("rules.cc_pl_004.message", field = field),
            )
            .with_suggestion(t!("rules.cc_pl_004.suggestion", field = field)),
        );
    }
}

fn check_recommended_field(
    value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if is_field_missing(value, field) {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CC-PL-004",
                t!("rules.cc_pl_004_recommended.message", field = field),
            )
            .with_suggestion(t!("rules.cc_pl_004_recommended.suggestion", field = field)),
        );
    }
}

fn is_valid_semver(version: &str) -> bool {
    semver::Version::parse(version).is_ok()
}

/// Normalize partial semver strings: "1" -> "1.0.0", "1.2" -> "1.2.0"
fn normalize_partial_semver(version: &str) -> Option<String> {
    let parts: Vec<&str> = version.split('.').collect();
    match parts.len() {
        1 => {
            if parts[0].chars().all(|c| c.is_ascii_digit()) {
                Some(format!("{}.0.0", parts[0]))
            } else {
                None
            }
        }
        2 => {
            if parts.iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
                Some(format!("{}.{}.0", parts[0], parts[1]))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Find a unique string value span for a JSON key.
/// Returns (value_start, value_end, value_content_without_quotes).
fn find_unique_json_string_value_range(content: &str, key: &str) -> Option<(usize, usize, String)> {
    crate::span_utils::find_unique_json_string_value_range(content, key)
}

/// Check if a path string is invalid for a component path.
/// Must be relative (no absolute paths), must not use `..` traversal.
fn is_invalid_component_path(p: &str) -> bool {
    let trimmed = p.trim();
    if trimmed.is_empty() {
        return false;
    }
    // Absolute paths: starts with `/` or `\`
    if trimmed.starts_with('/') || trimmed.starts_with('\\') {
        return true;
    }
    // Windows drive letter paths: C:\... or C:/...
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return true;
        }
    }
    // Check for `..` traversal in any component (split on both / and \)
    trimmed.split(['/', '\\']).any(|part| part == "..")
}

/// Check if a path is a relative path missing a `./` prefix (autofixable).
/// This is separate from `is_invalid_component_path`: paths like `skills/foo`
/// are not absolute or traversal, but should have `./` prepended. Claude Code
/// v2.1.221 explicitly accepts `skills: "."` to load a root-level SKILL.md.
fn is_autofixable_path(field: &str, p: &str) -> bool {
    let trimmed = p.trim();
    // Must not be empty, not already have ./ prefix, and not be invalid
    !trimmed.is_empty()
        && !(field == "skills" && trimmed == ".")
        && !trimmed.starts_with("./")
        && !trimmed.starts_with(".\\")
        && !is_invalid_component_path(trimmed)
}

/// Check if a path starts with `.claude-plugin/`.
/// Normalizes an optional leading `./` or `.\\` before checking.
fn path_inside_claude_plugin(p: &str) -> bool {
    let trimmed = p.trim();
    let normalized = trimmed
        .strip_prefix("./")
        .or_else(|| trimmed.strip_prefix(".\\"))
        .unwrap_or(trimmed);
    normalized.starts_with(".claude-plugin/")
        || normalized.starts_with(".claude-plugin\\")
        || normalized == ".claude-plugin"
}

/// Extract string paths from a JSON value that can be a string or array of strings.
fn extract_paths(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    }
}

fn path_points_to_default_component(p: &str, component: &str) -> bool {
    let normalized = p
        .trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string();
    normalized == component
}

fn extract_valid_shadowing_paths(value: &serde_json::Value) -> Option<Vec<String>> {
    match value {
        serde_json::Value::String(s) => Some(vec![s.clone()]),
        serde_json::Value::Array(arr) => arr.iter().map(|v| v.as_str().map(String::from)).collect(),
        _ => None,
    }
}

/// CC-PL-015: Detect root default component folders shadowed by manifest paths.
fn check_default_component_shadowing(
    raw_value: &serde_json::Value,
    path: &Path,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(plugin_dir) = path.parent() else {
        return;
    };
    let Some(plugin_root) = plugin_dir.parent() else {
        return;
    };

    let fs = config.fs();
    // Only replace-semantics fields can shadow their default folder. Per the
    // "Path behavior rules" section of the plugins reference:
    //
    // - Replaces the default: `commands`, `agents`, `workflows`, `outputStyles`,
    //   `experimental.themes`, `experimental.monitors`.
    // - Adds to the default: `skills` - "The default `skills/` directory is
    //   always scanned, and directories listed in `skills` are loaded alongside
    //   it", so it can never shadow and must not be listed here.
    // - Own merge rules: `hooks`, `mcpServers`, `lspServers` - combining is
    //   defined per-section, not as replacement, so they are excluded too.
    //
    // `skills` and `hooks` were previously included, which warned that a
    // default folder was ignored when it was in fact still loaded.
    //
    // Each entry is (manifest key, default folder name). They differ for
    // `outputStyles` -> `output-styles/` and for the `experimental.*` keys,
    // whose folders sit at the plugin root without the prefix.
    for (component, default_dir) in [
        ("commands", "commands"),
        ("agents", "agents"),
        ("workflows", "workflows"),
        ("outputStyles", "output-styles"),
        ("experimental.themes", "themes"),
        ("experimental.monitors", "monitors"),
    ] {
        let manifest_value = match component.split_once('.') {
            Some((parent, child)) => raw_value.get(parent).and_then(|v| v.get(child)),
            None => raw_value.get(component),
        };
        let Some(manifest_value) = manifest_value else {
            continue;
        };
        if !fs.is_dir(&plugin_root.join(default_dir)) {
            continue;
        }

        let Some(manifest_paths) = extract_valid_shadowing_paths(manifest_value) else {
            continue;
        };

        let includes_default = manifest_paths
            .iter()
            .any(|p| path_points_to_default_component(p, component));
        if includes_default {
            continue;
        }

        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CC-PL-015",
                t!("rules.cc_pl_015.message", component = component),
            )
            .with_suggestion(t!("rules.cc_pl_015.suggestion", component = component)),
        );
    }
}

/// CC-PL-007: Validate component paths are relative without `..` traversal.
/// Also flags relative paths missing a `./` prefix (with safe autofix).
/// Look up a manifest field, traversing one level of dotted key.
///
/// `raw_value.get("experimental.themes")` never matches, because the key is
/// nested under `experimental`. The path-bearing fields include two such keys,
/// so both CC-PL-007 and CC-PL-008 need this.
fn manifest_field<'a>(
    raw_value: &'a serde_json::Value,
    field: &str,
) -> Option<&'a serde_json::Value> {
    match field.split_once('.') {
        Some((parent, child)) => raw_value.get(parent).and_then(|v| v.get(child)),
        None => raw_value.get(field),
    }
}

fn check_component_paths(
    raw_value: &serde_json::Value,
    field: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(val) = manifest_field(raw_value, field) {
        for p in extract_paths(val) {
            if field == "mcpServers" && is_https_mcp_bundle_url(&p) {
                continue;
            }
            if is_invalid_component_path(&p) {
                // Absolute or traversal path: error without autofix
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-007",
                        t!("rules.cc_pl_007.message", field = field, path = p.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_007.suggestion")),
                );
            } else if is_autofixable_path(field, &p) {
                // Relative path missing ./ prefix: error with safe autofix
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-PL-007",
                    t!("rules.cc_pl_007.message", field = field, path = p.as_str()),
                )
                .with_suggestion(t!("rules.cc_pl_007.suggestion"));

                if let Some((start, end, _)) = find_unique_json_string_value_range(content, field) {
                    let fixed = format!("./{}", p.trim());
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        &fixed,
                        format!("Prepend './' to path: '{}'", p.trim()),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }
    }
}

/// The manifest reference accepts HTTPS bundle URLs ending in .mcpb or .dxt.
/// Query/fragment suffixes are not documented as bundle forms; keep the
/// exception limited to a real HTTPS URL naming the bundle itself.
fn is_https_mcp_bundle_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url.host_str().is_some()
        && url.query().is_none()
        && url.fragment().is_none()
        && (url.path().ends_with(".mcpb") || url.path().ends_with(".dxt"))
}

/// CC-PL-008: Detect component paths pointing inside .claude-plugin/.
fn check_component_inside_claude_plugin(
    raw_value: &serde_json::Value,
    field: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(val) = manifest_field(raw_value, field) {
        for p in extract_paths(val) {
            if path_inside_claude_plugin(&p) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-PL-008",
                        t!("rules.cc_pl_008.message", field = field, path = p.as_str()),
                    )
                    .with_suggestion(t!("rules.cc_pl_008.suggestion")),
                );
            }
        }
    }
}

/// Check if a string is a valid identifier (alphanumeric + underscores, not starting with a number).
fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::fs;
    use tempfile::TempDir;

    fn write_plugin(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_cc_pl_001_manifest_not_in_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        // Copilot CLI and VS Code also read a root plugin.json, so with no
        // signal either way the location is a warning, not an error.
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-001")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].level, crate::diagnostics::DiagnosticLevel::Warning);
    }

    /// Write `files` under a temp plugin directory and validate its root
    /// `plugin.json`.
    fn validate_root_manifest(
        manifest: &str,
        files: &[(&str, &str)],
        config: &LintConfig,
    ) -> Vec<Diagnostic> {
        let temp = TempDir::new().unwrap();
        for (name, content) in files {
            write_plugin(&temp.path().join(name), content);
        }
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(&plugin_path, manifest);
        PluginValidator.validate(&plugin_path, manifest, config)
    }

    fn cc_pl_001_levels(diagnostics: &[Diagnostic]) -> Vec<crate::diagnostics::DiagnosticLevel> {
        diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-001")
            .map(|d| d.level)
            .collect()
    }

    #[test]
    fn test_cc_pl_001_root_copilot_legacy_manifest_is_a_warning_only() {
        use crate::diagnostics::DiagnosticLevel;
        // The legacy example from "Creating a plugin for GitHub Copilot CLI":
        // its paths have no `./` prefix, which Claude Code would reject, so
        // the Claude content checks must not run on it.
        let manifest = r#"{
  "name": "my-dev-tools",
  "description": "React development utilities",
  "agents": "agents/",
  "skills": ["skills/", "extra-skills/"],
  "hooks": "hooks.json",
  "mcpServers": ".mcp.json"
}"#;
        let diagnostics = validate_root_manifest(manifest, &[], &LintConfig::default());
        assert_eq!(
            cc_pl_001_levels(&diagnostics),
            vec![DiagnosticLevel::Warning]
        );
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert!(diagnostics[0].message.contains("GitHub Copilot"));

        // Both tools selected: still only a warning.
        let both = LintConfig::builder()
            .tools(vec![
                "claude-code".to_string(),
                "github-copilot".to_string(),
            ])
            .build()
            .unwrap();
        let diagnostics = validate_root_manifest(manifest, &[], &both);
        assert_eq!(
            cc_pl_001_levels(&diagnostics),
            vec![DiagnosticLevel::Warning]
        );
    }

    #[test]
    fn test_cc_pl_001_claude_manifest_in_wrong_place_is_an_error() {
        use crate::diagnostics::DiagnosticLevel;
        // A field only Claude Code documents marks a Claude Code manifest.
        let manifest = r#"{"name":"helper","userConfig":{"token":{"type":"string","title":"Token","description":"API token","sensitive":true}}}"#;
        let diagnostics = validate_root_manifest(manifest, &[], &LintConfig::default());
        assert_eq!(cc_pl_001_levels(&diagnostics), vec![DiagnosticLevel::Error]);
        // The other manifest checks still run on a misplaced Claude manifest.
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-004"),
            "{diagnostics:?}"
        );

        // A run that selects Claude Code and not Copilot.
        let claude_only = LintConfig::builder()
            .tools(vec!["claude-code".to_string()])
            .build()
            .unwrap();
        let diagnostics = validate_root_manifest(r#"{"name":"helper"}"#, &[], &claude_only);
        assert_eq!(cc_pl_001_levels(&diagnostics), vec![DiagnosticLevel::Error]);
        // The deprecated `target` still selects Claude Code.
        let target = LintConfig::builder()
            .target(crate::config::TargetTool::ClaudeCode)
            .build_unchecked();
        let diagnostics = validate_root_manifest(r#"{"name":"helper"}"#, &[], &target);
        assert_eq!(cc_pl_001_levels(&diagnostics), vec![DiagnosticLevel::Error]);
    }

    #[test]
    fn test_cc_pl_001_skips_agent_plugins_and_copilot_root_manifests() {
        let claude_only = LintConfig::builder()
            .tools(vec!["claude-code".to_string()])
            .build()
            .unwrap();
        let cases: &[(&str, &[(&str, &str)])] = &[
            // Agent Plugins `$schema`, any version.
            (
                r#"{"$schema":"https://agent-plugins.org/schemas/1.1.0/plugin.schema.json","name":"p"}"#,
                &[],
            ),
            // Agent Plugins layout without the manifest `$schema`: a root
            // mcp.json that declares the Agent Plugins MCP schema.
            (
                r#"{"name":"p"}"#,
                &[(
                    "mcp.json",
                    r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/mcp.schema.json","mcpServers":{}}"#,
                )],
            ),
            // Copilot client directory of an Agent Plugins package.
            (r#"{"name":"p"}"#, &[("com.github.copilot/lsp.json", "{}")]),
            // The Claude Code manifest is in .claude-plugin/; this one is for
            // the tools that read the root first.
            (
                r#"{"name":"p","agents":"agents/"}"#,
                &[(".claude-plugin/plugin.json", r#"{"name":"p"}"#)],
            ),
            // Copilot-only manifest fields.
            (r#"{"name":"p","category":"dev","tags":["x"]}"#, &[]),
            (r#"{"name":"p","extensions":"extensions/"}"#, &[]),
        ];
        for (manifest, files) in cases {
            for config in [&LintConfig::default(), &claude_only] {
                let diagnostics = validate_root_manifest(manifest, files, config);
                assert!(
                    diagnostics.is_empty(),
                    "{manifest} {files:?}: {diagnostics:?}"
                );
            }
        }
    }

    #[test]
    fn test_cc_pl_001_mcp_json_without_agent_plugins_schema_is_not_a_signal() {
        // A plain root mcp.json is Cursor's or the portable file of other
        // tools; it says nothing about the manifest.
        let diagnostics = validate_root_manifest(
            r#"{"name":"p"}"#,
            &[("mcp.json", r#"{"mcpServers":{}}"#)],
            &LintConfig::default(),
        );
        assert_eq!(
            cc_pl_001_levels(&diagnostics),
            vec![crate::diagnostics::DiagnosticLevel::Warning]
        );
    }

    #[test]
    fn test_agent_plugin_root_manifest_skips_claude_rules() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        let content = r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"example"}"#;
        write_plugin(&plugin_path, content);

        let diagnostics = PluginValidator.validate(&plugin_path, content, &LintConfig::default());

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_cursor_plugin_manifest_skips_claude_rules() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".cursor-plugin").join("plugin.json");
        let content = r#"{"name":"slack","logo":"logo.svg","mcpServers":"../.cursor-mcp.json"}"#;
        write_plugin(&plugin_path, content);

        let diagnostics = PluginValidator.validate(&plugin_path, content, &LintConfig::default());

        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_cc_pl_002_components_in_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0"}"#,
        );
        fs::create_dir_all(temp.path().join(".claude-plugin").join("skills")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-002"));
    }

    #[test]
    fn test_cc_pl_003_invalid_semver() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let version_diagnostic = diagnostics.iter().find(|d| d.rule == "CC-PL-003").unwrap();
        assert_eq!(
            version_diagnostic.level,
            crate::diagnostics::DiagnosticLevel::Info
        );
    }

    #[test]
    fn test_cc_pl_003_opaque_versions_are_advisory_not_parse_errors() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join(".claude-plugin/plugin.json");
        let validator = PluginValidator;
        for version in ["release-candidate", "2026.10", "branch/main"] {
            let content =
                format!(r#"{{"name":"test-plugin","description":"desc","version":"{version}"}}"#);
            write_plugin(&path, &content);
            let diagnostics = validator.validate(&path, &content, &LintConfig::default());
            assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
            let diagnostic = diagnostics.iter().find(|d| d.rule == "CC-PL-003").unwrap();
            assert_eq!(diagnostic.level, crate::diagnostics::DiagnosticLevel::Info);
        }
    }

    #[test]
    fn test_cc_pl_003_has_fix() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        let content = r#"{"name":"test-plugin","description":"desc","version":"1.0"}"#;
        write_plugin(&plugin_path, content);

        let validator = PluginValidator;
        let diagnostics = validator.validate(&plugin_path, content, &LintConfig::default());

        let cc_pl_003: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-003")
            .collect();
        assert_eq!(cc_pl_003.len(), 1);
        assert!(
            cc_pl_003[0].has_fixes(),
            "CC-PL-003 should have auto-fix for partial semver"
        );
        let fix = &cc_pl_003[0].fixes[0];
        assert!(!fix.safe, "CC-PL-003 fix should be unsafe");
        assert_eq!(
            fix.replacement, "1.0.0",
            "Fix should normalize '1.0' to '1.0.0'"
        );
    }

    #[test]
    fn test_cc_pl_003_valid_prerelease_version() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"4.0.0-rc.1"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_003_valid_build_metadata() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":"1.0.0+build.123"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_003_skips_empty_version() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test-plugin","description":"desc","version":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_004 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-004")
            .expect("CC-PL-004 should be reported for empty version");
        assert_eq!(
            pl_004.level,
            crate::diagnostics::DiagnosticLevel::Warning,
            "Empty version should be a warning, not an error"
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-003"));
    }

    #[test]
    fn test_cc_pl_004_missing_recommended_fields() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test-plugin"}"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_004: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-004")
            .collect();
        assert_eq!(
            pl_004.len(),
            2,
            "Should warn for missing description and version"
        );
        for d in &pl_004 {
            assert_eq!(
                d.level,
                crate::diagnostics::DiagnosticLevel::Warning,
                "Missing description/version should be warnings, not errors"
            );
        }
    }

    #[test]
    fn test_cc_pl_005_empty_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"  ","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cc_pl_005 = diagnostics
            .iter()
            .find(|d| d.rule == "CC-PL-005")
            .expect("CC-PL-005 should be reported");
        assert!(cc_pl_005.has_fixes());
        let fix = &cc_pl_005.fixes[0];
        assert_eq!(fix.replacement, "my-plugin");
        assert!(!fix.safe);
    }

    // ===== CC-PL-006: Plugin Parse Error =====

    #[test]
    fn test_cc_pl_006_invalid_json() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let parse_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-006")
            .collect();
        assert_eq!(parse_errors.len(), 1);
        assert!(parse_errors[0].message.contains("Failed to parse"));
    }

    #[test]
    fn test_cc_pl_006_truncated_json() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_empty_file() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, "");

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_valid_json_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    #[test]
    fn test_cc_pl_006_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid }"#);

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-006".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-006"));
    }

    // ===== Additional edge case tests =====

    #[test]
    fn test_cc_pl_001_valid_location_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-001"));
    }

    #[test]
    fn test_cc_pl_001_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-001".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-001"));
    }

    #[test]
    fn test_cc_pl_002_no_components_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );
        // No skills/agents/hooks/commands directories

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-002"));
    }

    #[test]
    fn test_cc_pl_002_multiple_components() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );
        // Create multiple disallowed directories
        fs::create_dir_all(temp.path().join(".claude-plugin").join("skills")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("agents")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("hooks")).unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin").join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_002_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-002")
            .collect();
        assert_eq!(pl_002_errors.len(), 4);
    }

    #[test]
    fn test_cc_pl_004_all_fields_present_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"A test plugin","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-004"));
    }

    #[test]
    fn test_cc_pl_004_empty_string_values() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"","version":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_004_warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-004")
            .collect();
        // Both description and version are empty - reported as warnings
        assert_eq!(pl_004_warnings.len(), 2);
        for d in &pl_004_warnings {
            assert_eq!(
                d.level,
                crate::diagnostics::DiagnosticLevel::Warning,
                "Empty description/version should be warnings"
            );
        }
    }

    #[test]
    fn test_cc_pl_004_missing_name_is_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"description":"d","version":"1.0.0"}"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let name_error = diagnostics
            .iter()
            .find(|d| {
                d.rule == "CC-PL-004" && d.level == crate::diagnostics::DiagnosticLevel::Error
            })
            .expect("CC-PL-004 error should be reported for missing name");
        assert!(
            name_error.message.contains("name"),
            "Error message should mention 'name'"
        );
    }

    #[test]
    fn test_cc_pl_004_only_name_present_no_errors() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test"}"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        // No CC-PL-004 errors - only warnings for missing description/version
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "CC-PL-004"
                    && d.level == crate::diagnostics::DiagnosticLevel::Error),
            "With name present, there should be zero CC-PL-004 errors"
        );

        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.rule == "CC-PL-004" && d.level == crate::diagnostics::DiagnosticLevel::Warning
            })
            .collect();
        assert_eq!(
            warnings.len(),
            2,
            "Should have warnings for missing description and version"
        );
        assert!(
            warnings.iter().any(|d| d.message.contains("description")),
            "Should mention 'description' in warning"
        );
        assert!(
            warnings.iter().any(|d| d.message.contains("version")),
            "Should mention 'version' in warning"
        );
    }

    #[test]
    fn test_cc_pl_004_non_string_name_is_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":123,"description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let name_error = diagnostics.iter().find(|d| {
            d.rule == "CC-PL-004" && d.level == crate::diagnostics::DiagnosticLevel::Error
        });
        assert!(
            name_error.is_some(),
            "Non-string name should trigger CC-PL-004 error"
        );
    }

    #[test]
    fn test_cc_pl_004_non_string_recommended_fields_are_warnings() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":123,"version":true}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.rule == "CC-PL-004" && d.level == crate::diagnostics::DiagnosticLevel::Warning
            })
            .collect();
        assert_eq!(
            warnings.len(),
            2,
            "Non-string description and version should trigger warnings"
        );
    }

    #[test]
    fn test_cc_pl_003_skips_when_version_absent() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{"name":"test"}"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-PL-003"),
            "CC-PL-003 should not fire when version is absent"
        );
    }

    #[test]
    fn test_cc_pl_003_fires_despite_non_string_description() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":123,"version":"1.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-003"),
            "CC-PL-003 should fire for invalid semver even with non-string description"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-004"
                && d.level == crate::diagnostics::DiagnosticLevel::Warning),
            "CC-PL-004 warning should fire for non-string description"
        );
    }

    #[test]
    fn test_cc_pl_004_disabled_via_config() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{}"#);

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-004".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-PL-004"),
            "CC-PL-004 should not fire when disabled"
        );
    }

    #[test]
    fn test_cc_pl_004_whitespace_only_name_is_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"   ","description":"desc","version":"1.0.0"}"#,
        );
        let content = fs::read_to_string(&plugin_path).unwrap();

        // CC-PL-005 owns a blank name, so CC-PL-004 does not report it twice.
        let diagnostics = PluginValidator.validate(&plugin_path, &content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-005"));
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-PL-004"),
            "{diagnostics:?}"
        );

        // With CC-PL-005 disabled, CC-PL-004 still reports the blank name as an error.
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-005".to_string()];
        let diagnostics = PluginValidator.validate(&plugin_path, &content, &config);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CC-PL-004"
                    && d.level == crate::diagnostics::DiagnosticLevel::Error),
            "Whitespace-only name should trigger CC-PL-004 error when CC-PL-005 is off"
        );
    }

    #[test]
    fn test_cc_pl_004_whitespace_only_recommended_fields() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"  ","version":"  "}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let warnings: Vec<_> = diagnostics
            .iter()
            .filter(|d| {
                d.rule == "CC-PL-004" && d.level == crate::diagnostics::DiagnosticLevel::Warning
            })
            .collect();
        assert_eq!(
            warnings.len(),
            2,
            "Whitespace-only description and version should trigger warnings"
        );
    }

    #[test]
    fn test_cc_pl_005_non_empty_name_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"my-plugin","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-005"));
    }

    #[test]
    fn test_config_disabled_plugins_category() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let mut config = LintConfig::default();
        config.rules_mut().plugins = false;

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(diagnostics.is_empty());
    }

    // ===== CC-PL-007: Invalid Component Path =====

    #[test]
    fn test_cc_pl_007_absolute_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"/usr/local/bin/cmd"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_windows_absolute_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"C:\\Users\\skills"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_traversal_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"../outside/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_embedded_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","hooks":"./valid/../escape"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_array_of_paths() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":["./valid","../invalid"]}"#,
        );
        // Component paths must exist (CC-PL-007 existence check).
        fs::create_dir_all(temp.path().join("valid")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-007")
            .collect();
        assert_eq!(
            pl_007.len(),
            1,
            "Only the invalid path should trigger CC-PL-007"
        );
    }

    #[test]
    fn test_cc_pl_007_valid_relative_path_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./commands/"}"#,
        );
        // Component paths must exist (CC-PL-007 existence check).
        fs::create_dir_all(temp.path().join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_plugin_root_skills_path_is_valid() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"."}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Claude Code v2.1.221 accepts '.' as the plugin-root skills path"
        );
    }

    #[test]
    fn test_cc_pl_007_dot_path_exception_is_scoped_to_skills() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"."}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Only the documented skills field accepts '.' as a root path"
        );
    }

    #[test]
    fn test_cc_pl_007_no_path_fields_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    #[test]
    fn test_cc_pl_007_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"/absolute"}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-007".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-007"));
    }

    // ===== CC-PL-008: Component Inside .claude-plugin =====

    #[test]
    fn test_cc_pl_008_path_inside_claude_plugin() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":".claude-plugin/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_array_with_mixed_paths() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":["./valid",".claude-plugin/invalid"]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_valid_path_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"./skills/"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    #[test]
    fn test_cc_pl_008_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":".claude-plugin/agents"}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-008".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-008"));
    }

    // ===== CC-PL-009: Invalid Author Object =====

    #[test]
    fn test_cc_pl_009_empty_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":""}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_whitespace_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":"  "}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_missing_author_name() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"email":"a@b.com"}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_author_not_object() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":"just a string"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_valid_author_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":"Test Author"}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_no_author_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    #[test]
    fn test_cc_pl_009_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","author":{"name":""}}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-009".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-009"));
    }

    // ===== CC-PL-010: Invalid Homepage URL =====

    #[test]
    fn test_cc_pl_010_invalid_url() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"not-a-url"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_ftp_url() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"ftp://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_valid_https_url_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"https://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_valid_http_url_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"http://example.com"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_no_homepage_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_empty_homepage_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":""}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    #[test]
    fn test_cc_pl_010_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":"not-a-url"}"#,
        );

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-010".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-010"));
    }

    // ===== Review feedback tests =====

    #[test]
    fn test_cc_pl_007_windows_forward_slash_absolute() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"C:/Users/skills"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "C:/ forward-slash Windows paths should be detected"
        );
    }

    #[test]
    fn test_cc_pl_007_trailing_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","hooks":"./foo/.."}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Trailing /.. should be detected as traversal"
        );
    }

    #[test]
    fn test_cc_pl_007_mixed_slash_traversal() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"./foo/..\\bar"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-007"),
            "Mixed slash traversal should be detected"
        );
    }

    #[test]
    fn test_cc_pl_007_autofixable_path() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"commands/run"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_007: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-007")
            .collect();
        assert_eq!(
            pl_007.len(),
            1,
            "Missing ./ prefix should trigger CC-PL-007"
        );
        assert!(!pl_007[0].fixes.is_empty(), "Should have a safe autofix");
        assert!(pl_007[0].fixes[0].safe, "Autofix should be safe");
        assert!(
            pl_007[0].fixes[0].replacement.starts_with("./"),
            "Autofix should prepend ./"
        );
    }

    #[test]
    fn test_cc_pl_008_dot_slash_prefix_bypass() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","agents":"./.claude-plugin/agents"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-008"),
            "./.claude-plugin/ should still be detected"
        );
    }

    #[test]
    fn test_cc_pl_010_non_string_homepage() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","homepage":123}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "CC-PL-010"),
            "Non-string homepage should trigger CC-PL-010"
        );
    }

    // ===== CC-PL-006 suggestion test =====

    #[test]
    fn test_cc_pl_006_has_suggestion() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, r#"{ invalid json }"#);

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let cc_pl_006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-006")
            .collect();
        assert_eq!(cc_pl_006.len(), 1);
        assert!(
            cc_pl_006[0].suggestion.is_some(),
            "CC-PL-006 should have a suggestion"
        );
        assert!(
            cc_pl_006[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("Validate JSON syntax"),
            "CC-PL-006 suggestion should mention JSON syntax"
        );
    }

    // ===== CC-PL-011: LSP Server Missing Required Fields =====

    #[test]
    fn test_cc_pl_011_missing_command() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","lspServers":{"myServer":{"extensionToLanguage":{".rs":"rust"}}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-011")
            .collect();
        assert_eq!(pl_011.len(), 1);
        assert!(
            pl_011[0].message.contains("command"),
            "Should mention missing 'command'"
        );
    }

    #[test]
    fn test_cc_pl_011_missing_extension_to_language() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","lspServers":{"myServer":{"command":"rust-analyzer"}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-011")
            .collect();
        assert_eq!(pl_011.len(), 1);
        assert!(
            pl_011[0].message.contains("extensionToLanguage"),
            "Should mention missing 'extensionToLanguage'"
        );
    }

    #[test]
    fn test_cc_pl_011_missing_both_fields() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","lspServers":{"myServer":{}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_011: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-011")
            .collect();
        assert_eq!(pl_011.len(), 2, "Should report both missing fields");
    }

    #[test]
    fn test_cc_pl_011_valid_lsp_server_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","lspServers":{"myServer":{"command":"rust-analyzer","extensionToLanguage":{".rs":"rust"}}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-011"));
    }

    #[test]
    fn test_cc_pl_011_no_lsp_servers_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-011"));
    }

    // ===== CC-PL-012: Invalid userConfig Key =====

    #[test]
    fn test_cc_pl_012_key_starts_with_number() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","userConfig":{"1invalid":{"type":"string"}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-012")
            .collect();
        assert_eq!(pl_012.len(), 1);
        assert!(
            pl_012[0].message.contains("1invalid"),
            "Should mention the invalid key"
        );
    }

    #[test]
    fn test_cc_pl_012_key_with_special_chars() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","userConfig":{"my-key":{"type":"string"},"my.key":{"type":"number"}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-012")
            .collect();
        assert_eq!(
            pl_012.len(),
            2,
            "Both keys with special characters should be flagged"
        );
    }

    #[test]
    fn test_cc_pl_012_valid_keys_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","userConfig":{"valid_key":{},"_also_valid":{},"camelCase123":{}}}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-012"));
    }

    #[test]
    fn test_cc_pl_012_no_user_config_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-012"));
    }

    // ===== CC-PL-013: Channels Entry Missing Server Field =====

    #[test]
    fn test_cc_pl_013_missing_server() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","mcpServers":{"myMcp":{}},"channels":[{"type":"inject"}]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_013: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-013")
            .collect();
        assert_eq!(pl_013.len(), 1);
        assert!(
            pl_013[0].message.contains("missing"),
            "Should mention missing 'server' field"
        );
    }

    #[test]
    fn test_cc_pl_013_server_not_in_mcp_servers() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","mcpServers":{"myMcp":{}},"channels":[{"server":"nonexistent"}]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        let pl_013: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-PL-013")
            .collect();
        assert_eq!(pl_013.len(), 1);
        assert!(
            pl_013[0].message.contains("nonexistent"),
            "Should mention the unresolved server reference"
        );
    }

    #[test]
    fn test_cc_pl_013_valid_server_reference_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","mcpServers":{"myMcp":{}},"channels":[{"server":"myMcp"}]}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-013"));
    }

    #[test]
    fn test_cc_pl_013_no_channels_no_error() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-013"));
    }

    // ===== CC-PL-014: Plugin Agent Uses Unsupported Field =====

    // ===== CC-PL-015: Default Component Folder Shadowed by Manifest =====

    #[test]
    fn test_cc_pl_015_default_component_folder_shadowed() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./custom-commands"}"#,
        );
        fs::create_dir_all(temp.path().join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    #[test]
    fn test_cc_pl_015_manifest_includes_default_component() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":["./custom-commands","./commands"]}"#,
        );
        fs::create_dir_all(temp.path().join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    #[test]
    fn test_cc_pl_015_no_default_component_folder() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./custom-commands"}"#,
        );

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    #[test]
    fn test_cc_pl_015_ignores_default_component_file() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./custom-commands"}"#,
        );
        fs::write(temp.path().join("commands"), "").unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    #[test]
    fn test_cc_pl_015_skips_invalid_manifest_path_shape() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":{}}"#,
        );
        fs::create_dir_all(temp.path().join("commands")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    #[test]
    fn test_cc_pl_015_disabled() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","commands":"./custom-commands"}"#,
        );
        fs::create_dir_all(temp.path().join("commands")).unwrap();

        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CC-PL-015".to_string()];

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &config,
        );

        assert!(!diagnostics.iter().any(|d| d.rule == "CC-PL-015"));
    }

    /// `skills` is additive: "The default `skills/` directory is always
    /// scanned, and directories listed in `skills` are loaded alongside it."
    /// It can never shadow the default folder, so CC-PL-015 must stay quiet.
    #[test]
    fn test_cc_pl_015_skills_is_additive_not_shadowing() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","skills":"./custom-skills"}"#,
        );
        fs::create_dir_all(temp.path().join("skills")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-PL-015"),
            "`skills` adds to the default scan and must not be reported as shadowing, got: {:?}",
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-PL-015")
                .collect::<Vec<_>>()
        );
    }

    /// `hooks` has its own documented merge rules rather than replace
    /// semantics, so a `hooks/` folder alongside a `hooks` key is not shadowed.
    #[test]
    fn test_cc_pl_015_hooks_has_own_merge_rules() {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(
            &plugin_path,
            r#"{"name":"test","description":"desc","version":"1.0.0","hooks":"./my-extra-hooks.json"}"#,
        );
        fs::create_dir_all(temp.path().join("hooks")).unwrap();

        let validator = PluginValidator;
        let diagnostics = validator.validate(
            &plugin_path,
            &fs::read_to_string(&plugin_path).unwrap(),
            &LintConfig::default(),
        );

        assert!(
            !diagnostics.iter().any(|d| d.rule == "CC-PL-015"),
            "`hooks` merges rather than replaces and must not be reported as shadowing, got: {:?}",
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-PL-015")
                .collect::<Vec<_>>()
        );
    }

    /// The replace-semantics list also covers `workflows`, `outputStyles`, and
    /// the two `experimental.*` keys. `outputStyles` maps to `output-styles/`
    /// and the experimental keys to root-level folders without the prefix.
    #[test]
    fn test_cc_pl_015_covers_remaining_replace_fields() {
        for (manifest, default_dir) in [
            (r#""workflows":"./custom-workflows""#, "workflows"),
            (r#""outputStyles":"./custom-styles""#, "output-styles"),
            (r#""experimental":{"themes":"./custom-themes"}"#, "themes"),
            (
                r#""experimental":{"monitors":"./custom-monitors.json"}"#,
                "monitors",
            ),
        ] {
            let temp = TempDir::new().unwrap();
            let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
            write_plugin(
                &plugin_path,
                &format!(r#"{{"name":"test","description":"desc","version":"1.0.0",{manifest}}}"#),
            );
            fs::create_dir_all(temp.path().join(default_dir)).unwrap();

            let validator = PluginValidator;
            let diagnostics = validator.validate(
                &plugin_path,
                &fs::read_to_string(&plugin_path).unwrap(),
                &LintConfig::default(),
            );

            assert!(
                diagnostics.iter().any(|d| d.rule == "CC-PL-015"),
                "{manifest} replaces the default `{default_dir}/` and must report CC-PL-015, got: {diagnostics:?}"
            );
        }
    }
    // ===== CC-PL-016: plugin name must be kebab-case =====

    /// Validate a manifest at the documented `.claude-plugin/plugin.json`
    /// location so CC-PL-001 does not fire and mask the name rules.
    fn validate_manifest(content: &str) -> Vec<Diagnostic> {
        let temp = TempDir::new().unwrap();
        let plugin_path = temp.path().join(".claude-plugin").join("plugin.json");
        write_plugin(&plugin_path, content);
        PluginValidator.validate(&plugin_path, content, &LintConfig::default())
    }

    #[test]
    fn test_cc_pl_016_kebab_case_names_are_valid() {
        for name in ["my-plugin", "agnix", "a1", "tool-2-name"] {
            let content = format!(r#"{{"name":"{name}","version":"1.0.0"}}"#);
            let diagnostics = validate_manifest(&content);
            assert!(
                diagnostics.iter().all(|d| d.rule != "CC-PL-016"),
                "{name} should be accepted, got {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cc_pl_016_flags_non_kebab_case_names() {
        for name in [
            "My Plugin",
            "my_plugin",
            "MyPlugin",
            "-leading",
            "trailing-",
            "double--hyphen",
        ] {
            let content = format!(r#"{{"name":"{name}","version":"1.0.0"}}"#);
            assert_eq!(
                validate_manifest(&content)
                    .iter()
                    .filter(|d| d.rule == "CC-PL-016")
                    .count(),
                1,
                "expected CC-PL-016 for {name}"
            );
        }
    }

    #[test]
    fn test_cc_pl_016_flags_control_and_invisible_characters() {
        // Claude Code v2.1.247 rejects marketplace names containing these;
        // kebab-case excludes them by construction.
        for name in ["my\u{7}plugin", "my\u{200b}plugin", "my\u{a0}plugin"] {
            let content = serde_json::json!({"name": name, "version": "1.0.0"}).to_string();
            assert!(
                validate_manifest(&content)
                    .iter()
                    .any(|d| d.rule == "CC-PL-016"),
                "expected CC-PL-016 for {name:?}"
            );
        }
    }

    #[test]
    fn test_cc_pl_016_empty_name_is_left_to_cc_pl_005() {
        // CC-PL-005 owns the empty case; reporting both would double-report.
        let diagnostics = validate_manifest(r#"{"name":"","version":"1.0.0"}"#);
        assert!(diagnostics.iter().all(|d| d.rule != "CC-PL-016"));
        assert!(diagnostics.iter().any(|d| d.rule == "CC-PL-005"));
    }
}

#[cfg(test)]
mod tests_claude_ext {
    //! CC-PL-007 (entry types, existence), CC-PL-010, CC-PL-018, CC-PL-019,
    //! CC-PL-021, CC-PL-022 and CC-PL-023.
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use std::fs;
    use tempfile::TempDir;

    fn plugin(manifest: &str, extra: &[(&str, &str)]) -> (TempDir, Vec<Diagnostic>) {
        let temp = TempDir::new().unwrap();
        for (rel, content) in extra {
            let path = temp.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
        }
        let path = temp.path().join(".claude-plugin/plugin.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, manifest).unwrap();
        let diagnostics = PluginValidator.validate(&path, manifest, &LintConfig::default());
        (temp, diagnostics)
    }

    fn hits<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    #[test]
    fn cc_pl_007_entry_types_and_existence() {
        let (_t, diagnostics) = plugin(
            r#"{"name": "p", "agents": ["./agents/a.md", "./agents/", {"name": "x"}], "skills": ["./skills/x/SKILL.md"], "commands": "./missing/"}"#,
            &[
                ("agents/a.md", "---\nname: a\ndescription: A\n---\n"),
                ("skills/x/SKILL.md", "x"),
            ],
        );
        let pl007 = hits(&diagnostics, "CC-PL-007");
        // agents dir entry, agents object entry, skills .md entry, missing commands dir
        assert_eq!(pl007.len(), 4, "{diagnostics:?}");
        assert!(pl007.iter().all(|d| d.level == DiagnosticLevel::Error));
        assert!(pl007.iter().any(|d| d.message.contains("does not exist")));
    }

    #[test]
    fn cc_pl_010_homepage_must_parse() {
        let (_t, diagnostics) = plugin(r#"{"name": "p", "homepage": "not a url"}"#, &[]);
        let h = hits(&diagnostics, "CC-PL-010");
        assert_eq!(h.len(), 1);
        assert_eq!(h[0].level, DiagnosticLevel::Error);
        assert_eq!(h[0].line, 1);
        let (_t, diagnostics) = plugin(r#"{"name": "p", "homepage": "mailto:x@y.z"}"#, &[]);
        assert_eq!(
            hits(&diagnostics, "CC-PL-010")[0].level,
            DiagnosticLevel::Info
        );
        let (_t, diagnostics) = plugin(r#"{"name": "p", "homepage": "https://x.dev/docs"}"#, &[]);
        assert!(hits(&diagnostics, "CC-PL-010").is_empty());
    }

    #[test]
    fn cc_pl_018_reserved_names() {
        for (name, level) in [
            ("claude-tools", Some(DiagnosticLevel::Error)),
            ("cc-plugin-x", Some(DiagnosticLevel::Error)),
            ("claude", Some(DiagnosticLevel::Error)),
            ("official-claude-tools", Some(DiagnosticLevel::Error)),
            ("Claude_Code", Some(DiagnosticLevel::Error)),
            ("mcp-for-claude", Some(DiagnosticLevel::Warning)),
            ("claudette", None),
            ("deploy-tools", None),
        ] {
            let (_t, diagnostics) = plugin(&format!(r#"{{"name": "{name}"}}"#), &[]);
            let h = hits(&diagnostics, "CC-PL-018");
            assert_eq!(h.first().map(|d| d.level), level, "{name}: {diagnostics:?}");
        }
        // Anthropic's own plugins are exempt.
        let (_t, diagnostics) = plugin(
            r#"{"name": "claude-code-setup", "author": {"name": "Anthropic"}}"#,
            &[],
        );
        assert!(hits(&diagnostics, "CC-PL-018").is_empty());
    }

    #[test]
    fn cc_pl_019_dependency_ranges() {
        for (range, ok) in [
            ("~2.1.0", true),
            ("^2.0", true),
            (">=1.4", true),
            ("=2.1.0", true),
            (">= 1.2.3 < 2", true),
            ("1.x || >=2.5.0", true),
            ("1.2 - 2.3", true),
            ("^2.0.0-0", true),
            ("latest", false),
            ("~>", false),
            ("2.1.0.1", false),
        ] {
            assert_eq!(is_semver_range(range), ok, "{range}");
        }
        let (_t, diagnostics) = plugin(
            r#"{"name": "p", "dependencies": ["a", {"name": "b", "version": "latest"}, {"name": "c", "version": "^1.2"}]}"#,
            &[],
        );
        assert_eq!(hits(&diagnostics, "CC-PL-019").len(), 1, "{diagnostics:?}");
    }

    #[test]
    fn cc_pl_021_manifest_settings() {
        let (_t, diagnostics) = plugin(
            r#"{"name": "p", "settings": {"agent": "x", "permissions": {}}}"#,
            &[],
        );
        let h = hits(&diagnostics, "CC-PL-021");
        assert_eq!(h.len(), 1, "{diagnostics:?}");
        assert!(h[0].message.contains("permissions"));
    }

    #[test]
    fn cc_pl_022_023_monitors() {
        let (_t, diagnostics) = plugin(
            r#"{"name": "p", "experimental": {"monitors": [{"name": "m", "command": "watch ${user_config.url}", "description": "d", "interval": 5}]}}"#,
            &[],
        );
        assert_eq!(hits(&diagnostics, "CC-PL-022").len(), 1, "{diagnostics:?}");
        assert_eq!(hits(&diagnostics, "CC-PL-023").len(), 1, "{diagnostics:?}");
        // The default monitors/monitors.json file is checked too.
        let (_t, diagnostics) = plugin(
            r#"{"name": "p"}"#,
            &[(
                "monitors/monitors.json",
                r#"[{"name": "m", "command": "x ${user_config.k}", "description": "d"}]"#,
            )],
        );
        let h = hits(&diagnostics, "CC-PL-022");
        assert_eq!(h.len(), 1, "{diagnostics:?}");
        assert!(h[0].file.ends_with("monitors/monitors.json"));
    }

    #[test]
    fn cc_pl_023_strict_objects() {
        let (_t, diagnostics) = plugin(
            r#"{"name": "p",
  "userConfig": {
    "token": {"type": "string", "title": "T", "description": "D", "secret": true},
    "tone": {"type": "string", "title": "T", "description": "D", "options": ["a", "b"], "default": "c"},
    "ok": {"type": "string", "title": "T", "description": "D", "options": ["a"], "required": true}
  },
  "mcpServers": {"s": {"command": "x"}},
  "channels": [{"server": "s", "label": "x"}],
  "lspServers": {"go": {"command": "gopls", "extensionToLanguage": {".go": "go"}, "port": 1}}
}"#,
            &[],
        );
        let h = hits(&diagnostics, "CC-PL-023");
        // secret, tone default, channels label, lsp port
        assert_eq!(h.len(), 4, "{diagnostics:?}");
        assert!(h.iter().all(|d| d.level == DiagnosticLevel::Error));
    }
}
