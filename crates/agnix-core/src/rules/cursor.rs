//! Cursor project rules validation rules (CUR-001 to CUR-020)
//!
//! Validates:
//! - CUR-001: Empty .mdc rule file (HIGH) - files must have content
//! - CUR-002: Missing frontmatter (MEDIUM) - .mdc files should have frontmatter
//! - CUR-003: Invalid YAML frontmatter (HIGH) - frontmatter must be valid YAML
//! - CUR-004: Invalid glob pattern (HIGH) - globs field must contain valid patterns
//! - CUR-005: Unknown frontmatter keys (MEDIUM) - warn about unrecognized keys
//! - CUR-006: Legacy .cursorrules detected (MEDIUM) - migration warning
//! - CUR-007: alwaysApply with redundant globs (MEDIUM) - globs ignored when alwaysApply is true
//! - CUR-008: Invalid alwaysApply type (HIGH) - must be boolean, not string
//! - CUR-009: Missing description for agent-requested rule (MEDIUM) - agent needs description
//! - CUR-010: Invalid .cursor/hooks.json schema (HIGH)
//! - CUR-011: Unknown hook event name in .cursor/hooks.json (MEDIUM)
//! - CUR-012: Hook entry missing required command field (HIGH)
//! - CUR-013: Invalid hook type value (HIGH)
//! - CUR-014: Invalid Cursor subagent frontmatter (HIGH)
//! - CUR-015: Empty Cursor subagent body (MEDIUM)
//! - CUR-016: Invalid .cursor/environment.json schema (HIGH)
//! - CUR-017: Invalid hook entry field types (MEDIUM) - timeout/loop_limit/failClosed type checks
//! - CUR-018: Invalid or missing prompt hook field (MEDIUM) - type:"prompt" requires a non-empty string
//! - CUR-019: Invalid model field on prompt hook (LOW) - model must be a string
//! - CUR-020: Ignored .md project rule (HIGH) - project rules must use .mdc

use crate::{
    FileType,
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    parsers::frontmatter::split_frontmatter,
    rules::{Validator, ValidatorMetadata, json_type_name},
    schemas::cursor::{
        ParsedMdcFrontmatter, is_body_empty, is_content_empty, parse_mdc_frontmatter,
        validate_glob_pattern,
    },
    schemas::gemini_settings::strip_jsonc_comments,
};
use rust_i18n::t;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use std::path::Path;

const RULE_IDS: &[&str] = &[
    "CUR-001", "CUR-002", "CUR-003", "CUR-004", "CUR-005", "CUR-006", "CUR-007", "CUR-008",
    "CUR-009", "CUR-010", "CUR-011", "CUR-012", "CUR-013", "CUR-014", "CUR-015", "CUR-016",
    "CUR-017", "CUR-018", "CUR-019", "CUR-020",
];

const CURSOR_HOOK_EVENTS: &[&str] = &[
    "workspaceOpen",
    "sessionStart",
    "sessionEnd",
    "preToolUse",
    "postToolUse",
    "postToolUseFailure",
    "subagentStart",
    "subagentStop",
    "beforeShellExecution",
    "afterShellExecution",
    "beforeMCPExecution",
    "afterMCPExecution",
    "beforeReadFile",
    "afterFileEdit",
    "beforeSubmitPrompt",
    "preCompact",
    "stop",
    "afterAgentResponse",
    "afterAgentThought",
    "beforeTabFileRead",
    "afterTabFileEdit",
];

const CURSOR_HOOK_TYPES: &[&str] = &["command", "prompt"];

pub struct CursorValidator;

fn line_byte_range(content: &str, line_number: usize) -> Option<(usize, usize)> {
    if line_number == 0 {
        return None;
    }

    let mut current_line = 1usize;
    let mut line_start = 0usize;

    for (idx, ch) in content.char_indices() {
        if current_line == line_number && ch == '\n' {
            return Some((line_start, idx + 1));
        }
        if ch == '\n' {
            current_line += 1;
            line_start = idx + 1;
        }
    }

    if current_line == line_number {
        Some((line_start, content.len()))
    } else {
        None
    }
}

/// Find the 1-indexed line number of a YAML field in parsed frontmatter.
fn find_field_line(parsed: &ParsedMdcFrontmatter, field_prefix: &str) -> usize {
    parsed
        .raw
        .lines()
        .enumerate()
        .find(|(_, line)| line.trim_start().starts_with(field_prefix))
        .map(|(idx, _)| parsed.start_line + 1 + idx)
        .unwrap_or(parsed.start_line)
}

/// Get the byte range of a YAML block (key line + indented continuation lines).
/// This is used for auto-fix deletion of multi-line fields like list-style `globs:`.
fn yaml_block_byte_range(content: &str, start_line: usize) -> Option<(usize, usize)> {
    let (block_start, mut block_end) = line_byte_range(content, start_line)?;

    // Extend to include subsequent indented lines (list items under the key)
    let mut current_line = start_line + 1;
    while let Some((next_start, next_end)) = line_byte_range(content, current_line) {
        let line_text = &content[next_start..next_end.min(content.len())];
        // Stop if the line is not indented (new top-level key or closing ---)
        if !line_text.starts_with(' ') && !line_text.starts_with('\t') {
            break;
        }
        block_end = next_end;
        current_line += 1;
    }

    Some((block_start, block_end))
}

/// Find the byte range of a quoted YAML value for a given key in frontmatter.
/// Returns the range including quotes (e.g., `"true"` or `'false'`).
/// Wrapper around the shared helper.
fn find_yaml_quoted_value_range(
    content: &str,
    parsed: &ParsedMdcFrontmatter,
    key: &str,
) -> Option<(usize, usize)> {
    crate::rules::find_yaml_value_range(content, parsed, key, true)
}

fn is_valid_cursor_agent_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--")
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn is_valid_cursor_model_id(model: &str) -> bool {
    fn is_valid_part(value: &str) -> bool {
        !value.is_empty()
            && value
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    }

    let trimmed = model.trim();
    if trimmed != model {
        return false;
    }
    let model = trimmed;
    let Some(open_bracket) = model.find('[') else {
        return is_valid_part(model);
    };

    if !model.ends_with(']') {
        return false;
    }

    let base = &model[..open_bracket];
    let parameters = &model[open_bracket + 1..model.len() - 1];
    if !is_valid_part(base) || parameters.contains(['[', ']']) {
        return false;
    }

    parameters.is_empty()
        || parameters.split(',').all(|parameter| {
            let Some((name, value)) = parameter.split_once('=') else {
                return false;
            };
            is_valid_part(name) && is_valid_part(value)
        })
}

