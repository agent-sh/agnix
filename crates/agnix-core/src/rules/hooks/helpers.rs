use crate::diagnostics::{Diagnostic, Fix};
use crate::rules::find_closest_value;
use crate::schemas::hooks::HooksSchema;
use regex::Regex;
use rust_i18n::t;
use std::path::Path;
use std::sync::OnceLock;

struct DangerousPattern {
    regex: Regex,
    pattern: &'static str,
    reason: &'static str,
}

static DANGEROUS_PATTERNS: OnceLock<Vec<DangerousPattern>> = OnceLock::new();
static SCRIPT_PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();

fn dangerous_patterns() -> &'static Vec<DangerousPattern> {
    DANGEROUS_PATTERNS.get_or_init(|| {
        let patterns: &[(&str, &str)] = &[
            (
                r"(?i)rm\s+-rf\s+/",
                "Recursive delete from root is extremely dangerous",
            ),
            (
                r"(?i)rm\s+-rf\s+\*",
                "Recursive delete with wildcard could delete unintended files",
            ),
            (
                r"(?i)rm\s+-rf\s+\.\.",
                "Recursive delete of parent directories is dangerous",
            ),
            (
                r"(?i)git\s+reset\s+--hard",
                "Hard reset discards uncommitted changes permanently",
            ),
            (
                r"(?i)git\s+clean\s+-fd",
                "Git clean -fd removes untracked files permanently",
            ),
            (
                r"(?i)git\s+push\s+.*--force",
                "Force push can overwrite remote history",
            ),
            (r"(?i)drop\s+database", "Dropping database is irreversible"),
            (r"(?i)drop\s+table", "Dropping table is irreversible"),
            (r"(?i)truncate\s+table", "Truncating table deletes all data"),
            (
                r"(?i)curl\s+.*\|\s*sh",
                "Piping curl to shell is a security risk",
            ),
            (
                r"(?i)curl\s+.*\|\s*bash",
                "Piping curl to bash is a security risk",
            ),
            (
                r"(?i)wget\s+.*\|\s*sh",
                "Piping wget to shell is a security risk",
            ),
            (r"(?i)chmod\s+777", "chmod 777 gives everyone full access"),
            (
                r"(?i)>\s*/dev/sd[a-z]",
                "Writing directly to block devices can destroy data",
            ),
            (r"(?i)mkfs\.", "Formatting filesystem destroys all data"),
            (r"(?i)dd\s+if=.*of=/dev/", "dd to device can destroy data"),
            (
                r"(?i)\|\|\s*true\s*$",
                "Error suppression with '|| true' silently hides hook failures",
            ),
            (
                r"(?i)2>\s*/dev/null",
                "Redirecting stderr to /dev/null hides error messages",
            ),
        ];
        patterns
            .iter()
            .map(|&(pattern, reason)| {
                let regex = Regex::new(pattern).unwrap_or_else(|e| {
                    panic!("BUG: invalid dangerous pattern regex '{}': {}", pattern, e)
                });
                DangerousPattern {
                    regex,
                    pattern,
                    reason,
                }
            })
            .collect()
    })
}

fn script_patterns() -> &'static Vec<Regex> {
    SCRIPT_PATTERNS.get_or_init(|| {
        [
            r#"["']?([^\s"']+\.sh)["']?\b"#,
            r#"["']?([^\s"']+\.bash)["']?\b"#,
            r#"["']?([^\s"']+\.py)["']?\b"#,
            r#"["']?([^\s"']+\.js)["']?\b"#,
            r#"["']?([^\s"']+\.ts)["']?\b"#,
        ]
        .iter()
        .map(|p| {
            Regex::new(p)
                .unwrap_or_else(|e| panic!("BUG: invalid script pattern regex '{}': {}", p, e))
        })
        .collect()
    })
}

