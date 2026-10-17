//! Gemini CLI extension manifest validation rules (GM-005, GM-008, GM-028)
//!
//! Validates `gemini-extension.json` the way Gemini CLI loads it
//! (extension-manager.ts) and `gemini extensions validate` checks it
//! (commands/extensions/validate.ts):
//! - GM-005: the manifest does not load (HIGH): JSON syntax, missing `name`
//!   or `version`, a name with characters outside `[a-zA-Z0-9-]`
//! - GM-008: a `contextFileName` entry the loader rejects (absolute path,
//!   `..`) or that does not exist in the extension directory (HIGH)
//! - GM-028: manifest advice (LOW): no description, uppercase name,
//!   non-semver version
//! - GM-011, GM-014, GM-015, GM-023: `mcpServers` names, transports, the
//!   ignored `trust`, and portable commands (`rules::gemini_mcp`)
//! - GM-022: a `settings[]` entry that looks like a secret without
//!   `sensitive: true`

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::{
        Validator, ValidatorMetadata,
        gemini_mcp::{JsonSink, McpSource, check_mcp_servers},
    },
    schemas::{
        gemini_extension::{
            ContextPathProblem, check_context_file_path, is_loadable_extension_name, is_semver,
            looks_like_secret_setting,
        },
        kiro_json::{JsonMember, JsonNode, JsonValue, LineIndex, parse_json_document},
    },
};
use rust_i18n::t;
use std::path::Path;

/// Rules this validator owns. It also emits GM-011, GM-014 and GM-015
/// through `rules::gemini_mcp`; GeminiSettingsValidator lists those.
const RULE_IDS: &[&str] = &["GM-005", "GM-008", "GM-022", "GM-023", "GM-028"];

pub struct GeminiExtensionValidator;

impl Validator for GeminiExtensionValidator {
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
        let mut cx = Cx {
            path,
            content,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        };

        // The manifest is read with JSON.parse: strict JSON.
        let doc = match parse_json_document(content, false) {
            Ok(doc) => doc,
            Err(error) => {
                if config.is_rule_enabled("GM-005") {
                    cx.diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            error.line,
                            error.column,
                            "GM-005",
                            t!(
                                "rules.gm_005.message",
                                description =
                                    t!("rules.gm_005.parse_error", error = error.message.as_str())
                            ),
                        )
                        .with_suggestion(t!("rules.gm_005.suggestion")),
                    );
                }
                return cx.diagnostics;
            }
        };
        let root = &doc.root;
        if root.value.as_object().is_none() {
            let description = t!("rules.gm_005.not_object", found = root.value.type_name());
            cx.gm_005((root.start, root.end), description);
            return cx.diagnostics;
        }

        cx.check_name(root);
        cx.check_version(root);
        cx.check_description(root);
        let extension_root = path.parent().unwrap_or(Path::new(""));
        if let Some(member) = root.value.get("contextFileName") {
            cx.check_context_file_name(member, extension_root);
        }
        if let Some(member) = root.value.get("settings") {
            cx.check_settings(member);
        }
        if let Some(member) = root.value.get("mcpServers") {
            let mut sink = JsonSink::new(path, content, config);
            let source = McpSource::Extension {
                root: extension_root.to_path_buf(),
            };
            check_mcp_servers(&mut sink, member, &source);
            cx.diagnostics.append(&mut sink.diagnostics);
        }
        cx.diagnostics
    }
}

/// JavaScript truthiness of a JSON value (`if (!rawConfig.name)`).
fn is_truthy(value: &JsonValue) -> bool {
    match value {
        JsonValue::Null => false,
        JsonValue::Bool(b) => *b,
        JsonValue::Number(n) => n.as_f64().is_some_and(|v| v != 0.0),
        JsonValue::String(s) => !s.is_empty(),
        JsonValue::Array(_) | JsonValue::Object(_) => true,
    }
}

