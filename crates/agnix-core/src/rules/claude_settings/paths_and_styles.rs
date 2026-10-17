//! Settings values that name files or styles:
//!
//! - CC-OS-007: `outputStyle` that differs from a style name only in case.

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
