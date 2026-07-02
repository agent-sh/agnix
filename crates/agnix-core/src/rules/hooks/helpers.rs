use crate::config::PerFileLintConfig;
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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

const SESSION_START_MATCHERS: &[&str] = &["startup", "resume", "clear", "compact"];
const SETUP_MATCHERS: &[&str] = &["init", "maintenance"];
const SESSION_END_MATCHERS: &[&str] = &[
    "clear",
    "resume",
    "logout",
    "prompt_input_exit",
    "bypass_permissions_disabled",
    "other",
];
const NOTIFICATION_MATCHERS: &[&str] = &[
    "permission_prompt",
    "idle_prompt",
    "auth_success",
    "elicitation_dialog",
    "elicitation_complete",
    "elicitation_response",
    "agent_needs_input",
    "agent_completed",
];
const COMPACT_MATCHERS: &[&str] = &["manual", "auto"];
const CONFIG_CHANGE_MATCHERS: &[&str] = &[
    "user_settings",
    "project_settings",
    "local_settings",
    "policy_settings",
    "skills",
];
const STOP_FAILURE_MATCHERS: &[&str] = &[
    "rate_limit",
    "overloaded",
    "authentication_failed",
    "oauth_org_not_allowed",
    "billing_error",
    "invalid_request",
    "model_not_found",
    "server_error",
    "max_output_tokens",
    "unknown",
];
const INSTRUCTIONS_LOADED_MATCHERS: &[&str] = &[
    "session_start",
    "nested_traversal",
    "path_glob_match",
    "include",
    "compact",
];

fn known_matcher_values(event: &str) -> Option<&'static [&'static str]> {
    match event {
        "SessionStart" => Some(SESSION_START_MATCHERS),
        "Setup" => Some(SETUP_MATCHERS),
        "SessionEnd" => Some(SESSION_END_MATCHERS),
        "Notification" => Some(NOTIFICATION_MATCHERS),
        "PreCompact" | "PostCompact" => Some(COMPACT_MATCHERS),
        "ConfigChange" => Some(CONFIG_CHANGE_MATCHERS),
        "StopFailure" => Some(STOP_FAILURE_MATCHERS),
        "InstructionsLoaded" => Some(INSTRUCTIONS_LOADED_MATCHERS),
        _ => None,
    }
}

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

/// CC-HK-004: Matcher on event without matcher support.
/// Valid no-matcher events are handled by CC-HK-018 instead (info-level),
/// because Claude Code silently ignores the matcher at runtime.
pub(super) fn validate_cc_hk_004_matcher_forbidden(
    event: &str,
    matcher: &Option<String>,
    matcher_idx: usize,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !HooksSchema::supports_matcher(event)
        && matcher.is_some()
        && !HooksSchema::ignores_matcher(event)
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
    let expanded = expand_tilde_prefix(script_path);
    let resolved = expanded
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
    if after_claude.contains('$') {
        return true;
    }
    // A `~user/...` form is considered unresolved because we can't look
    // up arbitrary users without nss, and Claude Code hands the whole
    // string to the shell at runtime. A bare `~/...` is handled by the
    // expansion in `resolve_script_path` - if HOME can be read, the path
    // is considered resolved and CC-HK-008 runs the existence check
    // against $HOME/...; otherwise we treat it as unresolved here to
    // avoid false-positives on machines without a discoverable HOME.
    if is_tilde_user_path(path) {
        return true;
    }
    if path.starts_with("~/") || path.starts_with("~\\") || path == "~" {
        #[cfg(feature = "filesystem")]
        {
            return dirs::home_dir().is_none();
        }
        #[cfg(not(feature = "filesystem"))]
        {
            return true;
        }
    }
    false
}

