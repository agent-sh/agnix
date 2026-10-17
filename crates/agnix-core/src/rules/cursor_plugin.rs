//! Cursor plugin manifests: CUR-030 (required fields and name format) and
//! CUR-040 (submission checklist: relative paths, declared variables, unique
//! names).
//!
//! "Every Cursor Plugin requires a `.cursor-plugin/plugin.json` manifest
//! file", and a multi-plugin repository has `.cursor-plugin/marketplace.json`
//! at its root.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::config_json::{JsonCx, key_span, node_span},
    schemas::kiro_json::{JsonNode, JsonValue, parse_json_document},
};
use regex::Regex;
use rust_i18n::t;
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::OnceLock;

/// Manifest fields that hold paths inside the plugin.
const PATH_FIELDS: &[&str] = &[
    "rules",
    "agents",
    "skills",
    "commands",
    "hooks",
    "mcpServers",
    "logo",
];

/// Paths of a marketplace plugin entry.
const MARKETPLACE_ENTRY_PATH_FIELDS: &[&str] = &[
    "source",
    "rules",
    "agents",
    "skills",
    "commands",
    "hooks",
    "mcpServers",
    "logo",
];

/// Variables Cursor expands itself in plugin `mcp.json` files, which need no
/// `variables` declaration.
const BUILT_IN_VARIABLES: &[&str] = &[
    "CURSOR_PLUGIN_ROOT",
    "CLAUDE_PLUGIN_ROOT",
    "userHome",
    "workspaceFolder",
    "workspaceFolderBasename",
    "pathSeparator",
    "/",
];

/// "Lowercase, kebab-case (alphanumerics, hyphens, and periods). Must start
/// and end with an alphanumeric character."
fn is_plugin_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.'))
        && bytes[0].is_ascii_alphanumeric()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
}

/// `${VAR}` placeholders that are plugin variables.
fn placeholder_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\$\{([^}:]+)\}").expect("placeholder pattern"))
}

/// True for a manifest path that leaves the plugin: absolute, a drive path,
/// or with a `..` segment. URLs (for `logo`) are allowed.
fn escapes_plugin(value: &str) -> bool {
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    let bytes = value.as_bytes();
    value.starts_with('/')
        || value.starts_with('\\')
        || value.starts_with('~')
        || (bytes.len() > 1 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':')
        || value.split(['/', '\\']).any(|segment| segment == "..")
}

pub(crate) fn validate_plugin_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let marketplace = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| name == "marketplace.json");
    let document = match parse_json_document(content, false) {
        Ok(document) => document,
        Err(error) => {
            if cx.enabled("CUR-030") {
                cx.push_at(
                    DiagnosticLevel::Error,
                    "CUR-030",
                    (error.line, error.column),
                    t!("rules.cur_030.parse_error", error = error.message.as_str()),
                    t!("rules.cur_030.parse_error_suggestion"),
                );
            }
            return cx.diagnostics;
        }
    };
    let root = &document.root;
    if root.value.as_object().is_none() {
        if cx.enabled("CUR-030") {
            cx.push(
                DiagnosticLevel::Error,
                "CUR-030",
                node_span(root),
                t!("rules.cur_030.not_object"),
                t!("rules.cur_030.parse_error_suggestion"),
            );
        }
        return cx.diagnostics;
    }
    if marketplace {
        validate_marketplace(&mut cx, root);
    } else {
        validate_manifest(&mut cx, root, path, config);
    }
    cx.diagnostics
}

/// CUR-030 for a `name` member (required, kebab-case).
fn check_name(cx: &mut JsonCx<'_>, object: &JsonNode, what: &str) {
    if !cx.enabled("CUR-030") {
        return;
    }
    match object.value.get("name") {
        None => cx.push(
            DiagnosticLevel::Error,
            "CUR-030",
            object
                .value
                .as_object()
                .and_then(|m| m.first())
                .map_or(node_span(object), key_span),
            t!("rules.cur_030.missing_name", what = what),
            t!("rules.cur_030.name_suggestion"),
        ),
        Some(member) => match member.value.value.as_str() {
            Some(name) if is_plugin_name(name) => {}
            Some(name) => cx.push(
                DiagnosticLevel::Error,
                "CUR-030",
                node_span(&member.value),
                t!("rules.cur_030.invalid_name", what = what, name = name),
                t!("rules.cur_030.name_suggestion"),
            ),
            None => cx.push(
                DiagnosticLevel::Error,
                "CUR-030",
                node_span(&member.value),
                t!("rules.cur_030.name_not_string", what = what),
                t!("rules.cur_030.name_suggestion"),
            ),
        },
    }
}