/// CC-HK-005: Missing type field
pub(super) fn validate_cc_hk_005_missing_type_field(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            if hook.get("type").is_none() {
                                let hook_location =
                                    format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                                diagnostics.push(
                                    Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-005",
                                        t!(
                                            "rules.cc_hk_005.message",
                                            location = hook_location.as_str()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.cc_hk_005.suggestion")),
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CC-HK-011: Invalid timeout value
pub(super) fn validate_cc_hk_011_invalid_timeout_values(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            if let Some(timeout_val) = hook.get("timeout") {
                                let is_invalid = match timeout_val {
                                    serde_json::Value::Number(n) => {
                                        // A valid timeout must be a positive integer.
                                        // as_u64() returns Some only for non-negative integer
                                        // JSON numbers within the u64 range; it returns None
                                        // for negatives, any floats (including 30.0), or
                                        // out-of-range values.
                                        if let Some(val) = n.as_u64() {
                                            val == 0 // Zero is invalid
                                        } else {
                                            true // Negative, float, or out of range
                                        }
                                    }
                                    _ => true, // String, bool, null, object, array are invalid
                                };
                                if is_invalid {
                                    let hook_location = format!(
                                        "hooks.{}[{}].hooks[{}]",
                                        event, matcher_idx, hook_idx
                                    );
                                    let mut diagnostic = Diagnostic::error(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-011",
                                        t!(
                                            "rules.cc_hk_011.message",
                                            location = hook_location.as_str()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.cc_hk_011.suggestion"));

                                    // Unsafe auto-fix: replace invalid timeout with conservative default 30s.
                                    // Emit only when the exact key/value pair is uniquely located.
                                    if let Ok(serialized) = serde_json::to_string(timeout_val) {
                                        if let Some((start, end)) = find_unique_json_key_value_span(
                                            content,
                                            "timeout",
                                            &serialized,
                                        ) {
                                            diagnostic = diagnostic.with_fix(Fix::replace(
                                                start,
                                                end,
                                                "30",
                                                "Set timeout to 30 seconds",
                                                false,
                                            ));
                                        }
                                    }

                                    diagnostics.push(diagnostic);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Deprecated event names and their recommended replacements.
/// Each entry is `(deprecated_name, replacement_name)`.
pub(super) const DEPRECATED_EVENTS: &[(&str, &str)] = &[("Setup", "SessionStart")];

/// CC-HK-019: Deprecated event name
pub(super) fn validate_cc_hk_019_deprecated_event(
    event: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(&(deprecated, replacement)) = DEPRECATED_EVENTS.iter().find(|&&(d, _)| d == event) {
        let mut diagnostic = Diagnostic::warning(
            path.to_path_buf(),
            1,
            0,
            "CC-HK-019",
            t!(
                "rules.cc_hk_019.message",
                event = deprecated,
                replacement = replacement
            ),
        )
        .with_suggestion(t!(
            "rules.cc_hk_019.suggestion",
            event = deprecated,
            replacement = replacement
        ));

        // Unsafe auto-fix: replace deprecated event key with replacement
        if let Some((start, end)) = find_event_key_position(content, event) {
            let replacement_text = format!("\"{}\"", replacement);
            diagnostic = diagnostic.with_fix(Fix::replace(
                start,
                end,
                replacement_text,
                t!(
                    "rules.cc_hk_019.fix",
                    event = deprecated,
                    replacement = replacement
                ),
                false,
            ));
        }

        diagnostics.push(diagnostic);
    }
}

/// CC-HK-001: Invalid event name with auto-fix support
pub(super) fn validate_cc_hk_001_event_name(
    event: &str,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if HooksSchema::VALID_EVENTS.contains(&event) {
        return true;
    }

    let closest = find_closest_event(event);
    let mut diagnostic = Diagnostic::error(
        path.to_path_buf(),
        1,
        0,
        "CC-HK-001",
        t!(
            "rules.cc_hk_001.message",
            event = event,
            valid = format!("{:?}", HooksSchema::VALID_EVENTS)
        ),
    )
    .with_suggestion(closest.suggestion);

    // Add auto-fix if we found a matching event
    if let Some(corrected) = closest.corrected_event {
        if let Some((start, end)) = find_event_key_position(content, event) {
            let replacement = format!("\"{}\"", corrected);
            let description = t!("rules.cc_hk_001.fix", old = event, new = corrected);
            // Case-only fixes are safe (high confidence)
            let fix = Fix::replace(start, end, replacement, description, closest.is_case_fix);
            diagnostic = diagnostic.with_fix(fix);
        }
    }

    diagnostics.push(diagnostic);
    false
}

/// CC-HK-003: Matcher hint for tool events.
/// Omitting matcher is valid and means "match all tools".
pub(super) fn validate_cc_hk_003_matcher_hint(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if HooksSchema::is_tool_event(event) && matcher.is_none() {
        let hook_location = format!("hooks.{}[{}]", event, matcher_idx);
        diagnostics.push(
            Diagnostic::info(
                path.to_path_buf(),
                1,
                0,
                "CC-HK-003",
                t!(
                    "rules.cc_hk_003.message",
                    event = event,
                    location = hook_location.as_str()
                ),
            )
            .with_suggestion(t!("rules.cc_hk_003.suggestion")),
        );
    }
}

/// CC-HK-004: Matcher on non-tool event
/// Note: Stop and UserPromptSubmit are handled by CC-HK-018 instead (info-level).
pub(super) fn validate_cc_hk_004_matcher_forbidden(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Skip events handled by CC-HK-018 (matcher silently ignored rather than forbidden)
    const CC_HK_018_EVENTS: &[&str] = &["UserPromptSubmit", "Stop"];
    if !HooksSchema::supports_matcher(event)
        && matcher.is_some()
        && !CC_HK_018_EVENTS.contains(&event)
    {
        let hook_location = format!("hooks.{}[{}]", event, matcher_idx);
        let mut diagnostic = Diagnostic::error(
            path.to_path_buf(),
            1,
            0,
            "CC-HK-004",
            t!(
                "rules.cc_hk_004.message",
                event = event,
                location = hook_location.as_str()
            ),
        )
        .with_suggestion(t!("rules.cc_hk_004.suggestion"));

        // Safe auto-fix: remove matcher line on non-tool events.
        // Emit only when we can uniquely identify the exact matcher property.
        if let Some(matcher_value) = matcher {
            if let Some((start, end)) = find_unique_matcher_line_span(content, matcher_value) {
                diagnostic = diagnostic.with_fix(Fix::delete(
                    start,
                    end,
                    "Remove matcher from non-tool event",
                    true,
                ));
            }
        }

        diagnostics.push(diagnostic);
    }
}

pub(super) fn check_dangerous_patterns(command: &str) -> Option<(&'static str, &'static str)> {
    for dp in dangerous_patterns() {
        if dp.regex.is_match(command) {
            return Some((dp.pattern, dp.reason));
        }
    }
    None
}

pub(super) fn extract_script_paths(command: &str) -> Vec<String> {
    let mut paths = Vec::new();
    for re in script_patterns() {
        for caps in re.captures_iter(command) {
            if let Some(m) = caps.get(1) {
                let path = m.as_str().trim_matches(|c| c == '"' || c == '\'');
                if path.contains("://") || path.starts_with("http") {
                    continue;
                }
                // Skip regex patterns and glob patterns that happen to end in
                // script extensions (e.g., `\.py$`, `*.js`, `[^/]*.sh`)
                if path.starts_with('\\')
                    || path.starts_with('[')
                    || path.starts_with('*')
                    || path.starts_with('(')
                    || path.contains("]*")
                {
                    continue;
                }
                paths.push(path.to_string());
            }
        }
    }
    paths
}

pub(super) fn resolve_script_path(script_path: &str, project_dir: &Path) -> std::path::PathBuf {
    let resolved = script_path
        .replace("$CLAUDE_PROJECT_DIR", &project_dir.display().to_string())
        .replace("${CLAUDE_PROJECT_DIR}", &project_dir.display().to_string());

    let path = std::path::PathBuf::from(&resolved);

    if path.is_relative() {
        project_dir.join(path)
    } else {
        path
    }
}

pub(super) fn has_unresolved_env_vars(path: &str) -> bool {
    let after_claude = path
        .replace("$CLAUDE_PROJECT_DIR", "")
        .replace("${CLAUDE_PROJECT_DIR}", "");
    after_claude.contains('$')
}

pub(super) struct ClosestEventMatch {
    pub(super) suggestion: String,
    /// The correct event name if a good match was found
    pub(super) corrected_event: Option<String>,
    /// Whether this is a case-only difference (high confidence)
    pub(super) is_case_fix: bool,
}

pub(super) fn find_closest_event(invalid_event: &str) -> ClosestEventMatch {
    let lower_event = invalid_event.to_lowercase();

    // Check for exact case-insensitive match first (high confidence fix)
    for valid in HooksSchema::VALID_EVENTS {
        if valid.to_lowercase() == lower_event {
            return ClosestEventMatch {
                suggestion: format!("Did you mean '{}'? Event names are case-sensitive.", valid),
                corrected_event: Some(valid.to_string()),
                is_case_fix: true,
            };
        }
    }

    // Check for partial matches (lower confidence)
    for valid in HooksSchema::VALID_EVENTS {
        let valid_lower = valid.to_lowercase();
        if valid_lower.contains(&lower_event) || lower_event.contains(&valid_lower) {
            return ClosestEventMatch {
                suggestion: format!("Did you mean '{}'?", valid),
                corrected_event: Some(valid.to_string()),
                is_case_fix: false,
            };
        }
    }

    ClosestEventMatch {
        suggestion: format!("Valid events are: {}", HooksSchema::VALID_EVENTS.join(", ")),
        corrected_event: None,
        is_case_fix: false,
    }
}

/// Find the byte position of an event key in JSON content
/// Returns (start, end) byte positions of the event key (including quotes)
pub(super) fn find_event_key_position(content: &str, event: &str) -> Option<(usize, usize)> {
    crate::span_utils::find_event_key_span(content, event)
}

/// Find a unique JSON key/value span for a specific key and serialized value.
/// Returns the value span only (not including the key/colon).
fn find_unique_json_key_value_span(
    content: &str,
    key: &str,
    serialized_value: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_json_key_value(content, key, serialized_value)
}

/// Iterate over all raw hook entries in the JSON value, calling `f` for each one.
fn for_each_raw_hook<F>(raw_value: &serde_json::Value, mut f: F)
where
    F: FnMut(&str, usize, usize, &serde_json::Value),
{
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            f(event, matcher_idx, hook_idx, hook);
                        }
                    }
                }
            }
        }
    }
}

/// CC-HK-013: Async on non-command hook (raw JSON check).
/// Only flags known non-command types (prompt/agent); unknown types are handled by CC-HK-016.
pub(super) fn validate_cc_hk_013_async_field(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let known_non_command = ["prompt", "agent", "http"];
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if hook.get("async").is_some() {
            if let Some(hook_type) = hook.get("type").and_then(|t| t.as_str()) {
                // Only flag async on known non-command types.
                // Unknown/invalid types are handled by CC-HK-016.
                if known_non_command.contains(&hook_type) {
                    let hook_location =
                        format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                    let mut diagnostic = Diagnostic::error(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-013",
                        t!(
                            "rules.cc_hk_013.message",
                            hook_type = hook_type,
                            location = hook_location.as_str()
                        ),
                    )
                    .with_suggestion(t!("rules.cc_hk_013.suggestion"));

                    // Safe auto-fix: remove the async field line
                    if let Some((start, end)) = find_unique_json_field_line_span(content, "async") {
                        diagnostic = diagnostic.with_fix(Fix::delete(
                            start,
                            end,
                            t!("rules.cc_hk_013.fix"),
                            true,
                        ));
                    }

                    diagnostics.push(diagnostic);
                }
            }
        }
    });
}

/// CC-HK-014: Once outside skill/agent frontmatter (raw JSON check)
pub(super) fn validate_cc_hk_014_once_field(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if hook.get("once").is_some() {
            let hook_location = format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-HK-014",
                    t!("rules.cc_hk_014.message", location = hook_location.as_str()),
                )
                .with_suggestion(t!("rules.cc_hk_014.suggestion")),
            );
        }
    });
}