/// Expand a leading `~/` or `~\` (POSIX or Windows) to `$HOME/...` when
/// `dirs::home_dir()` is available. `~user/...` forms are left untouched
/// (caller treats them as unresolved via `has_unresolved_env_vars`).
/// Any `~` not at the very start of the string is left untouched - the
/// shell only special-cases a literal leading tilde, and Claude Code
/// passes the command to the shell verbatim.
fn expand_tilde_prefix(path: &str) -> String {
    if path == "~" || path.starts_with("~/") || path.starts_with("~\\") {
        #[cfg(feature = "filesystem")]
        {
            if let Some(home) = dirs::home_dir() {
                let home_str = home.display().to_string();
                if path == "~" {
                    return home_str;
                }
                // Strip the `~` and keep the separator so join semantics
                // match what the shell would do on either platform.
                return format!("{}{}", home_str, &path[1..]);
            }
        }
    }
    path.to_string()
}

/// Detect the `~user/...` form (tilde immediately followed by a non-slash
/// character). agnix does not resolve these because it would require
/// reading arbitrary user home directories via nss/passwd, and the
/// signal-to-noise tradeoff doesn't justify the platform complexity.
fn is_tilde_user_path(path: &str) -> bool {
    let bytes = path.as_bytes();
    if bytes.first() != Some(&b'~') {
        return false;
    }
    match bytes.get(1) {
        None => false,                     // bare "~"
        Some(b'/') | Some(b'\\') => false, // "~/..." or "~\..."
        Some(_) => true,                   // "~user..."
    }
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub(super) fn validate_cc_hk_013_async_field(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let known_non_command = ["prompt", "agent", "http", "mcp_tool"];
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
#[allow(dead_code)]
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
#[allow(dead_code)]
pub(super) fn validate_cc_hk_016_unknown_type(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid_types = ["command", "prompt", "agent", "http", "mcp_tool"];
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
#[allow(dead_code)]
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
                    .with_suggestion("Set 'once' to true or false"),
                );
            }
        }
    });
}

/// CC-HK-024: HTTP hook `headers` with `$VAR` but missing `allowedEnvVars`.
/// When headers contain `$` variable interpolation patterns, warn if `allowedEnvVars` is not set.
#[allow(dead_code)]
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

