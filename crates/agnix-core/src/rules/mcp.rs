//! MCP (Model Context Protocol) validation (MCP-001 to MCP-024, MCP-027 to
//! MCP-033). MCP-025 and MCP-026 encoded Claude Code facts and moved to
//! `claude_mcp` (CC-MCP-004, CC-MCP-005), which runs only on Claude Code MCP
//! files.
//!
//! The MCP specification defines tool, resource and prompt definitions and the
//! JSON-RPC messages (MCP-001..008, MCP-013..016, MCP-020, MCP-030..033). It
//! does not define client configuration files: the `mcpServers` map, `type`,
//! `command`, `url` and `args` are each client's format (MCP-009..012,
//! MCP-022..024), so those rules follow the clients' documentation. The
//! server-level security checks shared with other clients' config files
//! (MCP-018, MCP-019, MCP-027..029) live in `mcp_servers`. Servers a plugin
//! manifest declares inline in `mcpServers` (Claude Code, legacy Copilot and
//! Cursor manifests) get the server-level rules, with each client's `type`
//! values.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    file_types::FileType,
    rules::claude_json_spans::Seg,
    rules::mcp_servers::{Locator, ServerView, check_server},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::{JsonNode, JsonValue, parse_json_document},
    schemas::mcp::{
        CURRENT_MCP_PROTOCOL_VERSIONS, McpServerConfig, McpToolSchema, VALID_MCP_ANNOTATION_HINTS,
        VALID_MCP_CAPABILITY_KEYS, extract_request_protocol_version,
        extract_response_protocol_version, is_initialize_message, is_initialize_response,
        validate_json_schema_structure,
    },
};
use rust_i18n::t;
use std::path::Path;

fn skip_ascii_whitespace(content: &str, mut idx: usize) -> usize {
    let bytes = content.as_bytes();
    while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
        idx += 1;
    }
    idx
}

fn is_inside_json_string(content: &str, offset: usize) -> bool {
    let bytes = content.as_bytes();
    let mut idx = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    while idx < offset.min(bytes.len()) {
        let ch = bytes[idx];
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == b'\\' {
                escaped = true;
            } else if ch == b'"' {
                in_string = false;
            }
        } else if ch == b'"' {
            in_string = true;
        }
        idx += 1;
    }

    in_string
}

fn find_json_key_position(content: &str, key: &str) -> Option<usize> {
    let pattern = format!("\"{}\"", key);
    let mut search_from = 0usize;
    let bytes = content.as_bytes();

    while let Some(rel) = content[search_from..].find(&pattern) {
        let pos = search_from + rel;
        if !is_inside_json_string(content, pos) {
            let idx = skip_ascii_whitespace(content, pos + pattern.len());
            if idx < bytes.len() && bytes[idx] == b':' {
                return Some(pos);
            }
        }
        search_from = pos + pattern.len();
    }

    None
}

fn find_json_array_start_for_key(content: &str, key: &str) -> Option<usize> {
    let key_pos = find_json_key_position(content, key)?;
    let bytes = content.as_bytes();
    let mut idx = skip_ascii_whitespace(content, key_pos + key.len() + 2);
    if idx >= bytes.len() || bytes[idx] != b':' {
        return None;
    }
    idx = skip_ascii_whitespace(content, idx + 1);
    if idx >= bytes.len() || bytes[idx] != b'[' {
        return None;
    }
    Some(idx)
}

fn find_json_object_start_for_key(content: &str, key: &str) -> Option<usize> {
    let key_pos = find_json_key_position(content, key)?;
    let bytes = content.as_bytes();
    let mut idx = skip_ascii_whitespace(content, key_pos + key.len() + 2);
    if idx >= bytes.len() || bytes[idx] != b':' {
        return None;
    }
    idx = skip_ascii_whitespace(content, idx + 1);
    if idx >= bytes.len() || bytes[idx] != b'{' {
        return None;
    }
    Some(idx)
}

/// Find the line number (1-based) of a JSON field in the raw content
/// Returns (line, column) or (1, 0) if not found
fn find_json_field_location(content: &str, field_name: &str) -> (usize, usize) {
    let pattern = format!("\"{}\"", field_name);
    if let Some(pos) =
        find_json_key_position(content, field_name).or_else(|| content.find(&pattern))
    {
        let line = content[..pos].matches('\n').count() + 1;
        let last_newline = content[..pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let col = pos - last_newline;
        return (line, col);
    }
    (1, 0)
}

/// Find a unique value span for a JSON scalar key (string/number/bool/null).
/// Returns the full value span (including quotes for strings).
fn find_unique_json_scalar_value_span(content: &str, key: &str) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_json_scalar_span(content, key)
}

use super::find_unique_json_string_value_span;

fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, b) in content.bytes().enumerate() {
        if b == b'\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
    let mut low = 0usize;
    let mut high = line_starts.len();
    while low + 1 < high {
        let mid = (low + high) / 2;
        if line_starts[mid] <= offset {
            low = mid;
        } else {
            high = mid;
        }
    }
    let line_start = line_starts[low];
    (low + 1, offset.saturating_sub(line_start))
}

/// Collect object spans for entries in the first `tools` array.
/// Spans are byte offsets `(start, end)` for each object entry.
fn collect_tools_array_object_spans(content: &str) -> Vec<(usize, usize)> {
    let Some(array_pos) = find_json_array_start_for_key(content, "tools") else {
        return Vec::new();
    };

    let mut spans = Vec::new();
    let array_start = array_pos + 1;
    let mut in_string = false;
    let mut escaped = false;
    let mut brace_depth = 0usize;
    let mut current_start: Option<usize> = None;

    for (rel_idx, ch) in content[array_start..].char_indices() {
        let abs_idx = array_start + rel_idx;

        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if brace_depth == 0 {
                    current_start = Some(abs_idx);
                }
                brace_depth += 1;
            }
            '}' => {
                brace_depth = brace_depth.saturating_sub(1);
                if brace_depth == 0
                    && let Some(start) = current_start.take()
                {
                    spans.push((start, abs_idx + ch.len_utf8()));
                }
            }
            ']' if brace_depth == 0 => break,
            _ => {}
        }
    }

    spans
}

fn tool_location_from_span(
    tool_spans: &[(usize, usize)],
    tool_index: usize,
    line_starts: &[usize],
) -> Option<(usize, usize)> {
    tool_spans
        .get(tool_index)
        .map(|(start, _)| line_col_at(*start, line_starts))
}

fn find_json_field_location_in_span(
    content: &str,
    field_name: &str,
    span: (usize, usize),
    line_starts: &[usize],
) -> Option<(usize, usize)> {
    let (start, end) = span;
    if start >= end
        || end > content.len()
        || !content.is_char_boundary(start)
        || !content.is_char_boundary(end)
    {
        return None;
    }

    let pattern = format!("\"{}\"", field_name);
    content[start..end]
        .find(&pattern)
        .map(|rel| line_col_at(start + rel, line_starts))
}

const RULE_IDS: &[&str] = &[
    "MCP-001", "MCP-002", "MCP-003", "MCP-004", "MCP-006", "MCP-007", "MCP-008", "MCP-009",
    "MCP-010", "MCP-011", "MCP-012", "MCP-013", "MCP-014", "MCP-015", "MCP-016", "MCP-017",
    "MCP-018", "MCP-019", "MCP-020", "MCP-021", "MCP-022", "MCP-023", "MCP-024", "MCP-027",
    "MCP-028", "MCP-029", "MCP-030", "MCP-031", "MCP-032", "MCP-033",
];

/// `type` values each client documents for an MCP server entry. The MCP
/// specification defines no client config format, so the valid set depends
/// on who reads the file.
const CLIENT_SERVER_TYPES: &[(&str, &[&str])] = &[
    // "the `type` field accepts `streamable-http` as an alias for `http`";
    // WebSocket example `"type":"ws"`.
    (
        "claude-code",
        &["stdio", "http", "streamable-http", "sse", "ws"],
    ),
    // Copilot cloud agent "accepts `local`, `stdio`, `http`, or `sse`";
    // VS Code `.vscode/mcp.json` uses stdio, http and sse.
    ("github-copilot", &["stdio", "local", "http", "sse"]),
    // Codex plugin `.mcp.json` uses `http`; Agent Plugins `mcp.json` uses
    // `streamable-http`.
    ("codex", &["stdio", "http", "streamable-http"]),
    // Cursor lists stdio, SSE and Streamable HTTP transports; remote entries
    // may omit `type`.
    ("cursor", &["stdio", "http", "sse", "streamable-http"]),
    // Cline: `streamableHttp` or legacy `sse`.
    ("cline", &["stdio", "sse", "streamableHttp"]),
];

/// Clients that read a generic MCP file at `path`. Empty when the location
/// is not one specific clients document (a plugin or skill `mcp.json`).
fn mcp_file_readers(path: &Path) -> &'static [&'static str] {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match (name.as_str(), parent.as_str()) {
        // Claude Code project servers, Copilot CLI's portable config and
        // Codex plugins all use `.mcp.json`.
        (".mcp.json", _) => &["claude-code", "github-copilot", "codex"],
        ("mcp.json", ".cursor") => &["cursor"],
        ("mcp.json", ".vscode") | ("mcp.json", ".github") | ("mcp-config.json", _) => {
            &["github-copilot"]
        }
        _ => &[],
    }
}

/// The `type` values valid for an MCP server in the file at `path`: the
/// union of what its readers accept, narrowed to the selected tools when
/// `tools` names some of them. A file no specific client owns accepts the
/// union of every client's values.
fn valid_server_types(path: &Path, config: &PerFileLintConfig<'_>) -> Vec<&'static str> {
    let readers = if is_codex_plugin_mcp_file(path, config) {
        &["codex"][..]
    } else {
        mcp_file_readers(path)
    };
    server_types_for_readers(readers, config)
}

/// The `type` values valid for a server that `readers` read, narrowed to the
/// selected tools as [`valid_server_types`] describes.
fn server_types_for_readers(
    readers: &[&'static str],
    config: &PerFileLintConfig<'_>,
) -> Vec<&'static str> {
    let selected: Vec<&str> = readers
        .iter()
        .copied()
        .filter(|reader| {
            config.tools().iter().any(|t| {
                t.eq_ignore_ascii_case(reader)
                    || crate::config::LintConfig::is_tool_alias(t, reader)
            })
        })
        .collect();
    let clients: Vec<&str> = if !selected.is_empty() {
        selected
    } else if !readers.is_empty() {
        readers.to_vec()
    } else {
        CLIENT_SERVER_TYPES
            .iter()
            .map(|(client, _)| *client)
            .collect()
    };
    let mut types: Vec<&'static str> = CLIENT_SERVER_TYPES
        .iter()
        .filter(|(client, _)| clients.contains(client))
        .flat_map(|(_, types)| types.iter().copied())
        .collect();
    let mut seen = std::collections::HashSet::new();
    types.retain(|t| seen.insert(*t));
    types
}

/// The transport a `type` value selects: `local` is stdio, `streamable-http`
/// and Cline's `streamableHttp` are Streamable HTTP.
fn normalized_transport(server_type: &str) -> &str {
    match server_type {
        "local" => "stdio",
        "streamable-http" | "streamableHttp" => "http",
        other => other,
    }
}

/// An Amp skill's sibling `mcp.json` (next to a `SKILL.md`), which uses a
/// flat server map where remote entries carry only `url`.
fn is_skill_mcp_file(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("mcp.json"))
        && path
            .parent()
            .is_some_and(|dir| config.fs().is_file(&dir.join("SKILL.md")))
}

/// `.vscode/mcp.json`, which VS Code reads as JSON with comments.
fn is_vscode_mcp_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("mcp.json"))
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(".vscode"))
}

pub struct McpValidator;

impl Validator for McpValidator {
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

        // Early return if MCP category is disabled
        if !config.rules().mcp {
            return diagnostics;
        }

        // Client configs that embed MCP servers (OpenCode, Gemini settings,
        // extensions and agents, Amp settings and skills, Codex, Copilot
        // agents) get only the server-level checks. The sibling refinement
        // matters here: a Gemini extension's `agents/*.md` is a Gemini agent
        // only next to its `gemini-extension.json`.
        let file_type = crate::file_types::refine_file_type(
            path,
            crate::file_types::detect_file_type(path),
            config.fs().as_ref(),
        );
        if crate::rules::mcp_servers::is_client_config_with_mcp(file_type) {
            return crate::rules::mcp_servers::validate_client_config(
                file_type, path, content, config,
            );
        }
        // Plugin manifests may declare servers inline in `mcpServers`; only
        // those servers are checked here.
        if matches!(
            file_type,
            FileType::Plugin | FileType::CopilotPlugin | FileType::CursorPlugin
        ) {
            return validate_plugin_manifest_servers(file_type, path, content, config);
        }

        // VS Code reads `.vscode/mcp.json` as JSON with comments; blank them
        // (offsets are kept) so documented comments are not a parse error.
        let stripped;
        let content = if is_vscode_mcp_file(path) {
            stripped = crate::schemas::kiro_agent::strip_jsonc(content).0;
            stripped.as_str()
        } else {
            content
        };

        // Try to parse as JSON
        let raw_value: serde_json::Value = match serde_json::from_str(content) {
            Ok(v) => v,
            Err(e) => {
                if config.is_rule_enabled("MCP-007") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "MCP-007",
                            t!("rules.mcp_007.message", error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.mcp_007.suggestion")),
                    );
                }
                return diagnostics;
            }
        };

        // Check for JSON-RPC version (MCP-001)
        if config.is_rule_enabled("MCP-001") {
            validate_jsonrpc_version(&raw_value, path, content, &mut diagnostics);
        }

        // Check for protocol version mismatch (MCP-008)
        if config.is_rule_enabled("MCP-008") {
            validate_protocol_version(&raw_value, path, content, config, &mut diagnostics);
        }

        let line_starts = compute_line_starts(content);
        let tool_spans = collect_tools_array_object_spans(content);

        // Get tools array from various locations (also reports parse errors for invalid entries)
        let tools = extract_tools(
            &raw_value,
            path,
            &mut diagnostics,
            &tool_spans,
            &line_starts,
        );

        // MCP-030: tool names should be unique within a server.
        if config.is_rule_enabled("MCP-030") {
            validate_duplicate_tool_names(
                &tools,
                path,
                content,
                &tool_spans,
                &line_starts,
                &mut diagnostics,
            );
        }

        // Validate each successfully parsed tool
        for (idx, tool) in tools.iter().enumerate() {
            validate_tool(
                tool,
                path,
                config,
                &mut diagnostics,
                ToolLookupContext {
                    content,
                    line_starts: &line_starts,
                    tool_span: tool_spans.get(idx).copied(),
                    tool_index: idx,
                },
            );
        }

        // Validate resource and prompt schema requirements.
        validate_resource_definitions(&raw_value, path, content, config, &mut diagnostics);
        validate_prompt_definitions(&raw_value, path, content, config, &mut diagnostics);

        // MCP-020: validate capability keys (walks `capabilities`, not
        // `mcpServers` - kept adjacent to the server-name rules only by
        // historical grouping).
        validate_capability_keys(&raw_value, path, content, config, &mut diagnostics);
        // MCP-032: capabilities and sampling options deprecated in 2026-07-28.
        validate_deprecated_capabilities(&raw_value, path, content, config, &mut diagnostics);

        // MCP-023 walks the top-level `mcpServers` key list; files where the
        // rule is disabled pay nothing.
        let server_name_offsets = if config.is_rule_enabled("MCP-023") {
            collect_mcp_server_name_offsets(content)
        } else {
            Vec::new()
        };

        if config.is_rule_enabled("MCP-023") {
            validate_duplicate_server_names(
                path,
                &server_name_offsets,
                &line_starts,
                &mut diagnostics,
            );
        }

        // Validate MCP server configurations (MCP-009 to MCP-012, MCP-017 to
        // MCP-019, MCP-021, MCP-022, MCP-024, MCP-025, MCP-027 to MCP-029)
        let locator = Locator::new(content);
        for (name, server) in extract_mcp_servers(&raw_value) {
            let raw_server = find_raw_server(&raw_value, &name);
            validate_server(
                &name,
                &server,
                raw_server,
                ServerContext {
                    path,
                    content,
                    locator: &locator,
                    position: None,
                    inline: None,
                },
                config,
                &mut diagnostics,
            );
        }

        diagnostics
    }
}

// ---------------------------------------------------------------------------
// Inline servers in plugin manifests
// ---------------------------------------------------------------------------

/// A server map declared inline in a plugin manifest's `mcpServers`.
pub(crate) struct InlineServerMap<'a> {
    /// The map: server name to server config.
    pub node: &'a JsonNode,
    /// Path from the manifest root to the map.
    pub segs: Vec<Seg<'static>>,
}

/// The inline server maps of a plugin manifest: `mcpServers` as an object,
/// or each object in an `mcpServers` array (the strings there are file paths
/// or bundle URLs, linted as their own files where agnix detects them). With
/// `unwrap`, an inline object that holds an `mcpServers` object (the
/// `.mcp.json` shape) yields the inner object.
pub(crate) fn inline_server_maps(root: &JsonNode, unwrap: bool) -> Vec<InlineServerMap<'_>> {
    let Some(member) = root.value.get("mcpServers") else {
        return Vec::new();
    };
    let mut maps: Vec<InlineServerMap<'_>> = Vec::new();
    match &member.value.value {
        JsonValue::Object(_) => maps.push(InlineServerMap {
            node: &member.value,
            segs: vec![Seg::Key("mcpServers")],
        }),
        JsonValue::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                if item.value.as_object().is_some() {
                    maps.push(InlineServerMap {
                        node: item,
                        segs: vec![Seg::Key("mcpServers"), Seg::Index(index)],
                    });
                }
            }
        }
        _ => {}
    }
    if unwrap {
        for map in &mut maps {
            if let Some(inner) = map.node.value.get("mcpServers")
                && inner.value.value.as_object().is_some()
            {
                map.node = &inner.value;
                map.segs.push(Seg::Key("mcpServers"));
            }
        }
    }
    maps
}