use crate::rules::find_unique_json_string_value_span;

/// CC-HK-016: Validate hook type "agent" - check for unknown types (raw JSON check)
/// CC-HK-016: Unknown hook type (raw JSON check).
/// Also catches non-string type values (e.g., numbers, booleans).
pub(super) fn validate_cc_hk_016_unknown_type(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid_types = ["command", "prompt", "agent", "http"];
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if let Some(type_value) = hook.get("type") {
            let hook_type_str;
            let is_invalid = if let Some(s) = type_value.as_str() {
                hook_type_str = s.to_string();
                !valid_types.contains(&s)
            } else {
                // Non-string type value (number, bool, null, etc.) is always invalid
                hook_type_str = type_value.to_string();
                true
            };
            if is_invalid {
                let hook_location = format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                let mut diagnostic = Diagnostic::error(
                    path.to_path_buf(),
                    1,
                    0,
                    "CC-HK-016",
                    t!(
                        "rules.cc_hk_016.message",
                        hook_type = hook_type_str.as_str(),
                        location = hook_location.as_str()
                    ),
                )
                .with_suggestion(t!("rules.cc_hk_016.suggestion"));

                // Unsafe auto-fix: replace with closest valid hook type (string values only).
                if let Some(hook_type_s) = type_value.as_str() {
                    if let Some(suggested) = find_closest_value(hook_type_s, &valid_types) {
                        if let Some((start, end)) =
                            find_unique_json_string_value_span(content, "type", hook_type_s)
                        {
                            diagnostic = diagnostic.with_fix(Fix::replace(
                                start,
                                end,
                                suggested,
                                t!("rules.cc_hk_016.fix", fixed = suggested),
                                false,
                            ));
                        }
                    }
                }

                diagnostics.push(diagnostic);
            }
        }
    });
}

