//! Claude Code MCP configuration rules (CC-MCP-001 to CC-MCP-005).
//!
//! These encode Claude Code's own handling of MCP server entries, so they run
//! only on the MCP servers Claude Code reads: a project `.mcp.json`, a
//! plugin's `.mcp.json`, and the servers a `.claude-plugin/plugin.json`
//! declares inline in `mcpServers` (checked as plugin servers). CC-MCP-003
//! checks MCP tool and server descriptions that Claude Code truncates and
//! runs on any MCP JSON file that lists tools.
//!
//! - CC-MCP-001: credential variable in a remote server's `url` or `headers`,
//!   which Claude Code reads as empty
//! - CC-MCP-002: `${CLAUDE_PROJECT_DIR}` without a default in a project
//!   `.mcp.json` `command` or `args`
//! - CC-MCP-003: tool description or server instructions past the length
//!   Claude Code keeps
//! - CC-MCP-004: non-boolean `alwaysLoad` (formerly MCP-025)
//! - CC-MCP-005: server name Claude Code reserves (formerly MCP-026)

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::claude_json_spans::{JsonLocator, Seg},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use serde_json::Value;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CC-MCP-001",
    "CC-MCP-002",
    "CC-MCP-003",
    "CC-MCP-004",
    "CC-MCP-005",
];

/// "Claude Code reserves the names of its built-in servers, including
/// `workspace`, `claude-in-chrome`, `computer-use`, `Claude Preview`, and
/// `Claude Browser`."
const RESERVED_SERVER_NAMES: &[&str] = &[
    "workspace",
    "claude-in-chrome",
    "computer-use",
    "Claude Preview",
    "Claude Browser",
];

/// Credential variables that read as empty in a remote server's `url` and
/// `headers`. The page lists them with "such as", so this is the documented
/// subset, not the full list.
const EMPTY_CREDENTIAL_VARS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "AWS_BEARER_TOKEN_BEDROCK",
    "HTTPS_PROXY",
    "NPM_TOKEN",
];

const REMOTE_TYPES: &[&str] = &["http", "streamable-http", "sse", "ws"];

/// "Server instructions, and the description of a tool that loads upfront:
/// 4,096 characters"; a deferred tool keeps 16,384. The limit was 2,048
/// before v2.1.296 (2026-10-09).
const UPFRONT_DESCRIPTION_LIMIT: usize = 4_096;
const LEGACY_UPFRONT_DESCRIPTION_LIMIT: usize = 2_048;
const DEFERRED_DESCRIPTION_LIMIT: usize = 16_384;

pub struct ClaudeMcpValidator;

/// What kind of Claude Code MCP file `path` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpScope {
    Project,
    Plugin,
}

fn claude_mcp_scope(path: &Path, config: &PerFileLintConfig<'_>) -> Option<McpScope> {
    if path.file_name().is_none_or(|n| n != ".mcp.json") {
        return None;
    }
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    if config.fs().exists(&dir.join(".claude-plugin")) {
        Some(McpScope::Plugin)
    } else {
        Some(McpScope::Project)
    }
}

/// The server map: `mcpServers`, or (plugin `.mcp.json`, which may "omit the
/// `mcpServers` wrapper") a top-level map of server objects.
fn server_map(root: &Value, scope: McpScope) -> Option<(&serde_json::Map<String, Value>, bool)> {
    if let Some(map) = root.get("mcpServers").and_then(Value::as_object) {
        return Some((map, true));
    }
    let map = root.as_object()?;
    (scope == McpScope::Plugin
        && !map.is_empty()
        && map.values().all(|v| {
            v.as_object().is_some_and(|o| {
                o.contains_key("command") || o.contains_key("url") || o.contains_key("type")
            })
        }))
    .then_some((map, false))
}

fn join_path<'a>(prefix: &[Seg<'a>], name: &'a str, extra: &[Seg<'a>]) -> Vec<Seg<'a>> {
    let mut segs = prefix.to_vec();
    segs.push(Seg::Key(name));
    segs.extend_from_slice(extra);
    segs
}

/// `${NAME}` or `${NAME:-default}` references in a string.
fn braced_vars(value: &str) -> Vec<&str> {
    let mut vars = Vec::new();
    let mut rest = value;
    while let Some(start) = rest.find("${") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('}') else { break };
        let inner = &after[..end];
        let name = inner.split(":-").next().unwrap_or(inner);
        if !name.is_empty() {
            vars.push(name);
        }
        rest = &after[end + 1..];
    }
    vars
}