/// The serde value at `segs` below `root`.
pub(crate) fn json_value_at<'v>(
    root: &'v serde_json::Value,
    segs: &[Seg<'_>],
) -> Option<&'v serde_json::Value> {
    segs.iter().try_fold(root, |value, seg| match seg {
        Seg::Key(key) => value.get(*key),
        Seg::Index(index) => value.get(*index),
    })
}

/// True for a `.claude-plugin/plugin.json`.
pub(crate) fn is_claude_plugin_manifest(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("plugin.json"))
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(".claude-plugin"))
}

/// Who reads the inline servers of the plugin manifest at `path`, or `None`
/// when the manifest format has no inline servers.
///
/// - `.claude-plugin/plugin.json`: Claude Code ("inline server configs keyed
///   by name"), and Copilot CLI, which reads that location for legacy
///   plugins.
/// - Any other `plugin.json` (a root manifest), `.github/plugin/plugin.json`
///   and `.plugin/plugin.json`: legacy Copilot plugins, whose `mcpServers` is
///   a "Path to an MCP configuration file ... or inline server definitions".
/// - `.cursor-plugin/plugin.json`: Cursor, which takes "inline MCP server
///   config" and infers "the transport from `command` or `url`".
fn plugin_manifest_readers(file_type: FileType, path: &Path) -> Option<InlineReaders> {
    let is_plugin_json = path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("plugin.json"));
    match file_type {
        FileType::Plugin if is_claude_plugin_manifest(path) => Some(InlineReaders {
            readers: &["claude-code", "github-copilot"],
            url_only_is_remote: false,
        }),
        FileType::Plugin | FileType::CopilotPlugin => Some(InlineReaders {
            readers: &["github-copilot"],
            url_only_is_remote: false,
        }),
        FileType::CursorPlugin if is_plugin_json => Some(InlineReaders {
            readers: &["cursor"],
            url_only_is_remote: true,
        }),
        _ => None,
    }
}

/// The server-level MCP checks (MCP-009 to MCP-012, MCP-017 to MCP-019,
/// MCP-021 to MCP-024, MCP-027 to MCP-029) on the servers a plugin manifest
/// declares inline in `mcpServers`. Diagnostics point at each server's key
/// in the manifest. An Agent Plugins manifest has no `mcpServers` field
/// ("Component path fields such as ... `mcpServers` ... are not Agent
/// Plugins 1.0 manifest fields"), so it is skipped.
fn validate_plugin_manifest_servers(
    file_type: FileType,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let Some(inline) = plugin_manifest_readers(file_type, path) else {
        return diagnostics;
    };
    // Manifest parse errors belong to the manifest's own validator.
    let Ok(document) = parse_json_document(content, false) else {
        return diagnostics;
    };
    let Ok(raw_value) = serde_json::from_str::<serde_json::Value>(content) else {
        return diagnostics;
    };
    if raw_value
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .is_some_and(|schema| {
            schema.starts_with(crate::rules::codex_plugin::AGENT_PLUGIN_SCHEMA_PREFIX)
        })
    {
        return diagnostics;
    }
    // Claude Code documents only the bare map ("Uses the map as server
    // configs keyed by name"); the Copilot and Cursor pages do not say
    // whether the inline object may keep the `.mcp.json` wrapper.
    let unwrap = !is_claude_plugin_manifest(path);
    let line_starts = compute_line_starts(content);
    for map in inline_server_maps(&document.root, unwrap) {
        let Some(members) = map.node.value.as_object() else {
            continue;
        };
        let key_offsets: Vec<(String, usize)> = members
            .iter()
            .map(|member| (member.key.clone(), member.key_start))
            .collect();
        if config.is_rule_enabled("MCP-023") {
            validate_duplicate_server_names(path, &key_offsets, &line_starts, &mut diagnostics);
        }
        let Some(servers) = json_value_at(&raw_value, &map.segs).and_then(|v| v.as_object()) else {
            continue;
        };
        let locator = Locator::within(content, map.node.start, map.node.end, key_offsets);
        for (name, server) in parse_server_map(servers) {
            // serde_json keeps the last of duplicate keys; so does the span.
            let position = members
                .iter()
                .rev()
                .find(|member| member.key == name)
                .map(|member| line_col_at(member.key_start, &line_starts));
            validate_server(
                &name,
                &server,
                servers.get(&name),
                ServerContext {
                    path,
                    content,
                    locator: &locator,
                    position,
                    inline: Some(inline),
                },
                config,
                &mut diagnostics,
            );
        }
    }
    diagnostics
}

/// Extract tools from various MCP config formats, reporting parse errors for invalid entries
fn extract_tools(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    tool_spans: &[(usize, usize)],
    line_starts: &[usize],
) -> Vec<McpToolSchema> {
    let mut tools = Vec::new();

    if let Some(arr) = raw_value.get("tools").and_then(|v| v.as_array()) {
        // Check for tools array at root level
        for (idx, tool_val) in arr.iter().enumerate() {
            match serde_json::from_value::<McpToolSchema>(tool_val.clone()) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    // Report invalid tool entries instead of silently skipping
                    let (line, col) =
                        tool_location_from_span(tool_spans, idx, line_starts).unwrap_or((1, 0));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "mcp::invalid_tool",
                            t!("rules.invalid_tool", num = idx + 1, error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.invalid_tool_suggestion")),
                    );
                }
            }
        }
    }

    // tools/list response shape: {"result": {"tools": [...]}}
    if let Some(arr) = raw_value
        .get("result")
        .and_then(|v| v.get("tools"))
        .and_then(|v| v.as_array())
    {
        let start = tools.len();
        for (idx, tool_val) in arr.iter().enumerate() {
            match serde_json::from_value::<McpToolSchema>(tool_val.clone()) {
                Ok(tool) => tools.push(tool),
                Err(e) => {
                    let (line, col) = tool_location_from_span(tool_spans, start + idx, line_starts)
                        .unwrap_or((1, 0));
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "mcp::invalid_tool",
                            t!("rules.invalid_tool", num = idx + 1, error = e.to_string()),
                        )
                        .with_suggestion(t!("rules.invalid_tool_suggestion")),
                    );
                }
            }
        }
    }

    if !tools.is_empty() {
        return tools;
    }

    // Check if root is a single tool definition (has name OR inputSchema OR description)
    // This allows detecting incomplete tools for validation
    let has_tool_fields = raw_value.get("name").is_some()
        || raw_value.get("inputSchema").is_some()
        || raw_value.get("description").is_some()
        || raw_value.get("title").is_some()
        || raw_value.get("outputSchema").is_some()
        || raw_value.get("icons").is_some();

    if has_tool_fields {
        match serde_json::from_value::<McpToolSchema>(raw_value.clone()) {
            Ok(tool) => tools.push(tool),
            Err(e) => {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "mcp::invalid_tool",
                        t!("rules.invalid_tool_single", error = e.to_string()),
                    )
                    .with_suggestion(t!("rules.invalid_tool_suggestion")),
                );
            }
        }
    }

    tools
}

fn extract_mcp_servers(raw_value: &serde_json::Value) -> Vec<(String, McpServerConfig)> {
    // Primary shape: `{ "mcpServers": { "name": {...}, ... } }`.
    // This is what Claude Code, Cursor, Windsurf, and the MCP spec use.
    if let Some(servers_obj) = raw_value.get("mcpServers").and_then(|v| v.as_object()) {
        return parse_server_map(servers_obj);
    }

    // VS Code `.vscode/mcp.json` "defines servers in a top-level `servers`
    // object" (next to `inputs`). Every entry must look like a server (it
    // has `command` or `url`): other JSON files named like MCP configs use a
    // `servers` key for other records, such as a tool cache keyed by server.
    if let Some(servers_obj) = raw_value.get("servers").and_then(|v| v.as_object())
        && !servers_obj.is_empty()
        && servers_obj.values().all(looks_like_mcp_server_config)
    {
        return parse_server_map(servers_obj);
    }

    // Fallback shape: flat top-level server map `{ "name": {...}, ... }`.
    // Accepted by Codex plugin MCP loading as of rust-v0.123.0 (openai/codex
    // "Make plugin MCP loading accept both mcpServers and top-level server
    // maps"). The plain MCP spec and other tools use `mcpServers`.
    //
    // To avoid misreading a Claude Code `.mcp.json` typo as flat-form, gate
    // this fallback strictly: every top-level value MUST look like a server
    // config (object with `command` or `url`). That excludes generic MCP
    // payloads (which have `jsonrpc`, `tools`, `resources`, etc.) and random
    // JSON. A file with a single `mcpServers` typo like `mcpServersx` won't
    // match because the value of that key is itself a server map, not a
    // server config.
    if let Some(root_obj) = raw_value.as_object()
        && !root_obj.is_empty()
        && root_obj.values().all(looks_like_mcp_server_config)
    {
        return parse_server_map(root_obj);
    }

    Vec::new()
}

fn find_raw_server<'a>(
    raw_value: &'a serde_json::Value,
    name: &str,
) -> Option<&'a serde_json::Value> {
    if let Some(servers) = raw_value.get("mcpServers").and_then(|v| v.as_object()) {
        servers.get(name)
    } else if let Some(servers) = raw_value
        .get("servers")
        .and_then(|v| v.as_object())
        .filter(|servers| servers.contains_key(name))
    {
        servers.get(name)
    } else {
        raw_value.as_object().and_then(|root| root.get(name))
    }
}

/// Parse a JSON object into `(name, McpServerConfig)` entries. Strict
/// deserialize first; falls back to lenient parsing (preserving what we
/// can) when strict fails so partial configs still surface per-server
/// diagnostics rather than dropping the entry entirely.
fn parse_server_map(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> Vec<(String, McpServerConfig)> {
    obj.iter()
        .map(|(name, server_value)| {
            let server = serde_json::from_value::<McpServerConfig>(server_value.clone())
                .unwrap_or_else(|_| parse_mcp_server_lenient(server_value));
            (name.clone(), server)
        })
        .collect()
}

/// Conservative classifier: does this JSON value look like an MCP server
/// config? A server config is an object with at least one of `command` (for
/// stdio servers) or `url` (for HTTP/SSE/WebSocket servers). We intentionally
/// do NOT accept bare objects with just `args` or just `env` - those could
/// easily be non-server data.
fn looks_like_mcp_server_config(value: &serde_json::Value) -> bool {
    let Some(obj) = value.as_object() else {
        return false;
    };
    obj.contains_key("command") || obj.contains_key("url")
}

fn parse_mcp_server_lenient(value: &serde_json::Value) -> McpServerConfig {
    let Some(obj) = value.as_object() else {
        return McpServerConfig {
            server_type: None,
            command: None,
            args: None,
            env: None,
            url: None,
            always_load: None,
        };
    };

    McpServerConfig {
        server_type: obj
            .get("type")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        command: obj.get("command").cloned(),
        args: obj.get("args").cloned(),
        env: parse_env_lenient(obj.get("env")),
        url: obj
            .get("url")
            .and_then(|v| v.as_str())
            .map(ToOwned::to_owned),
        // Map explicit `null` to absent so the lenient path matches the
        // strict serde path. For `Option<Value>`, serde's `Option` impl
        // already treats explicit `null` as `None` (independent of
        // `#[serde(default)]`, which only affects missing keys). Without
        // this filter the lenient branch would clone `Value::Null` and
        // MCP-025 would false-positive on `alwaysLoad: null` whenever
        // some sibling field forces the fallback.
        always_load: obj.get("alwaysLoad").filter(|v| !v.is_null()).cloned(),
    }
}

fn parse_env_lenient(
    env_value: Option<&serde_json::Value>,
) -> Option<std::collections::HashMap<String, String>> {
    let env_obj = env_value.and_then(|v| v.as_object())?;

    let env = env_obj
        .iter()
        .filter_map(|(key, value)| {
            if value.is_null() {
                None
            } else {
                Some((
                    key.clone(),
                    value
                        .as_str()
                        .map(ToOwned::to_owned)
                        .unwrap_or_else(|| value.to_string()),
                ))
            }
        })
        .collect();

    Some(env)
}

fn iter_root_or_result_array<'a>(
    value: &'a serde_json::Value,
    key: &str,
) -> Vec<&'a Vec<serde_json::Value>> {
    let mut arrays = Vec::new();
    if let Some(arr) = value.get(key).and_then(|v| v.as_array()) {
        arrays.push(arr);
    }
    if let Some(arr) = value
        .get("result")
        .and_then(|v| v.get(key))
        .and_then(|v| v.as_array())
    {
        arrays.push(arr);
    }
    arrays
}

fn is_non_empty_string(value: Option<&serde_json::Value>) -> bool {
    value
        .and_then(|v| v.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

fn validate_resource_definitions(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for resources in iter_root_or_result_array(value, "resources") {
        for (idx, resource) in resources.iter().enumerate() {
            let Some(obj) = resource.as_object() else {
                continue;
            };

            let resource_prefix = format!("Resource #{}: ", idx + 1);
            if config.is_rule_enabled("MCP-015") {
                if !is_non_empty_string(obj.get("uri")) {
                    let (line, col) = find_json_field_location(content, "resources");
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "MCP-015",
                            format!("{}missing required field 'uri'", resource_prefix),
                        )
                        .with_suggestion("Add a non-empty URI to the resource definition"),
                    );
                }
                if !is_non_empty_string(obj.get("name")) {
                    let (line, col) = find_json_field_location(content, "resources");
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            line,
                            col,
                            "MCP-015",
                            format!("{}missing required field 'name'", resource_prefix),
                        )
                        .with_suggestion("Add a non-empty name to the resource definition"),
                    );
                }
            }
        }
    }
}

fn validate_prompt_definitions(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for prompts in iter_root_or_result_array(value, "prompts") {
        for (idx, prompt) in prompts.iter().enumerate() {
            let Some(obj) = prompt.as_object() else {
                continue;
            };

            let prompt_prefix = format!("Prompt #{}: ", idx + 1);
            let (line, col) = find_json_field_location(content, "prompts");

            if config.is_rule_enabled("MCP-016") && !is_non_empty_string(obj.get("name")) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-016",
                        format!("{}missing required field 'name'", prompt_prefix.as_str()),
                    )
                    .with_suggestion("Add a non-empty prompt name"),
                );
            }
        }
    }
}

/// True unless the project pins an MCP revision older than 2026-07-28, the
/// revision that deprecated Roots, Sampling and Logging and added
/// `x-mcp-header`.
fn targets_2026_07_28(config: &PerFileLintConfig<'_>) -> bool {
    !config.is_mcp_revision_pinned() || config.get_mcp_protocol_version() >= "2026-07-28"
}

/// MCP-032: Roots, Sampling and Logging are deprecated in 2026-07-28 ("new
/// implementations should not add support for them"), and so are the
/// `includeContext` values `thisServer` and `allServers`.
fn validate_deprecated_capabilities(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("MCP-032") || !targets_2026_07_28(config) {
        return;
    }
    let candidates = [
        value.get("capabilities"),
        value.get("params").and_then(|v| v.get("capabilities")),
        value.get("result").and_then(|v| v.get("capabilities")),
    ];
    let mut reported = std::collections::BTreeSet::new();
    for caps in candidates.into_iter().flatten() {
        let Some(caps) = caps.as_object() else {
            continue;
        };
        for key in ["roots", "sampling", "logging"] {
            if caps.contains_key(key) && reported.insert(key) {
                let (line, col) = find_json_field_location(content, key);
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-032",
                        t!("rules.mcp_032.message", feature = key),
                    )
                    .with_suggestion(t!("rules.mcp_032.suggestion")),
                );
            }
        }
    }
    let include_context = value
        .get("params")
        .and_then(|p| p.get("includeContext"))
        .and_then(|v| v.as_str());
    if let Some(mode) = include_context
        && matches!(mode, "thisServer" | "allServers")
    {
        let (line, col) = find_json_field_location(content, "includeContext");
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line,
                col,
                "MCP-032",
                t!(
                    "rules.mcp_032.message",
                    feature = format!("includeContext: \"{mode}\"")
                ),
            )
            .with_suggestion(t!("rules.mcp_032.include_context_suggestion")),
        );
    }
}

/// MCP-030: "Tool names SHOULD be unique within a server."
fn validate_duplicate_tool_names(
    tools: &[McpToolSchema],
    path: &Path,
    content: &str,
    tool_spans: &[(usize, usize)],
    line_starts: &[usize],
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut counts: std::collections::BTreeMap<&str, Vec<usize>> =
        std::collections::BTreeMap::new();
    for (idx, tool) in tools.iter().enumerate() {
        if let Some(name) = tool
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            counts.entry(name).or_default().push(idx);
        }
    }
    for (name, indexes) in counts {
        if indexes.len() < 2 {
            continue;
        }
        let second = indexes[1];
        let (line, col) = tool_spans
            .get(second)
            .and_then(|span| find_json_field_location_in_span(content, "name", *span, line_starts))
            .or_else(|| tool_location_from_span(tool_spans, second, line_starts))
            .unwrap_or_else(|| find_json_field_location(content, name));
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-030",
                t!("rules.mcp_030.message", name = name, count = indexes.len()),
            )
            .with_suggestion(t!("rules.mcp_030.suggestion")),
        );
    }
}