/// Find a unique JSON field line span that can be safely deleted.
/// Matches a line like `  "field": value,\n` and returns byte range.
/// Returns None if 0 or 2+ matches (uniqueness guard).
pub(super) fn find_unique_json_field_line_span(
    content: &str,
    field_name: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_json_field_line(content, field_name)
}

/// Find a unique matcher line span that can be safely deleted.
/// Includes trailing newline when present.
pub(super) fn find_unique_matcher_line_span(
    content: &str,
    matcher_value: &str,
) -> Option<(usize, usize)> {
    crate::span_utils::find_unique_json_matcher_line(content, matcher_value)
}

/// CC-HK-020: HTTP hook missing required `url` field.
/// When a hook has `type: "http"`, it MUST have a `url` field that is a non-empty string.
pub(super) fn validate_cc_hk_020_http_missing_url(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if let Some(type_val) = hook.get("type").and_then(|t| t.as_str()) {
            if type_val == "http" {
                let missing = match hook.get("url") {
                    None => true,
                    Some(serde_json::Value::String(s)) => s.is_empty(),
                    Some(_) => true, // non-string url is invalid
                };
                if missing {
                    let hook_location =
                        format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                    diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            1,
                            0,
                            "CC-HK-020",
                            format!(
                                "HTTP hook at {} is missing required 'url' field",
                                hook_location
                            ),
                        )
                        .with_suggestion(
                            "Add a 'url' field with a valid HTTP(S) endpoint to the hook"
                                .to_string(),
                        ),
                    );
                }
            }
        }
    });
}

