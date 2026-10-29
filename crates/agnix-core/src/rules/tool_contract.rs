//! Offline checks against pinned vendor configuration contracts.
//!
//! Only bundled schemas are compiled. Network and file reference resolution
//! are disabled in the jsonschema dependency. User `$schema` values are editor
//! hints. Recognized IDs can identify a bundled format, but user schemas are
//! never compiled or fetched and never initiate I/O.

use crate::{
    diagnostics::DiagnosticLevel,
    rules::config_json::JsonCx,
    schemas::kiro_json::{JsonNode, JsonValue, parse_json_document},
};
use serde_json::Value;
use std::sync::LazyLock;

#[derive(Clone, Copy)]
pub(crate) enum Schema {
    PiSettings,
    PiModels,
    PiKeybindings,
    PiTheme,
    AntigravitySettings,
    AntigravityPlugin,
    GrokConfig,
    GrokAgent,
    GrokPlugin,
    GrokPersona,
    GrokRole,
    GrokSandbox,
    GrokMarketplace,
    GrokLsp,
}

impl Schema {
    pub fn source(self) -> &'static str {
        match self {
            Self::PiSettings => include_str!("../schemas/vendor/pi-settings.json"),
            Self::PiModels => include_str!("../schemas/vendor/pi-models.json"),
            Self::PiKeybindings => include_str!("../schemas/vendor/pi-keybindings.json"),
            Self::PiTheme => include_str!("../schemas/vendor/pi-theme.json"),
            Self::AntigravitySettings => {
                include_str!("../schemas/vendor/antigravity-settings.json")
            }
            Self::AntigravityPlugin => include_str!("../schemas/vendor/antigravity-plugin.json"),
            Self::GrokConfig
            | Self::GrokAgent
            | Self::GrokPlugin
            | Self::GrokPersona
            | Self::GrokRole
            | Self::GrokSandbox
            | Self::GrokMarketplace
            | Self::GrokLsp => include_str!("../schemas/grok_contract.json"),
        }
    }

    fn validator(self) -> &'static jsonschema::Validator {
        static SCHEMAS: LazyLock<[jsonschema::Validator; 14]> = LazyLock::new(|| {
            [
                Schema::PiSettings,
                Schema::PiModels,
                Schema::PiKeybindings,
                Schema::PiTheme,
                Schema::AntigravitySettings,
                Schema::AntigravityPlugin,
                Schema::GrokConfig,
                Schema::GrokAgent,
                Schema::GrokPlugin,
                Schema::GrokPersona,
                Schema::GrokRole,
                Schema::GrokSandbox,
                Schema::GrokMarketplace,
                Schema::GrokLsp,
            ]
            .map(|schema| {
                let mut value: Value = serde_json::from_str(schema.source())
                    .expect("bundled tool schema must be valid JSON");
                let fragment = match schema {
                    Schema::GrokConfig => Some("config"),
                    Schema::GrokAgent => Some("agent"),
                    Schema::GrokPlugin => Some("plugin"),
                    Schema::GrokPersona => Some("persona"),
                    Schema::GrokRole => Some("role"),
                    Schema::GrokSandbox => Some("sandbox"),
                    Schema::GrokMarketplace => Some("marketplace"),
                    Schema::GrokLsp => Some("lsp"),
                    _ => None,
                };
                if let Some(fragment) = fragment {
                    value["$ref"] = Value::String(format!("#/$defs/{fragment}"));
                }
                jsonschema::validator_for(&value)
                    .expect("bundled tool schema must compile without external references")
            })
        });
        &SCHEMAS[self as usize]
    }

    pub fn issues(self, value: &Value) -> Vec<SchemaIssue> {
        // These clients ignore editor metadata. In particular, a custom
        // `$schema` URL is not a runtime configuration error.
        let mut instance = value.clone();
        if !matches!(self, Self::GrokLsp) {
            if let Some(object) = instance.as_object_mut() {
                object.remove("$schema");
            }
        }
        self.validator()
            .iter_errors(&instance)
            .map(|error| SchemaIssue {
                pointer: error.instance_path().to_string(),
                keyword: error.kind().keyword().to_string(),
            })
            .collect()
    }
}

pub(crate) struct SchemaIssue {
    pub pointer: String,
    pub keyword: String,
}