fn validate_manifest(
    cx: &mut JsonCx<'_>,
    root: &JsonNode,
    path: &Path,
    config: &PerFileLintConfig<'_>,
) {
    check_name(cx, root, "plugin");
    if cx.enabled("CUR-030")
        && let Some(author) = root.value.get("author")
        && author.value.value.as_object().is_some()
        && author.value.value.get("name").is_none()
    {
        cx.push(
            DiagnosticLevel::Error,
            "CUR-030",
            key_span(author),
            t!("rules.cur_030.author_name"),
            t!("rules.cur_030.author_name_suggestion"),
        );
    }
    if !cx.enabled("CUR-040") {
        return;
    }
    check_paths(cx, root, PATH_FIELDS);

    // "Cursor Plugins using variables declare every `${VAR}` from `mcp.json`
    // in the manifest schema."
    let declared: BTreeSet<String> = root
        .value
        .get("variables")
        .and_then(|m| m.value.value.get("properties"))
        .and_then(|m| m.value.value.as_object())
        .map(|props| props.iter().map(|p| p.key.clone()).collect())
        .unwrap_or_default();
    let Some(plugin_root) = path.parent().and_then(Path::parent) else {
        return;
    };
    let text = match root.value.get("mcpServers").map(|m| &m.value) {
        Some(node) if node.value.as_object().is_some() => Some(json_text(&node.value)),
        Some(node) => node
            .value
            .as_str()
            .filter(|file| !escapes_plugin(file))
            .and_then(|file| {
                config
                    .fs()
                    .read_to_string(&plugin_root.join(file.trim_start_matches("./")))
                    .ok()
            }),
        None => config
            .fs()
            .read_to_string(&plugin_root.join("mcp.json"))
            .ok(),
    };
    let Some(text) = text else {
        return;
    };
    let mut reported = BTreeSet::new();
    for capture in placeholder_regex().captures_iter(&text) {
        let name = capture[1].trim();
        if BUILT_IN_VARIABLES.contains(&name)
            || name.starts_with("env:")
            || declared.contains(name)
            || !reported.insert(name.to_string())
        {
            continue;
        }
        let span = match root.value.get("variables") {
            Some(variables) => key_span(variables),
            None => root
                .value
                .as_object()
                .and_then(|m| m.first())
                .map_or(node_span(root), key_span),
        };
        cx.push(
            DiagnosticLevel::Warning,
            "CUR-040",
            span,
            t!("rules.cur_040.undeclared_variable", name = name),
            t!("rules.cur_040.undeclared_variable_suggestion", name = name),
        );
    }
}

/// A compact rendering of a JSON value's strings, enough to find `${VAR}`.
fn json_text(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Array(items) => items
            .iter()
            .map(|i| json_text(&i.value))
            .collect::<Vec<_>>()
            .join(" "),
        JsonValue::Object(members) => members
            .iter()
            .map(|m| json_text(&m.value.value))
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

/// CUR-040: "All paths in manifest are relative and valid (no `..`, no
/// absolute paths)". Marketplace entries also have a `source` path.
fn check_paths(cx: &mut JsonCx<'_>, object: &JsonNode, fields: &[&str]) {
    for field in fields {
        let Some(member) = object.value.get(field) else {
            continue;
        };
        let values: Vec<&JsonNode> = match &member.value.value {
            JsonValue::String(_) => vec![&member.value],
            JsonValue::Array(items) => items
                .iter()
                .filter(|i| i.value.as_str().is_some())
                .collect(),
            _ => Vec::new(),
        };
        for node in values {
            let Some(text) = node.value.as_str() else {
                continue;
            };
            if escapes_plugin(text) {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CUR-040",
                    node_span(node),
                    t!("rules.cur_040.path", field = *field, path = text),
                    t!("rules.cur_040.path_suggestion"),
                );
            }
        }
    }
}