fn find_root_json_key_location(content: &str, key: &str) -> (usize, usize) {
    let bytes = content.as_bytes();
    let mut index = 0usize;
    let mut object_depth = 0usize;
    let mut location = None;

    while index < bytes.len() {
        if bytes[index] == b'/' && index + 1 < bytes.len() {
            if bytes[index + 1] == b'/' {
                index += 2;
                while index < bytes.len() && !matches!(bytes[index], b'\n' | b'\r') {
                    index += 1;
                }
                continue;
            }
            if bytes[index + 1] == b'*' {
                index += 2;
                while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/')
                {
                    index += 1;
                }
                index = (index + 2).min(bytes.len());
                continue;
            }
        }

        if bytes[index] != b'"' {
            if bytes[index] == b'{' {
                object_depth += 1;
            } else if bytes[index] == b'}' {
                object_depth = object_depth.saturating_sub(1);
            }
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        let value_start = index;
        let mut escaped = false;
        let mut has_escape = false;
        while index < bytes.len() {
            if escaped {
                escaped = false;
            } else if bytes[index] == b'\\' {
                escaped = true;
                has_escape = true;
            } else if bytes[index] == b'"' {
                break;
            }
            index += 1;
        }

        if index >= bytes.len() {
            break;
        }

        let value_end = index;
        index += 1;
        let mut next = index;
        loop {
            while next < bytes.len() && bytes[next].is_ascii_whitespace() {
                next += 1;
            }
            if bytes.get(next..next + 2) == Some(b"//") {
                next += 2;
                while next < bytes.len() && !matches!(bytes[next], b'\n' | b'\r') {
                    next += 1;
                }
                continue;
            }
            if bytes.get(next..next + 2) == Some(b"/*") {
                next += 2;
                while next + 1 < bytes.len() && !(bytes[next] == b'*' && bytes[next + 1] == b'/') {
                    next += 1;
                }
                next = (next + 2).min(bytes.len());
                continue;
            }
            break;
        }

        let key_matches = if has_escape {
            serde_json::from_str::<String>(&content[start..=value_end])
                .is_ok_and(|decoded| decoded == key)
        } else {
            bytes.get(value_start..value_end) == Some(key.as_bytes())
        };

        if object_depth == 1 && key_matches && bytes.get(next) == Some(&b':') {
            let line = content[..start]
                .bytes()
                .filter(|byte| *byte == b'\n')
                .count()
                + 1;
            let line_start = content[..start]
                .rfind('\n')
                .map(|offset| offset + 1)
                .unwrap_or(0);
            let column = content[line_start..start].chars().count() + 1;
            location = Some((line, column));
        }
    }

    location.unwrap_or((1, 0))
}

fn cursor_environment_error(
    path: &Path,
    content: &str,
    field: &str,
    message: impl Into<String>,
) -> Diagnostic {
    let (line, column) = find_root_json_key_location(content, field);
    Diagnostic::error(path.to_path_buf(), line, column, "CUR-016", message)
}

fn validate_cursor_terminal_object(
    path: &Path,
    content: &str,
    terminal: &serde_json::Map<String, JsonValue>,
    index: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if terminal
        .get("command")
        .and_then(JsonValue::as_str)
        .is_none()
    {
        diagnostics.push(
            cursor_environment_error(
                path,
                content,
                "terminals",
                t!("rules.cur_016.terminal", index = index),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    for field in ["name", "description"] {
        if terminal.get(field).is_some_and(|value| !value.is_string()) {
            diagnostics.push(
                cursor_environment_error(
                    path,
                    content,
                    "terminals",
                    t!(
                        "rules.cur_016.terminal_field_type",
                        index = index,
                        field = field
                    ),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
        }
    }
}

fn validate_cursor_hooks_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let parsed = match serde_json::from_str::<JsonValue>(content) {
        Ok(value) => value,
        Err(error) => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.parse_error", error = error.to_string()),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    let root = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.message"),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    // `version` is documented with a default ("| `version` | number | `1` |
    // Config schema version |") and several of the doc's own examples omit it,
    // so absence is valid. Only a present-but-non-numeric value is wrong. Typed
    // "number" rather than integer, so `1.0` is accepted too.
    if config.is_rule_enabled("CUR-010")
        && let Some(version) = root.get("version")
        && !version.is_number()
    {
        diagnostics.push(
            Diagnostic::error(
                path.to_path_buf(),
                1,
                0,
                "CUR-010",
                t!("rules.cur_010.invalid_version"),
            )
            .with_suggestion(t!("rules.cur_010.suggestion")),
        );
    }

    let hooks = match root.get("hooks") {
        Some(JsonValue::Object(map)) => map,
        Some(other) => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.invalid_hooks", got = json_type_name(other)),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
        None => {
            if config.is_rule_enabled("CUR-010") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-010",
                        t!("rules.cur_010.missing_hooks"),
                    )
                    .with_suggestion(t!("rules.cur_010.suggestion")),
                );
            }
            return diagnostics;
        }
    };

    for (event_name, hooks_value) in hooks {
        if config.is_rule_enabled("CUR-011") && !CURSOR_HOOK_EVENTS.contains(&event_name.as_str()) {
            let mut diagnostic = Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CUR-011",
                t!("rules.cur_011.message", event = event_name.as_str()),
            )
            .with_suggestion(t!("rules.cur_011.suggestion"));

            if let Some(suggested) =
                crate::rules::find_closest_value(event_name.as_str(), CURSOR_HOOK_EVENTS)
            {
                if let Some((start, end)) =
                    crate::span_utils::find_event_key_span(content, event_name)
                {
                    diagnostic = diagnostic.with_fix(Fix::replace(
                        start,
                        end,
                        format!("\"{}\"", suggested),
                        format!("Replace hook event with '{}'", suggested),
                        false,
                    ));
                }
            }

            diagnostics.push(diagnostic);
        }

        let hook_array = match hooks_value.as_array() {
            Some(entries) => entries,
            None => {
                if config.is_rule_enabled("CUR-010") {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-010",
                            t!(
                                "rules.cur_010.invalid_event_hooks",
                                event = event_name.as_str(),
                                got = json_type_name(hooks_value)
                            ),
                        )
                        .with_suggestion(t!("rules.cur_010.suggestion")),
                    );
                }
                continue;
            }
        };

        for (index, hook_entry) in hook_array.iter().enumerate() {
            let hook_obj = match hook_entry.as_object() {
                Some(obj) => obj,
                None => {
                    if config.is_rule_enabled("CUR-010") {
                        diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-010",
                                t!(
                                    "rules.cur_010.invalid_hook_entry",
                                    event = event_name.as_str(),
                                    index = index + 1
                                ),
                            )
                            .with_suggestion(t!("rules.cur_010.suggestion")),
                        );
                    }
                    continue;
                }
            };

            if config.is_rule_enabled("CUR-013")
                && let Some(type_value) = hook_obj.get("type")
            {
                let type_str = type_value.as_str();
                if type_str.is_none() || !CURSOR_HOOK_TYPES.contains(&type_str.unwrap_or_default())
                {
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-013",
                        t!(
                            "rules.cur_013.message",
                            event = event_name.as_str(),
                            index = index + 1
                        ),
                    )
                    .with_suggestion(t!("rules.cur_013.suggestion"));

                    if let Some(invalid_type) = type_str {
                        if let Some(suggested) =
                            crate::rules::find_closest_value(invalid_type, CURSOR_HOOK_TYPES)
                        {
                            if let Some((start, end)) =
                                crate::rules::find_unique_json_string_value_span(
                                    content,
                                    "type",
                                    invalid_type,
                                )
                            {
                                diagnostic = diagnostic.with_fix(Fix::replace(
                                    start,
                                    end,
                                    suggested,
                                    format!("Replace hook type with '{}'", suggested),
                                    false,
                                ));
                            }
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }

            // Prompt hooks have no `command`: the doc's own example carries only
            // `type`, `prompt` and `timeout`, since "Prompt hooks use an LLM to
            // evaluate a natural language condition". CUR-018 checks `prompt`
            // for those, so requiring `command` here rejected the documented
            // form.
            let is_prompt_hook = hook_obj.get("type").and_then(JsonValue::as_str) == Some("prompt");

            if config.is_rule_enabled("CUR-012") && !is_prompt_hook {
                let has_valid_command = hook_obj
                    .get("command")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|command| !command.trim().is_empty());

                if !has_valid_command {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-012",
                            t!(
                                "rules.cur_012.message",
                                event = event_name.as_str(),
                                index = index + 1
                            ),
                        )
                        .with_suggestion(t!("rules.cur_012.suggestion")),
                    );
                }
            }

            // CUR-017: Invalid hook entry field types
            if config.is_rule_enabled("CUR-017") {
                if let Some(timeout) = hook_obj.get("timeout") {
                    match timeout.as_f64() {
                        Some(n) if n > 0.0 => {} // valid
                        Some(n) => {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CUR-017",
                                    format!(
                                        "Hook entry {} in '{}' has invalid 'timeout': expected a positive number, got {}",
                                        index + 1,
                                        event_name,
                                        n
                                    ),
                                )
                                .with_suggestion("Set 'timeout' to a positive number of seconds."),
                            );
                        }
                        None => {
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CUR-017",
                                    format!(
                                        "Hook entry {} in '{}' has invalid 'timeout': expected a positive number, got {}",
                                        index + 1,
                                        event_name,
                                        json_type_name(timeout)
                                    ),
                                )
                                .with_suggestion("Set 'timeout' to a positive number of seconds."),
                            );
                        }
                    }
                }

                if let Some(loop_limit) = hook_obj.get("loop_limit") {
                    if !loop_limit.is_null() && !loop_limit.is_number() {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-017",
                                format!(
                                    "Hook entry {} in '{}' has invalid 'loop_limit': expected a number or null, got {}",
                                    index + 1,
                                    event_name,
                                    json_type_name(loop_limit)
                                ),
                            )
                            .with_suggestion("Set 'loop_limit' to a number or null."),
                        );
                    }
                }

                if let Some(fail_closed) = hook_obj.get("failClosed") {
                    if !fail_closed.is_boolean() {
                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-017",
                                format!(
                                    "Hook entry {} in '{}' has invalid 'failClosed': expected a boolean, got {}",
                                    index + 1,
                                    event_name,
                                    json_type_name(fail_closed)
                                ),
                            )
                            .with_suggestion("Set 'failClosed' to true or false."),
                        );
                    }
                }
            }

            // CUR-018: Prompt-type hook missing or invalid prompt field
            if config.is_rule_enabled("CUR-018") {
                let is_prompt_type = hook_obj
                    .get("type")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|t| t == "prompt");

                let has_valid_prompt = hook_obj
                    .get("prompt")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|prompt| !prompt.trim().is_empty());

                if is_prompt_type && !has_valid_prompt {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-018",
                            format!(
                                "Hook entry {} in '{}' has type 'prompt' but lacks a non-empty string 'prompt' field",
                                index + 1,
                                event_name
                            ),
                        )
                        .with_suggestion("Add a 'prompt' field with the prompt string for this hook."),
                    );
                }
            }

            // CUR-019: Invalid model field on prompt hook
            if config.is_rule_enabled("CUR-019") {
                let is_prompt_type = hook_obj
                    .get("type")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|t| t == "prompt");

                if is_prompt_type {
                    if let Some(model) = hook_obj.get("model") {
                        if !model.is_string() {
                            diagnostics.push(
                                Diagnostic::info(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CUR-019",
                                    format!(
                                        "Hook entry {} in '{}' has invalid 'model': expected a string, got {}",
                                        index + 1,
                                        event_name,
                                        json_type_name(model)
                                    ),
                                )
                                .with_suggestion("Set 'model' to a string model identifier."),
                            );
                        }
                    }
                }
            }
        }
    }

    diagnostics
}

fn validate_cursor_agent_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let parts = split_frontmatter(content);

    if config.is_rule_enabled("CUR-014") {
        if !parts.has_frontmatter || !parts.has_closing {
            diagnostics.push(
                Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-014",
                    t!("rules.cur_014.message"),
                )
                .with_suggestion(t!("rules.cur_014.suggestion")),
            );
        } else {
            let frontmatter = match serde_yaml::from_str::<YamlValue>(&parts.frontmatter) {
                Ok(value) => Some(value),
                Err(_) => {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.invalid_frontmatter"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                    None
                }
            };

            if let Some(frontmatter_map) = frontmatter.as_ref().and_then(YamlValue::as_mapping) {
                let key = |name: &str| YamlValue::String(name.to_string());

                match frontmatter_map.get(key("name")) {
                    Some(YamlValue::String(name)) if is_valid_cursor_agent_name(name) => {}
                    Some(YamlValue::String(_)) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.invalid_name"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    Some(_) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.name_not_string"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    None => {} // name is optional, defaults to filename
                }

                match frontmatter_map.get(key("description")) {
                    Some(YamlValue::String(_)) => {}
                    Some(_) => diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.description_not_string"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    ),
                    None => {} // description is optional
                }

                if let Some(model_value) = frontmatter_map.get(key("model")) {
                    match model_value {
                        YamlValue::String(model) if is_valid_cursor_model_id(model) => {}
                        YamlValue::String(_) => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-014",
                                t!("rules.cur_014.invalid_model"),
                            )
                            .with_suggestion(t!("rules.cur_014.suggestion")),
                        ),
                        _ => diagnostics.push(
                            Diagnostic::error(
                                path.to_path_buf(),
                                1,
                                0,
                                "CUR-014",
                                t!("rules.cur_014.model_not_string"),
                            )
                            .with_suggestion(t!("rules.cur_014.suggestion")),
                        ),
                    }
                }

                if let Some(readonly_value) = frontmatter_map.get(key("readonly"))
                    && !matches!(readonly_value, YamlValue::Bool(_))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.readonly_not_bool"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                }

                if let Some(is_background_value) = frontmatter_map.get(key("is_background"))
                    && !matches!(is_background_value, YamlValue::Bool(_))
                {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CUR-014",
                            t!("rules.cur_014.is_background_not_bool"),
                        )
                        .with_suggestion(t!("rules.cur_014.suggestion")),
                    );
                }
            } else if frontmatter.is_some() && config.is_rule_enabled("CUR-014") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-014",
                        t!("rules.cur_014.invalid_frontmatter"),
                    )
                    .with_suggestion(t!("rules.cur_014.suggestion")),
                );
            }
        }
    }

    if config.is_rule_enabled("CUR-015")
        && parts.has_frontmatter
        && parts.has_closing
        && parts.body.trim().is_empty()
    {
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                1,
                0,
                "CUR-015",
                t!("rules.cur_015.message"),
            )
            .with_suggestion(t!("rules.cur_015.suggestion")),
        );
    }

    diagnostics
}