impl Validator for ClaudeMcpValidator {
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
        if !config.rules().mcp {
            return diagnostics;
        }
        // MCP-007 reports parse errors for every MCP file.
        let Ok(root) = serde_json::from_str::<Value>(content) else {
            return diagnostics;
        };
        let locator = JsonLocator::new(content);

        // A Claude Code plugin manifest: only its inline `mcpServers`.
        if crate::file_types::detect_file_type(path) == crate::file_types::FileType::Plugin {
            if crate::rules::mcp::is_claude_plugin_manifest(path) {
                check_inline_servers(path, content, &root, &locator, config, &mut diagnostics);
            }
            return diagnostics;
        }

        check_descriptions(path, &root, &locator, config, &mut diagnostics);

        let Some(scope) = claude_mcp_scope(path, config) else {
            return diagnostics;
        };
        check_servers(
            path,
            content,
            &root,
            scope,
            &locator,
            config,
            &mut diagnostics,
        );
        diagnostics
    }
}

/// The servers a `.claude-plugin/plugin.json` declares inline in
/// `mcpServers` (a map, or maps inside an array), checked as plugin servers.
fn check_inline_servers(
    path: &Path,
    content: &str,
    root: &Value,
    locator: &JsonLocator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(document) = crate::schemas::kiro_json::parse_json_document(content, false) else {
        return;
    };
    for map in crate::rules::mcp::inline_server_maps(&document.root, false) {
        let Some(servers) =
            crate::rules::mcp::json_value_at(root, &map.segs).and_then(Value::as_object)
        else {
            continue;
        };
        let pointer: String = map
            .segs
            .iter()
            .map(|seg| match seg {
                Seg::Key(key) => format!("/{}", key.replace('~', "~0").replace('/', "~1")),
                Seg::Index(index) => format!("/{index}"),
            })
            .collect();
        check_server_map(
            path,
            content,
            servers,
            &map.segs,
            &pointer,
            McpScope::Plugin,
            locator,
            config,
            diagnostics,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn check_servers<'v>(
    path: &Path,
    content: &str,
    root: &'v Value,
    scope: McpScope,
    locator: &JsonLocator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some((servers, wrapped)) = server_map(root, scope) else {
        return;
    };
    let (prefix, pointer): (Vec<Seg<'v>>, &str) = if wrapped {
        (vec![Seg::Key("mcpServers")], "/mcpServers")
    } else {
        (Vec::new(), "")
    };
    check_server_map(
        path,
        content,
        servers,
        &prefix,
        pointer,
        scope,
        locator,
        config,
        diagnostics,
    );
}

/// CC-MCP-001, CC-MCP-002, CC-MCP-004 and CC-MCP-005 on one server map at
/// `prefix` (JSON pointer `pointer`) in the file.
#[allow(clippy::too_many_arguments)]
fn check_server_map<'v>(
    path: &Path,
    content: &str,
    servers: &'v serde_json::Map<String, Value>,
    prefix: &[Seg<'v>],
    pointer: &str,
    scope: McpScope,
    locator: &JsonLocator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    {
        for (name, server) in servers {
            let at = |extra: &[Seg<'v>]| join_path(prefix, name.as_str(), extra);

            // CC-MCP-005: reserved names. A plugin's servers are scoped as
            // `plugin:<plugin>:<server>`, so their keys can't collide.
            if scope == McpScope::Project && config.is_rule_enabled("CC-MCP-005") {
                if RESERVED_SERVER_NAMES.contains(&name.as_str()) {
                    let (line, col) = locator.key_pos(&at(&[]));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-MCP-005",
                            t!("rules.cc_mcp_005.message", name = name.as_str()),
                        )
                        .with_suggestion(t!("rules.cc_mcp_005.suggestion")),
                    );
                } else if name == "anthropic-skills" {
                    let (line, col) = locator.key_pos(&at(&[]));
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-MCP-005",
                            t!("rules.cc_mcp_005.prompts_hidden"),
                        )
                        .with_suggestion(t!("rules.cc_mcp_005.prompts_hidden_suggestion")),
                    );
                }
            }

            let Some(server) = server.as_object() else {
                continue;
            };
            let server_type = server.get("type").and_then(Value::as_str);

            // CC-MCP-001: covered credential variables read as empty in a
            // remote server's url and headers.
            let remote = server_type.is_some_and(|t| REMOTE_TYPES.contains(&t));
            if remote && config.is_rule_enabled("CC-MCP-001") {
                let mut fields: Vec<(Vec<Seg<'v>>, &str, String)> = Vec::new();
                if let Some(url) = server.get("url").and_then(Value::as_str) {
                    fields.push((at(&[Seg::Key("url")]), url, "url".to_string()));
                }
                if let Some(headers) = server.get("headers").and_then(Value::as_object) {
                    for (header, value) in headers {
                        if let Some(value) = value.as_str() {
                            fields.push((
                                at(&[Seg::Key("headers"), Seg::Key(header.as_str())]),
                                value,
                                format!("headers.{header}"),
                            ));
                        }
                    }
                }
                for (segs, value, field) in fields {
                    for var in braced_vars(value) {
                        if !EMPTY_CREDENTIAL_VARS.contains(&var) {
                            continue;
                        }
                        let (line, col) = locator.value_pos(&segs);
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                line,
                                col,
                                "CC-MCP-001",
                                t!(
                                    "rules.cc_mcp_001.message",
                                    var = var,
                                    server = name.as_str(),
                                    field = field.as_str()
                                ),
                            )
                            .with_suggestion(t!("rules.cc_mcp_001.suggestion", var = var)),
                        );
                    }
                }
            }

            // CC-MCP-002: project `.mcp.json` command/args need a default for
            // `${CLAUDE_PROJECT_DIR}`; plugin configs substitute it directly.
            if scope == McpScope::Project && config.is_rule_enabled("CC-MCP-002") {
                let mut values: Vec<(Vec<Seg<'v>>, &str)> = Vec::new();
                if let Some(command) = server.get("command").and_then(Value::as_str) {
                    values.push((at(&[Seg::Key("command")]), command));
                }
                if let Some(args) = server.get("args").and_then(Value::as_array) {
                    for (i, arg) in args.iter().enumerate() {
                        if let Some(arg) = arg.as_str() {
                            values.push((at(&[Seg::Key("args"), Seg::Index(i)]), arg));
                        }
                    }
                }
                if let Some((segs, _)) = values
                    .into_iter()
                    .find(|(_, v)| v.contains("${CLAUDE_PROJECT_DIR}"))
                {
                    let (line, col) = locator.value_pos(&segs);
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            line,
                            col,
                            "CC-MCP-002",
                            t!("rules.cc_mcp_002.message", server = name.as_str()),
                        )
                        .with_suggestion(t!("rules.cc_mcp_002.suggestion")),
                    );
                }
            }

            // CC-MCP-004: alwaysLoad must be a JSON boolean.
            if config.is_rule_enabled("CC-MCP-004")
                && let Some(value) = server.get("alwaysLoad")
                && !value.is_boolean()
                && !value.is_null()
            {
                let segs = at(&[Seg::Key("alwaysLoad")]);
                let (line, col) = locator.value_pos(&segs);
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-MCP-004",
                    t!(
                        "rules.cc_mcp_004.message",
                        server = name.as_str(),
                        actual = crate::rules::json_type_name(value)
                    ),
                )
                .with_suggestion(t!("rules.cc_mcp_004.suggestion"));
                if let Some(quoted) = value.as_str() {
                    let escaped = name.replace('~', "~0").replace('/', "~1");
                    let pointer = format!("{pointer}/{escaped}/alwaysLoad");
                    if let Some((start, end)) = crate::schemas::mcp::quoted_json_boolean_span(
                        content,
                        &pointer,
                        "alwaysLoad",
                        quoted,
                    ) {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            quoted,
                            t!("rules.boolean_unwrap.json_fix", value = quoted),
                            false,
                        ));
                    }
                }
                diagnostics.push(diagnostic);
            }
        }
    }
}