/// CC-HK-021: Invalid `if` field syntax.
/// The `if` field must be a non-empty string and is only valid on tool events.
pub(super) fn validate_cc_hk_021_invalid_if_field(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let tool_events = HooksSchema::TOOL_EVENTS;
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                        for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                            if let Some(if_val) = hook.get("if") {
                                let hook_location =
                                    format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);

                                // Check if the event is a tool event
                                if !tool_events.contains(&event.as_str()) {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-HK-021",
                                            format!(
                                                "Hook at {} has 'if' field on non-tool event '{}'; 'if' is only valid on tool events: {}",
                                                hook_location, event, HooksSchema::TOOL_EVENTS.join(", ")
                                            ),
                                        )
                                        .with_suggestion(
                                            "Remove the 'if' field or move the hook to a tool event".to_string(),
                                        ),
                                    );
                                    continue;
                                }

                                // Validate it is a non-empty string
                                let is_invalid = match if_val {
                                    serde_json::Value::String(s) => s.is_empty(),
                                    _ => true,
                                };
                                if is_invalid {
                                    diagnostics.push(
                                        Diagnostic::warning(
                                            path.to_path_buf(),
                                            1,
                                            0,
                                            "CC-HK-021",
                                            format!(
                                                "Hook at {} has invalid 'if' field; must be a non-empty string",
                                                hook_location
                                            ),
                                        )
                                        .with_suggestion(
                                            "Set 'if' to a valid filter expression string".to_string(),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// CC-HK-022: Invalid `shell` value.
/// When present, `shell` must be either "bash" or "powershell".
pub(super) fn validate_cc_hk_022_invalid_shell(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid_shells = ["bash", "powershell"];
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if let Some(shell_val) = hook.get("shell") {
            let is_invalid = match shell_val.as_str() {
                Some(s) => !valid_shells.contains(&s),
                None => true, // non-string is invalid
            };
            if is_invalid {
                let hook_location = format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                let shell_display = shell_val
                    .as_str()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| shell_val.to_string());
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-022",
                        format!(
                            "Hook at {} has invalid 'shell' value '{}'; must be 'bash' or 'powershell'",
                            hook_location, shell_display
                        ),
                    )
                    .with_suggestion(
                        "Set 'shell' to either 'bash' or 'powershell'".to_string(),
                    ),
                );
            }
        }
    });
}