fn validate_cursor_environment_file(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if !config.is_rule_enabled("CUR-016") {
        return diagnostics;
    }

    let path_buf = path.to_path_buf();

    let stripped = strip_jsonc_comments(content);
    let parsed = match serde_json::from_str::<JsonValue>(&stripped) {
        Ok(value) => value,
        Err(error) => {
            let line = error.line();
            let column = error.column();
            diagnostics.push(
                Diagnostic::error(
                    path_buf.clone(),
                    line,
                    column,
                    "CUR-016",
                    t!("rules.cur_016.parse_error", error = error.to_string()),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
            return diagnostics;
        }
    };

    let root = match parsed.as_object() {
        Some(obj) => obj,
        None => {
            diagnostics.push(
                Diagnostic::error(
                    path_buf.clone(),
                    1,
                    0,
                    "CUR-016",
                    t!("rules.cur_016.message"),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
            return diagnostics;
        }
    };

    // Cursor's VS Code JSON service treats root `$schema` as the schema-association
    // key even though the published schema's closed property set does not declare it.
    const ALLOWED_ROOT_FIELDS: &[&str] = &[
        "$schema",
        "name",
        "user",
        "install",
        "start",
        "repositoryDependencies",
        "ports",
        "terminals",
        "build",
        "snapshot",
        "agentCanUpdateSnapshot",
    ];

    for field in root.keys() {
        if field != "update" && !ALLOWED_ROOT_FIELDS.contains(&field.as_str()) {
            diagnostics.push(
                cursor_environment_error(
                    path,
                    content,
                    field,
                    t!("rules.cur_016.unknown_root_field", field = field.as_str()),
                )
                .with_suggestion(t!("rules.cur_016.unknown_root_field_suggestion")),
            );
        }
    }

    for field in ["$schema", "name", "user", "snapshot"] {
        if root.get(field).is_some_and(|value| !value.is_string()) {
            diagnostics.push(
                cursor_environment_error(
                    path,
                    content,
                    field,
                    t!("rules.cur_016.string_field", field = field),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
        }
    }

    if root
        .get("agentCanUpdateSnapshot")
        .is_some_and(|value| !value.is_boolean())
    {
        diagnostics.push(
            cursor_environment_error(
                path,
                content,
                "agentCanUpdateSnapshot",
                t!("rules.cur_016.agent_can_update_snapshot"),
            )
            .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    match root.get("install") {
        Some(v) if v.as_str().is_none() => {
            diagnostics.push(
                cursor_environment_error(path, content, "install", t!("rules.cur_016.install"))
                    .with_suggestion(t!("rules.cur_016.suggestion")),
            );
        }
        // `install` is absent from every `required` array in the published
        // environment schema (cursor.com/schemas/environment.schema.json:
        // `definitions.common` has no `required` at all, `definitions.container`
        // has `required: []`), and the setup page's own snapshot example omits
        // it. Requiring it rejected valid config, so absence is fine - only a
        // present-but-wrong type is reported, by the arm above.
        None => {}
        _ => {}
    }

    if let Some(start) = root.get("start")
        && start.as_str().is_none()
    {
        diagnostics.push(
            cursor_environment_error(path, content, "start", t!("rules.cur_016.start"))
                .with_suggestion(t!("rules.cur_016.suggestion")),
        );
    }

    // `update` is not a property in the published schema, and the root sets
    // `unevaluatedProperties: false`, so it is now invalid rather than optional.
    // The setup page notes the install script "was previously called the update
    // script", so this is a rename: report it as one instead of blessing it.
    if root.get("update").is_some() {
        let (line, column) = find_root_json_key_location(content, "update");
        diagnostics.push(
            Diagnostic::warning(
                path_buf.clone(),
                line,
                column,
                "CUR-016",
                t!("rules.cur_016.update_renamed"),
            )
            .with_suggestion(t!("rules.cur_016.update_renamed_suggestion")),
        );
    }

    if let Some(build) = root.get("build") {
        match build.as_object() {
            Some(build_obj) => {
                if !build_obj
                    .get("dockerfile")
                    .is_some_and(JsonValue::is_string)
                {
                    diagnostics.push(
                        cursor_environment_error(
                            path,
                            content,
                            "build",
                            t!("rules.cur_016.build_dockerfile"),
                        )
                        .with_suggestion(t!("rules.cur_016.suggestion")),
                    );
                }
                if let Some(context) = build_obj.get("context")
                    && context.as_str().is_none()
                {
                    diagnostics.push(
                        cursor_environment_error(
                            path,
                            content,
                            "build",
                            t!("rules.cur_016.build_context"),
                        )
                        .with_suggestion(t!("rules.cur_016.suggestion")),
                    );
                }

                for field in build_obj.keys() {
                    if !["dockerfile", "context"].contains(&field.as_str()) {
                        diagnostics.push(
                            cursor_environment_error(
                                path,
                                content,
                                "build",
                                t!("rules.cur_016.unknown_build_field", field = field.as_str()),
                            )
                            .with_suggestion(t!("rules.cur_016.unknown_build_field_suggestion")),
                        );
                    }
                }
            }
            None => {
                diagnostics.push(
                    cursor_environment_error(
                        path,
                        content,
                        "build",
                        t!("rules.cur_016.invalid_build"),
                    )
                    .with_suggestion(t!("rules.cur_016.suggestion")),
                );
            }
        }
    }

    if let Some(dependencies) = root.get("repositoryDependencies") {
        let valid = dependencies
            .as_array()
            .is_some_and(|items| items.iter().all(JsonValue::is_string));
        if !valid {
            diagnostics.push(
                cursor_environment_error(
                    path,
                    content,
                    "repositoryDependencies",
                    t!("rules.cur_016.repository_dependencies"),
                )
                .with_suggestion(t!("rules.cur_016.suggestion")),
            );
        }
    }

    if let Some(ports_value) = root.get("ports") {
        match ports_value.as_array() {
            Some(ports) => {
                for (index, port) in ports.iter().enumerate() {
                    let Some(port_obj) = port.as_object() else {
                        diagnostics.push(
                            cursor_environment_error(
                                path,
                                content,
                                "ports",
                                t!("rules.cur_016.port_object", index = index),
                            )
                            .with_suggestion(t!("rules.cur_016.suggestion")),
                        );
                        continue;
                    };

                    let valid_port = port_obj.get("port").is_some_and(|value| {
                        value.as_f64().is_some_and(|number| {
                            number.fract() == 0.0 && (1.0..=65535.0).contains(&number)
                        })
                    });
                    if !valid_port {
                        diagnostics.push(
                            cursor_environment_error(
                                path,
                                content,
                                "ports",
                                t!("rules.cur_016.port_number", index = index),
                            )
                            .with_suggestion(t!("rules.cur_016.suggestion")),
                        );
                    }

                    if port_obj.get("name").is_some_and(|value| !value.is_string()) {
                        diagnostics.push(
                            cursor_environment_error(
                                path,
                                content,
                                "ports",
                                t!("rules.cur_016.port_name", index = index),
                            )
                            .with_suggestion(t!("rules.cur_016.suggestion")),
                        );
                    }
                }
            }
            None => diagnostics.push(
                cursor_environment_error(path, content, "ports", t!("rules.cur_016.ports_array"))
                    .with_suggestion(t!("rules.cur_016.suggestion")),
            ),
        }
    }

    if let Some(terminals_value) = root.get("terminals") {
        match terminals_value {
            JsonValue::Array(terminals) => {
                for (index, terminal) in terminals.iter().enumerate() {
                    let index_label = (index + 1).to_string();
                    if let Some(obj) = terminal.as_object() {
                        validate_cursor_terminal_object(
                            path,
                            content,
                            obj,
                            &index_label,
                            &mut diagnostics,
                        );
                    } else if let Some(nested) = terminal.as_array() {
                        for (nested_index, entry) in nested.iter().enumerate() {
                            let nested_label = format!("{}][{}", index + 1, nested_index + 1);
                            if let Some(obj) = entry.as_object() {
                                validate_cursor_terminal_object(
                                    path,
                                    content,
                                    obj,
                                    &nested_label,
                                    &mut diagnostics,
                                );
                            } else {
                                diagnostics.push(
                                    cursor_environment_error(
                                        path,
                                        content,
                                        "terminals",
                                        t!("rules.cur_016.terminal", index = nested_label),
                                    )
                                    .with_suggestion(t!("rules.cur_016.suggestion")),
                                );
                            }
                        }
                    } else {
                        diagnostics.push(
                            cursor_environment_error(
                                path,
                                content,
                                "terminals",
                                t!("rules.cur_016.terminal_not_object", index = index + 1),
                            )
                            .with_suggestion(t!("rules.cur_016.suggestion")),
                        );
                    }
                }
            }
            other => {
                diagnostics.push(
                    cursor_environment_error(
                        path,
                        content,
                        "terminals",
                        t!(
                            "rules.cur_016.invalid_terminals",
                            got = json_type_name(other)
                        ),
                    )
                    .with_suggestion(t!("rules.cur_016.suggestion")),
                );
            }
        }
    }

    diagnostics
}

impl Validator for CursorValidator {
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

        let file_type = crate::detect_file_type(path);

        if file_type == FileType::CursorRule
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            if config.is_rule_enabled("CUR-020") {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-020",
                        t!("rules.cur_020.message"),
                    )
                    .with_suggestion(t!("rules.cur_020.suggestion")),
                );
            }
            return diagnostics;
        }

        match file_type {
            FileType::CursorHooks => return validate_cursor_hooks_file(path, content, config),
            FileType::CursorAgent => return validate_cursor_agent_file(path, content, config),
            FileType::CursorEnvironment => {
                return validate_cursor_environment_file(path, content, config);
            }
            _ => {}
        }

        // Determine if this is a .mdc rule file or legacy .cursorrules
        let is_legacy = file_type == FileType::CursorRulesLegacy;

        // CUR-006: Legacy .cursorrules detected (WARNING)
        if is_legacy && config.is_rule_enabled("CUR-006") {
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CUR-006",
                    t!("rules.cur_006.message"),
                )
                .with_suggestion(t!("rules.cur_006.suggestion")),
            );
            // For legacy files, just check if empty and return
            if config.is_rule_enabled("CUR-001") && is_content_empty(content) {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_006.legacy_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_legacy_empty")),
                );
            }
            return diagnostics;
        }

        // CUR-001: Empty .mdc rule file (ERROR)
        let parsed_frontmatter = parse_mdc_frontmatter(content);

        if config.is_rule_enabled("CUR-001") {
            if let Some(parsed) = parsed_frontmatter.as_ref() {
                // Skip CUR-001 if there's a frontmatter parse error - CUR-003 will handle it
                if parsed.parse_error.is_none() && is_body_empty(&parsed.body) {
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            parsed.end_line + 1,
                            0,
                            "CUR-001",
                            t!("rules.cur_001.message_no_content"),
                        )
                        .with_suggestion(t!("rules.cur_001.suggestion_no_content")),
                    );
                }
            } else if is_content_empty(content) {
                // No frontmatter and no content
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-001",
                        t!("rules.cur_001.message_empty"),
                    )
                    .with_suggestion(t!("rules.cur_001.suggestion_empty")),
                );
            }
        }

        // Parse frontmatter for further validation
        let parsed = match parsed_frontmatter {
            Some(p) => p,
            None => {
                // CUR-002: Missing frontmatter in .mdc file (WARNING)
                if config.is_rule_enabled("CUR-002") && !is_content_empty(content) {
                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CUR-002",
                        t!("rules.cur_002.message"),
                    )
                    .with_suggestion(t!("rules.cur_002.suggestion"));

                    // Unsafe auto-fix: insert template frontmatter at start of file.
                    diagnostic = diagnostic.with_fix(Fix::insert(
                        0,
                        "---\ndescription: \nglobs: \n---\n",
                        t!("rules.cur_002.fix"),
                        false,
                    ));

                    diagnostics.push(diagnostic);
                }
                return diagnostics;
            }
        };

        // CUR-003: Invalid YAML frontmatter (ERROR)
        if config.is_rule_enabled("CUR-003") {
            if let Some(ref error) = parsed.parse_error {
                diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        parsed.start_line,
                        0,
                        "CUR-003",
                        t!("rules.cur_003.message", error = error.as_str()),
                    )
                    .with_suggestion(t!("rules.cur_003.suggestion")),
                );
                // Can't continue validating if YAML is broken
                return diagnostics;
            }
        }

        // CUR-004: Invalid glob pattern (ERROR)
        if config.is_rule_enabled("CUR-004") {
            if let Some(ref schema) = parsed.schema {
                if let Some(ref globs) = schema.globs {
                    let globs_line = find_field_line(&parsed, "globs:");

                    for pattern in globs.patterns() {
                        let validation = validate_glob_pattern(pattern);
                        if !validation.valid {
                            diagnostics.push(
                                Diagnostic::error(
                                    path.to_path_buf(),
                                    globs_line,
                                    0,
                                    "CUR-004",
                                    t!(
                                        "rules.cur_004.message",
                                        pattern = pattern,
                                        error = validation.error.unwrap_or_default()
                                    ),
                                )
                                .with_suggestion(t!("rules.cur_004.suggestion")),
                            );
                        }
                    }
                }
            }
        }

        // CUR-005: Unknown frontmatter keys (WARNING)
        if config.is_rule_enabled("CUR-005") {
            for unknown in &parsed.unknown_keys {
                let mut diagnostic = Diagnostic::warning(
                    path.to_path_buf(),
                    unknown.line,
                    unknown.column,
                    "CUR-005",
                    t!("rules.cur_005.message", key = unknown.key.as_str()),
                )
                .with_suggestion(t!("rules.cur_005.suggestion", key = unknown.key.as_str()));

                // Safe auto-fix: remove unknown top-level frontmatter key line.
                if let Some((start, end)) = line_byte_range(content, unknown.line) {
                    diagnostic = diagnostic.with_fix(Fix::delete(
                        start,
                        end,
                        format!("Remove unknown frontmatter key '{}'", unknown.key),
                        true,
                    ));
                }

                diagnostics.push(diagnostic);
            }
        }

        // CUR-007, CUR-008, CUR-009 require a successfully parsed schema
        if let Some(ref schema) = parsed.schema {
            // CUR-008: Invalid alwaysApply type (ERROR)
            // Must be a boolean, not a quoted string like "true" or "false"
            if config.is_rule_enabled("CUR-008") {
                if let Some(crate::schemas::cursor::AlwaysApplyField::String(s)) =
                    schema.always_apply.as_ref()
                {
                    let always_apply_line = find_field_line(&parsed, "alwaysApply:");

                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        always_apply_line,
                        0,
                        "CUR-008",
                        t!("rules.cur_008.message"),
                    )
                    .with_suggestion(t!("rules.cur_008.suggestion"));

                    // Safe auto-fix: convert quoted string to boolean
                    let lower = s.to_lowercase();
                    if lower == "true" || lower == "false" {
                        let bool_str = if lower == "true" { "true" } else { "false" };
                        // Find the quoted value in the raw frontmatter
                        if let Some((start, end)) =
                            find_yaml_quoted_value_range(content, &parsed, "alwaysApply")
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                bool_str,
                                t!("rules.cur_008.fix", value = s.as_str(), fixed = bool_str),
                                true,
                            ));
                        }
                    }

                    diagnostics.push(diagnostic);
                }
            }

            // CUR-007: alwaysApply with redundant globs (WARNING)
            // When alwaysApply: true, globs are ignored
            if config.is_rule_enabled("CUR-007") {
                let is_always_apply = schema
                    .always_apply
                    .as_ref()
                    .and_then(|a| a.as_bool())
                    .unwrap_or(false);

                if is_always_apply && schema.globs.is_some() {
                    let globs_line = find_field_line(&parsed, "globs:");

                    let mut diagnostic = Diagnostic::warning(
                        path.to_path_buf(),
                        globs_line,
                        0,
                        "CUR-007",
                        t!("rules.cur_007.message"),
                    )
                    .with_suggestion(t!("rules.cur_007.suggestion"));

                    // Safe auto-fix: remove the entire globs block (key + indented children)
                    if let Some((start, end)) = yaml_block_byte_range(content, globs_line) {
                        diagnostic = diagnostic.with_fix(Fix::delete(
                            start,
                            end,
                            "Remove redundant globs field".to_string(),
                            true,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }

            // CUR-009: Missing description for agent-requested rule (WARNING)
            // If no alwaysApply and no globs, the rule is "agent-requested" and
            // the agent uses the description to decide when to apply it.
            if config.is_rule_enabled("CUR-009") {
                let has_always_apply = schema.always_apply.is_some();
                let has_globs = schema.globs.is_some();
                let has_description = !schema
                    .description
                    .as_deref()
                    .unwrap_or("")
                    .trim()
                    .is_empty();

                if !has_always_apply && !has_globs && !has_description {
                    diagnostics.push(
                        Diagnostic::warning(
                            path.to_path_buf(),
                            parsed.start_line,
                            0,
                            "CUR-009",
                            t!("rules.cur_009.message"),
                        )
                        .with_suggestion(t!("rules.cur_009.suggestion")),
                    );
                }
            }
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate_mdc(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/rules/typescript.mdc"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_legacy(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursorrules"), content, &LintConfig::default())
    }

    fn validate_mdc_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(".cursor/rules/typescript.mdc"), content, config)
    }

    fn validate_cursor_rule(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn validate_cursor_hooks(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/hooks.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_cursor_agent(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/agents/reviewer.md"),
            content,
            &LintConfig::default(),
        )
    }

    fn validate_cursor_environment(content: &str) -> Vec<Diagnostic> {
        let validator = CursorValidator;
        validator.validate(
            Path::new(".cursor/environment.json"),
            content,
            &LintConfig::default(),
        )
    }

    // ===== CUR-001: Empty Rule File =====

    #[test]
    fn test_cur_001_empty_mdc_file() {
        let diagnostics = validate_mdc("");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert_eq!(cur_001[0].level, DiagnosticLevel::Error);
        assert!(cur_001[0].message.contains("empty"));
    }

    #[test]
    fn test_cur_001_whitespace_only() {
        let diagnostics = validate_mdc("   \n\n\t  ");
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
    }

    #[test]
    fn test_cur_001_valid_mdc_file() {
        let content = r#"---
description: TypeScript rules
globs: "**/*.ts"
---
# TypeScript Rules

Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    #[test]
    fn test_cur_001_empty_body_after_frontmatter() {
        let content = r#"---
description: Empty body
globs: "**/*.ts"
---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert_eq!(cur_001.len(), 1);
        assert!(cur_001[0].message.contains("no content after frontmatter"));
    }

    #[test]
    fn test_cur_001_skips_when_parse_error() {
        // When frontmatter has parse error (missing closing ---),
        // CUR-001 should NOT trigger - CUR-003 handles it
        let content = r#"---
description: Unclosed frontmatter
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(
            cur_001.is_empty(),
            "CUR-001 should not trigger when parse_error exists"
        );

        // Verify CUR-003 triggers instead
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    // ===== CUR-002: Missing Frontmatter =====

    #[test]
    fn test_cur_002_missing_frontmatter() {
        let content = "# Rules without frontmatter";
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1);
        assert_eq!(cur_002[0].level, DiagnosticLevel::Warning);
        assert!(
            cur_002[0]
                .message
                .contains("missing recommended frontmatter")
        );
    }

    #[test]
    fn test_cur_002_has_autofix() {
        let content = "# Rules without frontmatter";
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert_eq!(cur_002.len(), 1);
        assert!(
            cur_002[0].has_fixes(),
            "CUR-002 should have auto-fix for missing frontmatter"
        );
        let fix = &cur_002[0].fixes[0];
        assert!(!fix.safe, "CUR-002 fix should be unsafe");
        assert_eq!(fix.start_byte, 0, "Fix should insert at start of file");
        assert_eq!(fix.end_byte, 0, "Fix should be an insert (start == end)");
        assert!(
            fix.replacement.contains("---\n"),
            "Fix should contain frontmatter markers"
        );
        assert!(
            fix.replacement.contains("description:"),
            "Fix should contain description field"
        );
        assert!(
            fix.replacement.contains("globs:"),
            "Fix should contain globs field"
        );
    }

    #[test]
    fn test_cur_002_no_autofix_for_empty() {
        // Empty files should not trigger CUR-002 (CUR-001 handles them)
        let diagnostics = validate_mdc("");
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert!(cur_002.is_empty());
    }

    #[test]
    fn test_cur_002_has_frontmatter() {
        let content = r#"---
description: Valid
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-002").collect();
        assert!(cur_002.is_empty());
    }

    // ===== CUR-003: Invalid YAML Frontmatter =====

    #[test]
    fn test_cur_003_invalid_yaml() {
        let content = r#"---
globs: [unclosed
---
# Body
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert_eq!(cur_003[0].level, DiagnosticLevel::Error);
        assert!(cur_003[0].message.contains("Invalid YAML"));
    }

    #[test]
    fn test_cur_003_unclosed_frontmatter() {
        let content = r#"---
description: Test
# Missing closing ---
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert_eq!(cur_003.len(), 1);
        assert!(cur_003[0].message.contains("missing closing ---"));
    }

    #[test]
    fn test_cur_003_valid_yaml() {
        let content = r#"---
description: Valid YAML
globs: "**/*.ts"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-003").collect();
        assert!(cur_003.is_empty());
    }

    // ===== CUR-004: Invalid Glob Pattern =====

    #[test]
    fn test_cur_004_invalid_glob() {
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert_eq!(cur_004[0].level, DiagnosticLevel::Error);
        assert!(cur_004[0].message.contains("Invalid glob pattern"));
    }

    #[test]
    fn test_cur_004_invalid_glob_in_array() {
        let content = r#"---
description: Some bad globs
globs:
  - "**/*.ts"
  - "[unclosed"
  - "**/*.js"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        assert!(cur_004[0].message.contains("[unclosed"));
    }

    #[test]
    fn test_cur_004_valid_glob_patterns() {
        let patterns = vec!["**/*.ts", "*.rs", "src/**/*.js", "tests/**/*.test.ts"];

        for pattern in patterns {
            let content = format!(
                r#"---
description: Test
globs: "{}"
---
# Rules
"#,
                pattern
            );
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_line_number_accuracy() {
        // Test that CUR-004 reports the line number of the globs field, not frontmatter start
        let content = r#"---
description: Bad glob
globs: "[unclosed"
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert_eq!(cur_004.len(), 1);
        // globs: is on line 3 (line 1 is ---, line 2 is description, line 3 is globs)
        assert_eq!(
            cur_004[0].line, 3,
            "CUR-004 should point to the globs field line"
        );
    }

    // ===== CUR-005: Unknown Frontmatter Keys =====

    #[test]
    fn test_cur_005_unknown_keys() {
        let content = r#"---
description: Valid key
unknownKey: value
anotherBadKey: 123
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert_eq!(cur_005.len(), 2);
        assert_eq!(cur_005[0].level, DiagnosticLevel::Warning);
        assert!(cur_005.iter().any(|d| d.message.contains("unknownKey")));
        assert!(cur_005.iter().any(|d| d.message.contains("anotherBadKey")));
        assert!(
            cur_005.iter().all(|d| d.has_fixes()),
            "All unknown key diagnostics should include safe deletion fixes"
        );
        assert!(cur_005.iter().all(|d| d.fixes[0].safe));
    }

    #[test]
    fn test_cur_005_no_unknown_keys() {
        let content = r#"---
description: Valid
globs: "**/*.rs"
alwaysApply: true
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(cur_005.is_empty());
    }

    // ===== CUR-006: Legacy .cursorrules =====

    #[test]
    fn test_cur_006_legacy_file() {
        let content = "# Legacy rules content";
        let diagnostics = validate_legacy(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert_eq!(cur_006.len(), 1);
        assert_eq!(cur_006[0].level, DiagnosticLevel::Warning);
        assert!(cur_006[0].message.contains("Legacy .cursorrules"));
        assert!(cur_006[0].message.contains("migrating"));
    }

    #[test]
    fn test_cur_006_legacy_empty() {
        let content = "";
        let diagnostics = validate_legacy(content);
        // Should have both CUR-006 (legacy warning) and CUR-001 (empty file)
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_mdc_file_no_cur_006() {
        // .mdc files should NOT trigger CUR-006
        let content = r#"---
description: Modern format
---
# Rules
"#;
        let diagnostics = validate_mdc(content);
        let cur_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-006").collect();
        assert!(cur_006.is_empty());
    }

    // ===== CUR-010 to CUR-016: Cursor hooks/agents/environment =====

    /// `version` is documented with a default of 1 and several of the doc's own
    /// examples omit it, so a bare `hooks` object is valid.
    #[test]
    fn test_cur_010_version_is_optional() {
        let diagnostics = validate_cursor_hooks(r#"{"hooks": {}}"#);
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-010"),
            "an omitted `version` defaults to 1 and must not be flagged, got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// A present-but-non-numeric `version` is still wrong. The field is typed
    /// "number", so `1.0` is fine and `"one"` is not.
    #[test]
    fn test_cur_010_version_must_be_numeric_when_present() {
        let bad = validate_cursor_hooks(r#"{"version":"one","hooks":{}}"#);
        assert!(
            bad.iter().any(|d| d.rule == "CUR-010"),
            "a string version must be flagged, got: {bad:?}"
        );

        let float_ok = validate_cursor_hooks(r#"{"version":1.0,"hooks":{}}"#);
        assert!(
            float_ok.iter().all(|d| d.rule != "CUR-010"),
            "`version` is typed number, so 1.0 is valid, got: {float_ok:?}"
        );
    }

    #[test]
    fn test_cur_010_hooks_json_parse_error() {
        let diagnostics = validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":[}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hooks_root_must_be_object() {
        let diagnostics = validate_cursor_hooks(r#"["not", "an", "object"]"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hooks_field_must_be_object() {
        let diagnostics = validate_cursor_hooks(r#"{"version":1,"hooks":[]}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_event_hooks_must_be_array() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":{"type":"command","command":"echo hi"}}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_010_hook_entries_must_be_objects() {
        let diagnostics =
            validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":["echo hi"]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-010"));
    }

    #[test]
    fn test_cur_011_unknown_hook_event() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"unknownEvent":[{"type":"command","command":"echo hi"}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-011"));
    }

    #[test]
    fn test_cur_011_workspace_open_event() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"workspaceOpen":[{"type":"command","command":"echo open"}]}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-011"));
    }

    #[test]
    fn test_cur_012_missing_hook_command() {
        let diagnostics =
            validate_cursor_hooks(r#"{"version":1,"hooks":{"sessionStart":[{"type":"command"}]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-012"));
    }

    #[test]
    fn test_cur_012_command_must_be_non_empty_string() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":""}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-012"));
    }

    #[test]
    fn test_cur_013_invalid_hook_type() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"agent","command":"echo hi"}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-013"));
    }

    #[test]
    fn test_cur_014_invalid_cursor_agent_frontmatter() {
        let content = r#"---
name: ReviewerAgent
description: 123
readonly: "true"
---
Review code changes."#;
        let diagnostics = validate_cursor_agent(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-014"));
    }

    #[test]
    fn test_cur_014_malformed_yaml_reports_once() {
        let diagnostics = validate_cursor_agent(
            r#"---
name: reviewer-agent
description: [unclosed
---
Review changes."#,
        );
        let cur_014: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-014").collect();
        assert_eq!(cur_014.len(), 1);
    }

    #[test]
    fn test_cur_014_missing_optional_fields_no_error() {
        let diagnostics = validate_cursor_agent(
            r#"---
model: fast
---
Review changes."#,
        );
        assert!(
            !diagnostics.iter().any(|d| d.rule == "CUR-014"),
            "Missing name/description should not trigger CUR-014 (both are optional)",
        );
    }

    #[test]
    fn test_cur_014_accepts_parameterized_model_ids() {
        for model in [
            "composer-2.5[]",
            "composer-2.5[fast=false]",
            "claude-opus-5[effort=high]",
            "claude-opus-5[effort=high,context=300k]",
        ] {
            let content = format!("---\nmodel: {model}\n---\nReview changes.");
            let diagnostics = validate_cursor_agent(&content);
            assert!(
                diagnostics.iter().all(|d| d.rule != "CUR-014"),
                "documented model ID '{model}' must validate, got: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cur_014_rejects_malformed_parameterized_model_ids() {
        for model in [
            "composer-2.5[fast]",
            "composer-2.5[=false]",
            "composer-2.5[fast=]",
            "composer-2.5[fast=false,]",
            "composer-2.5[fast=false",
            "\" composer-2.5[] \"",
        ] {
            let content = format!("---\nmodel: {model}\n---\nReview changes.");
            let diagnostics = validate_cursor_agent(&content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "CUR-014"),
                "malformed model ID '{model}' must be rejected, got: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cur_015_empty_cursor_agent_body() {
        let content = r#"---
name: reviewer-agent
description: Reviews pull requests
model: fast
readonly: true
is_background: false
---
"#;
        let diagnostics = validate_cursor_agent(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-015"));
    }

    #[test]
    fn test_cur_016_invalid_environment_schema() {
        let diagnostics =
            validate_cursor_environment(r#"{"install":42,"terminals":[{"name":"main"}]}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_parse_error() {
        let diagnostics = validate_cursor_environment(r#"{"install":}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_accepts_json_comments() {
        let diagnostics = validate_cursor_environment(
            r#"{
  // Cursor's schema explicitly allows comments.
  "install": "npm ci",
  /* Keep the dev server running for the agent. */
  "start": "npm run dev"
}"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-016"),
            "comments are valid in environment.json, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_cur_016_environment_rejects_trailing_comma() {
        let diagnostics = validate_cursor_environment(
            r#"{
  "install": "npm ci",
}"#,
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "Cursor's environment schema disallows trailing commas"
        );
    }

    #[test]
    fn test_cur_016_environment_rejects_unterminated_json_comment() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci"} /*"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "an unterminated block comment must remain invalid JSONC"
        );
    }

    #[test]
    fn test_cur_016_comment_removal_does_not_join_json_tokens() {
        let diagnostics = validate_cursor_environment(r#"{"agentCanUpdateSnapshot":tr/*x*/ue}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "comments must not turn adjacent invalid tokens into a valid value"
        );
    }

    #[test]
    fn test_cur_016_environment_diagnostic_points_to_field() {
        let diagnostics = validate_cursor_environment(
            r#"{
  // Keep diagnostics aligned with the original JSONC input.
  "install": "npm ci",
  "start": 42
}"#,
        );
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "CUR-016" && d.message.contains("start"))
            .expect("invalid start field should be reported");
        assert_eq!((diagnostic.line, diagnostic.column), (4, 3));
    }

    #[test]
    fn test_cur_016_field_location_handles_escaped_key_and_comment_before_colon() {
        let diagnostics =
            validate_cursor_environment("{\n  \"st\\u0061rt\" /* documented JSONC */ : 42\n}");
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "CUR-016" && d.message.contains("start"))
            .expect("escaped start field should be reported");
        assert_eq!((diagnostic.line, diagnostic.column), (2, 3));
    }

    #[test]
    fn test_cur_016_root_field_location_skips_matching_nested_key() {
        let diagnostics = validate_cursor_environment(
            r#"{
  "terminals": [
    {"name": "nested", "command": "npm test"}
  ],
  "name": 42
}"#,
        );
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "CUR-016" && d.message.contains("Field 'name'"))
            .expect("invalid root name field should be reported");
        assert_eq!((diagnostic.line, diagnostic.column), (5, 3));
    }

    #[test]
    fn test_cur_016_root_field_location_uses_last_duplicate_key() {
        let diagnostics = validate_cursor_environment(
            r#"{
  "start": "npm test",
  "start": 42
}"#,
        );
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "CUR-016" && d.message.contains("start"))
            .expect("invalid final start field should be reported");
        assert_eq!((diagnostic.line, diagnostic.column), (3, 3));
    }

    #[test]
    fn test_cur_016_root_field_location_counts_unicode_characters() {
        let diagnostics = validate_cursor_environment(r#"{"name":"é","start":42}"#);
        let diagnostic = diagnostics
            .iter()
            .find(|d| d.rule == "CUR-016" && d.message.contains("start"))
            .expect("invalid start field should be reported");
        assert_eq!((diagnostic.line, diagnostic.column), (1, 13));
    }

    #[test]
    fn test_cur_016_environment_root_must_be_object() {
        let diagnostics = validate_cursor_environment(r#"["install","terminals"]"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    /// `install` appears in no `required` array in the published environment
    /// schema, and the setup page's snapshot example omits it, so absence is
    /// valid. Only a present-but-wrong type is reported.
    #[test]
    fn test_cur_016_environment_install_is_optional() {
        let diagnostics = validate_cursor_environment(r#"{"start":"npm run dev"}"#);
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-016"),
            "an omitted `install` is valid per the schema, got: {:?}",
            diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
        );

        // The doc's own snapshot form must also validate clean.
        let snapshot = validate_cursor_environment(
            r#"{"snapshot":"snap_abc123","start":"sudo service docker start"}"#,
        );
        assert!(
            snapshot.iter().all(|d| d.rule != "CUR-016"),
            "the documented snapshot example must validate, got: {snapshot:?}"
        );
    }

    #[test]
    fn test_cur_016_environment_terminals_optional() {
        // terminals is now optional - no error without it
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci"}"#);
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-016"),
            "Missing terminals should not trigger CUR-016, got: {:?}",
            diagnostics
                .iter()
                .map(|d| (&d.rule, &d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cur_016_environment_invalid_terminals_type() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci","terminals":{}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_start_must_be_string() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci","start":42}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    /// `update` is reported regardless of its type now, since the key itself is
    /// no longer in the schema. Kept as a distinct case from the string form so
    /// the rename notice is not accidentally narrowed to one value type later.
    fn test_cur_016_environment_update_reported_whatever_its_type() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci","update":42}"#);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CUR-016" && d.message.contains("renamed")),
            "got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_cur_016_environment_build_must_be_object() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci","build":"invalid"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-016"));
    }

    #[test]
    fn test_cur_016_environment_build_fields_must_be_strings() {
        let diagnostics = validate_cursor_environment(
            r#"{"install":"npm ci","build":{"dockerfile":42,"context":true}}"#,
        );
        let cur_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-016").collect();
        assert!(
            cur_016.len() >= 2,
            "Expected errors for both build.dockerfile and build.context"
        );
    }

    #[test]
    fn test_cur_016_environment_valid_build() {
        let diagnostics = validate_cursor_environment(
            r#"{"install":"npm ci","build":{"dockerfile":"Dockerfile","context":".."}}"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-016"),
            "Valid build should not trigger CUR-016, got: {:?}",
            diagnostics
                .iter()
                .map(|d| (&d.rule, &d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cur_016_environment_install_null() {
        let diagnostics = validate_cursor_environment(r#"{"install":null}"#);
        let cur_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-016").collect();
        assert!(
            cur_016
                .iter()
                .any(|d| d.message.contains("must be a string")),
            "install: null should trigger the install message, got: {:?}",
            cur_016.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    /// `update` is absent from the published schema and the root sets
    /// `unevaluatedProperties: false`, so it is invalid rather than optional.
    /// The setup page calls it the former name of the install script, so it is
    /// reported as a rename rather than silently accepted.
    #[test]
    fn test_cur_016_environment_update_reported_as_renamed() {
        let diagnostics =
            validate_cursor_environment(r#"{"install":"npm ci","update":"apt-get update"}"#);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CUR-016" && d.message.contains("renamed")),
            "`update` must be reported as renamed to `install`, got: {:?}",
            diagnostics
                .iter()
                .map(|d| (&d.rule, &d.message))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cur_016_environment_terminal_non_object() {
        let diagnostics =
            validate_cursor_environment(r#"{"install":"npm ci","terminals":[42,"string"]}"#);
        let cur_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-016").collect();
        assert!(
            cur_016.len() >= 2,
            "Expected at least 2 CUR-016 errors for non-object terminal entries, got {}",
            cur_016.len()
        );
    }

    #[test]
    fn test_cur_016_environment_build_dockerfile_invalid() {
        let diagnostics =
            validate_cursor_environment(r#"{"install":"npm ci","build":{"dockerfile":42}}"#);
        let cur_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-016").collect();
        assert_eq!(
            cur_016.len(),
            1,
            "Expected exactly 1 CUR-016 error for invalid build.dockerfile, got: {:?}",
            cur_016.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cur_016_environment_build_context_invalid() {
        let diagnostics = validate_cursor_environment(
            r#"{"install":"npm ci","build":{"dockerfile":"Dockerfile","context":true}}"#,
        );
        let cur_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-016").collect();
        assert_eq!(
            cur_016.len(),
            1,
            "Expected exactly 1 CUR-016 error for invalid build.context, got: {:?}",
            cur_016.iter().map(|d| &d.message).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_cur_016_environment_snapshot_must_be_string() {
        let diagnostics = validate_cursor_environment(r#"{"install":"npm ci","snapshot":42}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "the published schema types snapshot as a string, got: {diagnostics:?}"
        );
    }

    #[test]
    fn test_cur_016_environment_validates_full_published_schema() {
        let valid = validate_cursor_environment(
            r#"{
  "$schema": "https://cursor.com/schemas/environment.schema.json",
  "name": "development",
  "user": "ubuntu",
  "install": "npm ci",
  "start": "npm run dev",
  "repositoryDependencies": ["github.com/acme/shared"],
  "ports": [{"name": "web", "port": 3000}],
  "terminals": [{"name": "app", "command": "npm run dev", "description": "dev server"}],
  "build": {"dockerfile": "Dockerfile", "context": ".."},
  "snapshot": "snap_abc123",
  "agentCanUpdateSnapshot": true
}"#,
        );
        assert!(
            valid.iter().all(|d| d.rule != "CUR-016"),
            "the full documented schema must validate, got: {valid:?}"
        );

        for (name, content) in [
            ("unknown root field", r#"{"unknown":true}"#),
            ("invalid schema association", r#"{"$schema":42}"#),
            ("invalid name", r#"{"name":1}"#),
            ("invalid user", r#"{"user":false}"#),
            (
                "invalid repository dependency",
                r#"{"repositoryDependencies":["github.com/acme/shared",42]}"#,
            ),
            ("invalid ports container", r#"{"ports":{}}"#),
            ("missing port", r#"{"ports":[{"name":"web"}]}"#),
            ("out-of-range port", r#"{"ports":[{"port":70000}]}"#),
            ("invalid port name", r#"{"ports":[{"name":1,"port":3000}]}"#),
            ("missing dockerfile", r#"{"build":{"context":".."}}"#),
            (
                "unknown build field",
                r#"{"build":{"dockerfile":"Dockerfile","target":"dev"}}"#,
            ),
            (
                "invalid terminal name",
                r#"{"terminals":[{"name":1,"command":"npm run dev"}]}"#,
            ),
            (
                "invalid terminal description",
                r#"{"terminals":[{"command":"npm run dev","description":1}]}"#,
            ),
            ("invalid snapshot", r#"{"snapshot":false}"#),
            (
                "invalid snapshot update flag",
                r#"{"agentCanUpdateSnapshot":"yes"}"#,
            ),
        ] {
            let diagnostics = validate_cursor_environment(content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "CUR-016"),
                "{name} must be rejected, got: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cursor_hooks_agents_environment_valid() {
        let hooks =
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo start"}]}}"#;
        let agent = r#"---
name: reviewer-agent
description: Reviews code quality
model: fast
readonly: true
is_background: false
---
Review the diff and suggest improvements."#;
        let environment = r#"{
  "install": "npm ci",
  "start": "npm run dev",
  "build": {"dockerfile": "Dockerfile", "context": ".."},
  "terminals": [{"name": "app", "command": "npm run dev"}]
}"#;

        assert!(
            !validate_cursor_hooks(hooks)
                .iter()
                .any(|d| d.rule.starts_with("CUR-01")),
            "Valid hooks fixture should not trigger CUR-010..CUR-016",
        );
        assert!(
            !validate_cursor_agent(agent)
                .iter()
                .any(|d| d.rule == "CUR-014" || d.rule == "CUR-015"),
            "Valid cursor agent fixture should not trigger CUR-014/CUR-015",
        );
        assert!(
            !validate_cursor_environment(environment)
                .iter()
                .any(|d| d.rule == "CUR-016"),
            "Valid environment fixture should not trigger CUR-016",
        );
    }

    // ===== CUR-011 auto-fix tests =====

    #[test]
    fn test_cur_011_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content =
            r#"{"version":1,"hooks":{"SessionStart":[{"type":"command","command":"echo hi"}]}}"#;
        let diagnostics = validate_cursor_hooks(content);
        let cur_011: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-011").collect();
        assert_eq!(cur_011.len(), 1);
        assert!(
            cur_011[0].has_fixes(),
            "CUR-011 should have auto-fix for case-mismatched hook event"
        );
        let fix = &cur_011[0].fixes[0];
        assert!(!fix.safe, "CUR-011 fix should be unsafe");
        assert!(
            fix.replacement.contains("sessionStart"),
            "Fix should suggest closest valid event name, got: {}",
            fix.replacement
        );
    }

    // ===== CUR-013 auto-fix tests =====

    #[test]
    fn test_cur_013_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content =
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"Command","command":"echo hi"}]}}"#;
        let diagnostics = validate_cursor_hooks(content);
        let cur_013: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-013").collect();
        assert_eq!(cur_013.len(), 1);
        assert!(
            cur_013[0].has_fixes(),
            "CUR-013 should have auto-fix for case-mismatched hook type"
        );
        let fix = &cur_013[0].fixes[0];
        assert!(!fix.safe, "CUR-013 fix should be unsafe");
        assert_eq!(
            fix.replacement, "command",
            "Fix should suggest 'command' as closest match"
        );
    }

    // ===== CUR-017: Invalid hook entry field types =====

    #[test]
    fn test_cur_017_timeout_must_be_positive_number() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","timeout":"slow"}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert_eq!(cur_017.len(), 1);
        assert_eq!(cur_017[0].level, DiagnosticLevel::Warning);
        assert!(cur_017[0].message.contains("timeout"));
    }

    #[test]
    fn test_cur_017_timeout_zero_is_invalid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","timeout":0}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert_eq!(cur_017.len(), 1);
        assert!(cur_017[0].message.contains("timeout"));
    }

    #[test]
    fn test_cur_017_timeout_negative_is_invalid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","timeout":-5}]}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-017"));
    }

    #[test]
    fn test_cur_017_timeout_positive_is_valid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","timeout":5000}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert!(cur_017.is_empty());
    }

    #[test]
    fn test_cur_017_loop_limit_must_be_number_or_null() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","loop_limit":"many"}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert_eq!(cur_017.len(), 1);
        assert!(cur_017[0].message.contains("loop_limit"));
    }

    #[test]
    fn test_cur_017_loop_limit_null_is_valid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","loop_limit":null}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert!(cur_017.is_empty());
    }

    #[test]
    fn test_cur_017_loop_limit_number_is_valid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","loop_limit":3}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert!(cur_017.is_empty());
    }

    #[test]
    fn test_cur_017_fail_closed_must_be_boolean() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","failClosed":"yes"}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert_eq!(cur_017.len(), 1);
        assert!(cur_017[0].message.contains("failClosed"));
    }

    #[test]
    fn test_cur_017_fail_closed_boolean_is_valid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","failClosed":true}]}}"#,
        );
        let cur_017: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-017").collect();
        assert!(cur_017.is_empty());
    }

    // ===== CUR-018: Prompt-type hook missing prompt field =====

    #[test]
    fn test_cur_018_prompt_type_missing_prompt_field() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi"}]}}"#,
        );
        let cur_018: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-018").collect();
        assert_eq!(cur_018.len(), 1);
        assert_eq!(cur_018[0].level, DiagnosticLevel::Warning);
        assert!(cur_018[0].message.contains("prompt"));
    }

    #[test]
    fn test_cur_018_prompt_type_with_prompt_field() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi","prompt":"Summarize the changes"}]}}"#,
        );
        let cur_018: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-018").collect();
        assert!(cur_018.is_empty());
    }

    #[test]
    fn test_cur_018_prompt_must_be_non_empty_string() {
        for prompt in ["null", "42", "\"\"", "\"   \""] {
            let content = format!(
                r#"{{"hooks":{{"beforeShellExecution":[{{"type":"prompt","prompt":{prompt}}}]}}}}"#
            );
            let diagnostics = validate_cursor_hooks(&content);
            assert!(
                diagnostics.iter().any(|d| d.rule == "CUR-018"),
                "invalid prompt value {prompt} must be rejected, got: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cur_018_command_type_no_prompt_no_warning() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi"}]}}"#,
        );
        let cur_018: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-018").collect();
        assert!(cur_018.is_empty());
    }

    // ===== CUR-019: Invalid model field on prompt hook =====

    #[test]
    fn test_cur_019_model_must_be_string() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi","prompt":"test","model":123}]}}"#,
        );
        let cur_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-019").collect();
        assert_eq!(cur_019.len(), 1);
        assert_eq!(cur_019[0].level, DiagnosticLevel::Info);
        assert!(cur_019[0].message.contains("model"));
    }

    #[test]
    fn test_cur_019_model_string_is_valid() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi","prompt":"test","model":"gpt-4"}]}}"#,
        );
        let cur_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-019").collect();
        assert!(cur_019.is_empty());
    }

    #[test]
    fn test_cur_019_model_on_command_type_no_warning() {
        // CUR-019 only applies to prompt-type hooks
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","model":123}]}}"#,
        );
        let cur_019: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-019").collect();
        assert!(cur_019.is_empty());
    }

    // ===== CUR-020: Ignored .md project rule =====

    #[test]
    fn test_cur_020_plain_markdown_rule_is_ignored_by_cursor() {
        for path in [".cursor/rules/typescript.md", ".cursor/rules/typescript.MD"] {
            let diagnostics = validate_cursor_rule(
                path,
                "---\ndescription: TypeScript rules\n---\nUse strict mode.",
            );
            assert!(
                diagnostics.iter().any(|d| d.rule == "CUR-020"),
                "Cursor ignores plain Markdown files in .cursor/rules, got: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cur_020_does_not_flag_legacy_or_unrelated_markdown() {
        for path in [".cursorrules", ".cursorrules.md", "docs/typescript.md"] {
            let diagnostics = validate_cursor_rule(path, "Use strict mode.");
            assert!(
                diagnostics.iter().all(|d| d.rule != "CUR-020"),
                "CUR-020 must stay scoped to .cursor/rules Markdown: {path}: {diagnostics:?}"
            );
        }
    }

    #[test]
    fn test_cur_020_mdc_rule_is_recognized() {
        for path in [
            ".cursor/rules/typescript.mdc",
            ".cursor/rules/typescript.MDC",
        ] {
            let diagnostics = validate_cursor_rule(
                path,
                "---\ndescription: TypeScript rules\n---\nUse strict mode.",
            );
            assert!(diagnostics.iter().all(|d| d.rule != "CUR-020"));
        }
    }

    #[test]
    fn test_cur_020_disabled_does_not_validate_ignored_markdown_as_mdc() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CUR-020".to_string()];
        let validator = CursorValidator;
        let diagnostics = validator.validate(
            Path::new(".cursor/rules/typescript.md"),
            "Plain Markdown without MDC frontmatter",
            &config,
        );
        assert!(
            diagnostics.is_empty(),
            "ignored .md rules must not fall through to MDC validation: {diagnostics:?}"
        );
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_cursor_category() {
        let mut config = LintConfig::default();
        config.rules_mut().cursor = false;

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("CUR-"))
            .collect();
        assert!(cur_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CUR-001".to_string()];

        let content = "";
        let diagnostics = validate_mdc_with_config(content, &config);

        let cur_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-001").collect();
        assert!(cur_001.is_empty());
    }

    // ===== Combined Issues =====

    #[test]
    fn test_multiple_issues() {
        let content = r#"---
unknownKey: value
---
"#;
        let diagnostics = validate_mdc(content);

        // Should have:
        // - CUR-001 for empty body
        // - CUR-005 for unknown key
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-001"),
            "Expected CUR-001"
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-005"),
            "Expected CUR-005"
        );
    }

    #[test]
    fn test_valid_mdc_no_issues() {
        let content = r#"---
description: TypeScript Guidelines
globs: "**/*.ts"
alwaysApply: false
---
# TypeScript Guidelines

Always use strict mode and explicit types.
"#;
        let diagnostics = validate_mdc(content);
        let errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Error)
            .collect();
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ===== Additional CUR rule tests =====

    #[test]
    fn test_cur_001_newlines_only() {
        let content = "\n\n\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_001_frontmatter_only() {
        // File with just frontmatter and no body
        let content = "---\ndescription: test\n---\n";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-001"));
    }

    #[test]
    fn test_cur_002_no_frontmatter_in_cursorrules() {
        // .cursorrules should NOT require frontmatter (CUR-002 is for .mdc files)
        let content = "Just plain text rules without frontmatter.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-002"));
    }

    #[test]
    fn test_cur_003_yaml_with_tabs() {
        // YAML doesn't allow tabs for indentation
        let content = "---\n\tdescription: test\n---\nBody";
        let diagnostics = validate_mdc(content);
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-003"));
    }

    #[test]
    fn test_cur_004_all_valid_patterns() {
        let valid_patterns = ["**/*.ts", "*.rs", "src/**/*.py", "{src,lib}/**/*.tsx"];

        for pattern in valid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(cur_004.is_empty(), "Pattern '{}' should be valid", pattern);
        }
    }

    #[test]
    fn test_cur_004_invalid_patterns() {
        let invalid_patterns = ["[invalid", "***", "**["];

        for pattern in invalid_patterns {
            let content = format!("---\nglobs: \"{}\"\n---\nBody", pattern);
            let diagnostics = validate_mdc(&content);
            let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(
                !cur_004.is_empty(),
                "Pattern '{}' should be invalid",
                pattern
            );
        }
    }

    #[test]
    fn test_cur_004_globs_as_array() {
        let content = r#"---
globs:
  - "**/*.ts"
  - "**/*.tsx"
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
        assert!(cur_004.is_empty(), "Array globs should be valid");
    }

    #[test]
    fn test_cur_005_all_known_keys() {
        let content = r#"---
description: Test rule
globs: "**/*.ts"
alwaysApply: false
---
Body"#;
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-005"));
    }

    #[test]
    fn test_cur_005_multiple_unknown_keys() {
        let content = r#"---
description: test
unknownKey1: value1
unknownKey2: value2
---
Body"#;
        let diagnostics = validate_mdc(content);
        let cur_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-005").collect();
        assert!(!cur_005.is_empty());
    }

    #[test]
    fn test_cur_006_legacy_file_with_content() {
        let content = "Some legacy cursor rules.";
        let validator = CursorValidator;
        let diagnostics =
            validator.validate(Path::new(".cursorrules"), content, &LintConfig::default());
        assert!(diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_mdc_file_no_cur_006_warning() {
        // .mdc files should not trigger CUR-006 (legacy warning)
        let content = "---\ndescription: test\n---\nRules content";
        let diagnostics = validate_mdc(content);
        assert!(!diagnostics.iter().any(|d| d.rule == "CUR-006"));
    }

    #[test]
    fn test_all_cur_rules_can_be_disabled() {
        let rules = [
            "CUR-001", "CUR-002", "CUR-003", "CUR-004", "CUR-005", "CUR-006", "CUR-007", "CUR-008",
            "CUR-009", "CUR-010", "CUR-011", "CUR-012", "CUR-013", "CUR-014", "CUR-015", "CUR-016",
            "CUR-017", "CUR-018", "CUR-019", "CUR-020",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            // Content that could trigger each rule
            let (content, path) = match rule {
                "CUR-001" => ("", ".cursor/rules/test.mdc"),
                "CUR-006" => ("content", ".cursorrules"),
                "CUR-007" => (
                    "---\nalwaysApply: true\nglobs: \"**/*.ts\"\n---\nBody",
                    ".cursor/rules/test.mdc",
                ),
                "CUR-008" => (
                    "---\nalwaysApply: \"true\"\n---\nBody",
                    ".cursor/rules/test.mdc",
                ),
                "CUR-009" => ("---\n\n---\nBody", ".cursor/rules/test.mdc"),
                "CUR-010" => ("{}", ".cursor/hooks.json"),
                "CUR-011" => (
                    r#"{"version":1,"hooks":{"unknownEvent":[{"type":"command","command":"echo hi"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-012" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"command"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-013" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"agent","command":"echo hi"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-014" => (
                    "---\nname: BadName\ndescription: 1\nreadonly: \"true\"\n---\nbody",
                    ".cursor/agents/reviewer.md",
                ),
                "CUR-015" => (
                    "---\nname: reviewer-agent\ndescription: test\n---\n",
                    ".cursor/agents/reviewer.md",
                ),
                "CUR-016" => ("{}", ".cursor/environment.json"),
                "CUR-017" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"command","command":"echo hi","timeout":"slow"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-018" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi"}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-019" => (
                    r#"{"version":1,"hooks":{"sessionStart":[{"type":"prompt","command":"echo hi","prompt":"test","model":123}]}}"#,
                    ".cursor/hooks.json",
                ),
                "CUR-020" => (
                    "---\ndescription: ignored\n---\nBody",
                    ".cursor/rules/test.md",
                ),
                _ => ("---\nunknown: value\n---\n", ".cursor/rules/test.mdc"),
            };

            let validator = CursorValidator;
            let diagnostics = validator.validate(Path::new(path), content, &config);

            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== CUR-007: alwaysApply with redundant globs =====

    #[test]
    fn test_cur_007_always_apply_with_globs() {
        let content = r#"---
description: TypeScript rules
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert_eq!(cur_007[0].level, DiagnosticLevel::Warning);
        assert!(cur_007[0].message.contains("redundant"));
    }

    #[test]
    fn test_cur_007_always_apply_with_globs_array() {
        let content = r#"---
alwaysApply: true
globs:
  - "**/*.ts"
  - "**/*.tsx"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
    }

    #[test]
    fn test_cur_007_always_apply_false_with_globs() {
        // alwaysApply: false with globs should NOT trigger CUR-007
        let content = r#"---
description: TypeScript rules
alwaysApply: false
globs: "**/*.ts"
---
# Rules
Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert!(cur_007.is_empty());
    }

    #[test]
    fn test_cur_007_always_apply_without_globs() {
        // alwaysApply: true without globs should NOT trigger CUR-007
        let content = r#"---
description: Global rules
alwaysApply: true
---
# Rules
Always apply these.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert!(cur_007.is_empty());
    }

    #[test]
    fn test_cur_007_has_autofix() {
        let content = r#"---
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert!(cur_007[0].has_fixes(), "CUR-007 should include an auto-fix");
        assert!(cur_007[0].fixes[0].safe, "CUR-007 fix should be safe");
    }

    #[test]
    fn test_cur_007_line_number_accuracy() {
        let content = r#"---
description: Test
alwaysApply: true
globs: "**/*.ts"
---
# Rules
Body.
"#;
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        // globs is on line 4 (line 1 is ---, line 2 is description, line 3 is alwaysApply, line 4 is globs)
        assert_eq!(
            cur_007[0].line, 4,
            "CUR-007 should point to the globs field line"
        );
    }

    #[test]
    fn test_cur_007_autofix_deletes_array_globs_block() {
        // When globs is in array form, the auto-fix must delete the entire block
        // (key line + indented list items), not just the key line
        let content = "---\nalwaysApply: true\nglobs:\n  - \"**/*.ts\"\n  - \"**/*.tsx\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-007").collect();
        assert_eq!(cur_007.len(), 1);
        assert!(cur_007[0].has_fixes());

        // Apply the fix and verify the result is valid YAML
        let fix = &cur_007[0].fixes[0];
        let fixed = format!("{}{}", &content[..fix.start_byte], &content[fix.end_byte..]);
        // The fixed content should not contain any globs-related lines
        assert!(
            !fixed.contains("globs"),
            "Fix should remove entire globs block, got: {:?}",
            fixed
        );
        assert!(
            !fixed.contains("**/*.ts"),
            "Fix should remove globs list items, got: {:?}",
            fixed
        );
    }

    // ===== CUR-008: Invalid alwaysApply type =====

    #[test]
    fn test_cur_008_string_true() {
        let content = r#"---
description: Test
alwaysApply: "true"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert_eq!(cur_008[0].level, DiagnosticLevel::Error);
        assert!(cur_008[0].message.contains("boolean"));
    }

    #[test]
    fn test_cur_008_string_false() {
        let content = r#"---
description: Test
alwaysApply: "false"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
    }

    #[test]
    fn test_cur_008_boolean_true() {
        // Proper boolean should NOT trigger CUR-008
        let content = r#"---
description: Test
alwaysApply: true
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert!(cur_008.is_empty());
    }

    #[test]
    fn test_cur_008_boolean_false() {
        // Proper boolean should NOT trigger CUR-008
        let content = r#"---
description: Test
alwaysApply: false
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert!(cur_008.is_empty());
    }

    #[test]
    fn test_cur_008_line_number_accuracy() {
        let content = r#"---
description: Test
alwaysApply: "true"
---
# Rules
Body.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert_eq!(
            cur_008[0].line, 3,
            "CUR-008 should point to the alwaysApply field line"
        );
    }

    #[test]
    fn test_cur_008_arbitrary_string() {
        // alwaysApply with arbitrary string should also trigger
        let content = r#"---
description: Test
alwaysApply: "yes"
---
# Rules
Body content.
"#;
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        // Arbitrary string "yes" should NOT get a fix (not "true" or "false")
        assert!(
            !cur_008[0].has_fixes(),
            "CUR-008 should not auto-fix arbitrary strings"
        );
    }

    #[test]
    fn test_cur_008_autofix_string_true() {
        let content = "---\ndescription: Test\nalwaysApply: \"true\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert!(
            cur_008[0].has_fixes(),
            "CUR-008 should have auto-fix for \"true\""
        );
        let fix = &cur_008[0].fixes[0];
        assert!(fix.safe, "CUR-008 fix should be safe");
        assert_eq!(
            fix.replacement, "true",
            "Fix should convert to unquoted boolean true"
        );
        // Verify the fix targets the quoted value
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "\"true\"", "Fix should target the quoted string");
    }

    #[test]
    fn test_cur_008_autofix_string_false() {
        let content = "---\ndescription: Test\nalwaysApply: \"false\"\n---\n# Rules\nBody.\n";
        let diagnostics = validate_mdc(content);
        let cur_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-008").collect();
        assert_eq!(cur_008.len(), 1);
        assert!(
            cur_008[0].has_fixes(),
            "CUR-008 should have auto-fix for \"false\""
        );
        let fix = &cur_008[0].fixes[0];
        assert_eq!(
            fix.replacement, "false",
            "Fix should convert to unquoted boolean false"
        );
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "\"false\"", "Fix should target the quoted string");
    }

    // ===== CUR-009: Missing description for agent-requested rule =====

    #[test]
    fn test_cur_009_no_description_no_globs_no_always_apply() {
        let content = r#"---

---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
        assert_eq!(cur_009[0].level, DiagnosticLevel::Warning);
        assert!(cur_009[0].message.contains("description"));
    }

    #[test]
    fn test_cur_009_empty_description() {
        let content = r#"---
description: ""
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
    }

    #[test]
    fn test_cur_009_whitespace_only_description() {
        let content = r#"---
description: "   "
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert_eq!(cur_009.len(), 1);
    }

    #[test]
    fn test_cur_009_with_description() {
        // Has description but no globs or alwaysApply - should NOT trigger
        let content = r#"---
description: TypeScript coding standards
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_globs() {
        // Has globs but no description - should NOT trigger (not agent-requested)
        let content = r#"---
globs: "**/*.ts"
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_always_apply() {
        // Has alwaysApply but no description - should NOT trigger (not agent-requested)
        let content = r#"---
alwaysApply: true
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }

    #[test]
    fn test_cur_009_with_always_apply_false() {
        // alwaysApply: false counts as "has alwaysApply" - should NOT trigger
        let content = r#"---
alwaysApply: false
---
# Rules
Some content here.
"#;
        let diagnostics = validate_mdc(content);
        let cur_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-009").collect();
        assert!(cur_009.is_empty());
    }
    /// The scalar `globs` form carries several patterns: "Separate multiple
    /// patterns with commas". Validating the whole string was intermittently
    /// wrong - the doc's own row happens to parse, `src/**, tests/**` does not.
    #[test]
    fn test_cur_004_accepts_comma_separated_globs() {
        for globs in [
            "docs/**/*.md, docs/**/*.mdx",
            "src/**, tests/**",
            "app/**,lib/**",
        ] {
            let content = format!(
                r#"---
description: Test
globs: {globs}
alwaysApply: false
---

Use strict mode.
"#
            );
            let diagnostics = validate_mdc(&content);
            let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-004").collect();
            assert!(
                hits.is_empty(),
                "comma-separated globs '{globs}' must validate, got: {hits:?}"
            );
        }
    }

    /// An invalid segment inside a comma list is still reported, and named.
    #[test]
    fn test_cur_004_still_flags_invalid_segment() {
        let content = r#"---
description: Test
globs: src/**, [unclosed
alwaysApply: false
---

Use strict mode.
"#;
        let diagnostics = validate_mdc(content);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CUR-004" && d.message.contains("[unclosed")),
            "the bad segment must be named, got: {diagnostics:?}"
        );
    }

    /// Prompt hooks have no `command` - the doc's example carries only `type`,
    /// `prompt` and `timeout`.
    #[test]
    fn test_cur_012_prompt_hooks_need_no_command() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"beforeShellExecution":[{"type":"prompt","prompt":"Does this command look safe to execute?","timeout":10}]}}"#,
        );
        let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "CUR-012").collect();
        assert!(
            hits.is_empty(),
            "the documented prompt-hook form must validate, got: {hits:?}"
        );
    }

    /// A command-type hook still requires `command`.
    #[test]
    fn test_cur_012_command_hooks_still_require_command() {
        let diagnostics = validate_cursor_hooks(
            r#"{"version":1,"hooks":{"stop":[{"type":"command","timeout":30}]}}"#,
        );
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-012"),
            "a command hook without `command` must still be flagged, got: {diagnostics:?}"
        );
    }

    /// Terminal entries require only `command`; `name` and `description` are
    /// optional per the published schema's `oneOf` branches.
    #[test]
    fn test_cur_016_terminal_name_is_optional() {
        let diagnostics = validate_cursor_environment(
            r#"{"terminals":[{"command":"npm run dev","description":"dev server"}]}"#,
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "CUR-016"),
            "`name` is optional on terminal entries, got: {diagnostics:?}"
        );
    }

    /// A terminal entry without `command` is still reported.
    #[test]
    fn test_cur_016_terminal_command_still_required() {
        let diagnostics =
            validate_cursor_environment(r#"{"terminals":[{"name":"dev","description":"x"}]}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "a terminal without `command` must be flagged, got: {diagnostics:?}"
        );
    }
    /// `terminals.items` is a `oneOf`: an array of command-bearing objects, or a
    /// single such object. Accepting the array branch without looking inside let
    /// `[[{"name":"x"}]]` pass with no `command`.
    #[test]
    fn test_cur_016_nested_terminal_array_contents_are_validated() {
        let ok = validate_cursor_environment(r#"{"terminals":[[{"command":"npm run dev"}]]}"#);
        assert!(
            ok.iter().all(|d| d.rule != "CUR-016"),
            "the array branch is schema-valid, got: {ok:?}"
        );

        let bad = validate_cursor_environment(
            r#"{"terminals":[[{"name":"no command"},{"description":1}]]}"#,
        );
        assert!(
            bad.iter().any(|d| d.message.contains("terminals[1][1]"))
                && bad.iter().any(|d| d.message.contains("terminals[1][2]")),
            "nested terminal diagnostics must identify each inner entry, got: {bad:?}"
        );
    }

    /// A terminal entry that is neither an object nor an array matches no `oneOf`
    /// branch.
    #[test]
    fn test_cur_016_terminal_scalar_is_rejected() {
        let diagnostics = validate_cursor_environment(r#"{"terminals":["npm run dev"]}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "CUR-016"),
            "a bare string matches no branch, got: {diagnostics:?}"
        );
    }
}