/// Read strict JSON with an optional UTF-8 BOM. Blanking its three bytes
/// preserves every source offset while the diagnostic counts the BOM as one
/// Unicode character. JSONC is enabled only for client formats that accept it.
pub(crate) fn json_document(
    cx: &mut JsonCx<'_>,
    content: &str,
    rule: &str,
    allow_jsonc: bool,
) -> Option<(JsonNode, Value)> {
    let normalized;
    let input = if let Some(rest) = content.strip_prefix('\u{feff}') {
        normalized = format!("   {rest}");
        normalized.as_str()
    } else {
        content
    };
    let document = match parse_json_document(input, allow_jsonc) {
        Ok(document) => document,
        Err(error) => {
            if cx.enabled(rule) {
                cx.push_at(
                    DiagnosticLevel::Error,
                    rule,
                    (error.line, error.column),
                    "Invalid configuration JSON.",
                    "Correct the JSON syntax; this configuration cannot be loaded.",
                );
            }
            return None;
        }
    };
    let value = node_value(&document.root);
    if !value.is_object() {
        if cx.enabled(rule) {
            cx.push(
                DiagnosticLevel::Error,
                rule,
                (document.root.start, document.root.end),
                "Configuration must contain a JSON object.",
                "Use an object with the fields documented for this configuration file.",
            );
        }
        return None;
    }
    Some((document.root, value))
}

fn node_value(node: &JsonNode) -> Value {
    match &node.value {
        JsonValue::Null => Value::Null,
        JsonValue::Bool(value) => Value::Bool(*value),
        JsonValue::Number(value) => Value::Number(value.clone()),
        JsonValue::String(value) => Value::String(value.clone()),
        JsonValue::Array(items) => Value::Array(items.iter().map(node_value).collect()),
        JsonValue::Object(members) => Value::Object(
            members
                .iter()
                .map(|member| (member.key.clone(), node_value(&member.value)))
                .collect(),
        ),
    }
}

/// Resolve a schema error's JSON Pointer into an original source span.
/// A missing child is reported on its nearest existing parent.
pub(crate) fn pointer_span(root: &JsonNode, pointer: &str) -> (usize, usize) {
    let mut node = root;
    for part in pointer
        .strip_prefix('/')
        .into_iter()
        .flat_map(|s| s.split('/'))
    {
        let decoded = part.replace("~1", "/").replace("~0", "~");
        let next = match &node.value {
            JsonValue::Object(_) => node.value.get(&decoded).map(|member| &member.value),
            JsonValue::Array(items) => decoded.parse::<usize>().ok().and_then(|i| items.get(i)),
            _ => None,
        };
        match next {
            Some(next) => node = next,
            None => break,
        }
    }
    (node.start, node.end)
}

pub(crate) fn check_schema(
    cx: &mut JsonCx<'_>,
    root: &JsonNode,
    value: &Value,
    schema: Schema,
    rule: &str,
) {
    if !cx.enabled(rule) {
        return;
    }
    for issue in schema.issues(value) {
        let field = if issue.pointer.is_empty() {
            "/"
        } else {
            &issue.pointer
        };
        cx.push(
            DiagnosticLevel::Error,
            rule,
            pointer_span(root, &issue.pointer),
            format!(
                "Configuration field {field} violates the documented {} constraint.",
                issue.keyword
            ),
            "Use the value types and choices in the tool's configuration reference.",
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_schemas_compile_offline_and_apply_nested_constraints() {
        for schema in [
            Schema::PiSettings,
            Schema::PiModels,
            Schema::PiKeybindings,
            Schema::PiTheme,
            Schema::AntigravitySettings,
            Schema::AntigravityPlugin,
        ] {
            let _ = schema.validator();
        }
        assert!(
            Schema::PiSettings
                .issues(&serde_json::json!({"compaction":{"reserveTokens":-1}}))
                .len()
                > 0
        );
        assert!(
            Schema::PiSettings
                .issues(&serde_json::json!({"defaultThinkingLevel":"max","futureSetting":true}))
                .is_empty()
        );
        assert!(
            Schema::AntigravitySettings
                .issues(
                    &serde_json::json!({"agentMode":"auto","historySize":-1,"futureSetting":true})
                )
                .is_empty()
        );
    }

    #[test]
    fn error_details_never_include_secret_instance_values() {
        let secret = "sk-example-credential-that-must-not-appear";
        let issues = Schema::PiSettings.issues(&serde_json::json!({"defaultThinkingLevel":secret}));
        assert!(!issues.is_empty());
        assert!(
            issues
                .iter()
                .all(|issue| !issue.pointer.contains(secret) && !issue.keyword.contains(secret))
        );
    }
}