/// CC-HK-023: `once` field outside skill context.
/// Validate that `once` is a boolean when present.
pub(super) fn validate_cc_hk_023_once_not_boolean(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if let Some(once_val) = hook.get("once") {
            if !once_val.is_boolean() {
                let hook_location = format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        1,
                        0,
                        "CC-HK-023",
                        format!(
                            "Hook at {} has non-boolean 'once' field; 'once' must be true or false",
                            hook_location
                        ),
                    )
                    .with_suggestion("Set 'once' to true or false".to_string()),
                );
            }
        }
    });
}

/// CC-HK-024: HTTP hook `headers` with `$VAR` but missing `allowedEnvVars`.
/// When headers contain `$` variable interpolation patterns, warn if `allowedEnvVars` is not set.
pub(super) fn validate_cc_hk_024_headers_env_vars(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for_each_raw_hook(raw_value, |event, matcher_idx, hook_idx, hook| {
        if let Some(type_val) = hook.get("type").and_then(|t| t.as_str()) {
            if type_val == "http" {
                if let Some(headers) = hook.get("headers").and_then(|h| h.as_object()) {
                    let has_env_var = headers
                        .values()
                        .any(|v| v.as_str().map(|s| s.contains('$')).unwrap_or(false));
                    if has_env_var {
                        let has_allowed = hook
                            .get("allowedEnvVars")
                            .and_then(|v| v.as_array())
                            .map(|a| !a.is_empty())
                            .unwrap_or(false);
                        if !has_allowed {
                            let hook_location =
                                format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-HK-024",
                                    format!(
                                        "HTTP hook at {} has headers with $VAR interpolation but no 'allowedEnvVars'",
                                        hook_location
                                    ),
                                )
                                .with_suggestion(
                                    "Add 'allowedEnvVars' array listing the environment variables used in headers".to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }
    });
}

/// CC-HK-025: Invalid matcher value for event type.
/// Events that support matchers have specific valid values.
/// Validates matcher values on `SessionStart` and `StopFailure`.
pub(super) fn validate_cc_hk_025_invalid_matcher_value(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    const SESSION_START_MATCHERS: &[&str] = &["startup", "resume", "clear", "compact"];
    const STOP_FAILURE_MATCHERS: &[&str] = &[
        "rate_limit",
        "authentication_failed",
        "billing_error",
        "invalid_request",
        "server_error",
        "max_output_tokens",
        "unknown",
    ];

    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            let valid_values: &[&str] = match event.as_str() {
                "SessionStart" => SESSION_START_MATCHERS,
                "StopFailure" => STOP_FAILURE_MATCHERS,
                _ => continue,
            };

            if let Some(matchers_arr) = matchers.as_array() {
                for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                    if let Some(matcher_val) = matcher.get("matcher").and_then(|m| m.as_str()) {
                        if !valid_values.contains(&matcher_val) {
                            let location = format!("hooks.{}[{}]", event, matcher_idx);
                            diagnostics.push(
                                Diagnostic::warning(
                                    path.to_path_buf(),
                                    1,
                                    0,
                                    "CC-HK-025",
                                    format!(
                                        "Matcher '{}' at {} is not a known value for event '{}'; expected one of: {}",
                                        matcher_val,
                                        location,
                                        event,
                                        valid_values.join(", ")
                                    ),
                                )
                                .with_suggestion(format!(
                                    "Use one of the known matcher values for '{}': {}.",
                                    event,
                                    valid_values.join(", ")
                                )),
                            );
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_dangerous_patterns_error_suppression() {
        // Test || true pattern
        let result = check_dangerous_patterns("some_command || true");
        assert!(result.is_some(), "Should detect || true pattern");
        let (pattern, reason) = result.unwrap();
        assert!(pattern.contains("true"));
        assert!(reason.contains("silently hides"));
    }

    #[test]
    fn test_check_dangerous_patterns_stderr_redirect() {
        // Test 2>/dev/null pattern
        let result = check_dangerous_patterns("command 2>/dev/null");
        assert!(result.is_some(), "Should detect 2>/dev/null pattern");
        let (pattern, reason) = result.unwrap();
        assert!(pattern.contains("2>"));
        assert!(reason.contains("hides error"));
    }

    #[test]
    fn test_check_dangerous_patterns_safe_commands() {
        // Commands without dangerous patterns should return None
        assert!(check_dangerous_patterns("echo hello").is_none());
        assert!(check_dangerous_patterns("npm install").is_none());
        assert!(check_dangerous_patterns("cargo build").is_none());
    }

    #[test]
    fn test_extract_script_paths_basic() {
        // Test basic script path extraction
        let paths = extract_script_paths("./scripts/test.sh");
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], "./scripts/test.sh");
    }

    #[test]
    fn test_extract_script_paths_multiple() {
        // Test multiple scripts
        let paths = extract_script_paths("run setup.sh && execute deploy.py");
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"setup.sh".to_string()));
        assert!(paths.contains(&"deploy.py".to_string()));
    }

    #[test]
    fn test_extract_script_paths_filters_regex() {
        // Should filter out regex patterns ending in script extensions
        let paths = extract_script_paths(r"find . -name '\.py$'");
        assert!(paths.is_empty(), "Should filter regex patterns");
    }

    #[test]
    fn test_extract_script_paths_filters_glob() {
        // Should filter out glob patterns
        let paths = extract_script_paths("process *.js files");
        assert!(paths.is_empty(), "Should filter glob patterns");
    }

    #[test]
    fn test_extract_script_paths_filters_bracket_patterns() {
        // Should filter out bracket patterns
        let paths = extract_script_paths("match [^/]*.sh pattern");
        assert!(paths.is_empty(), "Should filter bracket patterns");
    }

    #[test]
    fn test_extract_script_paths_filters_urls() {
        // Should filter out URLs
        let paths = extract_script_paths("download https://example.com/script.sh");
        assert!(paths.is_empty(), "Should filter URLs");
    }

    // ===== find_unique_json_field_line_span tests =====

    #[test]
    fn test_find_unique_json_field_line_span_unique() {
        let content = r#"{
  "type": "prompt",
  "async": true,
  "prompt": "hello"
}"#;
        let result = find_unique_json_field_line_span(content, "async");
        assert!(result.is_some(), "Should find unique async field");
        let (start, end) = result.unwrap();
        let matched = &content[start..end];
        assert!(
            matched.contains("\"async\""),
            "Match should contain async field, got: {:?}",
            matched
        );
    }

    #[test]
    fn test_find_unique_json_field_line_span_duplicate() {
        // Two hooks both have "async" - should return None
        let content = r#"{
  "hooks": {
    "Stop": [
      { "hooks": [{ "type": "prompt", "async": true }] },
      { "hooks": [{ "type": "prompt", "async": false }] }
    ]
  }
}"#;
        // The field appears twice, even though it's on two different lines
        // Our regex matches lines with "async": ... and there are two
        let result = find_unique_json_field_line_span(content, "async");
        assert!(
            result.is_none(),
            "Should return None when async appears twice"
        );
    }

    #[test]
    fn test_find_unique_json_field_line_span_missing() {
        let content = r#"{ "type": "command", "command": "echo hi" }"#;
        let result = find_unique_json_field_line_span(content, "async");
        assert!(result.is_none(), "Should return None when field is missing");
    }

    // ===== CC-HK-025: Invalid matcher value for event type =====

    #[test]
    fn test_cc_hk_025_invalid_session_start_matcher() {
        let content = r#"{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "invalid_value",
        "hooks": [{ "type": "command", "command": "echo hello" }]
      }
    ]
  }
}"#;
        let raw_value: serde_json::Value = serde_json::from_str(content).unwrap();
        let path = Path::new(".claude/settings.json");
        let mut diagnostics = Vec::new();

        validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "CC-HK-025");
        assert!(diagnostics[0].message.contains("invalid_value"));
        assert!(diagnostics[0].message.contains("SessionStart"));
    }

    #[test]
    fn test_cc_hk_025_valid_session_start_matchers() {
        let valid_matchers = ["startup", "resume", "clear", "compact"];
        for matcher in valid_matchers {
            let content = format!(
                r#"{{
  "hooks": {{
    "SessionStart": [
      {{
        "matcher": "{}",
        "hooks": [{{ "type": "command", "command": "echo hello" }}]
      }}
    ]
  }}
}}"#,
                matcher
            );
            let raw_value: serde_json::Value = serde_json::from_str(&content).unwrap();
            let path = Path::new(".claude/settings.json");
            let mut diagnostics = Vec::new();

            validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

            assert!(
                diagnostics.is_empty(),
                "Matcher '{}' should be valid for SessionStart",
                matcher
            );
        }
    }

    #[test]
    fn test_cc_hk_025_invalid_stop_failure_matcher() {
        let content = r#"{
  "hooks": {
    "StopFailure": [
      {
        "matcher": "network_error",
        "hooks": [{ "type": "command", "command": "echo error" }]
      }
    ]
  }
}"#;
        let raw_value: serde_json::Value = serde_json::from_str(content).unwrap();
        let path = Path::new(".claude/settings.json");
        let mut diagnostics = Vec::new();

        validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "CC-HK-025");
        assert!(diagnostics[0].message.contains("network_error"));
        assert!(diagnostics[0].message.contains("StopFailure"));
    }

    #[test]
    fn test_cc_hk_025_valid_stop_failure_matchers() {
        let valid_matchers = [
            "rate_limit",
            "authentication_failed",
            "billing_error",
            "invalid_request",
            "server_error",
            "max_output_tokens",
            "unknown",
        ];
        for matcher in valid_matchers {
            let content = format!(
                r#"{{
  "hooks": {{
    "StopFailure": [
      {{
        "matcher": "{}",
        "hooks": [{{ "type": "command", "command": "echo err" }}]
      }}
    ]
  }}
}}"#,
                matcher
            );
            let raw_value: serde_json::Value = serde_json::from_str(&content).unwrap();
            let path = Path::new(".claude/settings.json");
            let mut diagnostics = Vec::new();

            validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

            assert!(
                diagnostics.is_empty(),
                "Matcher '{}' should be valid for StopFailure",
                matcher
            );
        }
    }

    #[test]
    fn test_cc_hk_025_other_events_not_checked() {
        // Matchers on other events should not trigger CC-HK-025
        let content = r#"{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "arbitrary_value",
        "hooks": [{ "type": "command", "command": "echo hello" }]
      }
    ]
  }
}"#;
        let raw_value: serde_json::Value = serde_json::from_str(content).unwrap();
        let path = Path::new(".claude/settings.json");
        let mut diagnostics = Vec::new();

        validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

        assert!(
            diagnostics.is_empty(),
            "CC-HK-025 should not check PreToolUse matchers"
        );
    }
}