/// `String(value)` for the scalars `validateName` may see.
fn js_string(node: &JsonNode, content_slice: &str) -> String {
    match &node.value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Object(_) => "[object Object]".to_string(),
        _ => content_slice.to_string(),
    }
}

struct Cx<'a, 'c> {
    path: &'a Path,
    content: &'a str,
    lines: LineIndex<'a>,
    config: &'a PerFileLintConfig<'c>,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_, '_> {
    fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        span: (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        if !self.config.is_rule_enabled(rule) {
            return;
        }
        let (line, column) = self.lines.line_col(span.0);
        let (end_line, end_column) = self.lines.line_col(span.1.max(span.0));
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }

    fn gm_005(&mut self, span: (usize, usize), description: impl AsRef<str>) {
        self.push(
            DiagnosticLevel::Error,
            "GM-005",
            span,
            t!("rules.gm_005.message", description = description.as_ref()),
            t!("rules.gm_005.suggestion"),
        );
    }

    fn gm_028(&mut self, span: (usize, usize), message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Info,
            "GM-028",
            span,
            message,
            t!("rules.gm_028.suggestion"),
        );
    }

    /// Span of a member's value, or of the opening brace when it is absent.
    fn span_of(member: Option<&JsonMember>, root: &JsonNode) -> (usize, usize) {
        member.map_or((root.start, root.start + 1), |m| {
            (m.value.start, m.value.end)
        })
    }

    /// GM-005: "Invalid configuration ...: missing "name"", and
    /// `validateName`. GM-028: "The name should be lowercase".
    fn check_name(&mut self, root: &JsonNode) {
        let member = root.value.get("name");
        if !member.is_some_and(|m| is_truthy(&m.value.value)) {
            let span = Self::span_of(member, root);
            self.gm_005(span, t!("rules.gm_005.missing_field", field = "name"));
            return;
        }
        let Some(member) = member else { return };
        let span = (member.value.start, member.value.end);
        let raw = self.lines_slice(span);
        let name = js_string(&member.value, &raw);
        if !is_loadable_extension_name(&name) {
            self.gm_005(span, t!("rules.gm_005.invalid_name", name = name.as_str()));
        } else if name.chars().any(|c| c.is_ascii_uppercase()) {
            self.gm_028(
                span,
                t!("rules.gm_028.uppercase_name", name = name.as_str()),
            );
        }
    }

    /// GM-005: missing "version". GM-028: "Version '...' does not appear to
    /// be standard semver (e.g., 1.0.0)."
    fn check_version(&mut self, root: &JsonNode) {
        let member = root.value.get("version");
        let Some(member) = member.filter(|m| is_truthy(&m.value.value)) else {
            let span = Self::span_of(member, root);
            self.gm_005(span, t!("rules.gm_005.missing_field", field = "version"));
            return;
        };
        let span = (member.value.start, member.value.end);
        let semver = member.value.value.as_str().is_some_and(is_semver);
        if !semver {
            let raw = self.lines_slice(span);
            let version = js_string(&member.value, &raw);
            self.gm_028(
                span,
                t!("rules.gm_028.not_semver", version = version.as_str()),
            );
        }
    }

    /// GM-028: "`description`: A short description of the extension. This
    /// will be displayed on geminicli.com/extensions."
    fn check_description(&mut self, root: &JsonNode) {
        let member = root.value.get("description");
        let missing = match member.map(|m| &m.value.value) {
            None | Some(JsonValue::Null) => true,
            Some(JsonValue::String(text)) => text.trim().is_empty(),
            Some(_) => false,
        };
        if missing {
            let span = Self::span_of(member, root);
            self.gm_028(span, t!("rules.gm_028.missing_description"));
        }
    }

    /// GM-008: `contextFileName` is a string or an array of strings; each
    /// entry goes through `validateContextFilePath` and must exist.
    fn check_context_file_name(&mut self, member: &JsonMember, extension_root: &Path) {
        let node = &member.value;
        // `getContextFileNames`: a falsy value falls back to GEMINI.md.
        if !is_truthy(&node.value) {
            return;
        }
        match &node.value {
            JsonValue::String(entry) => self.check_context_entry(entry, node, extension_root),
            JsonValue::Array(items) => {
                for item in items {
                    match &item.value {
                        JsonValue::String(entry) => {
                            self.check_context_entry(entry, item, extension_root);
                        }
                        other => self.gm_008(
                            (item.start, item.end),
                            t!("rules.gm_008.not_string", found = other.type_name()),
                        ),
                    }
                }
            }
            other => self.gm_008(
                (node.start, node.end),
                t!(
                    "rules.gm_008.not_string_or_array",
                    found = other.type_name()
                ),
            ),
        }
    }

    fn check_context_entry(&mut self, entry: &str, node: &JsonNode, extension_root: &Path) {
        let span = (node.start, node.end);
        match check_context_file_path(entry) {
            Err(ContextPathProblem::NotNonEmptyString) => {
                self.gm_008(span, t!("rules.gm_008.empty"));
            }
            Err(ContextPathProblem::VariableAbsolute) => {
                self.gm_008(span, t!("rules.gm_008.variable_absolute", value = entry));
            }
            Err(ContextPathProblem::Absolute) => {
                self.gm_008(span, t!("rules.gm_008.absolute", value = entry));
            }
            Err(ContextPathProblem::ParentReference) => {
                self.gm_008(span, t!("rules.gm_008.parent", value = entry));
            }
            Ok(relative) => {
                // Other variables cannot be resolved here.
                if relative.contains("${") {
                    return;
                }
                let fs = self.config.fs().clone();
                if !fs.exists(&extension_root.join(&relative)) {
                    self.gm_008(span, t!("rules.gm_008.not_found", value = entry));
                }
            }
        }
    }

    /// GM-022: "If your extension requires API keys or other secrets, use
    /// the `sensitive: true` option in your manifest. This ensures keys are
    /// stored in the system keychain and obfuscated in the CLI output."
    fn check_settings(&mut self, member: &JsonMember) {
        let Some(items) = member.value.value.as_array() else {
            return;
        };
        for item in items {
            if item.value.as_object().is_none()
                || item
                    .value
                    .get("sensitive")
                    .and_then(|m| m.value.value.as_bool())
                    == Some(true)
            {
                continue;
            }
            let field = |key: &str| {
                item.value
                    .get(key)
                    .and_then(|m| m.value.value.as_str().map(|text| (m, text)))
            };
            let Some((hit, _)) = [field("envVar"), field("name")]
                .into_iter()
                .flatten()
                .find(|(_, text)| looks_like_secret_setting(text))
            else {
                continue;
            };
            let label = field("name")
                .or_else(|| field("envVar"))
                .map_or("", |(_, text)| text);
            self.push(
                DiagnosticLevel::Warning,
                "GM-022",
                (hit.value.start, hit.value.end),
                t!("rules.gm_022.message", setting = label),
                t!("rules.gm_022.suggestion"),
            );
        }
    }

    fn gm_008(&mut self, span: (usize, usize), message: impl Into<String>) {
        self.push(
            DiagnosticLevel::Error,
            "GM-008",
            span,
            message,
            t!("rules.gm_008.suggestion"),
        );
    }

    fn lines_slice(&self, span: (usize, usize)) -> String {
        self.content.get(span.0..span.1).unwrap_or("").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    /// The extension directory is `/ext`, holding the given files.
    fn config_with(files: &[&str]) -> LintConfig {
        let fs = MockFileSystem::new();
        for file in files {
            fs.add_file(format!("/ext/{file}"), "# context");
        }
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        config
    }

    fn validate_in(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        GeminiExtensionValidator.validate(Path::new("/ext/gemini-extension.json"), content, config)
    }

    fn validate(content: &str) -> Vec<Diagnostic> {
        validate_in(content, &config_with(&["GEMINI.md", "docs/CONTEXT.md"]))
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel)> {
        diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level))
            .collect()
    }

    const OK: &str = r#""version": "1.0.0", "description": "Test""#;

    // ===== GM-005 =====

    #[test]
    fn test_gm_005_invalid_json() {
        let diagnostics = validate("{ invalid }");
        assert_eq!(rules(&diagnostics), [("GM-005", DiagnosticLevel::Error)]);
    }

    #[test]
    fn test_gm_005_jsonc_comment_is_a_parse_error() {
        // JSON.parse does not accept comments.
        let diagnostics = validate("{\n  // note\n  \"name\": \"x\", \"version\": \"1.0.0\"\n}");
        assert_eq!(rules(&diagnostics), [("GM-005", DiagnosticLevel::Error)]);
        assert_eq!(diagnostics[0].line, 2);
    }

    #[test]
    fn test_gm_005_not_an_object() {
        let diagnostics = validate("[]");
        assert_eq!(rules(&diagnostics), [("GM-005", DiagnosticLevel::Error)]);
    }

    #[test]
    fn test_gm_005_missing_name_and_version_but_not_description() {
        // Description is display text (GM-028), not a load requirement.
        let diagnostics = validate("{}");
        let gm_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "GM-005").collect();
        assert_eq!(gm_005.len(), 2);
        assert!(gm_005.iter().all(|d| d.level == DiagnosticLevel::Error));
        assert!(gm_005[0].message.contains("'name'"));
        assert!(gm_005[1].message.contains("'version'"));
        assert!(diagnostics.iter().any(|d| d.rule == "GM-028"));
    }

    #[test]
    fn test_gm_005_falsy_values_are_missing() {
        for value in [r#""""#, "null", "false", "0"] {
            let content = format!(r#"{{"name": {value}, {OK}}}"#);
            let diagnostics = validate(&content);
            assert_eq!(
                rules(&diagnostics),
                [("GM-005", DiagnosticLevel::Error)],
                "{value}"
            );
            // Points at the value.
            assert_eq!(diagnostics[0].column, 10, "{value}");
        }
    }

    #[test]
    fn test_gm_005_name_characters() {
        for name in ["My Extension", "my_extension", "my.extension"] {
            let content = format!(r#"{{"name": "{name}", {OK}}}"#);
            let diagnostics = validate(&content);
            assert_eq!(
                rules(&diagnostics),
                [("GM-005", DiagnosticLevel::Error)],
                "{name}"
            );
            assert!(
                diagnostics[0]
                    .message
                    .contains("letters, numbers and dashes")
            );
        }
    }

    #[test]
    fn test_gm_005_valid_extension() {
        let content = format!(r#"{{"name": "my-extension", {OK}}}"#);
        assert!(validate(&content).is_empty());
    }

    // ===== GM-028 =====

    #[test]
    fn test_gm_028_uppercase_name_is_advice_only() {
        // validateName accepts A-Z; the docs say names "should be lowercase".
        let content = format!(r#"{{"name": "My-Extension", {OK}}}"#);
        let diagnostics = validate(&content);
        assert_eq!(rules(&diagnostics), [("GM-028", DiagnosticLevel::Info)]);
        assert!(diagnostics[0].message.contains("lowercase"));
    }

    #[test]
    fn test_gm_028_missing_or_empty_description() {
        for content in [
            r#"{"name": "ext", "version": "1.0.0"}"#,
            r#"{"name": "ext", "version": "1.0.0", "description": " "}"#,
        ] {
            let diagnostics = validate(content);
            assert_eq!(
                rules(&diagnostics),
                [("GM-028", DiagnosticLevel::Info)],
                "{content}"
            );
        }
    }

    #[test]
    fn test_gm_028_version_not_semver() {
        for version in [r#""1.0""#, r#""latest""#, "1"] {
            let content = format!(r#"{{"name": "ext", "version": {version}, "description": "d"}}"#);
            let diagnostics = validate(&content);
            assert_eq!(
                rules(&diagnostics),
                [("GM-028", DiagnosticLevel::Info)],
                "{version}"
            );
            assert!(diagnostics[0].message.contains("semver"));
        }
        let content = r#"{"name": "ext", "version": "v1.2.3", "description": "d"}"#;
        assert!(validate(content).is_empty());
    }

    // ===== GM-008 =====

    fn with_context(value: &str) -> String {
        format!(r#"{{"name": "ext", {OK}, "contextFileName": {value}}}"#)
    }

    #[test]
    fn test_gm_008_relative_subpaths_are_accepted() {
        // The former rule flagged any path and auto-fixed it to the basename,
        // which pointed the manifest at a file that does not exist.
        for value in [
            r#""GEMINI.md""#,
            r#""docs/CONTEXT.md""#,
            r#"["GEMINI.md", "docs/CONTEXT.md"]"#,
            r#""""#,
            "null",
        ] {
            let diagnostics = validate(&with_context(value));
            assert!(diagnostics.is_empty(), "{value}: {diagnostics:?}");
        }
    }

    #[test]
    fn test_gm_008_rejected_paths_are_errors_without_fixes() {
        for (value, needle) in [
            (r#""/abs/GEMINI.md""#, "absolute"),
            (r#""C:\\ctx\\GEMINI.md""#, "absolute"),
            (r#""${extensionPath}/GEMINI.md""#, "absolute"),
            (r#""../GEMINI.md""#, ".."),
            (r#""docs/../GEMINI.md""#, ".."),
            (r#""missing.md""#, "does not exist"),
            (r#"["GEMINI.md", 5]"#, "string"),
            (r#"["GEMINI.md", ""]"#, "non-empty"),
            ("5", "string or an array"),
        ] {
            let diagnostics = validate(&with_context(value));
            assert_eq!(
                rules(&diagnostics),
                [("GM-008", DiagnosticLevel::Error)],
                "{value}"
            );
            assert!(
                diagnostics[0].message.contains(needle),
                "{value}: {}",
                diagnostics[0].message
            );
            assert!(diagnostics[0].fixes.is_empty(), "{value}");
        }
    }

    #[test]
    fn test_gm_008_points_at_the_array_entry() {
        let content = "{\"name\": \"ext\", \"version\": \"1.0.0\", \"description\": \"d\",\n  \"contextFileName\": [\"GEMINI.md\",\n    \"../x.md\"]}";
        let diagnostics = validate(content);
        assert_eq!(rules(&diagnostics), [("GM-008", DiagnosticLevel::Error)]);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 5));
    }

    // ===== mcpServers and settings (GM-011, GM-014, GM-015, GM-022, GM-023) =====

    fn only<'d>(diagnostics: &'d [Diagnostic], rule: &str) -> Vec<&'d Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == rule).collect()
    }

    #[test]
    fn test_reference_manifest_example_is_clean() {
        // The `gemini-extension.json` example of extensions/reference, and
        // the settings example.
        let content = r#"{
  "name": "my-extension",
  "version": "1.0.0",
  "description": "My awesome extension",
  "mcpServers": {
    "my-server": {
      "command": "node",
      "args": ["${extensionPath}/my-server.js"],
      "cwd": "${extensionPath}"
    }
  },
  "contextFileName": "GEMINI.md",
  "excludeTools": ["run_shell_command"],
  "settings": [
    {
      "name": "API Key",
      "description": "Your API key for the service.",
      "envVar": "MY_API_KEY",
      "sensitive": true
    },
    {"name": "Region", "envVar": "MY_REGION"}
  ]
}"#;
        let config = config_with(&["GEMINI.md", "my-server.js"]);
        assert!(
            validate_in(content, &config).is_empty(),
            "{:?}",
            validate_in(content, &config)
        );
    }

    #[test]
    fn test_extension_mcp_server_checks() {
        let content = r#"{"name": "x", "version": "1.0.0", "description": "d", "mcpServers": {
  "my_server": {"command": "node", "args": ["${extensionPath}/s.js"]},
  "no-transport": {"excludeTools": ["a"]},
  "trusted": {"command": "node", "args": ["${extensionPath}/s.js"], "trust": true},
  "joined": {"command": "node dist/server.js"},
  "relative": {"command": "node", "args": ["dist/server.js", "--port", "8080"]},
  "relative-cmd": {"command": "./bin/server"},
  "relative-cwd": {"command": "node", "args": ["server.js"], "cwd": "dist"},
  "with-cwd": {"command": "./bin/server", "args": ["dist/server.js"], "cwd": "${extensionPath}"},
  "on-path": {"command": "npx", "args": ["-y", "@scope/server"]}
}}"#;
        let fs = MockFileSystem::new();
        fs.add_file("/ext/dist/server.js", "");
        fs.add_file("/ext/bin/server", "");
        fs.add_dir("/ext/dist");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        let diagnostics = validate_in(content, &config);
        let lines = |rule: &str| -> Vec<(DiagnosticLevel, usize)> {
            only(&diagnostics, rule)
                .iter()
                .map(|d| (d.level, d.line))
                .collect()
        };
        assert_eq!(lines("GM-011"), [(DiagnosticLevel::Warning, 2)]);
        // An extension server has no override form: a transport is required.
        assert_eq!(lines("GM-014"), [(DiagnosticLevel::Error, 3)]);
        assert_eq!(lines("GM-015"), [(DiagnosticLevel::Info, 4)]);
        assert_eq!(
            lines("GM-023"),
            [
                (DiagnosticLevel::Info, 5),
                (DiagnosticLevel::Info, 6),
                (DiagnosticLevel::Info, 7),
                (DiagnosticLevel::Info, 8)
            ]
        );
        assert!(only(&diagnostics, "GM-015")[0].message.contains("ignored"));
        assert!(
            only(&diagnostics, "GM-023")[0]
                .message
                .contains("node dist/server.js")
        );
    }

    #[test]
    fn test_extension_absolute_path_into_the_extension() {
        let content = r#"{"name": "x", "version": "1.0.0", "description": "d", "mcpServers": {
  "abs": {"command": "/ext/bin/server"},
  "system": {"command": "/usr/bin/python3", "args": ["${extensionPath}/s.py"]}
}}"#;
        let diagnostics = validate_in(content, &config_with(&["bin/server"]));
        let gm_023 = only(&diagnostics, "GM-023");
        assert_eq!(gm_023.len(), 1, "{diagnostics:?}");
        assert_eq!(gm_023[0].line, 2);
    }

    #[test]
    fn test_gm_022_secret_settings_without_sensitive() {
        let content = r#"{"name": "x", "version": "1.0.0", "description": "d", "settings": [
  {"name": "API Key", "envVar": "MY_API_KEY"},
  {"name": "Token", "envVar": "SERVICE_TOKEN", "sensitive": false},
  {"name": "Keyboard layout", "envVar": "KEYBOARD_LAYOUT"},
  {"name": "Secret", "envVar": "MY_SECRET", "sensitive": true},
  {"name": "Key file path", "envVar": "SSH_KEY_PATH"}
]}"#;
        let diagnostics = validate(content);
        let gm_022 = only(&diagnostics, "GM-022");
        assert_eq!(
            gm_022.iter().map(|d| (d.level, d.line)).collect::<Vec<_>>(),
            [(DiagnosticLevel::Warning, 2), (DiagnosticLevel::Warning, 3)]
        );
        assert!(gm_022[0].message.contains("'API Key'"));
        // Points at the envVar value.
        assert_eq!(gm_022[0].column, 33);
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_gemini_md_category() {
        let mut config = config_with(&[]);
        config.rules_mut().gemini_md = false;
        assert!(validate_in("{ invalid }", &config).is_empty());
        assert!(validate_in("{}", &config).is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = config_with(&[]);
        config.rules_mut().disabled_rules = vec!["GM-005".to_string(), "GM-028".to_string()];
        assert!(validate_in("{ invalid }", &config).is_empty());
        assert!(validate_in("{}", &config).is_empty());
    }
}