/// HTTP field-name token characters (RFC 9110 `tchar`).
fn is_http_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|b| {
            b.is_ascii_alphanumeric()
                || matches!(
                    b,
                    b'!' | b'#'
                        | b'$'
                        | b'%'
                        | b'&'
                        | b'\''
                        | b'*'
                        | b'+'
                        | b'-'
                        | b'.'
                        | b'^'
                        | b'_'
                        | b'`'
                        | b'|'
                        | b'~'
                )
        })
}

/// One problem with an `x-mcp-header` annotation (MCP-031).
struct HeaderAnnotationIssue {
    property: String,
    reason: String,
}

/// Walk `schema` for `x-mcp-header` annotations. `reachable` is true while
/// the path from the root is a chain of `properties` keys only; `path` is
/// that chain.
fn collect_header_annotations(
    schema: &serde_json::Value,
    reachable: bool,
    path: &str,
    found: &mut Vec<(String, String, bool, Option<String>)>,
    issues: &mut Vec<HeaderAnnotationIssue>,
) {
    let Some(obj) = schema.as_object() else {
        if let Some(items) = schema.as_array() {
            for item in items {
                collect_header_annotations(item, false, path, found, issues);
            }
        }
        return;
    };
    if let Some(value) = obj.get("x-mcp-header") {
        let property = if path.is_empty() { "(root)" } else { path }.to_string();
        match value.as_str() {
            None => issues.push(HeaderAnnotationIssue {
                property,
                reason: "the value is not a string".to_string(),
            }),
            Some(header) => {
                let schema_type = obj
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(ToOwned::to_owned);
                found.push((property, header.to_string(), reachable, schema_type));
            }
        }
    }
    for (key, child) in obj {
        match key.as_str() {
            "properties" => {
                if let Some(props) = child.as_object() {
                    for (name, prop) in props {
                        let child_path = if path.is_empty() {
                            name.clone()
                        } else {
                            format!("{path}.{name}")
                        };
                        collect_header_annotations(prop, reachable, &child_path, found, issues);
                    }
                }
            }
            "x-mcp-header" => {}
            _ if child.is_object() || child.is_array() => {
                collect_header_annotations(child, false, path, found, issues);
            }
            _ => {}
        }
    }
}

/// MCP-031: `x-mcp-header` constraints. Streamable HTTP clients "MUST reject
/// tool definitions where any `x-mcp-header` value violates these
/// constraints", dropping the tool from `tools/list`.
fn header_annotation_issues(
    input_schema: &serde_json::Value,
) -> (Vec<HeaderAnnotationIssue>, Vec<String>) {
    let mut found = Vec::new();
    let mut issues = Vec::new();
    collect_header_annotations(input_schema, true, "", &mut found, &mut issues);
    let mut sensitive = Vec::new();
    let mut seen: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for (property, header, reachable, schema_type) in found {
        let reason = if header.is_empty() {
            Some("the value is empty".to_string())
        } else if header.chars().any(char::is_control) {
            Some("the value contains a control character".to_string())
        } else if !is_http_token(&header) {
            Some(format!("'{header}' is not an HTTP field-name token"))
        } else if !reachable {
            Some("it is not reachable from the schema root through 'properties' alone".to_string())
        } else if let Some(other) = seen.get(&header.to_ascii_lowercase()) {
            Some(format!("'{header}' repeats the header of '{other}'"))
        } else {
            match schema_type.as_deref() {
                Some("string" | "integer" | "boolean") => None,
                Some(other) => Some(format!(
                    "it is set on a '{other}' parameter; only string, integer and boolean are allowed"
                )),
                None => Some(
                    "the parameter has no primitive 'type' (string, integer or boolean)"
                        .to_string(),
                ),
            }
        };
        seen.entry(header.to_ascii_lowercase())
            .or_insert_with(|| property.clone());
        if let Some(reason) = reason {
            issues.push(HeaderAnnotationIssue { property, reason });
            continue;
        }
        let lower = property.to_ascii_lowercase();
        if [
            "password", "secret", "token", "apikey", "api_key", "api-key",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
        {
            sensitive.push(property);
        }
    }
    (issues, sensitive)
}

fn validate_capability_keys(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !config.is_rule_enabled("MCP-020") {
        return;
    }

    let Some(caps_obj) = value
        .get("capabilities")
        .and_then(|v| v.as_object())
        .or_else(|| {
            value
                .get("result")
                .and_then(|v| v.get("capabilities"))
                .and_then(|v| v.as_object())
        })
    else {
        return;
    };

    for key in caps_obj.keys() {
        if !VALID_MCP_CAPABILITY_KEYS.contains(&key.as_str()) {
            let (line, col) = find_json_field_location(content, key);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-020",
                    format!("Unknown capability key '{}'", key),
                )
                .with_suggestion("Use only capability keys defined by the MCP specification"),
            );
        }
    }
}

fn validate_duplicate_server_names(
    path: &Path,
    server_name_offsets: &[(String, usize)],
    line_starts: &[usize],
    diagnostics: &mut Vec<Diagnostic>,
) {
    use std::collections::HashSet;

    let mut seen: HashSet<&str> = HashSet::new();

    for (name, offset) in server_name_offsets {
        if !seen.insert(name.as_str()) {
            let (line, col) = line_col_at(*offset, line_starts);
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-023",
                    format!("Duplicate MCP server name '{}'", name),
                )
                .with_suggestion("Rename duplicate mcpServers keys so each server name is unique"),
            );
        }
    }
}

/// Walks the top-level keys of the `mcpServers` object and yields each
/// `(name, quote_start_offset)` pair in source order. Used by MCP-023
/// (duplicates).
fn collect_mcp_server_name_offsets(content: &str) -> Vec<(String, usize)> {
    let bytes = content.as_bytes();
    let Some(object_start) = find_json_object_start_for_key(content, "mcpServers") else {
        return Vec::new();
    };
    let mut idx = object_start + 1;

    let mut depth = 1usize;
    let mut expecting_key = true;
    let mut names = Vec::new();

    while idx < bytes.len() && depth > 0 {
        let ch = bytes[idx] as char;
        if ch == '"' {
            let (raw, next_idx) = read_json_string_literal(content, idx);
            if depth == 1 && expecting_key {
                names.push((raw, idx));
                expecting_key = false;
            }
            idx = next_idx;
            continue;
        }

        match ch {
            '{' | '[' => depth += 1,
            '}' | ']' => {
                depth = depth.saturating_sub(1);
            }
            ',' if depth == 1 => expecting_key = true,
            _ => {}
        }
        idx += 1;
    }

    names
}

/// Best-effort JSON string decoder for the sole purpose of extracting
/// `mcpServers` keys for MCP-023 (duplicate detection) and MCP-026
/// (reserved names). Handles the common `\"` and `\\` escape pairs by
/// emitting the next character literally; does NOT decode `\uXXXX` unicode
/// escapes, `\n` / `\t` / `\r`, or the other JSON escape sequences.
///
/// Practical impact: a hypothetical server name written with a `\uXXXX`
/// escape in its JSON key (e.g. `"workspace"` for `workspace`) will
/// be read back as the literal characters `u`, `0`, `0`, `6`, `f`, ... and
/// will not match the reserved list. This is accepted because:
///
/// 1. Real `mcpServers` keys are ASCII identifiers in every config we've
///    observed - no one writes `o` for an `o` inside an identifier.
/// 2. MCP-023 has the same limitation, so MCP-026 is not a regression.
/// 3. A full JSON decoder would require threading `serde_json::Value`
///    positions through the walker, which isn't justified by the
///    evasion vector (deliberately escaping a reserved name to hide it
///    from the linter while keeping it functional - the linter user is
///    themselves).
///
/// If this ever matters (a validator ingesting third-party / untrusted
/// MCP configs), the `serde_json::Value::as_object()` pass later in
/// `validate()` already sees fully-decoded keys - route name-based
/// rules through there instead of this walker.
fn read_json_string_literal(content: &str, start_quote_idx: usize) -> (String, usize) {
    let bytes = content.as_bytes();
    let mut idx = start_quote_idx + 1;
    let mut escaped = false;
    let mut out = String::new();

    while idx < bytes.len() {
        let ch = bytes[idx] as char;
        if escaped {
            out.push(ch);
            escaped = false;
            idx += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            idx += 1;
            continue;
        }
        if ch == '"' {
            return (out, idx + 1);
        }
        out.push(ch);
        idx += 1;
    }

    (out, bytes.len())
}

/// MCP-001: Validate JSON-RPC version is "2.0"
fn validate_jsonrpc_version(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if jsonrpc field exists
    if let Some(jsonrpc) = value.get("jsonrpc") {
        let (line, col) = find_json_field_location(content, "jsonrpc");
        if let Some(version) = jsonrpc.as_str() {
            if version != "2.0" {
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-001",
                    t!("rules.mcp_001.invalid_version", version = version),
                )
                .with_suggestion(t!("rules.mcp_001.suggestion"));

                // Safe auto-fix: enforce jsonrpc: "2.0"
                if let Some((start, end)) = find_unique_json_scalar_value_span(content, "jsonrpc") {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        "\"2.0\"",
                        "Set jsonrpc version to \"2.0\"",
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        } else {
            let mut diagnostic = Diagnostic::error(
                path.to_path_buf(),
                line,
                col,
                "MCP-001",
                t!("rules.mcp_001.not_string"),
            )
            .with_suggestion(t!("rules.mcp_001.suggestion"));

            // Safe auto-fix: normalize non-string jsonrpc values to "2.0"
            if let Some((start, end)) = find_unique_json_scalar_value_span(content, "jsonrpc") {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    "\"2.0\"",
                    "Set jsonrpc version to \"2.0\"",
                    true,
                ));
            }

            diagnostics.push(diagnostic);
        }
    }
    // Note: jsonrpc field is only required for JSON-RPC messages, not tool definitions
    // So we don't report missing jsonrpc as an error
}

/// Whether a declared `protocolVersion` should be reported by MCP-008.
///
/// When a revision is pinned via `[spec_revisions] mcp_protocol`, the check is
/// exact - the user asked for one revision. When nothing is pinned, any
/// revision in [`CURRENT_MCP_PROTOCOL_VERSIONS`] is accepted: `2025-11-25` and
/// `2026-07-28` are behaviorally incompatible rather than sequential, so both
/// are legitimately current and demanding either one would just relocate the
/// false positive onto configs written for the other.
fn is_mcp_version_mismatch(actual: &str, expected: &str, version_pinned: bool) -> bool {
    if version_pinned {
        return actual != expected;
    }
    !CURRENT_MCP_PROTOCOL_VERSIONS.contains(&actual)
}

/// The value MCP-008 should name as expected.
///
/// A pinned revision names itself; unpinned names the full accepted set so the
/// message does not claim a single revision is required when two are.
fn mcp_expected_display(expected: &str, version_pinned: bool) -> String {
    if version_pinned {
        expected.to_string()
    } else {
        CURRENT_MCP_PROTOCOL_VERSIONS.join(" or ")
    }
}

/// MCP-008: Validate protocol version matches expected version
fn validate_protocol_version(
    value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let expected_version = config.get_mcp_protocol_version();
    let version_pinned = config.is_mcp_revision_pinned();
    let expected_display = mcp_expected_display(expected_version, version_pinned);

    // Check initialize request
    if is_initialize_message(value) {
        if let Some(actual_version) = extract_request_protocol_version(value) {
            if is_mcp_version_mismatch(&actual_version, expected_version, version_pinned) {
                let (line, col) = find_json_field_location(content, "protocolVersion");
                let mut diag = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-008",
                    t!(
                        "rules.mcp_008.message",
                        found = actual_version.as_str(),
                        expected = expected_display.as_str()
                    ),
                )
                .with_suggestion(t!(
                    "rules.mcp_008.request_suggestion",
                    expected = expected_display.as_str()
                ));

                if !version_pinned {
                    diag = diag.with_assumption(t!("rules.mcp_008.assumption"));
                }

                // Unsafe auto-fix only when version is explicitly pinned.
                if version_pinned {
                    if let Some((start, end)) = find_unique_json_string_value_span(
                        content,
                        "protocolVersion",
                        actual_version.as_str(),
                    ) {
                        diag = diag.with_fix(Fix::replace(
                            start,
                            end,
                            expected_version,
                            "Align protocolVersion with pinned MCP revision",
                            false,
                        ));
                    }
                }

                diagnostics.push(diag);
            }
        }
    }

    // Check initialize response
    if is_initialize_response(value) {
        if let Some(actual_version) = extract_response_protocol_version(value) {
            if is_mcp_version_mismatch(&actual_version, expected_version, version_pinned) {
                let (line, col) = find_json_field_location(content, "protocolVersion");
                let mut diag = Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-008",
                    t!(
                        "rules.mcp_008.message",
                        found = actual_version.as_str(),
                        expected = expected_display.as_str()
                    ),
                )
                .with_suggestion(t!(
                    "rules.mcp_008.response_suggestion",
                    found = actual_version.as_str(),
                    expected = expected_display.as_str()
                ));

                if !version_pinned {
                    diag = diag.with_assumption(t!("rules.mcp_008.assumption"));
                }

                // Unsafe auto-fix only when version is explicitly pinned.
                if version_pinned {
                    if let Some((start, end)) = find_unique_json_string_value_span(
                        content,
                        "protocolVersion",
                        actual_version.as_str(),
                    ) {
                        diag = diag.with_fix(Fix::replace(
                            start,
                            end,
                            expected_version,
                            "Align protocolVersion with pinned MCP revision",
                            false,
                        ));
                    }
                }

                diagnostics.push(diag);
            }
        }
    }
}

/// Validate a single MCP tool
struct ToolLookupContext<'a> {
    content: &'a str,
    line_starts: &'a [usize],
    tool_span: Option<(usize, usize)>,
    tool_index: usize,
}

fn validate_tool(
    tool: &McpToolSchema,
    path: &Path,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
    lookup: ToolLookupContext<'_>,
) {
    let ToolLookupContext {
        content,
        line_starts,
        tool_span,
        tool_index,
    } = lookup;

    // Always include tool index for clarity, even for first tool
    let tool_prefix = format!("Tool #{}: ", tool_index + 1);

    // Get base location for this tool (preferring a precomputed span).
    let tool_loc = tool_span
        .map(|(start, _)| line_col_at(start, line_starts))
        .unwrap_or((1, 0));

    // Find field location in the current tool span first, then fall back to global lookup.
    let find_field = |field: &str| -> (usize, usize) {
        if let Some(span) = tool_span
            && let Some(loc) = find_json_field_location_in_span(content, field, span, line_starts)
        {
            return loc;
        }
        let (line, col) = find_json_field_location(content, field);
        if line > 1 || col > 0 {
            (line, col)
        } else {
            tool_loc
        }
    };

    let (has_name, _, has_schema) = tool.has_required_fields();

    // MCP-013: Tool name format validation.
    if config.is_rule_enabled("MCP-013")
        && let Some(name) = tool.name.as_deref().map(str::trim)
        && !name.is_empty()
    {
        let valid_chars = name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-'));
        if name.chars().count() > 128 || !valid_chars {
            let (line, col) = find_field("name");
            // A warning: the spec says tool names "SHOULD" follow this format.
            let mut diagnostic = Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-013",
                format!(
                    "{}invalid tool name '{}': expected 1-128 chars using [a-zA-Z0-9_.-]",
                    tool_prefix, name
                ),
            )
            .with_suggestion(
                "Rename the tool to use only letters, numbers, underscore, dot, or hyphen",
            );

            // Sanitize: replace invalid chars with '_', truncate to 128
            let sanitized: String = name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-') {
                        c
                    } else {
                        '_'
                    }
                })
                .take(128)
                .collect();
            if !sanitized.is_empty() && sanitized != name {
                if let Some((start, end)) =
                    crate::rules::find_unique_json_string_value_span(content, "name", name)
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        &sanitized,
                        format!("Sanitize tool name to '{}'", sanitized),
                        false,
                    ));
                }
            }

            diagnostics.push(diagnostic);
        }
    }

    // MCP-002: Missing required tool fields
    if config.is_rule_enabled("MCP-002") {
        if !has_name {
            let (line, col) = if tool.name.is_some() {
                find_field("name")
            } else {
                tool_loc
            };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-002",
                    t!("rules.mcp_002.missing_name", prefix = tool_prefix.as_str()),
                )
                .with_suggestion(t!("rules.mcp_002.missing_name_suggestion")),
            );
        }
        if !has_schema {
            // Check if content has a "parameters" field that might be a misnaming
            let has_parameters_field = content.contains("\"parameters\"");
            let suggestion = if has_parameters_field {
                format!(
                    "{}. Found 'parameters' field - did you mean 'inputSchema'?",
                    t!("rules.mcp_002.missing_schema_suggestion")
                )
            } else {
                t!("rules.mcp_002.missing_schema_suggestion").to_string()
            };
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    tool_loc.0,
                    tool_loc.1,
                    "MCP-002",
                    t!(
                        "rules.mcp_002.missing_schema",
                        prefix = tool_prefix.as_str()
                    ),
                )
                .with_suggestion(suggestion),
            );
        }
    }

    // MCP-003: Invalid inputSchema JSON Schema.
    if config.is_rule_enabled("MCP-003") {
        if let Some(schema) = &tool.input_schema {
            let (line, col) = find_field("inputSchema");
            let schema_errors = validate_json_schema_structure(schema);
            for error in schema_errors {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-003",
                        t!(
                            "rules.mcp_003.message",
                            prefix = tool_prefix.as_str(),
                            error = error
                        ),
                    )
                    .with_suggestion(t!("rules.mcp_003.suggestion")),
                );
            }
        }
    }

    // MCP-014: Invalid outputSchema JSON Schema.
    if config.is_rule_enabled("MCP-014")
        && let Some(schema) = &tool.output_schema
    {
        let (line, col) = find_field("outputSchema");
        let schema_errors = validate_json_schema_structure(schema);
        for error in schema_errors {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-014",
                    format!("{}invalid outputSchema: {}", tool_prefix, error),
                )
                .with_suggestion("Ensure outputSchema is a valid JSON Schema object"),
            );
        }
    }

    // MCP-004: Missing or short tool description
    if config.is_rule_enabled("MCP-004") && !tool.has_meaningful_description() {
        let (line, col) = if tool.description.is_some() {
            find_field("description")
        } else {
            tool_loc
        };
        let desc_len = tool.description.as_ref().map(|d| d.len()).unwrap_or(0);
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line,
                col,
                "MCP-004",
                t!(
                    "rules.mcp_004.message",
                    prefix = tool_prefix.as_str(),
                    len = desc_len
                ),
            )
            .with_suggestion(t!("rules.mcp_004.suggestion")),
        );
    }

    // MCP-031: invalid x-mcp-header annotations (2026-07-28).
    if config.is_rule_enabled("MCP-031")
        && targets_2026_07_28(config)
        && let Some(schema) = &tool.input_schema
    {
        let (issues, sensitive) = header_annotation_issues(schema);
        let (line, col) = find_field("x-mcp-header");
        for issue in issues {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-031",
                    t!(
                        "rules.mcp_031.message",
                        prefix = tool_prefix.as_str(),
                        property = issue.property.as_str(),
                        reason = issue.reason.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_031.suggestion")),
            );
        }
        for property in sensitive {
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-031",
                    t!(
                        "rules.mcp_031.sensitive",
                        prefix = tool_prefix.as_str(),
                        property = property.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_031.sensitive_suggestion")),
            );
        }
    }

    // MCP-033: a tool without parameters should say so. The spec lists
    // `{ "type": "object", "additionalProperties": false }` as "Recommended"
    // for tools that take no parameters; `{ "type": "object" }` "accepts any
    // object".
    if config.is_rule_enabled("MCP-033")
        && let Some(schema) = tool.input_schema.as_ref().and_then(|v| v.as_object())
        && schema.get("type").and_then(|t| t.as_str()) == Some("object")
        && schema.len() == 1
    {
        let (line, col) = find_field("inputSchema");
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                line,
                col,
                "MCP-033",
                t!("rules.mcp_033.message", prefix = tool_prefix.as_str()),
            )
            .with_suggestion(t!("rules.mcp_033.suggestion")),
        );
    }

    // MCP-006: unknown annotation keys (INFO). The spec's trust rule ("clients
    // MUST consider tool annotations to be untrusted unless they come from
    // trusted servers") is a duty of the client, not of the tool author, so a
    // tool that sets annotations is not reported. Only keys outside the
    // `ToolAnnotations` hints are. Keys are sorted so the message is the same
    // on every run (HashMap iteration order is not).
    if config.is_rule_enabled("MCP-006")
        && let Some(annotations) = &tool.annotations
    {
        let mut unknown_keys: Vec<&str> = annotations
            .keys()
            .map(String::as_str)
            .filter(|key| !VALID_MCP_ANNOTATION_HINTS.contains(key))
            .collect();
        unknown_keys.sort_unstable();

        if !unknown_keys.is_empty() {
            let (line, col) = find_field("annotations");
            diagnostics.push(
                Diagnostic::info(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-006",
                    t!(
                        "rules.mcp_006.message",
                        prefix = tool_prefix.as_str(),
                        keys = unknown_keys.join(", ")
                    ),
                )
                .with_suggestion(t!("rules.mcp_006.suggestion")),
            );
        }
    }
}