/// Consolidated raw-JSON validation for hooks.
///
/// Walks the hooks tree at most twice instead of 11 separate traversals:
///   - Pass 1: CC-HK-005 (missing type field). Early return if any fires.
///   - Pass 2: CC-HK-011, CC-HK-016, CC-HK-013, CC-HK-014, CC-HK-020..025.
///     CC-HK-016 triggers an early return that discards CC-HK-013/014/020..025.
///
/// Returns `true` when validation should continue to typed parsing,
/// `false` when a structural error caused an early return.
pub(super) fn validate_all_raw_hooks(
    raw_value: &serde_json::Value,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let hooks_obj = match raw_value.get("hooks").and_then(|h| h.as_object()) {
        Some(obj) => obj,
        None => return true,
    };

    // ---- Pass 1: CC-HK-005 (missing type) ----
    if config.is_rule_enabled("CC-HK-005") {
        let before = diagnostics.len();
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
        if diagnostics.len() > before {
            return false;
        }
    }

    // ---- Pass 2: all remaining raw-JSON rules in one traversal ----
    let check_011 = config.is_rule_enabled("CC-HK-011");
    let check_016 = config.is_rule_enabled("CC-HK-016");
    let check_013 = config.is_rule_enabled("CC-HK-013");
    let check_014 = config.is_rule_enabled("CC-HK-014");
    let check_020 = config.is_rule_enabled("CC-HK-020");
    let check_021 = config.is_rule_enabled("CC-HK-021");
    let check_022 = config.is_rule_enabled("CC-HK-022");
    let check_023 = config.is_rule_enabled("CC-HK-023");
    let check_024 = config.is_rule_enabled("CC-HK-024");
    let check_025 = config.is_rule_enabled("CC-HK-025");
    let check_026 = config.is_rule_enabled("CC-HK-026");
    let check_027 = config.is_rule_enabled("CC-HK-027");

    let valid_types = ["command", "prompt", "agent", "http", "mcp_tool"];
    let known_non_command = ["prompt", "agent", "http", "mcp_tool"];
    let valid_shells = ["bash", "powershell"];
    let tool_events = HooksSchema::TOOL_EVENTS;

    let mut found_016 = false;
    // Diagnostics gated on CC-HK-016 NOT firing (post-016 checks).
    let mut conditional_diags: Vec<Diagnostic> = Vec::new();

    for (event, matchers) in hooks_obj {
        // --- CC-HK-025: matcher-level check (not per-hook) ---
        if check_025 {
            if let Some(valid_values) = known_matcher_values(event) {
                if let Some(matchers_arr) = matchers.as_array() {
                    for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                        if let Some(matcher_val) = matcher.get("matcher").and_then(|m| m.as_str()) {
                            if !valid_values.contains(&matcher_val) {
                                let location = format!("hooks.{}[{}]", event, matcher_idx);
                                conditional_diags.push(
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

        // --- Per-hook checks ---
        if let Some(matchers_arr) = matchers.as_array() {
            for (matcher_idx, matcher) in matchers_arr.iter().enumerate() {
                if let Some(hooks_arr) = matcher.get("hooks").and_then(|h| h.as_array()) {
                    for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                        let hook_location =
                            format!("hooks.{}[{}].hooks[{}]", event, matcher_idx, hook_idx);

                        // -- CC-HK-011: Invalid timeout value --
                        if check_011 {
                            if let Some(timeout_val) = hook.get("timeout") {
                                let is_invalid = match timeout_val {
                                    serde_json::Value::Number(n) => {
                                        if let Some(val) = n.as_u64() {
                                            val == 0
                                        } else {
                                            true
                                        }
                                    }
                                    _ => true,
                                };
                                if is_invalid {
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

                        // -- CC-HK-016: Unknown hook type --
                        if check_016 {
                            if let Some(type_value) = hook.get("type") {
                                let hook_type_str;
                                let is_invalid = if let Some(s) = type_value.as_str() {
                                    hook_type_str = s.to_string();
                                    !valid_types.contains(&s)
                                } else {
                                    hook_type_str = type_value.to_string();
                                    true
                                };
                                if is_invalid {
                                    found_016 = true;
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

                                    if let Some(hook_type_s) = type_value.as_str() {
                                        if let Some(suggested) =
                                            find_closest_value(hook_type_s, &valid_types)
                                        {
                                            if let Some((start, end)) =
                                                find_unique_json_string_value_span(
                                                    content,
                                                    "type",
                                                    hook_type_s,
                                                )
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
                        }

                        // -- Post-016 checks (discarded if CC-HK-016 fires) --

                        // CC-HK-013: Async on non-command hook
                        if check_013 {
                            if hook.get("async").is_some() {
                                if let Some(hook_type) = hook.get("type").and_then(|t| t.as_str()) {
                                    if known_non_command.contains(&hook_type) {
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

                                        if let Some((start, end)) =
                                            find_unique_json_field_line_span(content, "async")
                                        {
                                            diagnostic = diagnostic.with_fix(Fix::delete(
                                                start,
                                                end,
                                                t!("rules.cc_hk_013.fix"),
                                                true,
                                            ));
                                        }

                                        conditional_diags.push(diagnostic);
                                    }
                                }
                            }
                        }

                        // CC-HK-014: Once outside skill/agent frontmatter
                        if check_014 {
                            if hook.get("once").is_some() {
                                conditional_diags.push(
                                    Diagnostic::warning(
                                        path.to_path_buf(),
                                        1,
                                        0,
                                        "CC-HK-014",
                                        t!(
                                            "rules.cc_hk_014.message",
                                            location = hook_location.as_str()
                                        ),
                                    )
                                    .with_suggestion(t!("rules.cc_hk_014.suggestion")),
                                );
                            }
                        }

                        // CC-HK-020: HTTP hook missing url
                        if check_020 {
                            if let Some(type_val) = hook.get("type").and_then(|t| t.as_str()) {
                                if type_val == "http" {
                                    let missing = match hook.get("url") {
                                        None => true,
                                        Some(serde_json::Value::String(s)) => s.is_empty(),
                                        Some(_) => true,
                                    };
                                    if missing {
                                        conditional_diags.push(
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
                        }

                        // CC-HK-026/CC-HK-027: MCP tool hook required fields.
                        // Documented at code.claude.com/docs/en/hooks#mcp-tool-hook-fields:
                        // `server` (required string) and `tool` (required string) identify
                        // an already-connected MCP server + the tool to invoke. `input`
                        // (optional object) carries tool arguments.
                        if check_026 || check_027 {
                            if let Some(type_val) = hook.get("type").and_then(|t| t.as_str()) {
                                if type_val == "mcp_tool" {
                                    if check_026 {
                                        let server_missing = match hook.get("server") {
                                            None => true,
                                            Some(serde_json::Value::String(s)) => s.is_empty(),
                                            Some(_) => true,
                                        };
                                        if server_missing {
                                            conditional_diags.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    1,
                                                    0,
                                                    "CC-HK-026",
                                                    t!(
                                                        "rules.cc_hk_026.message",
                                                        location = hook_location.as_str()
                                                    ),
                                                )
                                                .with_suggestion(t!("rules.cc_hk_026.suggestion")),
                                            );
                                        }
                                    }
                                    if check_027 {
                                        let tool_missing = match hook.get("tool") {
                                            None => true,
                                            Some(serde_json::Value::String(s)) => s.is_empty(),
                                            Some(_) => true,
                                        };
                                        if tool_missing {
                                            conditional_diags.push(
                                                Diagnostic::error(
                                                    path.to_path_buf(),
                                                    1,
                                                    0,
                                                    "CC-HK-027",
                                                    t!(
                                                        "rules.cc_hk_027.message",
                                                        location = hook_location.as_str()
                                                    ),
                                                )
                                                .with_suggestion(t!("rules.cc_hk_027.suggestion")),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        // CC-HK-021: Invalid if field syntax
                        if check_021 {
                            if let Some(if_val) = hook.get("if") {
                                if !tool_events.contains(&event.as_str()) {
                                    conditional_diags.push(
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
                                } else {
                                    let is_invalid = match if_val {
                                        serde_json::Value::String(s) => s.is_empty(),
                                        _ => true,
                                    };
                                    if is_invalid {
                                        conditional_diags.push(
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
                                                "Set 'if' to a valid filter expression string"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                }
                            }
                        }

                        // CC-HK-022: Invalid shell value
                        if check_022 {
                            if let Some(shell_val) = hook.get("shell") {
                                let is_invalid = match shell_val.as_str() {
                                    Some(s) => !valid_shells.contains(&s),
                                    None => true,
                                };
                                if is_invalid {
                                    let shell_display = shell_val
                                        .as_str()
                                        .map(|s| s.to_string())
                                        .unwrap_or_else(|| shell_val.to_string());
                                    conditional_diags.push(
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
                                            "Set 'shell' to either 'bash' or 'powershell'"
                                                .to_string(),
                                        ),
                                    );
                                }
                            }
                        }

                        // CC-HK-023: once field not boolean
                        if check_023 {
                            if let Some(once_val) = hook.get("once") {
                                if !once_val.is_boolean() {
                                    conditional_diags.push(
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
                                        .with_suggestion("Set 'once' to true or false"),
                                    );
                                }
                            }
                        }

                        // CC-HK-024: HTTP headers with $VAR but no allowedEnvVars
                        if check_024 {
                            if let Some(type_val) = hook.get("type").and_then(|t| t.as_str()) {
                                if type_val == "http" {
                                    if let Some(headers) =
                                        hook.get("headers").and_then(|h| h.as_object())
                                    {
                                        let has_env_var = headers.values().any(|v| {
                                            v.as_str().map(|s| s.contains('$')).unwrap_or(false)
                                        });
                                        if has_env_var {
                                            let has_allowed = hook
                                                .get("allowedEnvVars")
                                                .and_then(|v| v.as_array())
                                                .map(|a| !a.is_empty())
                                                .unwrap_or(false);
                                            if !has_allowed {
                                                conditional_diags.push(
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
                        }
                    }
                }
            }
        }
    }

    // If CC-HK-016 fired, discard post-016 diagnostics and signal early return.
    if found_016 {
        return false;
    }

    diagnostics.append(&mut conditional_diags);
    true
}

/// CC-HK-025: Invalid matcher value for event type.
/// Events that support matchers have specific valid values.
/// Validates matcher values on lifecycle events with documented enum values.
#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn validate_cc_hk_025_invalid_matcher_value(
    raw_value: &serde_json::Value,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(hooks_obj) = raw_value.get("hooks").and_then(|h| h.as_object()) {
        for (event, matchers) in hooks_obj {
            let Some(valid_values) = known_matcher_values(event) else {
                continue;
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
            "overloaded",
            "authentication_failed",
            "oauth_org_not_allowed",
            "billing_error",
            "invalid_request",
            "model_not_found",
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
    fn test_cc_hk_025_valid_notification_matchers() {
        let valid_matchers = [
            "permission_prompt",
            "idle_prompt",
            "auth_success",
            "elicitation_dialog",
            "elicitation_complete",
            "elicitation_response",
            "agent_needs_input",
            "agent_completed",
        ];
        for matcher in valid_matchers {
            let content = format!(
                r#"{{
  "hooks": {{
    "Notification": [
      {{
        "matcher": "{}",
        "hooks": [{{ "type": "command", "command": "echo notify" }}]
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
                "Matcher '{}' should be valid for Notification",
                matcher
            );
        }
    }

    #[test]
    fn test_cc_hk_025_invalid_notification_matcher() {
        let content = r#"{
  "hooks": {
    "Notification": [
      {
        "matcher": "unknown_notification",
        "hooks": [{ "type": "command", "command": "echo notify" }]
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
        assert!(diagnostics[0].message.contains("unknown_notification"));
        assert!(diagnostics[0].message.contains("Notification"));
    }

    #[test]
    fn test_cc_hk_025_valid_setup_and_session_end_matchers() {
        let cases = [
            ("Setup", "init"),
            ("Setup", "maintenance"),
            ("SessionEnd", "clear"),
            ("SessionEnd", "resume"),
            ("SessionEnd", "logout"),
            ("SessionEnd", "prompt_input_exit"),
            ("SessionEnd", "bypass_permissions_disabled"),
            ("SessionEnd", "other"),
            ("PreCompact", "manual"),
            ("PostCompact", "auto"),
            ("ConfigChange", "skills"),
            ("InstructionsLoaded", "path_glob_match"),
        ];

        for (event, matcher) in cases {
            let content = format!(
                r#"{{
  "hooks": {{
    "{}": [
      {{
        "matcher": "{}",
        "hooks": [{{ "type": "command", "command": "echo ok" }}]
      }}
    ]
  }}
}}"#,
                event, matcher
            );
            let raw_value: serde_json::Value = serde_json::from_str(&content).unwrap();
            let path = Path::new(".claude/settings.json");
            let mut diagnostics = Vec::new();

            validate_cc_hk_025_invalid_matcher_value(&raw_value, path, &mut diagnostics);

            assert!(
                diagnostics.is_empty(),
                "Matcher '{}' should be valid for {}",
                matcher,
                event
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

    // ===== Tilde-path handling (CC-HK-008 false-positive fix) =====

    #[test]
    fn test_is_tilde_user_path_detects_user_form() {
        assert!(is_tilde_user_path("~alice/bin/hook.sh"));
        assert!(is_tilde_user_path("~root/hook.py"));
    }

    #[test]
    fn test_is_tilde_user_path_rejects_bare_home() {
        assert!(!is_tilde_user_path("~"));
        assert!(!is_tilde_user_path("~/.claude/hooks/hook.py"));
        assert!(!is_tilde_user_path("~\\claude\\hooks\\hook.py"));
    }

    #[test]
    fn test_is_tilde_user_path_rejects_non_tilde_prefix() {
        assert!(!is_tilde_user_path("/abs/path"));
        assert!(!is_tilde_user_path("./relative/path"));
        assert!(!is_tilde_user_path("path/with/~tilde/inside"));
    }

    #[test]
    #[cfg(feature = "filesystem")]
    fn test_expand_tilde_prefix_replaces_home() {
        let home = dirs::home_dir().expect("test environment has HOME");
        let home_str = home.display().to_string();

        let expanded = expand_tilde_prefix("~/.claude/hooks/hook.py");
        assert_eq!(expanded, format!("{}/.claude/hooks/hook.py", home_str));

        let bare = expand_tilde_prefix("~");
        assert_eq!(bare, home_str);
    }

    #[test]
    #[cfg(feature = "filesystem")]
    fn test_expand_tilde_prefix_leaves_user_form_untouched() {
        // ~user/... is not expanded - we don't resolve arbitrary user homes.
        assert_eq!(expand_tilde_prefix("~alice/hook.sh"), "~alice/hook.sh");
    }

    #[test]
    fn test_expand_tilde_prefix_leaves_mid_string_tilde_alone() {
        // The shell only special-cases a *leading* tilde; a tilde mid-path
        // is literal. Preserve both to match Claude Code's pass-through
        // semantics.
        assert_eq!(
            expand_tilde_prefix("/opt/x/~backup/hook.sh"),
            "/opt/x/~backup/hook.sh"
        );
        assert_eq!(expand_tilde_prefix("no-tilde"), "no-tilde");
    }

    #[test]
    #[cfg(feature = "filesystem")]
    fn test_has_unresolved_env_vars_skips_resolvable_tilde() {
        // On a machine with HOME, `~/...` is resolvable -> NOT unresolved.
        if dirs::home_dir().is_some() {
            assert!(!has_unresolved_env_vars("~/.claude/hooks/hook.py"));
        }
    }

    #[test]
    fn test_has_unresolved_env_vars_flags_user_form() {
        // `~user/...` is always treated as unresolved: we don't look up
        // arbitrary user home directories. CC-HK-008 will skip these.
        assert!(has_unresolved_env_vars("~alice/.local/bin/hook.sh"));
        assert!(has_unresolved_env_vars("~root/hook.py"));
    }

    #[test]
    fn test_has_unresolved_env_vars_flags_dollar_vars_unchanged() {
        // Regression guard for the existing `$FOO` behavior - must not
        // regress now that the helper also considers `~`.
        assert!(has_unresolved_env_vars("$HOME/.claude/hook.sh"));
        assert!(has_unresolved_env_vars("/opt/$PROJECT/hook.sh"));
        assert!(!has_unresolved_env_vars("$CLAUDE_PROJECT_DIR/hook.sh"));
        assert!(!has_unresolved_env_vars("${CLAUDE_PROJECT_DIR}/hook.sh"));
    }

    #[test]
    #[cfg(feature = "filesystem")]
    fn test_resolve_script_path_expands_tilde() {
        let home = dirs::home_dir().expect("test environment has HOME");
        let resolved = resolve_script_path("~/.claude/hooks/hook.py", Path::new("/workspace"));
        // resolve_script_path returns absolute; compare via PathBuf so we
        // don't hard-code the separator across platforms.
        let expected = home.join(".claude").join("hooks").join("hook.py");
        assert_eq!(resolved, expected);
    }

    #[test]
    fn test_resolve_script_path_leaves_non_tilde_absolute_unchanged() {
        let resolved = resolve_script_path("/usr/local/bin/hook.sh", Path::new("/workspace"));
        assert_eq!(resolved, std::path::PathBuf::from("/usr/local/bin/hook.sh"));
    }

    #[test]
    fn test_resolve_script_path_joins_relative_paths() {
        // Regression guard: relative paths still resolve under project_dir,
        // not under $HOME, after the tilde fix.
        let resolved = resolve_script_path(".claude/hooks/hook.sh", Path::new("/workspace"));
        assert_eq!(
            resolved,
            std::path::PathBuf::from("/workspace/.claude/hooks/hook.sh")
        );
    }
}
