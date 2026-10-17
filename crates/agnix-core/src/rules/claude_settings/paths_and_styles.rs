//! Settings values that name files or styles:
//!
//! - CC-OS-007: `outputStyle` that differs from a style name only in case.
//! - CC-SET-040: `claudeMdExcludes` patterns that cannot match an absolute
//!   path, and an `autoMemoryDirectory` that is neither absolute nor `~/`.

use crate::{
    diagnostics::{Diagnostic, Fix},
    fs::FileSystem,
    schemas::kiro_json::{JsonNode, LineIndex},
};
use rust_i18n::t;
use std::path::Path;

/// Built-in output styles. "The value is case-sensitive, so write the
/// built-in names as `Proactive`, `Concise`, `Explanatory`, and `Learning`."
/// `default` is left out: a mismatch falls back to the Default style anyway.
const BUILTIN_OUTPUT_STYLES: &[&str] = &["Proactive", "Concise", "Explanatory", "Learning"];

/// Names of the custom output styles next to a settings file: the frontmatter
/// `name`, or the file stem ("The file name becomes the style name unless you
/// set `name` in the frontmatter").
fn custom_output_styles(settings_path: &Path, fs: &dyn FileSystem) -> Vec<String> {
    let Some(dir) = settings_path.parent().map(|p| p.join("output-styles")) else {
        return Vec::new();
    };
    let Ok(entries) = fs.read_dir(&dir) else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for entry in entries {
        if !entry.metadata.is_file || entry.path.extension().and_then(|e| e.to_str()) != Some("md")
        {
            continue;
        }
        let stem = entry
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        let name = fs
            .read_to_string(&entry.path)
            .ok()
            .and_then(|text| {
                crate::schemas::output_style::parse_frontmatter(&text)
                    .and_then(|parsed| parsed.schema)
                    .and_then(|schema| schema.name)
            })
            .filter(|name| !name.trim().is_empty())
            .unwrap_or(stem);
        names.push(name);
    }
    names
}

/// CC-OS-007: an `outputStyle` that matches a style only when case is ignored.
pub(super) fn check_output_style_case(
    path: &Path,
    content: &str,
    root: &JsonNode,
    fs: &dyn FileSystem,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(member) = root.value.get("outputStyle") else {
        return;
    };
    let Some(value) = member.value.value.as_str() else {
        return;
    };
    let custom = custom_output_styles(path, fs);
    if custom.iter().any(|name| name == value) || BUILTIN_OUTPUT_STYLES.contains(&value) {
        return;
    }
    let (expected, kind) = if let Some(builtin) = BUILTIN_OUTPUT_STYLES
        .iter()
        .find(|name| name.eq_ignore_ascii_case(value))
    {
        ((*builtin).to_string(), "built-in")
    } else if let Some(name) = custom.iter().find(|name| name.eq_ignore_ascii_case(value)) {
        (name.clone(), "custom")
    } else {
        // Unknown names may be user-level or plugin styles agnix cannot see.
        return;
    };

    let (line, column) = LineIndex::new(content).line_col(member.value.start);
    let replacement = serde_json::to_string(&expected).expect("string serialization");
    diagnostics.push(
        Diagnostic::warning(
            path.to_path_buf(),
            line,
            column,
            "CC-OS-007",
            t!(
                "rules.cc_os_007.message",
                value = value,
                kind = kind,
                expected = expected.as_str()
            ),
        )
        .with_suggestion(t!(
            "rules.cc_os_007.suggestion",
            expected = expected.as_str()
        ))
        // Selecting the intended style changes Claude's behavior: opt-in.
        .with_fix(Fix::replace(
            member.value.start,
            member.value.end,
            replacement,
            t!("rules.cc_os_007.fix", expected = expected.as_str()),
            false,
        )),
    );
}

/// True for a path written as absolute on Unix or Windows.
fn is_absolute_like(value: &str) -> bool {
    let bytes = value.as_bytes();
    value.starts_with('/')
        || value.starts_with('\\')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
}

/// CC-SET-040.
pub(super) fn check_memory_paths(
    path: &Path,
    content: &str,
    root: &JsonNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let lines = LineIndex::new(content);

    // "Patterns are matched against absolute file paths using glob syntax."
    // A pattern that is relative and starts with a literal segment, such as
    // `packages/web/CLAUDE.md`, can never match.
    if let Some(member) = root.value.get("claudeMdExcludes")
        && let Some(items) = member.value.value.as_array()
    {
        for item in items {
            let Some(pattern) = item.value.as_str() else {
                continue;
            };
            let trimmed = pattern.trim();
            if trimmed.is_empty()
                || is_absolute_like(trimmed)
                || trimmed.starts_with('*')
                || trimmed.starts_with('~')
            {
                continue;
            }
            let (line, column) = lines.line_col(item.start);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    column,
                    "CC-SET-040",
                    t!("rules.cc_set_040.exclude_message", pattern = pattern),
                )
                .with_suggestion(t!(
                    "rules.cc_set_040.exclude_suggestion",
                    pattern = trimmed.trim_start_matches("./")
                )),
            );
        }
    }

    // "The value must be an absolute path or start with `~/`."
    if let Some(member) = root.value.get("autoMemoryDirectory")
        && let Some(value) = member.value.value.as_str()
    {
        let trimmed = value.trim();
        if !trimmed.is_empty() && !is_absolute_like(trimmed) && !trimmed.starts_with("~/") {
            let (line, column) = lines.line_col(member.value.start);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    column,
                    "CC-SET-040",
                    t!("rules.cc_set_040.memory_dir_message", value = value),
                )
                .with_suggestion(t!("rules.cc_set_040.memory_dir_suggestion")),
            );
        }
    }
}