fn extract_http_host(url: &str) -> Option<String> {
    let trimmed = url.trim();
    let scheme_sep = trimmed.find("://")?;
    let host_and_path = &trimmed[scheme_sep + 3..];
    if host_and_path.is_empty() {
        return None;
    }
    let host_port_end = host_and_path
        .find(|c| ['/', '?', '#'].contains(&c))
        .unwrap_or(host_and_path.len());
    let host_port = &host_and_path[..host_port_end];
    if host_port.is_empty() {
        return None;
    }

    if host_port.starts_with('[') {
        if let Some(end) = host_port.find(']') {
            return Some(host_port[..=end].to_ascii_lowercase());
        }
        return None;
    }

    let host = host_port.split(':').next().unwrap_or(host_port);
    Some(host.to_ascii_lowercase())
}

fn is_local_http_host(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1" | "[::1]")
}

fn is_wildcard_http_host(host: &str) -> bool {
    matches!(host, "0.0.0.0" | "::" | "[::]" | "*")
}

fn has_usable_command(server: &McpServerConfig) -> bool {
    server.command.as_ref().is_some_and(|value| match value {
        serde_json::Value::String(command) => !command.trim().is_empty(),
        serde_json::Value::Array(items) => !items.is_empty(),
        serde_json::Value::Null => false,
        _ => true,
    })
}

fn is_cursor_mcp_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("mcp.json"))
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(".cursor"))
}

/// A Codex plugin's `.mcp.json`: next to `.codex-plugin/plugin.json`, in a
/// directory that is not also a Claude Code plugin. Codex picks the transport
/// from the fields present, so an entry with only a `url` is remote.
fn is_codex_plugin_mcp_file(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    let is_dot_mcp = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case(".mcp.json"));
    let Some(dir) = path.parent() else {
        return false;
    };
    let fs = config.fs();
    is_dot_mcp
        && fs.exists(&dir.join(".codex-plugin").join("plugin.json"))
        && !fs.exists(&dir.join(".claude-plugin").join("plugin.json"))
}

/// A Kiro power's `mcp.json`: inside `.kiro/powers/`, or next to the power's
/// `POWER.md` or `plugin.json`.
fn is_kiro_power_mcp_file(path: &Path, config: &PerFileLintConfig<'_>) -> bool {
    if !path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("mcp.json"))
    {
        return false;
    }
    let names: Vec<String> = path
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_ascii_lowercase())
        .collect();
    if names
        .windows(2)
        .any(|w| w[0] == ".kiro" && w[1] == "powers")
    {
        return true;
    }
    path.parent().is_some_and(|dir| {
        let fs = config.fs();
        fs.exists(&dir.join("POWER.md")) || fs.exists(&dir.join("plugin.json"))
    })
}

/// MCP-024: an entry with nothing in it. With the raw entry at hand, every
/// field must be blank (null, an empty string, array or object), so an entry
/// that holds only `headers` or another field is not "empty" (MCP-009 or
/// MCP-010 reports what it lacks).
fn is_empty_server_entry(server: &McpServerConfig, raw_server: Option<&serde_json::Value>) -> bool {
    if has_meaningful_server_config(server) {
        return false;
    }
    let Some(fields) = raw_server.and_then(serde_json::Value::as_object) else {
        return true;
    };
    fields.values().all(|value| match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(text) => text.trim().is_empty(),
        serde_json::Value::Array(items) => items.is_empty(),
        serde_json::Value::Object(map) => map.is_empty(),
        _ => false,
    })
}

fn has_meaningful_server_config(server: &McpServerConfig) -> bool {
    let has_type = server
        .server_type
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_command = has_usable_command(server);
    let has_args = server
        .args
        .as_ref()
        .is_some_and(|value| value.as_array().is_some_and(|arr| !arr.is_empty()));
    let has_url = server
        .url
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let has_env = server.env.as_ref().is_some_and(|env| !env.is_empty());
    // Explicit JSON `null` is conventionally "field absent" - match that for
    // alwaysLoad so `{ "alwaysLoad": null }` is still flagged as an empty
    // server by MCP-024 instead of masquerading as meaningful config.
    let has_always_load = server
        .always_load
        .as_ref()
        .is_some_and(|value| !value.is_null());
    has_type || has_command || has_args || has_url || has_env || has_always_load
}

/// Where a server entry lives, for diagnostics and fixes.
struct ServerContext<'a> {
    path: &'a Path,
    content: &'a str,
    locator: &'a Locator<'a>,
    /// Line and column of the server's key, when the caller knows it (inline
    /// servers in a plugin manifest); otherwise the first `"<name>"` key.
    position: Option<(usize, usize)>,
    /// Inline servers in a plugin manifest: the clients that read the
    /// manifest, and whether they read an entry with only a `url` as remote.
    /// `None` for MCP files, whose readers follow from the path.
    inline: Option<InlineReaders>,
}

/// Who reads the inline `mcpServers` of a plugin manifest.
#[derive(Clone, Copy)]
struct InlineReaders {
    readers: &'static [&'static str],
    url_only_is_remote: bool,
}