fn validate_marketplace(cx: &mut JsonCx<'_>, root: &JsonNode) {
    if cx.enabled("CUR-030") {
        check_name(cx, root, "marketplace");
        for field in ["owner", "plugins"] {
            if root.value.get(field).is_none() {
                cx.push(
                    DiagnosticLevel::Error,
                    "CUR-030",
                    root.value
                        .as_object()
                        .and_then(|m| m.first())
                        .map_or(node_span(root), key_span),
                    t!("rules.cur_030.marketplace_missing", field = field),
                    t!("rules.cur_030.marketplace_suggestion"),
                );
            }
        }
        if let Some(owner) = root.value.get("owner")
            && owner.value.value.as_object().is_some()
            && owner.value.value.get("name").is_none()
        {
            cx.push(
                DiagnosticLevel::Error,
                "CUR-030",
                key_span(owner),
                t!("rules.cur_030.owner_name"),
                t!("rules.cur_030.marketplace_suggestion"),
            );
        }
    }
    let Some(plugins) = root
        .value
        .get("plugins")
        .and_then(|m| m.value.value.as_array())
    else {
        return;
    };
    let mut seen = BTreeSet::new();
    for entry in plugins {
        if entry.value.as_object().is_none() {
            continue;
        }
        check_name(cx, entry, "marketplace plugin");
        if cx.enabled("CUR-040") {
            check_paths(cx, entry, MARKETPLACE_ENTRY_PATH_FIELDS);
            if let Some(member) = entry.value.get("name")
                && let Some(name) = member.value.value.as_str()
                && !seen.insert(name.to_string())
            {
                cx.push(
                    DiagnosticLevel::Warning,
                    "CUR-040",
                    node_span(&member.value),
                    t!("rules.cur_040.duplicate_name", name = name),
                    t!("rules.cur_040.duplicate_name_suggestion"),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::LintConfig;
    use crate::diagnostics::Diagnostic;
    use crate::rules::Validator;
    use crate::rules::cursor::CursorValidator;
    use std::path::Path;

    fn validate(path: &Path, content: &str) -> Vec<Diagnostic> {
        CursorValidator.validate(path, content, &LintConfig::default())
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    #[test]
    fn documented_manifest_is_clean() {
        let content = r#"{
  "name": "enterprise-plugin",
  "version": "1.2.0",
  "description": "Enterprise development tools",
  "author": { "name": "ACME DevTools", "email": "devtools@acme.com" },
  "keywords": ["enterprise"],
  "logo": "assets/logo.svg",
  "skills": "./skills/"
}"#;
        let diagnostics = validate(Path::new("my-plugin/.cursor-plugin/plugin.json"), content);
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        assert_eq!(
            crate::detect_file_type(Path::new("my-plugin/.cursor-plugin/plugin.json")),
            crate::FileType::CursorPlugin
        );
    }

    #[test]
    fn cur_030_name_and_required_fields() {
        let path = Path::new(".cursor-plugin/plugin.json");
        for (content, expected) in [
            (r#"{ "description": "x" }"#, 1),
            (r#"{ "name": "My_Plugin" }"#, 1),
            (r#"{ "name": "-plugin" }"#, 1),
            (r#"{ "name": "prompts.chat" }"#, 0),
            (r#"{ "name": "x", "author": { "email": "a@b.c" } }"#, 1),
            ("{ \"name\": ", 1),
        ] {
            let diagnostics = validate(path, content);
            assert_eq!(
                rule(&diagnostics, "CUR-030").len(),
                expected,
                "{content}: {diagnostics:#?}"
            );
        }
        let marketplace = Path::new(".cursor-plugin/marketplace.json");
        let diagnostics = validate(
            marketplace,
            r#"{ "name": "my-marketplace", "plugins": [{ "source": "a" }] }"#,
        );
        // Missing owner, and a plugin entry without a name.
        assert_eq!(rule(&diagnostics, "CUR-030").len(), 2, "{diagnostics:#?}");
    }

    #[test]
    fn cur_040_checklist() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("plugin");
        std::fs::create_dir_all(root.join(".cursor-plugin")).unwrap();
        std::fs::write(
            root.join("mcp.json"),
            r#"{ "mcpServers": { "api": { "url": "https://mcp.example.com", "headers": { "Authorization": "Bearer ${API_TOKEN}", "X-Org": "${ORG_ID}" } }, "local": { "command": "${CURSOR_PLUGIN_ROOT}/bin/server", "env": { "HOME_DIR": "${env:HOME}" } } } }"#,
        )
        .unwrap();
        let manifest = root.join(".cursor-plugin/plugin.json");
        let content = r#"{
  "name": "example-plugin",
  "rules": ["rules/", "../shared/rules/"],
  "logo": "https://example.com/logo.svg",
  "variables": { "type": "object", "properties": { "API_TOKEN": { "type": "string" } } }
}"#;
        let diagnostics = validate(&manifest, content);
        let cur_040 = rule(&diagnostics, "CUR-040");
        assert_eq!(cur_040.len(), 2, "{cur_040:#?}");
        assert!(
            cur_040
                .iter()
                .any(|d| d.message.contains("../shared/rules/"))
        );
        assert!(cur_040.iter().any(|d| d.message.contains("ORG_ID")));

        let marketplace = Path::new(".cursor-plugin/marketplace.json");
        let diagnostics = validate(
            marketplace,
            r#"{ "name": "m", "owner": { "name": "Org" }, "plugins": [{ "name": "a", "source": "a" }, { "name": "a", "source": "/abs/a" }] }"#,
        );
        // The duplicate name and the absolute source.
        assert_eq!(rule(&diagnostics, "CUR-040").len(), 2, "{diagnostics:#?}");
    }
}