/// CC-MCP-003: "Claude Code cuts a tool description or server instructions
/// that run past these default limits, and Claude sees only the text before
/// the cut."
fn check_descriptions(
    path: &Path,
    root: &Value,
    locator: &JsonLocator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("CC-MCP-003") {
        return;
    }
    let legacy = config
        .get_claude_code_version()
        .and_then(|v| semver::Version::parse(v.trim_start_matches('v')).ok())
        .is_some_and(|v| v < semver::Version::new(2, 1, 296));
    let upfront = if legacy {
        LEGACY_UPFRONT_DESCRIPTION_LIMIT
    } else {
        UPFRONT_DESCRIPTION_LIMIT
    };

    let mut tool_lists: Vec<(Vec<Seg<'_>>, &Vec<Value>)> = Vec::new();
    if let Some(tools) = root.get("tools").and_then(Value::as_array) {
        tool_lists.push((vec![Seg::Key("tools")], tools));
    }
    if let Some(tools) = root
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(Value::as_array)
    {
        tool_lists.push((vec![Seg::Key("result"), Seg::Key("tools")], tools));
    }
    for (base, tools) in tool_lists {
        for (i, tool) in tools.iter().enumerate() {
            let Some(description) = tool.get("description").and_then(Value::as_str) else {
                continue;
            };
            let len = description.chars().count();
            // A tool marked to load upfront keeps the shorter limit; any other
            // tool may be deferred (16,384) or upfront (4,096) depending on
            // tool search, so report it only past the upfront limit as LOW.
            let always_load = tool
                .get("_meta")
                .and_then(|m| m.get("anthropic/alwaysLoad"))
                .and_then(Value::as_bool);
            let limit = if always_load == Some(false) {
                DEFERRED_DESCRIPTION_LIMIT
            } else {
                upfront
            };
            if len <= limit {
                continue;
            }
            let name = tool.get("name").and_then(Value::as_str).unwrap_or("?");
            let mut segs = base.clone();
            segs.push(Seg::Index(i));
            segs.push(Seg::Key("description"));
            let (line, col) = locator.value_pos(&segs);
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-MCP-003",
                    t!(
                        "rules.cc_mcp_003.tool",
                        tool = name,
                        len = len,
                        limit = limit
                    ),
                )
                .with_suggestion(t!("rules.cc_mcp_003.suggestion")),
            );
        }
    }
    let instructions = [
        vec![Seg::Key("instructions")],
        vec![Seg::Key("result"), Seg::Key("instructions")],
    ];
    for segs in instructions {
        let mut value = Some(root);
        for seg in &segs {
            value = match seg {
                Seg::Key(k) => value.and_then(|v| v.get(*k)),
                Seg::Index(i) => value.and_then(|v| v.get(*i)),
            };
        }
        let Some(text) = value.and_then(Value::as_str) else {
            continue;
        };
        let len = text.chars().count();
        if len > upfront {
            let (line, col) = locator.value_pos(&segs);
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    col,
                    "CC-MCP-003",
                    t!("rules.cc_mcp_003.instructions", len = len, limit = upfront),
                )
                .with_suggestion(t!("rules.cc_mcp_003.suggestion")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;
    use std::fs;
    use tempfile::TempDir;

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        ClaudeMcpValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn rules<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    #[test]
    fn inline_plugin_manifest_servers_get_plugin_scope_checks() {
        let content = r#"{
  "name": "gw-tools",
  "mcpServers": {
    "workspace": { "command": "${CLAUDE_PROJECT_DIR}/bin/server", "alwaysLoad": "true" },
    "gw": {
      "type": "http",
      "url": "https://gw.example.com/mcp",
      "headers": { "Authorization": "Bearer ${ANTHROPIC_AUTH_TOKEN}" }
    }
  }
}"#;
        let diagnostics = validate("p/.claude-plugin/plugin.json", content);
        let cc_mcp_001 = rules(&diagnostics, "CC-MCP-001");
        assert_eq!(cc_mcp_001.len(), 1, "{diagnostics:?}");
        assert_eq!(cc_mcp_001[0].line, 8);
        let cc_mcp_004 = rules(&diagnostics, "CC-MCP-004");
        assert_eq!(cc_mcp_004.len(), 1, "{diagnostics:?}");
        assert_eq!(cc_mcp_004[0].line, 4);
        // Plugin servers are namespaced (`plugin:<plugin>:<server>`) and
        // substitute ${CLAUDE_PROJECT_DIR} directly: no CC-MCP-005 or
        // CC-MCP-002. CC-MCP-003 checks tool lists, which a manifest has not.
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule == "CC-MCP-001" || d.rule == "CC-MCP-004"),
            "{diagnostics:?}"
        );
        // The alwaysLoad fix edits the inline server and keeps valid JSON.
        let fix = &cc_mcp_004[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        let value: Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["mcpServers"]["workspace"]["alwaysLoad"], true);
    }

    #[test]
    fn inline_plugin_manifest_server_arrays_and_other_manifests() {
        let content = r#"{"name":"p","mcpServers":["./.mcp.json",{"s":{"command":"node","alwaysLoad":"false"}}]}"#;
        let diagnostics = validate(".claude-plugin/plugin.json", content);
        let cc_mcp_004 = rules(&diagnostics, "CC-MCP-004");
        assert_eq!(cc_mcp_004.len(), 1, "{diagnostics:?}");
        let fix = &cc_mcp_004[0].fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        let value: Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["mcpServers"][1]["s"]["alwaysLoad"], false);

        // Claude Code reads no other plugin.json.
        for path in ["plugin.json", "my-plugin/plugin.json"] {
            assert!(validate(path, content).is_empty(), "{path}");
        }
    }

    #[test]
    fn braced_variable_names() {
        assert_eq!(
            braced_vars("Bearer ${ANTHROPIC_AUTH_TOKEN} ${X:-y} $Z"),
            vec!["ANTHROPIC_AUTH_TOKEN", "X"]
        );
    }

    #[test]
    fn cc_mcp_001_credential_variables_in_remote_servers() {
        let content = r#"{
  "mcpServers": {
    "gw": {
      "type": "http",
      "url": "https://gw.example.com/mcp?key=${NPM_TOKEN}",
      "headers": { "Authorization": "Bearer ${ANTHROPIC_AUTH_TOKEN:-x}", "X-Mine": "${MY_API_KEY}" }
    },
    "base": { "type": "http", "url": "${ANTHROPIC_BASE_URL}/mcp" },
    "local": { "command": "node", "env": { "KEY": "${ANTHROPIC_API_KEY}" } }
  }
}"#;
        let diagnostics = validate(".mcp.json", content);
        let hits = rules(&diagnostics, "CC-MCP-001");
        assert_eq!(hits.len(), 2, "{diagnostics:?}");
        assert!(hits.iter().all(|d| d.level == DiagnosticLevel::Error));
        assert_eq!(hits[0].line, 5);
        // Not a Claude Code MCP file: no Claude rules.
        assert!(rules(&validate(".cursor/mcp.json", content), "CC-MCP-001").is_empty());
    }

    #[test]
    fn cc_mcp_002_project_dir_without_default() {
        let content = r#"{"mcpServers": {
  "a": {"command": "${CLAUDE_PROJECT_DIR}/bin/server"},
  "b": {"command": "node", "args": ["${CLAUDE_PROJECT_DIR:-.}/server.js"]}
}}"#;
        let diagnostics = validate(".mcp.json", content);
        let hits = rules(&diagnostics, "CC-MCP-002");
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].line, 2);
        // Plugin configs substitute the placeholder directly.
        let temp = TempDir::new().unwrap();
        fs::create_dir_all(temp.path().join(".claude-plugin")).unwrap();
        let path = temp.path().join(".mcp.json");
        fs::write(&path, content).unwrap();
        let diagnostics = ClaudeMcpValidator.validate(&path, content, &LintConfig::default());
        assert!(rules(&diagnostics, "CC-MCP-002").is_empty());
    }

    #[test]
    fn cc_mcp_003_long_descriptions() {
        let long = "x".repeat(5000);
        let content = format!(
            r#"{{"tools": [{{"name": "t", "description": "{long}", "inputSchema": {{"type": "object"}}}}, {{"name": "d", "description": "{long}", "_meta": {{"anthropic/alwaysLoad": false}}}}]}}"#
        );
        let diagnostics = validate("mcp-tools.json", &content);
        let hits = rules(&diagnostics, "CC-MCP-003");
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn cc_mcp_004_non_boolean_always_load() {
        let content = r#"{"mcpServers": {"a": {"command": "node", "alwaysLoad": "true"}, "b": {"command": "node", "alwaysLoad": true}}}"#;
        let diagnostics = validate(".mcp.json", content);
        let hits = rules(&diagnostics, "CC-MCP-004");
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        let fix = &hits[0].fixes[0];
        assert!(!fix.safe);
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        let value: Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["mcpServers"]["a"]["alwaysLoad"], true);
    }

    #[test]
    fn cc_mcp_005_reserved_names() {
        let content = r#"{"mcpServers": {
  "workspace": {"command": "a"},
  "Claude Browser": {"command": "b"},
  "computer-use": {"command": "c"},
  "anthropic-skills": {"command": "d"},
  "fs": {"command": "e"}
}}"#;
        let diagnostics = validate(".mcp.json", content);
        let hits = rules(&diagnostics, "CC-MCP-005");
        assert_eq!(hits.len(), 4, "{diagnostics:?}");
        assert_eq!(
            hits.iter()
                .filter(|d| d.level == DiagnosticLevel::Error)
                .count(),
            3
        );
        assert!(
            hits.iter()
                .any(|d| d.line == 2 && d.message.contains("workspace"))
        );
        // Other clients' MCP files don't get Claude Code's reserved names.
        assert!(rules(&validate(".cursor/mcp.json", content), "CC-MCP-005").is_empty());
    }
}