/// Validate a single MCP server configuration entry (MCP-009 to MCP-012,
/// MCP-017 to MCP-019, MCP-021, MCP-022, MCP-024, MCP-025, MCP-027 to MCP-029)
fn validate_server(
    name: &str,
    server: &McpServerConfig,
    raw_server: Option<&serde_json::Value>,
    ctx: ServerContext<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let ServerContext {
        path,
        content,
        locator,
        position,
        inline,
    } = ctx;
    let (line, col) = position.unwrap_or_else(|| find_json_field_location(content, name));
    let valid_server_types = |path: &Path, config: &PerFileLintConfig<'_>| match inline {
        Some(inline) => server_types_for_readers(inline.readers, config),
        None => valid_server_types(path, config),
    };
    let raw_server_type = raw_server
        .and_then(serde_json::Value::as_object)
        .and_then(|obj| obj.get("type"));

    // Cursor documents URL-only remote entries, Kiro and Codex pick the
    // transport from the fields present (a url without a command is remote;
    // a Codex plugin's `.mcp.json`), and an Amp skill's sibling `mcp.json`
    // lists remote servers with only a `url`.
    // Other clients, including Claude Code, still default an omitted type to
    // stdio ("Claude Code reads an entry with no `type` as a stdio server").
    let effective_type = server
        .server_type
        .as_deref()
        .map(normalized_transport)
        .unwrap_or_else(|| {
            if raw_server_type.is_none()
                && !has_usable_command(server)
                && server.url.is_some()
                && match inline {
                    Some(inline) => inline.url_only_is_remote,
                    None => {
                        is_cursor_mcp_file(path)
                            || is_kiro_power_mcp_file(path, config)
                            || is_skill_mcp_file(path, config)
                            || is_codex_plugin_mcp_file(path, config)
                    }
                }
            {
                "http"
            } else {
                "stdio"
            }
        });

    // MCP-011: Invalid server type (check first, skip other type-based rules if invalid)
    if config.is_rule_enabled("MCP-011") {
        if let Some(raw_type) = raw_server_type
            && !raw_type.is_string()
        {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-011",
                    t!(
                        "rules.mcp_011.message",
                        server = name,
                        server_type = raw_type.to_string(),
                        valid = valid_server_types(path, config).join(", ")
                    ),
                )
                .with_suggestion(t!("rules.mcp_011.suggestion")),
            );
            return;
        }

        if let Some(ref server_type) = server.server_type {
            let valid_types = valid_server_types(path, config);
            if server_type == "sdk" {
                // Claude Code "skips a `"type": "sdk"` entry": only an Agent
                // SDK host can register such a server.
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-011",
                        t!("rules.mcp_011.sdk", server = name),
                    )
                    .with_suggestion(t!("rules.mcp_011.suggestion")),
                );
                return;
            }
            if !valid_types.contains(&server_type.as_str()) {
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-011",
                    t!(
                        "rules.mcp_011.message",
                        server = name,
                        server_type = server_type.as_str(),
                        valid = valid_types.join(", ")
                    ),
                )
                .with_suggestion(t!("rules.mcp_011.suggestion"));

                // Unsafe auto-fix: replace with closest valid server type
                if let Some(closest) = super::find_closest_value(server_type.as_str(), &valid_types)
                {
                    if let Some((start, end)) =
                        find_unique_json_string_value_span(content, "type", server_type)
                    {
                        diagnostic = diagnostic.with_fix(Fix::replace(
                            start,
                            end,
                            closest,
                            t!("rules.mcp_011.fix", fixed = closest),
                            false,
                        ));
                    }
                }

                diagnostics.push(diagnostic);
                // Skip further type-based validation since type is invalid
                return;
            }
        }
    }

    // MCP-009: Missing command for stdio server
    // Treat null and empty string/array as missing (not a usable command).
    // An entry with nothing in it is MCP-024's finding; MCP-009 used to
    // report the same entry a second time.
    let empty_entry =
        config.is_rule_enabled("MCP-024") && is_empty_server_entry(server, raw_server);
    if config.is_rule_enabled("MCP-009") && effective_type == "stdio" && !empty_entry {
        if !has_usable_command(server) {
            // Without a `type` the entry is stdio by default; say so, and for
            // a `url` entry give the client's own advice ("add "type":
            // "http" (or "sse" / "ws")") instead of asking for a command.
            let (message, suggestion) = match (raw_server_type.is_none(), server.url.is_some()) {
                (true, true) => (
                    t!("rules.mcp_009.url_without_type", server = name),
                    t!("rules.mcp_009.url_without_type_suggestion"),
                ),
                (true, false) => (
                    t!("rules.mcp_009.no_type", server = name),
                    t!("rules.mcp_009.suggestion"),
                ),
                _ => (
                    t!("rules.mcp_009.message", server = name),
                    t!("rules.mcp_009.suggestion"),
                ),
            };
            diagnostics.push(
                Diagnostic::error(path.to_path_buf(), line, col, "MCP-009", message)
                    .with_suggestion(suggestion),
            );
        }
    }

    // MCP-022: args must be an array of strings when present.
    if config.is_rule_enabled("MCP-022")
        && let Some(args) = &server.args
    {
        let valid_args = args
            .as_array()
            .is_some_and(|arr| arr.iter().all(|item| item.is_string()));
        if !valid_args {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-022",
                    format!(
                        "Server '{}' has invalid 'args' value: expected array of strings",
                        name
                    ),
                )
                .with_suggestion("Set args to an array of strings, e.g. [\"--port\", \"3000\"]"),
            );
        }
    }

    // MCP-010: Missing url for http/sse server
    // Treat empty/whitespace-only URL as missing
    if config.is_rule_enabled("MCP-010") && ["http", "sse", "ws"].contains(&effective_type) {
        let has_url = server.url.as_deref().is_some_and(|u| !u.trim().is_empty());
        if !has_url {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-010",
                    t!(
                        "rules.mcp_010.message",
                        server = name,
                        server_type = effective_type
                    ),
                )
                .with_suggestion(t!("rules.mcp_010.suggestion")),
            );
        }
    }

    if config.is_rule_enabled("MCP-017")
        && effective_type == "http"
        && let Some(url) = server.url.as_deref()
    {
        let lower_url = url.trim().to_ascii_lowercase();
        if lower_url.starts_with("http://")
            && let Some(host) = extract_http_host(url)
            && !is_local_http_host(&host)
        {
            // A warning: the security guidance says clients "SHOULD require
            // HTTPS for all OAuth-related URLs"; no client rejects the entry.
            let mut diagnostic = Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-017",
                format!(
                    "Server '{}' uses insecure HTTP URL '{}'; use HTTPS for non-localhost endpoints",
                    name, url
                ),
            )
            .with_suggestion("Change the server URL to https:// for remote endpoints");

            // Replace http:// with https:// in the URL (case-insensitive)
            if let Some((start, end)) =
                crate::rules::find_unique_json_string_value_span(content, "url", url)
            {
                let fixed_url = format!("https://{}", &url[7..]);
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    fixed_url,
                    "Replace http:// with https://",
                    false,
                ));
            }

            diagnostics.push(diagnostic);
        }
    }

    if config.is_rule_enabled("MCP-021")
        && effective_type == "http"
        && let Some(url) = server.url.as_deref()
        && let Some(host) = extract_http_host(url)
        && is_wildcard_http_host(&host)
    {
        let mut diagnostic = Diagnostic::warning(
            path.to_path_buf(),
            line,
            col,
            "MCP-021",
            format!(
                "Server '{}' binds HTTP to '{}', which exposes all interfaces",
                name, host
            ),
        )
        .with_suggestion("Prefer localhost bindings unless remote network access is required");

        // Replace 0.0.0.0 with localhost in the URL
        if let Some((start, end)) =
            crate::rules::find_unique_json_string_value_span(content, "url", url)
        {
            let fixed_url = url.replace("0.0.0.0", "localhost");
            if fixed_url != url {
                diagnostic = diagnostic.with_fix(Fix::replace(
                    start,
                    end,
                    fixed_url,
                    "Replace 0.0.0.0 with localhost",
                    false,
                ));
            }
        }

        diagnostics.push(diagnostic);
    }

    // MCP-018, MCP-019, MCP-027, MCP-028, MCP-029: shared with the other
    // clients' config files.
    if let Some(raw_server) = raw_server {
        let view = ServerView::from_json(name, raw_server, Some(effective_type == "stdio"));
        check_server(&view, path, locator, config, diagnostics);
    }

    // MCP-012: Deprecated SSE transport
    if config.is_rule_enabled("MCP-012") && effective_type == "sse" {
        // A warning: HTTP+SSE is deprecated ("new implementations SHOULD NOT
        // adopt it") but Claude Code, Roo Code, Cline and Gemini still load it.
        let mut diag = Diagnostic::warning(
            path.to_path_buf(),
            line,
            col,
            "MCP-012",
            t!("rules.mcp_012.message", server = name),
        )
        .with_suggestion(t!("rules.mcp_012.suggestion"));

        // Unsafe auto-fix: change "sse" to "http"
        if let Some((start, end)) = find_unique_json_string_value_span(content, "type", "sse") {
            diag = diag.with_fix(Fix::replace(
                start,
                end,
                "http",
                t!("rules.mcp_012.fix"),
                false,
            ));
        }

        diagnostics.push(diag);
    }

    if config.is_rule_enabled("MCP-024") && is_empty_server_entry(server, raw_server) {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                line,
                col,
                "MCP-024",
                format!("Server '{}' has an empty configuration object", name),
            )
            .with_suggestion(
                "Define at least one meaningful field such as type, command, url, args, or env",
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use std::path::PathBuf;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = McpValidator;
        let path = PathBuf::from("test.mcp.json");
        validator.validate(&path, content, &LintConfig::default())
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = McpValidator;
        let path = PathBuf::from("test.mcp.json");
        validator.validate(&path, content, config)
    }

    fn validate_path(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = McpValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn rules_of<'a>(diagnostics: &'a [Diagnostic], rule: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == rule).collect()
    }

    // ===== 0.59 vendor survey: MCP-011 per client =====

    /// MCP-011 used to accept only stdio, http and sse for every file. The
    /// set is client-defined: Claude Code accepts `streamable-http` and `ws`,
    /// Copilot accepts `local`.
    #[test]
    fn test_mcp_011_accepts_each_readers_documented_types() {
        for server_type in ["streamable-http", "ws", "local", "stdio", "http", "sse"] {
            let content = format!(
                r#"{{"mcpServers": {{"s": {{"type": "{server_type}", "command": "node", "url": "https://mcp.example.com/x"}}}}}}"#
            );
            let diagnostics = validate_path(".mcp.json", &content);
            assert!(
                rules_of(&diagnostics, "MCP-011").is_empty(),
                "{server_type}: {diagnostics:?}"
            );
        }
        // Cursor documents no `local` or `ws`.
        let content = r#"{"mcpServers": {"s": {"type": "local", "command": "node"}}}"#;
        let diagnostics = validate_path(".cursor/mcp.json", content);
        let mcp_011 = rules_of(&diagnostics, "MCP-011");
        assert_eq!(mcp_011.len(), 1);
        assert!(
            mcp_011[0].message.contains("stdio"),
            "{}",
            mcp_011[0].message
        );
    }

    /// `streamable-http` is documented by Claude Code for `.mcp.json` (also
    /// in plugins), by Copilot CLI, Codex and Cursor for the Agent Plugins
    /// root `mcp.json`, and listed by Cursor as a transport. No error on those
    /// files, scoped to the client or not. Copilot's own files (VS Code
    /// `.vscode/mcp.json`, `.github/mcp.json`, Copilot CLI's
    /// `mcp-config.json`) document stdio/local/http/sse only.
    #[test]
    fn test_mcp_011_streamable_http_where_clients_document_it() {
        let server = r#"{"type": "streamable-http", "url": "https://mcp.example.com/mcp"}"#;
        let portable = format!(r#"{{"mcpServers": {{"s": {server}}}}}"#);
        let agent_plugins = format!(
            r#"{{"$schema": "https://agent-plugins.org/schemas/1.0.0/mcp.schema.json", "mcpServers": {{"s": {server}}}}}"#
        );
        let mcp_011 = |path: &str, content: &str, tools: &[&str]| {
            let mut config = LintConfig::default();
            config.set_tools(tools.iter().map(|t| t.to_string()).collect());
            let diagnostics = McpValidator.validate(Path::new(path), content, &config);
            rules_of(&diagnostics, "MCP-011").len()
        };
        for (path, content, tools) in [
            (".mcp.json", portable.as_str(), &[][..]),
            (".mcp.json", portable.as_str(), &["claude-code"][..]),
            (
                "my-plugin/.mcp.json",
                portable.as_str(),
                &["claude-code"][..],
            ),
            ("my-plugin/mcp.json", agent_plugins.as_str(), &[][..]),
            (
                "my-plugin/mcp.json",
                agent_plugins.as_str(),
                &["github-copilot"][..],
            ),
            (".cursor/mcp.json", portable.as_str(), &["cursor"][..]),
        ] {
            assert_eq!(mcp_011(path, content, tools), 0, "{path} {tools:?}");
        }
        let vscode = format!(r#"{{"servers": {{"s": {server}}}}}"#);
        assert_eq!(mcp_011(".vscode/mcp.json", &vscode, &[]), 1);
    }

    #[test]
    fn test_mcp_011_narrows_to_selected_tools() {
        let content = r#"{"mcpServers": {"s": {"type": "local", "command": "node"}}}"#;
        let mut claude = LintConfig::default();
        claude.set_tools(vec!["claude-code".to_string()]);
        let diagnostics = McpValidator.validate(Path::new(".mcp.json"), content, &claude);
        assert_eq!(
            rules_of(&diagnostics, "MCP-011").len(),
            1,
            "{diagnostics:?}"
        );
        let mut copilot = LintConfig::default();
        copilot.set_tools(vec!["github-copilot".to_string()]);
        let diagnostics = McpValidator.validate(Path::new(".mcp.json"), content, &copilot);
        assert!(
            rules_of(&diagnostics, "MCP-011").is_empty(),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_mcp_011_sdk_is_a_warning() {
        let content = r#"{"mcpServers": {"s": {"type": "sdk"}}}"#;
        let diagnostics = validate_path(".mcp.json", content);
        let mcp_011 = rules_of(&diagnostics, "MCP-011");
        assert_eq!(mcp_011.len(), 1);
        assert_eq!(
            mcp_011[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
        assert!(mcp_011[0].fixes.is_empty());
    }

    #[test]
    fn test_streamable_http_and_ws_need_a_url_and_get_https_checks() {
        let content = r#"{"mcpServers": {"a": {"type": "ws"}, "b": {"type": "streamable-http", "url": "http://mcp.example.com/mcp"}}}"#;
        let diagnostics = validate_path(".mcp.json", content);
        assert_eq!(
            rules_of(&diagnostics, "MCP-010").len(),
            1,
            "{diagnostics:?}"
        );
        let mcp_017 = rules_of(&diagnostics, "MCP-017");
        assert_eq!(mcp_017.len(), 1);
        assert_eq!(
            mcp_017[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_vscode_servers_map_and_comments() {
        let content = r#"{
  // VS Code reads this file as JSON with comments.
  "inputs": [],
  "servers": {
    "gh": { "type": "stdio", "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"] },
  }
}"#;
        let diagnostics = validate_path(".vscode/mcp.json", content);
        assert!(
            rules_of(&diagnostics, "MCP-007").is_empty(),
            "{diagnostics:?}"
        );
        let mcp_027 = rules_of(&diagnostics, "MCP-027");
        assert_eq!(mcp_027.len(), 1, "{diagnostics:?}");
        assert_eq!(mcp_027[0].line, 5);
        // Comments are still a parse error in other MCP files.
        assert_eq!(
            rules_of(&validate_path(".mcp.json", content), "MCP-007").len(),
            1
        );
    }

    /// An empty entry (`{}`, or a blank command) got both MCP-009 and MCP-024;
    /// MCP-024 reports it alone, and MCP-009 still reports a stdio entry
    /// that has other fields but no command.
    #[test]
    fn test_empty_server_reported_once_by_mcp_024() {
        let content = r#"{"mcpServers": {"none": {}, "blank": {"command": ""}, "argsonly": {"args": ["x"]}, "headersonly": {"headers": {"X-Trace": "on"}}}}"#;
        let diagnostics = validate_path(".mcp.json", content);
        let mcp_024 = rules_of(&diagnostics, "MCP-024");
        assert_eq!(mcp_024.len(), 2, "{diagnostics:?}");
        let mut mcp_009: Vec<_> = rules_of(&diagnostics, "MCP-009")
            .iter()
            .map(|d| d.message.clone())
            .collect();
        mcp_009.sort();
        // An entry with only headers is not empty: MCP-009 says what it lacks.
        assert_eq!(mcp_009.len(), 2, "{diagnostics:?}");
        assert!(mcp_009[0].contains("'argsonly'") && mcp_009[1].contains("'headersonly'"));
        // With MCP-024 disabled, MCP-009 reports the empty entries again.
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["MCP-024".to_string()];
        let diagnostics = McpValidator.validate(Path::new(".mcp.json"), content, &config);
        assert_eq!(
            rules_of(&diagnostics, "MCP-009").len(),
            4,
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_servers_map_of_non_server_records_is_not_read_as_servers() {
        // A tool cache keyed by server name (pi's mcp-cache.json) is not a
        // VS Code server map: no entry has `command` or `url`.
        let content = r#"{"version": 1, "servers": {"docs": {"configHash": "abc", "tools": [], "cachedAt": 1}}}"#;
        let diagnostics = validate_path("mcp-cache.json", content);
        for rule in ["MCP-009", "MCP-024"] {
            assert!(rules_of(&diagnostics, rule).is_empty(), "{diagnostics:?}");
        }
    }

    #[test]
    fn test_codex_plugin_mcp_json_url_only_entry_is_remote() {
        // Codex infers the transport, so a url-only entry in a Codex plugin's
        // `.mcp.json` is a remote server, not a stdio server missing a
        // command. The same file in a Claude Code plugin still needs `type`.
        let content = r#"{"mcpServers": {"notes": {"url": "https://example.com/mcp"}}}"#;
        let fs = crate::fs::MockFileSystem::new();
        fs.add_file("/p/notes/.codex-plugin/plugin.json", "{}");
        fs.add_file("/p/notes/.mcp.json", content);
        fs.add_file("/p/both/.codex-plugin/plugin.json", "{}");
        fs.add_file("/p/both/.claude-plugin/plugin.json", "{}");
        fs.add_file("/p/both/.mcp.json", content);
        let mut config = LintConfig::default();
        config.set_fs(std::sync::Arc::new(fs));
        let codex = McpValidator.validate(Path::new("/p/notes/.mcp.json"), content, &config);
        assert!(rules_of(&codex, "MCP-009").is_empty(), "{codex:?}");
        let both = McpValidator.validate(Path::new("/p/both/.mcp.json"), content, &config);
        assert_eq!(rules_of(&both, "MCP-009").len(), 1, "{both:?}");
    }

    #[test]
    fn test_amp_skill_mcp_json_url_only_entry_is_remote() {
        let fs = crate::fs::MockFileSystem::new();
        fs.add_file("/s/my-skill/SKILL.md", "---\nname: my-skill\n---\n");
        let mut config = LintConfig::default();
        config.set_fs(std::sync::Arc::new(fs));
        let content =
            r#"{"linear": {"url": "https://mcp.linear.app/sse", "includeTools": ["list_issues"]}}"#;
        let diagnostics =
            McpValidator.validate(Path::new("/s/my-skill/mcp.json"), content, &config);
        assert!(
            rules_of(&diagnostics, "MCP-009").is_empty(),
            "{diagnostics:?}"
        );
    }

    // ===== MCP-027 / MCP-028 / MCP-029 through McpValidator =====

    // ===== Inline servers in plugin manifests =====

    fn apply_first_fix(content: &str, diagnostic: &Diagnostic) -> String {
        let fix = diagnostic.fixes.first().expect("a fix");
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        fixed
    }

    #[test]
    fn test_inline_servers_in_claude_plugin_manifest() {
        // A keyword and the top-level `name` key share text with server names;
        // diagnostics must point into the server entries.
        let content = r#"{
  "name": "db-tools",
  "keywords": ["db", "gw"],
  "mcpServers": {
    "db": {
      "command": "npx",
      "args": ["-y", "@example/db-server"],
      "env": { "DB_PASSWORD": "hunter2hunter2hunter2" }
    },
    "gw": {
      "type": "http",
      "url": "http://gateway.internal.test/mcp",
      "headers": { "Authorization": "Bearer abcdefghijklmnopqrstuvwxyz" }
    },
    "name": { "url": "https://mcp.example.com/mcp" },
    "bad": { "type": "STDIO", "command": "node" }
  }
}"#;
        let diagnostics = validate_path("p/.claude-plugin/plugin.json", content);
        let lines = |rule: &str| -> Vec<usize> {
            rules_of(&diagnostics, rule)
                .iter()
                .map(|d| d.line)
                .collect()
        };
        assert_eq!(lines("MCP-027"), vec![7], "{diagnostics:?}");
        assert_eq!(lines("MCP-018"), vec![8], "{diagnostics:?}");
        assert_eq!(lines("MCP-017"), vec![10], "{diagnostics:?}");
        assert_eq!(lines("MCP-028"), vec![13], "{diagnostics:?}");
        // Claude Code reads an entry without `type` as stdio.
        assert_eq!(lines("MCP-009"), vec![15], "{diagnostics:?}");
        assert_eq!(lines("MCP-011"), vec![16], "{diagnostics:?}");
        // Only server-level rules run on a manifest.
        assert!(
            diagnostics.iter().all(|d| [
                "MCP-009", "MCP-011", "MCP-017", "MCP-018", "MCP-027", "MCP-028"
            ]
            .contains(&d.rule.as_str())),
            "{diagnostics:?}"
        );

        // The MCP-011 fix edits the server's own `type` and keeps the file
        // valid JSON.
        let mcp_011 = rules_of(&diagnostics, "MCP-011")[0];
        assert!(
            mcp_011.message.contains("streamable-http"),
            "{}",
            mcp_011.message
        );
        let fixed = apply_first_fix(content, mcp_011);
        let value: serde_json::Value = serde_json::from_str(&fixed).unwrap();
        assert_eq!(value["mcpServers"]["bad"]["type"], "stdio");
    }

    #[test]
    fn test_inline_servers_in_manifest_array_and_duplicates() {
        let content = r#"{
  "name": "p",
  "mcpServers": [
    "./mcp/servers.json",
    {
      "local": { "command": "node", "env": { "API_TOKEN": "tok-1234567890abcdef" } },
      "local": { "command": "node" }
    },
    { "remote": { "type": "sse", "url": "https://mcp.example.com/sse" } }
  ]
}"#;
        let diagnostics = validate_path(".claude-plugin/plugin.json", content);
        // The last `local` wins, so its env is not the one with the secret.
        assert!(
            rules_of(&diagnostics, "MCP-018").is_empty(),
            "{diagnostics:?}"
        );
        let mcp_023 = rules_of(&diagnostics, "MCP-023");
        assert_eq!(mcp_023.len(), 1, "{diagnostics:?}");
        assert_eq!(mcp_023[0].line, 7);
        let mcp_012 = rules_of(&diagnostics, "MCP-012");
        assert_eq!(mcp_012.len(), 1, "{diagnostics:?}");
        assert_eq!(mcp_012[0].line, 9);
    }

    #[test]
    fn test_inline_servers_skip_paths_and_agent_plugins_manifests() {
        for (path, content) in [
            (
                ".claude-plugin/plugin.json",
                r#"{"name":"p","mcpServers":"./.mcp.json"}"#,
            ),
            (
                ".claude-plugin/plugin.json",
                r#"{"name":"p","mcpServers":["./a.json","./b.mcpb"]}"#,
            ),
            // Agent Plugins has no `mcpServers` manifest field.
            (
                "plugin.json",
                r#"{"$schema":"https://agent-plugins.org/schemas/1.0.0/plugin.schema.json","name":"p","mcpServers":{"s":{}}}"#,
            ),
            (
                ".cursor-plugin/marketplace.json",
                r#"{"name":"m","mcpServers":{"s":{}}}"#,
            ),
            // Manifest parse errors belong to the manifest validators.
            (
                ".claude-plugin/plugin.json",
                r#"{"name":"p","mcpServers":{"#,
            ),
        ] {
            let diagnostics = validate_path(path, content);
            assert!(diagnostics.is_empty(), "{path} {content}: {diagnostics:?}");
        }
    }

    #[test]
    fn test_inline_servers_in_copilot_and_cursor_manifests() {
        // Copilot: `local` is a documented type, `ws` is not; the `.mcp.json`
        // wrapper inside the inline object is read through.
        let content = r#"{"name":"p","mcpServers":{"mcpServers":{"a":{"type":"local","command":"node"},"b":{"type":"ws","url":"wss://mcp.example.com"}}}}"#;
        for path in [
            ".github/plugin/plugin.json",
            ".plugin/plugin.json",
            "plugin.json",
        ] {
            let diagnostics = validate_path(path, content);
            let mcp_011 = rules_of(&diagnostics, "MCP-011");
            assert_eq!(mcp_011.len(), 1, "{path}: {diagnostics:?}");
            assert!(mcp_011[0].message.contains("'b'"));
        }
        // Cursor infers the transport: a url-only entry is remote.
        let diagnostics = validate_path(
            ".cursor-plugin/plugin.json",
            r#"{"name":"p","mcpServers":{"api":{"url":"https://mcp.example.com/mcp","headers":{"Authorization":"Bearer ${API_TOKEN}"}},"local":{"command":"npx","args":["-y","pkg@latest"]}}}"#,
        );
        assert!(
            rules_of(&diagnostics, "MCP-009").is_empty(),
            "{diagnostics:?}"
        );
        assert_eq!(
            rules_of(&diagnostics, "MCP-027").len(),
            1,
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_mcp_027_028_029_in_mcp_json() {
        let content = r#"{
  "mcpServers": {
    "fs": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/Users/alice"] },
    "remote": { "type": "http", "url": "https://mcp.example.com/mcp", "headers": { "Authorization": "Bearer abcdefghijklmnopqrstuvwxyz" } },
    "safe": { "type": "http", "url": "https://mcp.example.com/mcp", "headers": { "Authorization": "Bearer ${API_TOKEN}" } },
    "pinned": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-github@2025.4.8"] }
  }
}"#;
        let diagnostics = validate_path(".mcp.json", content);
        let mcp_027 = rules_of(&diagnostics, "MCP-027");
        assert_eq!(mcp_027.len(), 1, "{diagnostics:?}");
        assert!(mcp_027[0].message.contains("server-filesystem"));
        let mcp_028 = rules_of(&diagnostics, "MCP-028");
        assert_eq!(mcp_028.len(), 1, "{diagnostics:?}");
        assert_eq!(mcp_028[0].line, 4);
        assert!(mcp_028[0].message.contains("Authorization"));
        let mcp_029 = rules_of(&diagnostics, "MCP-029");
        assert_eq!(mcp_029.len(), 1);
        assert!(mcp_029[0].message.contains("/Users/alice"));
    }

    #[test]
    fn test_mcp_018_skips_references_and_placeholders() {
        let content = r#"{"mcpServers": {"s": {"command": "node", "env": {"API_KEY": "${API_KEY}", "TOKEN": "your-token-here", "SECRET": "GITHUB_SECRET", "PASSWORD": "literal-password-1"}}}}"#;
        let diagnostics = validate(content);
        let mcp_018 = rules_of(&diagnostics, "MCP-018");
        assert_eq!(mcp_018.len(), 1, "{diagnostics:?}");
        assert!(mcp_018[0].message.contains("PASSWORD"));
    }

    // ===== MCP-030..033 =====

    #[test]
    fn test_mcp_030_duplicate_tool_names() {
        let content = r#"{"tools": [
  {"name": "search", "description": "Search the docs index", "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}}},
  {"name": "search", "description": "Search the docs index", "inputSchema": {"type": "object", "properties": {"q": {"type": "string"}}}}
]}"#;
        let diagnostics = validate(content);
        let mcp_030 = rules_of(&diagnostics, "MCP-030");
        assert_eq!(mcp_030.len(), 1);
        assert_eq!(mcp_030[0].line, 3);
    }

    #[test]
    fn test_mcp_031_x_mcp_header_constraints() {
        let content = r#"{"tools": [{
  "name": "execute_sql",
  "description": "Execute SQL on a database",
  "inputSchema": {
    "type": "object",
    "properties": {
      "region": {"type": "string", "x-mcp-header": "Region"},
      "zone": {"type": "string", "x-mcp-header": "region"},
      "limit": {"type": "number", "x-mcp-header": "Limit"},
      "bad": {"type": "string", "x-mcp-header": "Bad Name"},
      "list": {"type": "array", "items": {"type": "string", "x-mcp-header": "Item"}},
      "api_token": {"type": "string", "x-mcp-header": "Token"}
    }
  }
}]}"#;
        let diagnostics = validate(content);
        let errors: Vec<_> = rules_of(&diagnostics, "MCP-031")
            .into_iter()
            .filter(|d| d.level == crate::diagnostics::DiagnosticLevel::Error)
            .collect();
        let messages: Vec<&str> = errors.iter().map(|d| d.message.as_str()).collect();
        assert_eq!(errors.len(), 4, "{messages:#?}");
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'zone'") && m.contains("repeats"))
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'limit'") && m.contains("'number'"))
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'bad'") && m.contains("token"))
        );
        assert!(
            messages
                .iter()
                .any(|m| m.contains("'list'") && m.contains("reachable"))
        );
        let sensitive: Vec<_> = rules_of(&diagnostics, "MCP-031")
            .into_iter()
            .filter(|d| d.level == crate::diagnostics::DiagnosticLevel::Info)
            .collect();
        assert_eq!(sensitive.len(), 1);
        assert!(sensitive[0].message.contains("api_token"));

        // A project pinned to 2025-11-25 predates x-mcp-header.
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(Some("2025-11-25".to_string()));
        assert!(rules_of(&validate_with_config(content, &config), "MCP-031").is_empty());
    }

    #[test]
    fn test_mcp_032_deprecated_capabilities() {
        let content = r#"{"jsonrpc": "2.0", "id": 1, "result": {"capabilities": {"logging": {}, "tools": {}}}}"#;
        let diagnostics = validate(content);
        let mcp_032 = rules_of(&diagnostics, "MCP-032");
        assert_eq!(mcp_032.len(), 1, "{diagnostics:?}");
        assert!(mcp_032[0].message.contains("logging"));
        assert_eq!(mcp_032[0].level, crate::diagnostics::DiagnosticLevel::Info);
        let sampling = r#"{"jsonrpc": "2.0", "id": 2, "method": "sampling/createMessage", "params": {"includeContext": "allServers", "messages": []}}"#;
        assert_eq!(rules_of(&validate(sampling), "MCP-032").len(), 1);
    }

    #[test]
    fn test_mcp_033_open_no_parameter_schema() {
        let open = r#"{"name": "ping", "description": "Check the server is up", "inputSchema": {"type": "object"}}"#;
        let diagnostics = validate(open);
        assert_eq!(rules_of(&diagnostics, "MCP-033").len(), 1);
        let closed = r#"{"name": "ping", "description": "Check the server is up", "inputSchema": {"type": "object", "additionalProperties": false}}"#;
        assert!(rules_of(&validate(closed), "MCP-033").is_empty());
    }

    #[test]
    fn test_find_json_key_position_ignores_string_literals() {
        let content = r#"{
            "note": "mention \"tools\" and \"mcpServers\" here only",
            "tools": []
        }"#;

        let key_pos = find_json_key_position(content, "tools");
        assert!(key_pos.is_some(), "Expected to find tools key position");
        let pos = key_pos.unwrap();
        assert_eq!(&content[pos..pos + "\"tools\"".len()], "\"tools\"");
    }

    #[test]
    fn test_collect_tools_array_spans_with_embedded_tools_string() {
        let content = r#"{
            "note": "text with escaped token: \"tools\" and fake array marker [ ]",
            "tools": [
                {"name": "one", "description": "desc", "inputSchema": {"type": "object"}}
            ]
        }"#;

        let spans = collect_tools_array_object_spans(content);
        assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_mcp_servers_validation_not_skipped_when_resources_are_malformed() {
        let content = r#"{
            "mcpServers": {
                "broken-stdio": {
                    "type": "stdio"
                }
            },
            "resources": [
                {"uri": 123, "name": "resource-name"}
            ]
        }"#;

        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "MCP-009 should still run even when resources has wrong field types"
        );
    }

    // MCP-001 Tests
    #[test]
    fn test_mcp_001_valid_jsonrpc_version() {
        let content = r#"{"jsonrpc": "2.0", "method": "test"}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    #[test]
    fn test_mcp_001_invalid_jsonrpc_version() {
        let content = r#"{"jsonrpc": "1.0", "method": "test"}"#;
        let diagnostics = validate(content);
        let mcp_001 = diagnostics
            .iter()
            .find(|d| d.rule == "MCP-001")
            .expect("MCP-001 should be reported");
        assert!(mcp_001.message.contains("Invalid JSON-RPC version"));
        assert!(mcp_001.has_fixes());
        let fix = &mcp_001.fixes[0];
        assert_eq!(fix.replacement, "\"2.0\"");
        assert!(fix.safe);
    }

    #[test]
    fn test_mcp_001_jsonrpc_not_string() {
        let content = r#"{"jsonrpc": 2.0, "method": "test"}"#;
        let diagnostics = validate(content);
        let mcp_001 = diagnostics
            .iter()
            .find(|d| d.rule == "MCP-001")
            .expect("MCP-001 should be reported");
        assert!(mcp_001.message.contains("must be a string"));
        assert!(mcp_001.has_fixes());
        let fix = &mcp_001.fixes[0];
        assert_eq!(fix.replacement, "\"2.0\"");
        assert!(fix.safe);
    }

    #[test]
    fn test_mcp_001_missing_jsonrpc_no_error() {
        // Missing jsonrpc is OK for tool definitions (only required for JSON-RPC messages)
        let content = r#"{"name": "test-tool", "description": "A test tool", "inputSchema": {"type": "object"}}"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    // MCP-002 Tests
    #[test]
    fn test_mcp_002_all_fields_present() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    #[test]
    fn test_mcp_002_missing_name() {
        let content = r#"{
            "description": "A test tool",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("Tool #1"));
        assert!(mcp_002[0].message.contains("'name'"));
    }

    #[test]
    fn test_optional_mcp_description_is_advisory() {
        for description in [None, Some(""), Some("short")] {
            let mut tool =
                serde_json::json!({"name": "test-tool", "inputSchema": {"type": "object"}});
            if let Some(description) = description {
                tool["description"] = description.into();
            }
            let diagnostics = validate(&tool.to_string());
            assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
            let advice: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-004").collect();
            assert_eq!(advice.len(), 1);
            assert_eq!(advice[0].level, crate::diagnostics::DiagnosticLevel::Info);
        }
    }

    #[test]
    fn test_mcp_002_missing_input_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing"
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("'inputSchema'"));
    }

    #[test]
    fn test_mcp_002_empty_name() {
        let content = r#"{
            "name": "",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("'name'"));
    }

    #[test]
    fn test_mcp_002_all_fields_missing() {
        let content = r#"{}"#;
        let diagnostics = validate(content);
        // Empty object won't be detected as a tool, so no MCP-002 errors
        // Tools are detected by having 'name' OR 'inputSchema' at root
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    // MCP-003 Tests
    #[test]
    fn test_mcp_003_valid_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_003_invalid_type_value() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "invalid_type"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("Invalid JSON Schema type"))
        );
    }

    #[test]
    fn test_mcp_003_schema_not_object() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": "not an object"
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("must be an object"))
        );
    }

    #[test]
    fn test_mcp_003_properties_not_object() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object", "properties": "not an object"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_003_required_not_array() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object", "required": "not an array"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_003_validates_output_schema() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "outputSchema": {"type": "not_a_real_type"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-014"));
    }

    // MCP-004 Tests
    #[test]
    fn test_mcp_004_meaningful_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "This is a meaningful description of the tool",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-004"));
    }

    #[test]
    fn test_mcp_004_short_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "Short",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-004"));
        assert!(diagnostics.iter().any(|d| d.message.contains("too short")));
    }

    #[test]
    fn test_mcp_004_empty_description() {
        let content = r#"{
            "name": "test-tool",
            "description": "",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
        assert!(
            diagnostics.iter().any(
                |d| d.rule == "MCP-004" && d.level == crate::diagnostics::DiagnosticLevel::Info
            )
        );
    }

    #[test]
    fn test_mcp_005_retired_consent_is_a_client_responsibility() {
        for extra in [
            "",
            ", \"requiresApproval\": false",
            ", \"confirmation\": \"\"",
        ] {
            let content = format!(
                r#"{{"name":"test", "description":"A testing tool", "inputSchema":{{"type":"object"}}{extra}}}"#
            );
            assert!(!validate(&content).iter().any(|d| d.rule == "MCP-005"));
        }
        assert!(!RULE_IDS.contains(&"MCP-005"));
    }

    // MCP-006 Tests
    #[test]
    fn test_mcp_006_no_annotations() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-006"));
    }

    #[test]
    fn test_mcp_006_unknown_key_is_info() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {"untrusted": "data"}
        }"#;
        let diagnostics = validate(content);
        let mcp_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-006").collect();
        assert_eq!(mcp_006.len(), 1);
        assert_eq!(mcp_006[0].level, crate::diagnostics::DiagnosticLevel::Info);
        assert!(mcp_006[0].message.contains("untrusted"));
    }

    #[test]
    fn test_mcp_006_empty_annotations() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {}
        }"#;
        let diagnostics = validate(content);
        // Empty annotations don't trigger warning
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-006"));
    }

    #[test]
    fn test_mcp_006_unknown_annotation_keys_sorted() {
        // Keys come from a HashMap; the message must not depend on its order.
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool for testing",
            "inputSchema": {"type": "object"},
            "annotations": {"zeta": 1, "dangerous": true, "readOnlyHint": true, "alpha": 2, "mid": 3}
        }"#;
        let first = validate(content);
        let messages: Vec<_> = first
            .iter()
            .filter(|d| d.rule == "MCP-006")
            .map(|d| d.message.clone())
            .collect();
        assert_eq!(messages.len(), 1);
        assert!(
            messages[0].ends_with("alpha, dangerous, mid, zeta"),
            "{messages:?}"
        );
        for _ in 0..20 {
            let again: Vec<_> = validate(content)
                .into_iter()
                .filter(|d| d.rule == "MCP-006")
                .map(|d| d.message)
                .collect();
            assert_eq!(again, messages);
        }
        assert!(!first.iter().any(|d| d.rule == "MCP-003"));
    }

    // Config wiring tests
    #[test]
    fn test_config_disabled_mcp_category() {
        let mut config = LintConfig::default();
        config.rules_mut().mcp = false;

        let content = r#"{"jsonrpc": "1.0"}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["MCP-001".to_string()];

        let content = r#"{"jsonrpc": "1.0"}"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-001"));
    }

    // Parse error test
    #[test]
    fn test_parse_error_handling() {
        let content = r#"not valid json"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    // Multiple tools test
    #[test]
    fn test_multiple_tools_validation() {
        let content = r#"{
            "tools": [
                {"name": "tool1", "description": "First tool description", "inputSchema": {"type": "object"}},
                {"name": "", "description": "Second tool", "inputSchema": {"type": "object"}}
            ]
        }"#;
        let diagnostics = validate(content);
        // First tool is valid, second tool has empty name
        let mcp_002 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-002")
            .collect::<Vec<_>>();
        assert_eq!(mcp_002.len(), 1);
        assert!(mcp_002[0].message.contains("Tool #2"));
    }

    // Tools array at root level
    #[test]
    fn test_tools_array_format() {
        let content = r#"{
            "tools": [
                {"name": "tool1", "description": "A tool for testing purposes", "inputSchema": {"type": "object"}, "requiresApproval": true}
            ]
        }"#;
        let diagnostics = validate(content);
        let errors = diagnostics
            .iter()
            .filter(|d| d.level == crate::diagnostics::DiagnosticLevel::Error)
            .collect::<Vec<_>>();
        assert!(errors.is_empty());
    }

    // MCP server config format (should not trigger tool validation)
    #[test]
    fn test_mcp_server_config_format() {
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        // Server config doesn't have tools, no tool validation errors
        assert!(diagnostics.is_empty());
    }

    // MCP-008 Tests
    #[test]
    fn test_mcp_008_initialize_request_matching_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2025-11-25",
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_request_mismatched_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {
                "protocolVersion": "2024-11-05",
                "clientInfo": {"name": "test-client", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-008")
            .collect::<Vec<_>>();
        assert_eq!(mcp_008.len(), 1);
        assert!(mcp_008[0].message.contains("Protocol version mismatch"));
        assert!(mcp_008[0].message.contains("2024-11-05"));
        assert!(mcp_008[0].message.contains("2025-11-25"));
        assert!(
            !mcp_008[0].has_fixes(),
            "Unpinned protocol mismatch should be suggestion-only"
        );
    }

    #[test]
    fn test_mcp_008_initialize_response_matching_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2025-11-25",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_response_mismatched_version() {
        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics
            .iter()
            .filter(|d| d.rule == "MCP-008")
            .collect::<Vec<_>>();
        assert_eq!(mcp_008.len(), 1);
        assert!(mcp_008[0].message.contains("Protocol version mismatch"));
    }

    #[test]
    fn test_mcp_008_custom_expected_version() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(Some("2024-11-05".to_string()));

        // This should now match
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    /// With no revision pinned, both current revisions are accepted.
    /// `2026-07-28` is the current protocol version; `2025-11-25` remains valid
    /// because the two are behaviorally incompatible rather than sequential.
    #[test]
    fn test_mcp_008_accepts_both_current_revisions_when_unpinned() {
        for version in ["2025-11-25", "2026-07-28"] {
            let content = format!(
                r#"{{
                "jsonrpc": "2.0",
                "method": "initialize",
                "id": 1,
                "params": {{"protocolVersion": "{version}"}}
            }}"#
            );
            let diagnostics = validate(&content);
            let mcp_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-008").collect();
            assert!(
                mcp_008.is_empty(),
                "revision '{version}' is current and must not be flagged when unpinned, got: {mcp_008:?}"
            );
        }
    }

    /// A revision outside the current set is still reported when unpinned, and
    /// the message names the accepted set rather than one revision.
    #[test]
    fn test_mcp_008_flags_stale_revision_when_unpinned() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate(content);
        let mcp_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-008").collect();
        assert_eq!(
            mcp_008.len(),
            1,
            "an out-of-date revision must still be flagged, got: {diagnostics:?}"
        );
        assert!(
            mcp_008[0].message.contains("2025-11-25") && mcp_008[0].message.contains("2026-07-28"),
            "unpinned message should name every accepted revision, got: {}",
            mcp_008[0].message
        );
    }

    /// Pinning one revision restores the exact check, so the other current
    /// revision is reported against the pinned value.
    #[test]
    fn test_mcp_008_pinned_revision_is_exact() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(Some("2025-11-25".to_string()));

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2026-07-28"}
        }"#;
        let diagnostics = validate_with_config(content, &config);
        let mcp_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-008").collect();
        assert_eq!(
            mcp_008.len(),
            1,
            "a pinned revision must be enforced exactly, got: {diagnostics:?}"
        );
        assert!(
            mcp_008[0].message.contains("2025-11-25") && !mcp_008[0].message.contains(" or "),
            "pinned message should name only the pinned revision, got: {}",
            mcp_008[0].message
        );
    }

    #[test]
    fn test_mcp_008_disabled_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["MCP-008".to_string()];

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate_with_config(content, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_non_initialize_message_no_error() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_initialize_without_protocol_version_no_error() {
        // Missing protocolVersion should not trigger MCP-008 (version negotiation may handle this)
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"clientInfo": {"name": "test"}}
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-008"));
    }

    #[test]
    fn test_mcp_008_warning_level() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;
        let diagnostics = validate(content);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");
        assert!(mcp_008.is_some());
        assert_eq!(
            mcp_008.unwrap().level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    // ===== Version-Aware MCP-008 Tests =====

    #[test]
    fn test_mcp_008_assumption_when_version_not_pinned() {
        // Create a config where mcp is NOT pinned
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(None);
        config.spec_revisions_mut().mcp_protocol = None;
        assert!(!config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should have an assumption note when version not pinned
        assert!(diag.assumption.is_some());
        let assumption = diag.assumption.as_ref().unwrap();
        assert!(assumption.contains("Using default MCP protocol version"));
        assert!(assumption.contains("[spec_revisions]"));
        assert!(
            !diag.has_fixes(),
            "Unpinned protocol mismatch should not emit auto-fix"
        );
    }

    #[test]
    fn test_mcp_008_no_assumption_when_version_pinned_via_spec_revisions() {
        let mut config = LintConfig::default();
        config.spec_revisions_mut().mcp_protocol = Some("2025-11-25".to_string());
        assert!(config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should NOT have an assumption note when version is pinned
        assert!(diag.assumption.is_none());
        assert!(diag.has_fixes(), "Pinned mismatch should emit auto-fix");
        assert_eq!(diag.fixes[0].replacement, "2025-11-25");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_no_assumption_when_version_pinned_via_legacy() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(Some("2025-11-25".to_string()));
        assert!(config.is_mcp_revision_pinned());

        let content = r#"{
            "jsonrpc": "2.0",
            "method": "initialize",
            "id": 1,
            "params": {"protocolVersion": "2024-11-05"}
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        let diag = mcp_008.unwrap();
        // Should NOT have an assumption note when version is pinned via legacy field
        assert!(diag.assumption.is_none());
        assert!(diag.has_fixes(), "Pinned mismatch should emit auto-fix");
        assert_eq!(diag.fixes[0].replacement, "2025-11-25");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_008_response_assumption_when_version_not_pinned() {
        let mut config = LintConfig::default();
        config.set_mcp_protocol_version(None);
        config.spec_revisions_mut().mcp_protocol = None;

        let content = r#"{
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": "test-server", "version": "1.0.0"}
            }
        }"#;

        let diagnostics = validate_with_config(content, &config);
        let mcp_008 = diagnostics.iter().find(|d| d.rule == "MCP-008");

        assert!(mcp_008.is_some());
        assert!(mcp_008.unwrap().assumption.is_some());
    }

    // ===== Additional MCP-007 Parse Error Tests =====

    #[test]
    fn test_mcp_007_invalid_json_syntax() {
        let content = r#"{ invalid json syntax }"#;

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_truncated_json() {
        let content = r#"{"jsonrpc": "2.0", "method": "test"#;

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_empty_file() {
        let content = "";

        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_valid_json_no_error() {
        let content = r#"{
            "jsonrpc": "2.0",
            "method": "tools/list",
            "id": 1
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    #[test]
    fn test_mcp_007_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["MCP-007".to_string()];

        let content = r#"{ invalid }"#;
        let validator = McpValidator;
        let diagnostics = validator.validate(Path::new("test.json"), content, &config);

        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-007"));
    }

    // ===== Additional MCP rule coverage =====

    #[test]
    fn test_mcp_002_nested_tools_array() {
        let content = r#"{
            "tools": [
                { "name": "tool1", "description": "First tool", "inputSchema": {} },
                { "name": "tool2", "description": "Second tool", "inputSchema": {} }
            ]
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-002"));
    }

    #[test]
    fn test_mcp_003_nested_schema_valid() {
        let content = r#"{
            "tools": [{
                "name": "complex-tool",
                "description": "A tool with nested schema",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "nested": {
                            "type": "object",
                            "properties": {
                                "value": { "type": "string" }
                            }
                        }
                    }
                }
            }]
        }"#;

        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-003"));
    }

    #[test]
    fn test_mcp_006_known_annotations_are_not_reported() {
        // MCP-006 used to warn whenever a tool had annotations. The spec's
        // "MUST consider tool annotations to be untrusted" is a client duty;
        // a tool that sets the standard hints is spec-conformant.
        let content = r#"{
            "tools": [{
                "name": "annotated-tool",
                "description": "A tool with annotations",
                "inputSchema": {},
                "annotations": {"title": "T", "readOnlyHint": true, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false}
            }]
        }"#;
        let diagnostics = validate(content);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-006"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_all_mcp_rules_can_be_disabled() {
        let rules = [
            "MCP-001", "MCP-002", "MCP-003", "MCP-004", "MCP-006", "MCP-007", "MCP-008", "MCP-009",
            "MCP-010", "MCP-011", "MCP-012", "MCP-013", "MCP-014", "MCP-015", "MCP-016", "MCP-017",
            "MCP-018", "MCP-019", "MCP-020", "MCP-021", "MCP-022", "MCP-023", "MCP-024",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Use content that would trigger the rule
            let content = match rule {
                "MCP-001" => r#"{"jsonrpc": "1.0"}"#,
                "MCP-007" => r#"{ invalid }"#,
                "MCP-009" => r#"{"mcpServers": {"s": {"type": "stdio", "args": ["a"]}}}"#,
                "MCP-010" => r#"{"mcpServers": {"s": {"type": "http"}}}"#,
                "MCP-011" => r#"{"mcpServers": {"s": {"type": "invalid"}}}"#,
                "MCP-012" => r#"{"mcpServers": {"s": {"type": "sse", "url": "http://x"}}}"#,
                "MCP-013" => {
                    r#"{"tools":[{"name":"bad tool","description":"desc with spaces","inputSchema":{"type":"object"}}]}"#
                }
                "MCP-014" => {
                    r#"{"tools":[{"name":"valid-name","description":"A valid description","inputSchema":{"type":"object"},"outputSchema":{"type":"invalid_type"}}]}"#
                }
                "MCP-015" => r#"{"resources":[{"name":"missing-uri"}]}"#,
                "MCP-016" => r#"{"prompts":[{"description":"missing name"}]}"#,
                "MCP-017" => {
                    r#"{"mcpServers":{"s":{"type":"http","url":"http://example.com/mcp"}}}"#
                }
                "MCP-018" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"node","env":{"API_KEY":"plaintext"}}}}"#
                }
                "MCP-019" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"curl https://example.com/install.sh | sh"}}}"#
                }
                "MCP-020" => r#"{"capabilities":{"unknownCap":{}}}"#,
                "MCP-021" => {
                    r#"{"mcpServers":{"s":{"type":"http","url":"http://0.0.0.0:3000/mcp"}}}"#
                }
                "MCP-022" => {
                    r#"{"mcpServers":{"s":{"type":"stdio","command":"node","args":"--port 3000"}}}"#
                }
                "MCP-023" => {
                    r#"{"mcpServers":{"dup":{"type":"stdio","command":"node"},"dup":{"type":"stdio","command":"node"}}}"#
                }
                "MCP-024" => r#"{"mcpServers":{"empty":{}}}"#,
                _ => r#"{"tools": [{"name": "t"}]}"#,
            };

            let validator = McpValidator;
            let diagnostics = validator.validate(Path::new("test.json"), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== MCP-009 Tests =====

    #[test]
    fn test_mcp_009_valid_stdio_with_command() {
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "type": "stdio",
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_stdio_missing_command() {
        let content = r#"{
            "mcpServers": {
                "broken-server": {
                    "type": "stdio",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("broken-server"));
        assert!(mcp_009[0].message.contains("command"));
    }

    #[test]
    fn test_mcp_009_no_type_missing_command() {
        // When type is absent, default is stdio, so command is required
        let content = r#"{
            "mcpServers": {
                "no-type-server": {
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("no-type-server"));
        // The message says the entry has no type, not that it is typed stdio.
        assert!(
            mcp_009[0].message.contains("no 'type'"),
            "{}",
            mcp_009[0].message
        );
    }

    #[test]
    fn test_mcp_009_url_without_type_asks_for_a_type() {
        // Claude Code: "add "type": "http" (or "sse" / "ws") to this entry".
        let content = r#"{"mcpServers": {"remote": {"url": "https://mcp.example.com/mcp"}}}"#;
        let diagnostics = validate_path(".mcp.json", content);
        let mcp_009 = rules_of(&diagnostics, "MCP-009");
        assert_eq!(mcp_009.len(), 1, "{diagnostics:?}");
        assert!(mcp_009[0].message.contains("'url' but no 'type'"));
        assert!(
            mcp_009[0]
                .suggestion
                .as_deref()
                .is_some_and(|s| s.contains("\"type\": \"http\""))
        );
    }

    #[test]
    fn test_mcp_009_no_type_with_command() {
        // When type is absent but command is present, no error
        let content = r#"{
            "mcpServers": {
                "default-server": {
                    "command": "python",
                    "args": ["-m", "server"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_http_server_no_command_ok() {
        // HTTP server doesn't need command
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_mcp_009_url_only_server_infers_http_transport() {
        let content = r#"{
            "mcpServers": {
                "cursor-remote": {
                    "url": "https://example.com/mcp",
                    "headers": {"Authorization": "Bearer ${env:MCP_TOKEN}"}
                }
            }
        }"#;
        let diagnostics = validate_path(".cursor/mcp.json", content);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.rule != "MCP-009" && d.rule != "MCP-010"),
            "Cursor's documented URL-only remote form must validate, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_mcp_009_cursor_server_without_url_or_command_stays_stdio() {
        let content = r#"{
            "mcpServers": {
                "incomplete": {
                    "headers": {"Authorization": "Bearer ${env:MCP_TOKEN}"}
                }
            }
        }"#;
        let diagnostics = validate_path(".cursor/mcp.json", content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "a Cursor entry without a URL or command must retain the stdio error"
        );
    }

    #[test]
    fn test_mcp_009_cursor_url_inference_respects_path_boundaries() {
        let content = r#"{
            "mcpServers": {
                "remote": {"url": "https://example.com/mcp"}
            }
        }"#;
        for path in ["mcp.json", ".cursor/sub/mcp.json"] {
            let diagnostics = validate_path(path, content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "MCP-009"),
                "URL-only inference must not apply to {path}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_mcp_009_kiro_power_url_only_server_is_remote() {
        let content = r#"{
            "mcpServers": {
                "autodesk-product-help": {
                    "url": "https://developer.api.autodesk.com/knowledge/public/v1/mcp",
                    "disabled": false,
                    "autoApprove": []
                }
            }
        }"#;
        for path in [
            ".kiro/powers/3dsmax-design-power/mcp.json",
            "/home/u/.kiro/powers/installed/design/mcp.json",
        ] {
            let diagnostics = validate_path(path, content);
            assert!(
                diagnostics.iter().all(|d| d.rule != "MCP-009"),
                "Kiro power url-only servers are remote ({path}): {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_mcp_009_kiro_published_power_detected_by_power_md_sibling() {
        let temp = tempfile::TempDir::new().unwrap();
        let power = temp.path().join("powers").join("design");
        let other = temp.path().join("other").join("design");
        std::fs::create_dir_all(&power).unwrap();
        std::fs::create_dir_all(&other).unwrap();
        std::fs::write(power.join("POWER.md"), "---\nname: design\n---\n").unwrap();
        let config = LintConfig::default();
        let content = r#"{"mcpServers": {"remote": {"url": "https://example.com/mcp"}}}"#;
        let diagnostics = McpValidator.validate(&power.join("mcp.json"), content, &config);
        assert!(
            diagnostics.iter().all(|d| d.rule != "MCP-009"),
            "{diagnostics:?}"
        );
        // Without a power sibling the stdio default still applies.
        let diagnostics = McpValidator.validate(&other.join("mcp.json"), content, &config);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn test_mcp_009_kiro_power_without_url_or_command_stays_stdio() {
        let content = r#"{"mcpServers": {"broken": {"args": ["x"]}}}"#;
        let diagnostics = validate_path(".kiro/powers/p/mcp.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-009"));
    }

    #[test]
    fn test_cursor_url_server_with_command_retains_stdio_default() {
        let content = r#"{
            "mcpServers": {
                "local": {
                    "command": "node",
                    "url": "http://api.example.com/mcp"
                }
            }
        }"#;
        let diagnostics = validate_path(".cursor/mcp.json", content);
        assert!(diagnostics.iter().all(|d| d.rule != "MCP-009"));
        assert!(diagnostics.iter().all(|d| d.rule != "MCP-017"));
    }

    #[test]
    fn test_cursor_url_only_server_reaches_mcp_017() {
        let content = r#"{
            "mcpServers": {
                "remote": {"url": "http://api.example.com/mcp"}
            }
        }"#;
        let diagnostics = validate_path(".cursor/mcp.json", content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-017"));
        assert!(diagnostics.iter().all(|d| d.rule != "MCP-009"));
    }

    #[test]
    fn test_cursor_url_server_does_not_infer_transport_for_non_string_type() {
        for invalid_type in ["42", "null"] {
            let content = format!(
                r#"{{
                    "mcpServers": {{
                        "cursor-remote": {{
                            "type": {invalid_type},
                            "url": "https://example.com/mcp"
                        }}
                    }}
                }}"#
            );
            let diagnostics = validate_path(".cursor/mcp.json", &content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "MCP-011"),
                "an explicit non-string type must not be treated as omitted: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_mcp_009_url_only_server_still_defaults_to_stdio_outside_cursor() {
        let content = r#"{
            "mcpServers": {
                "remote": {
                    "url": "https://example.com/mcp"
                }
            }
        }"#;
        let diagnostics = validate_path(".mcp.json", content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "Non-Cursor configs with no type must retain the stdio default"
        );
    }

    #[test]
    fn test_mcp_009_null_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "null-cmd": {
                    "type": "stdio",
                    "command": null
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "null command should trigger MCP-009"
        );
    }

    #[test]
    fn test_mcp_009_empty_string_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-cmd": {
                    "type": "stdio",
                    "command": ""
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "empty string command should trigger MCP-009"
        );
    }

    #[test]
    fn test_mcp_009_empty_array_command_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-arr": {
                    "type": "stdio",
                    "command": []
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "empty array command should trigger MCP-009"
        );
    }

    // ===== MCP-010 Tests =====

    #[test]
    fn test_mcp_010_valid_http_with_url() {
        let content = r#"{
            "mcpServers": {
                "remote-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_http_missing_url() {
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("http-server"));
        assert!(mcp_010[0].message.contains("url"));
    }

    #[test]
    fn test_mcp_010_sse_missing_url() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("sse-server"));
    }

    #[test]
    fn test_mcp_010_valid_sse_with_url() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_stdio_no_url_ok() {
        // Stdio server doesn't need url
        let content = r#"{
            "mcpServers": {
                "stdio-server": {
                    "type": "stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-010"));
    }

    #[test]
    fn test_mcp_010_empty_url_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "empty-url": {
                    "type": "http",
                    "url": ""
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "empty URL should trigger MCP-010"
        );
    }

    #[test]
    fn test_mcp_010_whitespace_url_triggers_error() {
        let content = r#"{
            "mcpServers": {
                "ws-url": {
                    "type": "http",
                    "url": "   "
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "whitespace-only URL should trigger MCP-010"
        );
    }

    // ===== MCP-011 Tests =====

    #[test]
    fn test_mcp_011_valid_types() {
        for server_type in &["stdio", "http", "sse"] {
            let content = format!(
                r#"{{
                    "mcpServers": {{
                        "server": {{
                            "type": "{}",
                            "command": "node",
                            "url": "http://localhost"
                        }}
                    }}
                }}"#,
                server_type
            );
            let diagnostics = validate(&content);
            assert!(
                !diagnostics.iter().any(|d| d.rule == "MCP-011"),
                "Type '{}' should be valid",
                server_type
            );
        }
    }

    #[test]
    fn test_mcp_011_invalid_type() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "websocket",
                    "url": "ws://localhost:8080"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        assert!(mcp_011[0].message.contains("bad-server"));
        assert!(mcp_011[0].message.contains("websocket"));
    }

    #[test]
    fn test_mcp_011_invalid_type_skips_other_rules() {
        // When type is invalid, MCP-009/MCP-010 should not trigger
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "grpc"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-011"));
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "MCP-009 should not trigger for invalid type"
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-010"),
            "MCP-010 should not trigger for invalid type"
        );
    }

    #[test]
    fn test_mcp_011_no_type_field_ok() {
        // Missing type is not invalid (it defaults to stdio)
        let content = r#"{
            "mcpServers": {
                "server": {
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-011"));
    }

    #[test]
    fn test_mcp_011_autofix_case_insensitive() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "Stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        assert!(
            mcp_011[0].has_fixes(),
            "MCP-011 should have auto-fix for case mismatch"
        );
        let fix = &mcp_011[0].fixes[0];
        assert!(!fix.safe, "MCP-011 fix should be unsafe");
        assert_eq!(fix.replacement, "stdio", "Fix should suggest 'stdio'");
    }

    #[test]
    fn test_mcp_011_no_autofix_nonsense() {
        let content = r#"{
            "mcpServers": {
                "bad-server": {
                    "type": "grpc"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-011").collect();
        assert_eq!(mcp_011.len(), 1);
        // "grpc" has no close match to stdio/http/sse - no fix
        assert!(
            !mcp_011[0].has_fixes(),
            "MCP-011 should not auto-fix nonsense values"
        );
    }

    // ===== MCP-012 Tests =====

    #[test]
    fn test_mcp_012_sse_deprecated_warning() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_012: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-012").collect();
        assert_eq!(mcp_012.len(), 1);
        assert!(mcp_012[0].message.contains("sse-server"));
        assert!(mcp_012[0].message.contains("deprecated"));
        // A warning since 0.59: HTTP+SSE is deprecated (SHOULD NOT adopt) but
        // clients still load it, so it is not an error.
        assert_eq!(
            mcp_012[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn test_mcp_012_sse_has_autofix() {
        let content = r#"{
            "mcpServers": {
                "sse-server": {
                    "type": "sse",
                    "url": "http://localhost:3000/sse"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_012 = diagnostics.iter().find(|d| d.rule == "MCP-012");
        assert!(mcp_012.is_some());
        let diag = mcp_012.unwrap();
        assert!(diag.has_fixes());
        assert_eq!(diag.fixes[0].replacement, "http");
        assert!(!diag.fixes[0].safe);
    }

    #[test]
    fn test_mcp_012_http_no_warning() {
        let content = r#"{
            "mcpServers": {
                "http-server": {
                    "type": "http",
                    "url": "http://localhost:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-012"));
    }

    #[test]
    fn test_mcp_012_stdio_no_warning() {
        let content = r#"{
            "mcpServers": {
                "server": {
                    "type": "stdio",
                    "command": "node"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "MCP-012"));
    }

    // ===== MCP-013..MCP-024 Tests =====

    #[test]
    fn test_mcp_013_invalid_tool_name() {
        let content = r#"{
            "tools": [{
                "name": "bad tool",
                "description": "A valid description",
                "inputSchema": {"type": "object"}
            }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-013"));
    }

    #[test]
    fn test_mcp_013_has_fix() {
        let content = r#"{"tools":[{"name":"bad tool","description":"desc","inputSchema":{"type":"object"}}]}"#;
        let diagnostics = validate(content);
        let mcp_013: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-013").collect();
        assert_eq!(mcp_013.len(), 1);
        assert!(
            mcp_013[0].has_fixes(),
            "MCP-013 should have auto-fix for invalid tool name chars"
        );
        let fix = &mcp_013[0].fixes[0];
        assert!(!fix.safe, "MCP-013 fix should be unsafe");
        assert!(
            !fix.replacement.contains(' '),
            "Fix should sanitize tool name by replacing invalid chars"
        );
    }

    #[test]
    fn test_mcp_017_has_fix() {
        let content = r#"{"mcpServers":{"s":{"type":"http","url":"http://example.com/mcp"}}}"#;
        let diagnostics = validate(content);
        let mcp_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-017").collect();
        assert_eq!(mcp_017.len(), 1);
        assert!(
            mcp_017[0].has_fixes(),
            "MCP-017 should have auto-fix to replace http:// with https://"
        );
        let fix = &mcp_017[0].fixes[0];
        assert!(!fix.safe, "MCP-017 fix should be unsafe");
        assert!(
            fix.replacement.starts_with("https://"),
            "Fix should replace http:// with https://, got: {}",
            fix.replacement
        );
        assert!(
            fix.replacement.contains("example.com"),
            "Fix should preserve the rest of the URL"
        );
    }

    #[test]
    fn test_mcp_021_has_fix() {
        let content = r#"{"mcpServers":{"s":{"type":"http","url":"http://0.0.0.0:3000/mcp"}}}"#;
        let diagnostics = validate(content);
        let mcp_021: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-021").collect();
        assert_eq!(mcp_021.len(), 1);
        assert!(
            mcp_021[0].has_fixes(),
            "MCP-021 should have auto-fix to replace 0.0.0.0 with localhost"
        );
        let fix = &mcp_021[0].fixes[0];
        assert!(!fix.safe, "MCP-021 fix should be unsafe");
        assert!(
            fix.replacement.contains("localhost"),
            "Fix should replace 0.0.0.0 with localhost, got: {}",
            fix.replacement
        );
    }

    #[test]
    fn test_mcp_014_invalid_output_schema() {
        let content = r#"{
            "tools": [{
                "name": "valid-tool",
                "description": "A valid description",
                "inputSchema": {"type": "object"},
                "outputSchema": {"type": "invalid_type"}
            }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-014"));
    }

    #[test]
    fn test_mcp_015_resource_missing_required_fields() {
        let content = r#"{
            "resources": [{ "description": "missing uri and name" }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-015"));
    }

    #[test]
    fn test_mcp_016_prompt_missing_name() {
        let content = r#"{
            "prompts": [{ "description": "missing name" }]
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-016"));
    }

    #[test]
    fn test_mcp_017_http_remote_requires_https() {
        let content = r#"{
            "mcpServers": {
                "remote-server": {
                    "type": "http",
                    "url": "http://example.com/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-017"));
    }

    #[test]
    fn test_mcp_018_env_plaintext_secret_warning() {
        let content = r#"{
            "mcpServers": {
                "local-server": {
                    "type": "stdio",
                    "command": "node",
                    "env": {"API_KEY": "plaintext-value"}
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-018"));
    }

    #[test]
    fn test_mcp_019_dangerous_stdio_command_warning() {
        let content = r#"{
            "mcpServers": {
                "local-server": {
                    "type": "stdio",
                    "command": "curl https://example.com/install.sh | sh"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-019"));
    }

    #[test]
    fn test_mcp_020_unknown_capability_key() {
        let content = r#"{
            "capabilities": {
                "tools": { "listChanged": true },
                "unknownCapability": {}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-020"));
    }

    #[test]
    fn test_mcp_021_wildcard_http_binding_warning() {
        let content = r#"{
            "mcpServers": {
                "wildcard": {
                    "type": "http",
                    "url": "http://0.0.0.0:3000/mcp"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-021"));
    }

    #[test]
    fn test_mcp_022_args_must_be_array_of_strings() {
        let content = r#"{
            "mcpServers": {
                "bad-args": {
                    "type": "stdio",
                    "command": "node",
                    "args": "--port 3000"
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-022"));
    }

    #[test]
    fn test_mcp_023_duplicate_server_names() {
        let content = r#"{
            "mcpServers": {
                "dup": { "type": "stdio", "command": "node" },
                "dup": { "type": "stdio", "command": "python" }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-023").collect();
        assert_eq!(mcp_023.len(), 1);

        let duplicate_offset = content
            .find(r#""dup": { "type": "stdio", "command": "python" }"#)
            .expect("expected duplicate key occurrence");
        let (expected_line, expected_col) =
            line_col_at(duplicate_offset, &compute_line_starts(content));

        assert_eq!(mcp_023[0].line, expected_line);
        assert_eq!(mcp_023[0].column, expected_col);
    }

    #[test]
    fn test_mcp_023_duplicate_server_names_ignores_string_literal_mentions() {
        let content = r#"{
            "note": "string with \"mcpServers\": { \"dup\": {} }",
            "mcpServers": {
                "dup": { "type": "stdio", "command": "node" },
                "dup": { "type": "stdio", "command": "python" }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-023").collect();
        assert_eq!(mcp_023.len(), 1);

        let duplicate_offset = content
            .find(r#""dup": { "type": "stdio", "command": "python" }"#)
            .expect("expected duplicate key occurrence");
        let (expected_line, expected_col) =
            line_col_at(duplicate_offset, &compute_line_starts(content));

        assert_eq!(mcp_023[0].line, expected_line);
        assert_eq!(mcp_023[0].column, expected_col);
    }

    #[test]
    fn test_mcp_023_reports_each_duplicate_occurrence_location() {
        let content = r#"{
            "mcpServers": {
                "dup": { "type": "stdio", "command": "node" },
                "dup": { "type": "stdio", "command": "python" },
                "dup": { "type": "stdio", "command": "ruby" }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_023: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-023").collect();
        assert_eq!(mcp_023.len(), 2);

        let second_duplicate_offset = content
            .find(r#""dup": { "type": "stdio", "command": "python" }"#)
            .expect("expected second duplicate key occurrence");
        let third_duplicate_offset = content
            .find(r#""dup": { "type": "stdio", "command": "ruby" }"#)
            .expect("expected third duplicate key occurrence");
        let line_starts = compute_line_starts(content);
        let second_location = line_col_at(second_duplicate_offset, &line_starts);
        let third_location = line_col_at(third_duplicate_offset, &line_starts);

        assert_eq!((mcp_023[0].line, mcp_023[0].column), second_location);
        assert_eq!((mcp_023[1].line, mcp_023[1].column), third_location);
    }

    #[test]
    fn test_mcp_024_empty_server_configuration() {
        let content = r#"{
            "mcpServers": {
                "empty": {}
            }
        }"#;
        let diagnostics = validate(content);
        assert!(diagnostics.iter().any(|d| d.rule == "MCP-024"));
    }

    #[test]
    fn test_mcp_024_not_triggered_by_always_load_only() {
        // MCP-024 flags "empty" servers. An `alwaysLoad` boolean alone is
        // still meaningless without a type/command/url, but it IS a
        // deliberate field so we should NOT report MCP-024 here. MCP-009
        // (missing command for the default stdio transport) IS the right
        // diagnostic for this case - assert it fires so the test doesn't
        // silently become vacuous if validation behavior changes.
        let content = r#"{
            "mcpServers": {
                "partial": { "alwaysLoad": true }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "MCP-024"),
            "MCP-024 should not fire when alwaysLoad is set"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-009"),
            "MCP-009 (missing command for stdio) should fire instead"
        );
    }

    #[test]
    fn test_mcp_024_triggers_when_only_field_is_null_always_load() {
        // An explicit JSON `null` for alwaysLoad is conventionally "absent";
        // it should NOT save the server from MCP-024. This guards against
        // Some(Value::Null) masquerading as a meaningful field.
        let content = r#"{
            "mcpServers": {
                "empty-ish": { "alwaysLoad": null }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.iter().any(|d| d.rule == "MCP-024"),
            "MCP-024 should fire when alwaysLoad is the only field and it's null"
        );
    }

    #[test]
    fn test_mcp_004_short_description_is_advisory() {
        let diagnostics = validate(r#"{"name":"test","description":"short","inputSchema":{}}"#);
        let hit = diagnostics.iter().find(|d| d.rule == "MCP-004").unwrap();
        assert_eq!(hit.level, crate::diagnostics::DiagnosticLevel::Info);
    }

    // ===== Multiple servers test =====

    #[test]
    fn test_multiple_servers_validation() {
        let content = r#"{
            "mcpServers": {
                "good-stdio": {
                    "type": "stdio",
                    "command": "node"
                },
                "bad-stdio": {
                    "type": "stdio"
                },
                "good-http": {
                    "type": "http",
                    "url": "http://localhost"
                },
                "bad-http": {
                    "type": "http"
                }
            }
        }"#;
        let diagnostics = validate(content);
        let mcp_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-009").collect();
        let mcp_010: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-010").collect();
        assert_eq!(mcp_009.len(), 1);
        assert!(mcp_009[0].message.contains("bad-stdio"));
        assert_eq!(mcp_010.len(), 1);
        assert!(mcp_010[0].message.contains("bad-http"));
    }

    // ===== Existing server config test should still pass =====

    #[test]
    fn test_mcp_server_config_valid_no_new_errors() {
        // The existing test_mcp_server_config_format test content should still
        // pass with no errors (server has command, no type = defaults to stdio)
        let content = r#"{
            "mcpServers": {
                "my-server": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Valid server config should have no errors, got: {:?}",
            diagnostics
        );
    }

    // ===== MCP-007 suggestion test =====

    #[test]
    fn test_mcp_007_has_suggestion() {
        let content = r#"{ invalid json }"#;
        let diagnostics = validate(content);

        let mcp_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-007").collect();
        assert_eq!(mcp_007.len(), 1);
        assert!(
            mcp_007[0].suggestion.is_some(),
            "MCP-007 should have a suggestion"
        );
        assert!(
            mcp_007[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("Validate JSON syntax"),
            "MCP-007 suggestion should mention JSON syntax"
        );
    }

    // ===== MCP-003 improved suggestion test =====

    #[test]
    fn test_mcp_003_suggestion_lists_valid_types() {
        let content = r#"{"name": "test-tool", "description": "A test tool that does things", "inputSchema": {"type": "invalid"}}"#;
        let diagnostics = validate(content);

        let mcp_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-003").collect();
        assert_eq!(mcp_003.len(), 1);
        assert!(
            mcp_003[0].suggestion.is_some(),
            "MCP-003 should have a suggestion"
        );
        let suggestion = mcp_003[0].suggestion.as_ref().unwrap();
        assert!(
            suggestion.contains("string")
                && suggestion.contains("number")
                && suggestion.contains("integer")
                && suggestion.contains("boolean")
                && suggestion.contains("object")
                && suggestion.contains("array")
                && suggestion.contains("null"),
            "MCP-003 suggestion should list all valid JSON Schema types, got: {}",
            suggestion
        );
    }

    // ===== MCP-006 improved suggestion test =====

    #[test]
    fn test_mcp_006_suggestion_lists_standard_hints() {
        let content = r#"{
            "name": "test-tool",
            "description": "A test tool that does useful things",
            "inputSchema": {"type": "object"},
            "annotations": {"title": "My Tool", "readonly": true}
        }"#;
        let diagnostics = validate(content);
        let mcp_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "MCP-006").collect();
        assert_eq!(mcp_006.len(), 1);
        let suggestion = mcp_006[0].suggestion.as_ref().unwrap();
        assert!(suggestion.contains("readOnlyHint"), "{suggestion}");
    }

    // ===== Flat top-level server-map shape (Codex rust-v0.123.0) =====
    //
    // Upstream openai/codex made plugin MCP loading accept both shapes.
    // agnix's MCP validator mirrors that compatibility so per-server rules
    // (MCP-009..012, MCP-024) fire on the flat form too - previously they
    // silently no-oped when `mcpServers` was missing.

    #[test]
    fn test_extract_mcp_servers_flat_shape_stdio_server() {
        let content = r#"{
            "fs": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"] }
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let servers = extract_mcp_servers(&v);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].0, "fs");
        assert_eq!(
            servers[0].1.command.as_ref().and_then(|v| v.as_str()),
            Some("npx")
        );
    }

    #[test]
    fn test_extract_mcp_servers_flat_shape_http_server() {
        let content = r#"{
            "remote": { "url": "https://api.example.com/mcp" }
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let servers = extract_mcp_servers(&v);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].0, "remote");
        assert_eq!(
            servers[0].1.url.as_deref(),
            Some("https://api.example.com/mcp")
        );
    }

    #[test]
    fn test_extract_mcp_servers_flat_shape_multi_server() {
        let content = r#"{
            "fs": { "command": "npx", "args": ["-y", "server-fs"] },
            "remote": { "url": "https://api.example.com/mcp" }
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let mut servers = extract_mcp_servers(&v);
        servers.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(servers.len(), 2);
        assert_eq!(servers[0].0, "fs");
        assert_eq!(servers[1].0, "remote");
    }

    #[test]
    fn test_extract_mcp_servers_mcpservers_shape_still_preferred() {
        // If both shapes somehow coexist (pathological), the explicit
        // `mcpServers` key wins. The flat-shape fallback is ONLY used when
        // no `mcpServers` key is present.
        let content = r#"{
            "mcpServers": { "real": { "command": "npx" } },
            "other": { "command": "should-not-be-extracted" }
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let servers = extract_mcp_servers(&v);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0].0, "real");
    }

    #[test]
    fn test_extract_mcp_servers_rejects_jsonrpc_payload_as_flat_shape() {
        // Guardrail: a generic JSON-RPC MCP response payload must NOT be
        // misread as a flat server map. These have `jsonrpc`, `result`,
        // `id`, etc. - none of which have `command` or `url`.
        let content = r#"{
            "jsonrpc": "2.0",
            "result": { "tools": [] },
            "id": 1
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let servers = extract_mcp_servers(&v);
        assert!(
            servers.is_empty(),
            "JSON-RPC response payload must not be read as flat server map, got {:?}",
            servers.iter().map(|(n, _)| n).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_extract_mcp_servers_rejects_mixed_flat_and_non_server_values() {
        // Guardrail: if a top-level key's value doesn't have `command` or
        // `url`, we can't confidently treat ANY top-level key as a server.
        // This prevents misreading a user's typo of `mcpServersx` (where
        // the value IS a server map but the outer key is wrong) as if
        // the server-map's children were themselves flat-shape entries.
        let content = r#"{
            "mcpServersx": {
                "fs": { "command": "npx" }
            }
        }"#;
        let v: serde_json::Value = serde_json::from_str(content).unwrap();
        let servers = extract_mcp_servers(&v);
        assert!(
            servers.is_empty(),
            "typo of mcpServers (mcpServersx) must not be read as flat server map"
        );
    }

    #[test]
    fn test_extract_mcp_servers_rejects_empty_object() {
        let v: serde_json::Value = serde_json::from_str("{}").unwrap();
        let servers = extract_mcp_servers(&v);
        assert!(servers.is_empty());
    }

    #[test]
    fn test_flat_shape_server_validation_now_fires() {
        // End-to-end: the main user-visible win of dual-shape support is
        // that per-server diagnostics actually run on flat-shape files.
        // Before, extract_mcp_servers returned empty when mcpServers was
        // absent, so any invalid server config silently passed.
        //
        // This fixture is flat-shape with an empty-string command. With
        // the dual-shape support, the validator now sees the server and
        // per-server rules (MCP-009 empty command, or similar) fire.
        let content = r#"{
            "fs": {
                "command": "",
                "args": []
            }
        }"#;
        let diagnostics = validate(content);
        // Before this change: no diagnostics would fire (empty extraction).
        // After: per-server rules see the flat-shape server and fire.
        let per_server: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("MCP-"))
            .collect();
        assert!(
            !per_server.is_empty(),
            "flat-shape server must surface per-server diagnostics, got none"
        );
    }
}
