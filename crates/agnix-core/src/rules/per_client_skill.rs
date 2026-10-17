//! Per-client skill validation
//!
//! Detects unsupported frontmatter fields in SKILL.md files based on the
//! client directory they reside in. For example, a skill in `.cursor/skills/`
//! should not use fields that Cursor does not support.

use crate::config::{LintConfig, PerFileLintConfig, TargetTool};
use crate::diagnostics::{Diagnostic, Fix};
use crate::parsers::frontmatter::split_frontmatter;
use crate::rules::kiro_steering::{
    YamlTopLevelEntry, frontmatter_edit_yields, single_line_scalar_text, yaml_top_level_entries,
};
use crate::rules::{Validator, ValidatorMetadata};
use rust_i18n::t;
use std::path::{Component, Path, PathBuf};

/// Known clients that host SKILL.md files.
///
/// The `Amp` variant is defined for completeness (AMP-SK-001 exists in rules.json)
/// but `.agents/` paths are mapped to `Codex` since both clients share that directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SkillClient {
    ClaudeCode,
    Cursor,
    Cline,
    Copilot,
    Codex,
    OpenCode,
    Windsurf,
    Kiro,
    Amp,
    RooCode,
    Unknown,
}

/// Universal frontmatter fields supported by the Agent Skills specification.
/// These are safe to use in any client.
const UNIVERSAL_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
    "allowed-tools",
];

/// SKILL.md frontmatter fields Kiro reads (kiro.dev Skills; the V3 skill
/// schema validates exactly these and passes every other key through unread).
/// `allowed-tools` is not among them: Kiro never restricts a skill's tools.
const KIRO_SKILL_FIELDS: &[&str] = &[
    "name",
    "description",
    "license",
    "compatibility",
    "metadata",
];

/// Fields other skill clients act on (Agent Skills `allowed-tools` and the
/// Claude Code extensions). Kiro ignores them, so a skill relying on one does
/// not behave in Kiro the way it does elsewhere.
const OTHER_CLIENT_SKILL_FIELDS: &[&str] = &[
    "allowed-tools",
    "disallowed-tools",
    "when_to_use",
    "arguments",
    "argument-hint",
    "disable-model-invocation",
    "user-invocable",
    "model",
    "context",
    "background",
    "agent",
    "hooks",
    "effort",
    "paths",
    "shell",
];

/// Groups the KR-SK-001 fixes that create a `metadata` block, so one run
/// applies at most one of them.
const KIRO_CREATE_METADATA_GROUP: &str = "kr-sk-001-create-metadata";

/// Check whether a client supports a given frontmatter field.
///
/// Claude Code supports all extension fields. Cursor additionally supports
/// `disable-model-invocation`. All other clients only support universal fields.
fn is_field_supported(client: SkillClient, field: &str) -> bool {
    if client == SkillClient::Kiro {
        return KIRO_SKILL_FIELDS.contains(&field);
    }
    if UNIVERSAL_FIELDS.contains(&field) {
        return true;
    }
    match client {
        SkillClient::ClaudeCode => true,
        SkillClient::Cursor => field == "disable-model-invocation",
        _ => false,
    }
}

/// Whether the Claude Code skill rules (`CC-SK-*`) should run for a skill with
/// the given resolved `client` under `config`.
///
/// They apply to Claude Code skills and to genuinely unscoped skills (a bare
/// `SKILL.md` that could be a Claude skill), but NOT to skills owned by another
/// known tool (Codex, OpenCode, Cursor, …), which have their own rule sets.
///
/// `resolve_skill_client` collapses both "unscoped" and "ambiguous multi-tool"
/// to `Unknown`. Those differ: an explicit `tools = ["codex", "cursor"]` scope
/// rules Claude *out*, while no scope leaves it plausible. So for `Unknown` we
/// re-check `config`: Claude rules apply only when the scope is empty or names
/// `claude-code`.
pub(crate) fn claude_skill_rules_apply(client: SkillClient, config: &LintConfig) -> bool {
    match client {
        SkillClient::ClaudeCode => true,
        SkillClient::Unknown => {
            let tools = config.tools();
            tools.is_empty()
                || tools
                    .iter()
                    .any(|t| skill_client_from_tool_str(t) == Some(SkillClient::ClaudeCode))
        }
        _ => false,
    }
}

/// Detect which client owns a SKILL.md based on its path components.
///
/// Iterates path components looking for the `skills` directory name, then
/// checks the parent component for a known client directory marker.
pub(crate) fn detect_client(path: &Path) -> SkillClient {
    let components: Vec<&str> = path
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();

    // Walk backwards from the end to find "skills", then look at the component before it.
    let Some(skills_idx) = components.iter().rposition(|c| *c == "skills") else {
        return SkillClient::Unknown;
    };
    let Some(owner_idx) = skills_idx.checked_sub(1) else {
        return SkillClient::Unknown;
    };
    match components[owner_idx] {
        ".claude" => SkillClient::ClaudeCode,
        ".cursor" => SkillClient::Cursor,
        ".cline" => SkillClient::Cline,
        ".github" => SkillClient::Copilot,
        ".agents" => SkillClient::Codex, // NOTE: Amp also uses .agents; AMP-SK-001 unreachable without config context
        // `$CODEX_HOME/skills`, by default `~/.codex/skills`, including the
        // `.system` skills Codex installs there.
        ".codex" => SkillClient::Codex,
        ".opencode" => SkillClient::OpenCode,
        // OpenCode's global skills: `~/.config/opencode/skills/<name>/SKILL.md`.
        "opencode" if owner_idx > 0 && components[owner_idx - 1] == ".config" => {
            SkillClient::OpenCode
        }
        ".windsurf" => SkillClient::Windsurf,
        ".kiro" => SkillClient::Kiro,
        ".roo" => SkillClient::RooCode,
        _ => SkillClient::Unknown,
    }
}

/// Normalize a plugin component path without allowing it to escape the plugin
/// root. Claude accepts both `/` and `\` separators in manifest paths.
fn safe_plugin_relative_path(raw: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let normalized = trimmed.replace('\\', "/");
    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return None;
    }

    let mut relative = PathBuf::new();
    for component in Path::new(&normalized).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => relative.push(part),
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(relative)
}

/// Return whether `path` is a skill owned by a Claude Code plugin.
///
/// Claude always scans the plugin root's `skills/` directory and loads any
/// additional string or array paths declared by the manifest's `skills` field.
/// Require the documented sibling manifest so a generic `skills/foo/SKILL.md`
/// tree is not misclassified as Claude-owned.
fn is_claude_plugin_skill(path: &Path, config: &LintConfig) -> bool {
    if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return false;
    }

    let Some(parent) = path.parent() else {
        return false;
    };

    for plugin_root in parent.ancestors() {
        let manifest_path = plugin_root.join(".claude-plugin").join("plugin.json");
        if !config.fs().is_file(&manifest_path) {
            continue;
        }

        if path.starts_with(plugin_root.join("skills")) {
            return true;
        }

        let Ok(manifest_content) = config.fs().read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_content) else {
            continue;
        };
        let Some(declared_skills) = manifest.get("skills") else {
            continue;
        };

        let declared_paths: Vec<&str> = match declared_skills {
            serde_json::Value::String(path) => vec![path],
            serde_json::Value::Array(paths) => {
                paths.iter().filter_map(serde_json::Value::as_str).collect()
            }
            _ => Vec::new(),
        };

        if declared_paths.into_iter().any(|declared| {
            safe_plugin_relative_path(declared).is_some_and(|relative| {
                if relative.as_os_str().is_empty() {
                    path.parent() == Some(plugin_root)
                } else {
                    path.starts_with(plugin_root.join(relative))
                }
            })
        }) {
            return true;
        }
    }

    false
}

/// Map a `tools = [...]` entry (or `--tools` value) to a [`SkillClient`].
/// Matching is case-insensitive to tolerate configs that use different casing.
fn skill_client_from_tool_str(tool: &str) -> Option<SkillClient> {
    Some(match tool.to_ascii_lowercase().as_str() {
        "claude-code" => SkillClient::ClaudeCode,
        "codex" => SkillClient::Codex,
        "opencode" => SkillClient::OpenCode,
        "kiro" => SkillClient::Kiro,
        "cursor" => SkillClient::Cursor,
        "cline" => SkillClient::Cline,
        "copilot" | "github-copilot" => SkillClient::Copilot,
        "windsurf" => SkillClient::Windsurf,
        "amp" => SkillClient::Amp,
        "roo-code" => SkillClient::RooCode,
        _ => return None,
    })
}

/// Resolve a skill's owning client from its path, falling back to the
/// configured target when the path carries no client marker (e.g. a bare
/// `skills/foo/SKILL.md` validated with `tools = ["claude-code"]`).
///
/// Returns [`SkillClient::Unknown`] when neither the path nor an unambiguous
/// config target identifies a client - in that case the generic agentskills.io
/// baseline applies. A `tools` array naming more than one distinct client is
/// treated as ambiguous and resolves directly to `Unknown` (the deprecated
/// single-valued `target` is only consulted when `tools` names no client).
pub(crate) fn resolve_skill_client(path: &Path, config: &LintConfig) -> SkillClient {
    let by_path = detect_client(path);
    if by_path != SkillClient::Unknown {
        return by_path;
    }
    if is_claude_plugin_skill(path, config) {
        return SkillClient::ClaudeCode;
    }

    let mut from_tools = config
        .tools()
        .iter()
        .filter_map(|t| skill_client_from_tool_str(t));
    if let Some(first) = from_tools.next() {
        if from_tools.all(|c| c == first) {
            return first;
        }
        return SkillClient::Unknown; // ambiguous multi-tool scope
    }

    match config.target() {
        TargetTool::ClaudeCode => SkillClient::ClaudeCode,
        TargetTool::Codex => SkillClient::Codex,
        TargetTool::Cursor => SkillClient::Cursor,
        TargetTool::Kiro => SkillClient::Kiro,
        TargetTool::Generic => SkillClient::Unknown,
    }
}

/// Return the rule ID for a per-client unsupported-field warning.
fn rule_id_for_client(client: SkillClient) -> Option<&'static str> {
    match client {
        SkillClient::Cursor => Some("CR-SK-001"),
        SkillClient::Cline => Some("CL-SK-001"),
        SkillClient::Copilot => Some("CP-SK-001"),
        SkillClient::Codex => Some("CX-SK-001"),
        SkillClient::OpenCode => Some("OC-SK-001"),
        SkillClient::Windsurf => Some("WS-SK-001"),
        SkillClient::Kiro => Some("KR-SK-001"),
        SkillClient::Amp => Some("AMP-SK-001"),
        SkillClient::RooCode => Some("RC-SK-001"),
        SkillClient::ClaudeCode | SkillClient::Unknown => None,
    }
}

/// Return a human-readable display name for a client.
fn client_display_name(client: SkillClient) -> &'static str {
    match client {
        SkillClient::ClaudeCode => "Claude Code",
        SkillClient::Cursor => "Cursor",
        SkillClient::Cline => "Cline",
        SkillClient::Copilot => "GitHub Copilot",
        SkillClient::Codex => "Codex CLI",
        SkillClient::OpenCode => "OpenCode",
        SkillClient::Windsurf => "Windsurf",
        SkillClient::Kiro => "Kiro",
        SkillClient::Amp => "Amp",
        SkillClient::RooCode => "Roo Code",
        SkillClient::Unknown => "Unknown",
    }
}

/// Return the i18n key prefix for a per-client rule.
fn i18n_key_for_client(client: SkillClient) -> Option<&'static str> {
    match client {
        SkillClient::Cursor => Some("cr_sk_001"),
        SkillClient::Cline => Some("cl_sk_001"),
        SkillClient::Copilot => Some("cp_sk_001"),
        SkillClient::Codex => Some("cx_sk_001"),
        SkillClient::OpenCode => Some("oc_sk_001"),
        SkillClient::Windsurf => Some("ws_sk_001"),
        SkillClient::Kiro => Some("kr_sk_001"),
        SkillClient::Amp => Some("amp_sk_001"),
        SkillClient::RooCode => Some("rc_sk_001"),
        SkillClient::ClaudeCode | SkillClient::Unknown => None,
    }
}

/// Compute line starts for position tracking.
fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

/// Convert a byte offset to (line, column), both 1-indexed.
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
    (low + 1, offset.saturating_sub(line_start) + 1)
}

/// Advance `fm_offset` past the current line and its newline terminator.
fn advance_past_line(fm_offset: usize, line_len: usize, fm_bytes: &[u8]) -> usize {
    let line_end = fm_offset + line_len;
    if line_end < fm_bytes.len() {
        if fm_bytes[line_end] == b'\n' {
            line_end + 1
        } else if line_end + 1 < fm_bytes.len()
            && fm_bytes[line_end] == b'\r'
            && fm_bytes[line_end + 1] == b'\n'
        {
            line_end + 2
        } else {
            line_end
        }
    } else {
        line_end
    }
}

const RULE_IDS: &[&str] = &[
    "AMP-SK-001",
    "CL-SK-001",
    "CP-SK-001",
    "CR-SK-001",
    "CX-SK-001",
    "KR-SK-001",
    "OC-SK-001",
    "RC-SK-001",
    "WS-SK-001",
    "XP-SK-001",
];

// ---------------------------------------------------------------------------
// Kiro (KR-SK-001)
// ---------------------------------------------------------------------------

/// Where a moved field can go.
enum KiroMetadataTarget {
    /// No `metadata` key: a fix may create the block in place.
    Absent,
    /// A block mapping with at least one child line.
    Block {
        /// Start of the `metadata:` line.
        start: usize,
        /// End of the `metadata:` line (before its line break).
        header_end: usize,
        /// Indentation of the existing children.
        indent: String,
    },
    /// Flow, null, scalar or otherwise unusual metadata: fields are not moved.
    Unusable,
}

fn kiro_metadata_target(
    frontmatter: &str,
    entries: &[YamlTopLevelEntry],
    mapping: &serde_yaml::Mapping,
) -> KiroMetadataTarget {
    let Some(value) = mapping.get(serde_yaml::Value::String("metadata".into())) else {
        return KiroMetadataTarget::Absent;
    };
    let Some(children) = value.as_mapping() else {
        return KiroMetadataTarget::Unusable;
    };
    let Some(entry) = entries.iter().find(|entry| entry.key == "metadata") else {
        return KiroMetadataTarget::Unusable;
    };
    // The value must start on the next line (not `metadata: {...}`).
    if children.is_empty()
        || entry.value_start != entry.line_end
        || entry.value_end <= entry.line_end
    {
        return KiroMetadataTarget::Unusable;
    }
    let indent = frontmatter[entry.line_end..entry.value_end]
        .lines()
        .map(|line| (line, line.trim_start()))
        .find(|(_, trimmed)| !trimmed.is_empty() && !trimmed.starts_with('#'))
        .map(|(line, trimmed)| &line[..line.len() - trimmed.len()]);
    match indent {
        Some(indent) if !indent.is_empty() && indent.bytes().all(|b| b == b' ') => {
            KiroMetadataTarget::Block {
                start: entry.start,
                header_end: entry.line_end,
                indent: indent.to_string(),
            }
        }
        _ => KiroMetadataTarget::Unusable,
    }
}

/// The value text to write under `metadata` for a single-line scalar, and the
/// value it must parse to. Strings keep their exact text; numbers and booleans
/// are quoted so `metadata` stays a string-to-string map.
fn kiro_movable_value(
    frontmatter: &str,
    entry: &YamlTopLevelEntry,
    value: &serde_yaml::Value,
) -> Option<(String, serde_yaml::Value)> {
    let scalar = single_line_scalar_text(frontmatter, entry)?;
    match value {
        serde_yaml::Value::String(_) => Some((
            frontmatter[entry.value_start..entry.line_end].to_string(),
            value.clone(),
        )),
        serde_yaml::Value::Number(_) | serde_yaml::Value::Bool(_) => Some((
            serde_json::to_string(scalar).ok()?,
            serde_yaml::Value::String(scalar.to_string()),
        )),
        _ => None,
    }
}

/// Build the KR-SK-001 fix for one unsupported top-level field: move it under
/// `metadata` when it is a single-line scalar, otherwise delete the whole
/// entry. Every candidate is applied to a copy and re-parsed; a fix is offered
/// only when the frontmatter parses to exactly the intended mapping. Moves are
/// safe for plain data fields (Kiro reads neither location); moving another
/// client's field, and every deletion, is unsafe.
fn kiro_field_fix(
    content: &str,
    parts: &crate::parsers::frontmatter::FrontmatterParts,
    yaml: &serde_yaml::Value,
    entries: &[YamlTopLevelEntry],
    entry: &YamlTopLevelEntry,
    value: &serde_yaml::Value,
    other_client_field: bool,
) -> Option<Fix> {
    let frontmatter = parts.frontmatter.as_str();
    let base = parts.frontmatter_start;
    let key = serde_yaml::Value::String(entry.key.clone());
    let mut without_key = yaml.clone();
    without_key.as_mapping_mut()?.shift_remove(&key);

    if let Some((text, moved)) = kiro_movable_value(frontmatter, entry, value) {
        let mut expected = without_key.clone();
        let target = kiro_metadata_target(frontmatter, entries, yaml.as_mapping()?);
        let candidate = match &target {
            KiroMetadataTarget::Absent => {
                expected.as_mapping_mut()?.insert(
                    serde_yaml::Value::String("metadata".into()),
                    serde_yaml::Value::Mapping(serde_yaml::Mapping::from_iter([(
                        key.clone(),
                        moved,
                    )])),
                );
                Some((
                    entry.start,
                    entry.line_end,
                    format!("metadata:\n  {}: {text}", entry.key),
                    true,
                ))
            }
            KiroMetadataTarget::Block {
                start,
                header_end,
                indent,
            } => {
                expected
                    .get_mut("metadata")?
                    .as_mapping_mut()?
                    .insert(key.clone(), moved);
                let inserted = format!("\n{indent}{}: {text}", entry.key);
                if entry.start < *start {
                    // Field above the block: drop its line, insert after the header.
                    let after_line = (entry.line_end + 1).min(*header_end);
                    Some((
                        entry.start,
                        *header_end,
                        format!("{}{inserted}", &frontmatter[after_line..*header_end]),
                        false,
                    ))
                } else if entry.start > 0 && frontmatter.as_bytes()[entry.start - 1] == b'\n' {
                    // Field below the block: insert after the header, drop its line.
                    Some((
                        *header_end,
                        entry.line_end,
                        format!("{inserted}{}", &frontmatter[*header_end..entry.start - 1]),
                        false,
                    ))
                } else {
                    None
                }
            }
            KiroMetadataTarget::Unusable => None,
        };
        if let Some((start, end, replacement, creates_metadata)) = candidate
            && frontmatter_edit_yields(content, base + start, base + end, &replacement, &expected)
        {
            let mut fix = Fix::replace(
                base + start,
                base + end,
                replacement,
                format!("Move '{}' under metadata", entry.key),
                !other_client_field,
            );
            if creates_metadata {
                fix = fix.with_group(KIRO_CREATE_METADATA_GROUP);
            }
            return Some(fix);
        }
    }

    // Delete the whole entry: key line, continuation lines and the line break.
    // A last entry also takes the line break before the closing `---`.
    let end = (base + entry.value_end + 1).min(content.len());
    frontmatter_edit_yields(content, base + entry.start, end, "", &without_key).then(|| {
        Fix::delete(
            base + entry.start,
            end,
            format!("Remove '{}', which Kiro ignores", entry.key),
            false,
        )
    })
}

/// KR-SK-001 for Kiro skills. Runs only on frontmatter that parses (AS-016
/// reports YAML errors; Kiro skips such skills) and reads the keys from the
/// parsed mapping, so nested or continuation lines are never mistaken for
/// fields.
fn validate_kiro_skill(
    path: &Path,
    content: &str,
    parts: &crate::parsers::frontmatter::FrontmatterParts,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter) else {
        return;
    };
    let Some(mapping) = yaml.as_mapping() else {
        return;
    };
    let entries = yaml_top_level_entries(&parts.frontmatter);
    let line_starts = compute_line_starts(content);
    for (key, value) in mapping {
        let Some(field) = key.as_str() else {
            continue;
        };
        if is_field_supported(SkillClient::Kiro, field) {
            continue;
        }
        let entry = entries.iter().find(|entry| entry.key == field);
        let offset = parts.frontmatter_start + entry.map_or(0, |entry| entry.start);
        let (line, col) = line_col_at(offset, &line_starts);
        let other_client_field = OTHER_CLIENT_SKILL_FIELDS.contains(&field);
        let message_key = if field == "allowed-tools" || field == "disallowed-tools" {
            "kr_sk_001_tools"
        } else if other_client_field {
            "kr_sk_001"
        } else {
            "kr_sk_001_data"
        };
        let message = t!(&format!("rules.{message_key}.message"), field = field);
        let suggestion = t!(&format!("rules.{message_key}.suggestion"), field = field);
        let mut diagnostic = if other_client_field {
            Diagnostic::warning(path.to_path_buf(), line, col, "KR-SK-001", message)
        } else {
            Diagnostic::info(path.to_path_buf(), line, col, "KR-SK-001", message)
        }
        .with_suggestion(suggestion);
        if let Some(entry) = entry
            && let Some(fix) = kiro_field_fix(
                content,
                parts,
                &yaml,
                &entries,
                entry,
                value,
                other_client_field,
            )
        {
            diagnostic = diagnostic.with_fix(fix);
        }
        diagnostics.push(diagnostic);
    }
}

pub struct PerClientSkillValidator;

impl Validator for PerClientSkillValidator {
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

        let parts = split_frontmatter(content);
        if !parts.has_frontmatter || !parts.has_closing {
            return diagnostics;
        }

        let client = resolve_skill_client(path, config);

        let per_client_rule = rule_id_for_client(client);
        let has_per_client = per_client_rule
            .map(|r| config.is_rule_enabled(r))
            .unwrap_or(false);
        let has_xp = config.is_rule_enabled("XP-SK-001");

        if !has_per_client && !has_xp {
            return diagnostics;
        }

        // Kiro: KR-SK-001 says everything XP-SK-001 would (Kiro supports only
        // universal fields), so it replaces XP-SK-001 for Kiro skills.
        if client == SkillClient::Kiro && has_per_client {
            validate_kiro_skill(path, content, &parts, &mut diagnostics);
            return diagnostics;
        }

        let line_starts = compute_line_starts(content);
        let frontmatter_str = &parts.frontmatter;
        let fm_bytes = frontmatter_str.as_bytes();
        let mut fm_offset = 0usize;

        for line in frontmatter_str.lines() {
            let trimmed = line.trim_start();
            let leading_ws = line.len() - trimmed.len();

            // Skip empty lines, comments, and indented lines (nested YAML values
            // like sub-keys under `metadata:` are not top-level fields)
            if trimmed.is_empty() || trimmed.starts_with('#') || leading_ws > 0 {
                fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                continue;
            }

            // Extract key from top-level "key: value" or "key:" pattern
            let key = match trimmed.find(':') {
                Some(colon_pos) => trimmed[..colon_pos].trim(),
                None => {
                    fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                    continue;
                }
            };

            // Universal fields are safe everywhere - skip them
            if UNIVERSAL_FIELDS.contains(&key) {
                fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
                continue;
            }

            // Non-universal top-level key: check against client support
            let abs_key_start = parts.frontmatter_start + fm_offset;
            let (line_num, col) = line_col_at(abs_key_start, &line_starts);

            // Compute byte range for the full line including trailing newline
            let abs_line_start = parts.frontmatter_start + fm_offset;
            let mut abs_line_end = abs_line_start + line.len();
            if abs_line_end < content.len() && content.as_bytes()[abs_line_end] == b'\n' {
                abs_line_end += 1;
            } else if abs_line_end + 1 < content.len()
                && content.as_bytes()[abs_line_end] == b'\r'
                && content.as_bytes()[abs_line_end + 1] == b'\n'
            {
                abs_line_end += 2;
            }

            // Check whether the next line is indented (multi-line YAML value).
            // If so, deleting only the key line would leave orphaned content,
            // so the auto-fix is marked unsafe.
            let next_line_indented = content
                .get(abs_line_end..)
                .and_then(|rest| rest.lines().next())
                .is_some_and(|next| {
                    let nxt = next.trim_start();
                    !nxt.is_empty() && next.len() > nxt.len()
                });
            let fix_is_safe = !next_line_indented;

            // Per-client rule: warn if client does not support this field
            if has_per_client {
                if let Some(rule_id) = per_client_rule {
                    if !is_field_supported(client, key) {
                        let i18n_key = i18n_key_for_client(client).unwrap_or("cr_sk_001");
                        let msg_key = format!("rules.{}.message", i18n_key);
                        let sug_key = format!("rules.{}.suggestion", i18n_key);

                        diagnostics.push(
                            Diagnostic::warning(
                                path.to_path_buf(),
                                line_num,
                                col,
                                rule_id,
                                t!(&msg_key, field = key, client = client_display_name(client)),
                            )
                            .with_suggestion(t!(
                                &sug_key,
                                field = key,
                                client = client_display_name(client)
                            ))
                            .with_fix(Fix::delete(
                                abs_line_start,
                                abs_line_end,
                                format!(
                                    "Remove unsupported field '{}' for {}",
                                    key,
                                    client_display_name(client)
                                ),
                                fix_is_safe,
                            )),
                        );
                    }
                }
            }

            // XP-SK-001: cross-platform portability warning for any skill
            // with non-universal fields, except Claude Code (which supports all)
            if has_xp && client != SkillClient::ClaudeCode {
                diagnostics.push(
                    Diagnostic::info(
                        path.to_path_buf(),
                        line_num,
                        col,
                        "XP-SK-001",
                        t!("rules.xp_sk_001.message", field = key),
                    )
                    .with_suggestion(t!("rules.xp_sk_001.suggestion", field = key)),
                );
            }

            fm_offset = advance_past_line(fm_offset, line.len(), fm_bytes);
        }

        diagnostics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::rules::Validator;
    use std::fs;

    fn make_skill(frontmatter: &str, body: &str) -> String {
        format!("---\n{}\n---\n{}", frontmatter, body)
    }

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        let validator = PerClientSkillValidator;
        let config = LintConfig::default();
        validator.validate(Path::new(path), content, &config)
    }

    // ===== detect_client tests =====

    #[test]
    fn test_detect_client_claude() {
        assert_eq!(
            detect_client(Path::new(".claude/skills/my-skill/SKILL.md")),
            SkillClient::ClaudeCode
        );
    }

    #[test]
    fn test_detect_client_cursor() {
        assert_eq!(
            detect_client(Path::new(".cursor/skills/my-skill/SKILL.md")),
            SkillClient::Cursor
        );
    }

    #[test]
    fn test_detect_client_cline() {
        assert_eq!(
            detect_client(Path::new(".cline/skills/my-skill/SKILL.md")),
            SkillClient::Cline
        );
    }

    #[test]
    fn test_detect_client_cline_alt() {
        // .clinerules is for Cline rules files, not skills; skills should be under .cline/
        assert_eq!(
            detect_client(Path::new(".clinerules/skills/my-skill/SKILL.md")),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_detect_client_copilot() {
        assert_eq!(
            detect_client(Path::new(".github/skills/my-skill/SKILL.md")),
            SkillClient::Copilot
        );
    }

    #[test]
    fn test_detect_client_codex() {
        assert_eq!(
            detect_client(Path::new(".agents/skills/my-skill/SKILL.md")),
            SkillClient::Codex
        );
    }

    #[test]
    fn test_detect_client_codex_home_skills() {
        assert_eq!(
            detect_client(Path::new("/home/u/.codex/skills/ship/SKILL.md")),
            SkillClient::Codex
        );
        assert_eq!(
            detect_client(Path::new(
                "/home/u/.codex/skills/.system/skill-creator/SKILL.md"
            )),
            SkillClient::Codex
        );
    }

    #[test]
    fn test_detect_client_opencode_global_config() {
        assert_eq!(
            detect_client(Path::new("/home/u/.config/opencode/skills/review/SKILL.md")),
            SkillClient::OpenCode
        );
        // A plain `opencode/` directory is not OpenCode's config directory.
        assert_eq!(
            detect_client(Path::new("projects/opencode/skills/review/SKILL.md")),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_detect_client_opencode() {
        assert_eq!(
            detect_client(Path::new(".opencode/skills/my-skill/SKILL.md")),
            SkillClient::OpenCode
        );
    }

    #[test]
    fn test_detect_client_windsurf() {
        assert_eq!(
            detect_client(Path::new(".windsurf/skills/my-skill/SKILL.md")),
            SkillClient::Windsurf
        );
    }

    #[test]
    fn test_detect_client_kiro() {
        assert_eq!(
            detect_client(Path::new(".kiro/skills/my-skill/SKILL.md")),
            SkillClient::Kiro
        );
    }

    #[test]
    fn test_detect_client_roo_code() {
        assert_eq!(
            detect_client(Path::new(".roo/skills/my-skill/SKILL.md")),
            SkillClient::RooCode
        );
    }

    #[test]
    fn test_detect_client_unknown_root() {
        assert_eq!(detect_client(Path::new("SKILL.md")), SkillClient::Unknown);
    }

    #[test]
    fn test_detect_client_unknown_no_skills() {
        assert_eq!(
            detect_client(Path::new("some/path/SKILL.md")),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_detect_client_nested() {
        assert_eq!(
            detect_client(Path::new("projects/foo/.cursor/skills/review/SKILL.md")),
            SkillClient::Cursor
        );
    }

    #[test]
    fn test_resolve_skill_client_claude_plugin_layouts() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_root = temp.path().join("my-plugin");
        let manifest = plugin_root.join(".claude-plugin").join("plugin.json");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(&manifest, "{}").unwrap();

        assert_eq!(
            resolve_skill_client(
                &plugin_root.join("skills/review/SKILL.md"),
                &LintConfig::default()
            ),
            SkillClient::ClaudeCode
        );

        assert_eq!(
            resolve_skill_client(&plugin_root.join("SKILL.md"), &LintConfig::default()),
            SkillClient::Unknown,
            "a root skill must be declared with skills: \".\""
        );

        fs::write(&manifest, r#"{"skills":"."}"#).unwrap();
        assert_eq!(
            resolve_skill_client(&plugin_root.join("SKILL.md"), &LintConfig::default()),
            SkillClient::ClaudeCode
        );
        assert_eq!(
            resolve_skill_client(&plugin_root.join("docs/SKILL.md"), &LintConfig::default()),
            SkillClient::Unknown,
            "skills: \".\" must not claim nested SKILL.md files"
        );
    }

    #[test]
    fn test_resolve_skill_client_claude_plugin_custom_skill_paths() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_root = temp.path().join("my-plugin");
        let manifest = plugin_root.join(".claude-plugin").join("plugin.json");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(
            &manifest,
            r#"{"skills":["./custom-skills",".\\more-skills"]}"#,
        )
        .unwrap();

        for relative in [
            "custom-skills/review/SKILL.md",
            "more-skills/deploy/SKILL.md",
        ] {
            assert_eq!(
                resolve_skill_client(&plugin_root.join(relative), &LintConfig::default()),
                SkillClient::ClaudeCode
            );
        }
    }

    #[test]
    fn test_resolve_skill_client_rejects_escaping_plugin_skill_path() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_root = temp.path().join("my-plugin");
        let manifest = plugin_root.join(".claude-plugin").join("plugin.json");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(&manifest, r#"{"skills":"../external-skills"}"#).unwrap();

        assert_eq!(
            resolve_skill_client(
                &temp.path().join("external-skills/review/SKILL.md"),
                &LintConfig::default()
            ),
            SkillClient::Unknown
        );

        fs::write(&manifest, r#"{"skills":"C:\\external-skills"}"#).unwrap();
        assert_eq!(
            resolve_skill_client(
                &plugin_root.join("C:/external-skills/review/SKILL.md"),
                &LintConfig::default()
            ),
            SkillClient::Unknown
        );
    }

    #[test]
    fn test_resolve_skill_client_generic_skills_without_plugin_manifest() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(
            resolve_skill_client(
                &temp.path().join("skills/review/SKILL.md"),
                &LintConfig::default()
            ),
            SkillClient::Unknown
        );
    }

    // ===== Validation tests =====

    #[test]
    fn test_universal_fields_no_diagnostics() {
        let content = make_skill(
            "name: my-skill\ndescription: A test skill\nlicense: MIT",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        assert!(
            diags.is_empty(),
            "Universal-only fields should produce no diagnostics, got {:?}",
            diags
        );
    }

    #[test]
    fn test_claude_code_no_diagnostics() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus\ncontext: fork\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        // Claude Code supports all extension fields -- no per-client warnings
        let per_client: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.rule.starts_with("CR-SK-")
                    || d.rule.starts_with("CL-SK-")
                    || d.rule.starts_with("CP-SK-")
                    || d.rule.starts_with("CX-SK-")
                    || d.rule.starts_with("OC-SK-")
                    || d.rule.starts_with("WS-SK-")
                    || d.rule.starts_with("KR-SK-")
                    || d.rule.starts_with("AMP-SK-")
                    || d.rule.starts_with("RC-SK-")
                    || d.rule == "XP-SK-001"
            })
            .collect();
        assert!(
            per_client.is_empty(),
            "Claude Code should have no per-client or XP-SK-001 warnings, got {:?}",
            per_client
        );
    }

    #[test]
    fn test_claude_plugin_custom_skill_skips_cross_platform_warning() {
        let temp = tempfile::tempdir().unwrap();
        let plugin_root = temp.path().join("my-plugin");
        let manifest = plugin_root.join(".claude-plugin").join("plugin.json");
        let skill_path = plugin_root.join("custom-skills/review/SKILL.md");
        fs::create_dir_all(manifest.parent().unwrap()).unwrap();
        fs::write(&manifest, r#"{"skills":"./custom-skills"}"#).unwrap();

        let content = make_skill(
            "description: Review changes\ndisallowed-tools: Bash",
            "Body",
        );
        let diagnostics =
            PerClientSkillValidator.validate(&skill_path, &content, &LintConfig::default());

        assert!(
            !diagnostics.iter().any(|d| d.rule == "XP-SK-001"),
            "Claude-only fields in manifest-owned skills are not portability issues: {diagnostics:?}"
        );
    }

    #[test]
    fn test_cursor_unsupported_model() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert_eq!(
            cr_diags.len(),
            1,
            "Expected 1 CR-SK-001 for unsupported 'model', got {}",
            cr_diags.len()
        );
        assert!(cr_diags[0].message.contains("model"));
    }

    #[test]
    fn test_cursor_disable_model_invocation_supported() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\ndisable-model-invocation: true",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(
            cr_diags.is_empty(),
            "Cursor supports disable-model-invocation, should have no CR-SK-001, got {:?}",
            cr_diags
        );
    }

    #[test]
    fn test_cline_unsupported_context() {
        let content = make_skill("name: my-skill\ndescription: A test\ncontext: fork", "Body");
        let diags = validate(".cline/skills/my-skill/SKILL.md", &content);
        let cl_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CL-SK-001").collect();
        assert_eq!(cl_diags.len(), 1);
        assert!(cl_diags[0].message.contains("context"));
    }

    #[test]
    fn test_copilot_unsupported_agent() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".github/skills/my-skill/SKILL.md", &content);
        let cp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CP-SK-001").collect();
        assert_eq!(cp_diags.len(), 1);
        assert!(cp_diags[0].message.contains("agent"));
    }

    #[test]
    fn test_codex_unsupported_hooks() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nhooks: some-value",
            "Body",
        );
        let diags = validate(".agents/skills/my-skill/SKILL.md", &content);
        let cx_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CX-SK-001").collect();
        assert_eq!(cx_diags.len(), 1);
        assert!(cx_diags[0].message.contains("hooks"));
    }

    #[test]
    fn test_opencode_unsupported_argument_hint() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nargument-hint: hint",
            "Body",
        );
        let diags = validate(".opencode/skills/my-skill/SKILL.md", &content);
        let oc_diags: Vec<_> = diags.iter().filter(|d| d.rule == "OC-SK-001").collect();
        assert_eq!(oc_diags.len(), 1);
        assert!(oc_diags[0].message.contains("argument-hint"));
    }

    #[test]
    fn test_windsurf_unsupported_user_invocable() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nuser-invocable: true",
            "Body",
        );
        let diags = validate(".windsurf/skills/my-skill/SKILL.md", &content);
        let ws_diags: Vec<_> = diags.iter().filter(|d| d.rule == "WS-SK-001").collect();
        assert_eq!(ws_diags.len(), 1);
        assert!(ws_diags[0].message.contains("user-invocable"));
    }

    #[test]
    fn test_kiro_unsupported_model() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: haiku", "Body");
        let diags = validate(".kiro/skills/my-skill/SKILL.md", &content);
        let kr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "KR-SK-001").collect();
        assert_eq!(kr_diags.len(), 1);
    }

    // ===== Kiro (KR-SK-001) =====

    const KIRO_PATH: &str = ".kiro/skills/my-skill/SKILL.md";

    fn kiro_diags(content: &str) -> Vec<Diagnostic> {
        validate(KIRO_PATH, content)
    }

    fn apply_fixes(content: &str, diagnostics: &[Diagnostic], safe_only: bool) -> String {
        use crate::fs::{FileSystem, MockFileSystem};
        let fs = std::sync::Arc::new(MockFileSystem::new());
        fs.add_file(KIRO_PATH, content);
        crate::fixes::apply_fixes_with_fs(diagnostics, false, safe_only, Some(fs.clone())).unwrap();
        fs.read_to_string(Path::new(KIRO_PATH)).unwrap()
    }

    fn frontmatter_of(content: &str) -> serde_yaml::Value {
        let parts = split_frontmatter(content);
        assert!(parts.has_frontmatter && parts.has_closing, "{content}");
        serde_yaml::from_str(&parts.frontmatter)
            .unwrap_or_else(|e| panic!("frontmatter must parse after fixes: {e}\n{content}"))
    }

    /// Apply fixes run after run (as repeated `agnix --fix` would) and check
    /// that every intermediate file parses and the body never changes.
    fn fix_until_stable(content: &str, safe_only: bool) -> String {
        let body = split_frontmatter(content).body;
        let mut current = content.to_string();
        for _ in 0..10 {
            let diagnostics = kiro_diags(&current);
            let next = apply_fixes(&current, &diagnostics, safe_only);
            frontmatter_of(&next);
            assert_eq!(split_frontmatter(&next).body, body);
            if next == current {
                return current;
            }
            current = next;
        }
        panic!("fixes did not converge:\n{current}");
    }

    #[test]
    fn test_kiro_aim_skill_fixes_keep_yaml_valid() {
        // The corpus shape that the old line-delete fix broke: a column-0
        // block sequence followed by a metadata block.
        let content = "---\nname: brazil\ndescription: Build system\nversion: 1.0.0\ntags:\n- skill\n- brazil\nmetadata:\n  brazil_package_interface: '1.0'\n  commit: abc\n---\n# Brazil\n";
        let diagnostics = kiro_diags(content);
        let kr: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SK-001")
            .collect();
        assert_eq!(kr.len(), 2, "{diagnostics:?}");
        assert!(
            diagnostics.iter().all(|d| d.rule != "XP-SK-001"),
            "XP-SK-001 duplicates KR-SK-001 on Kiro skills"
        );
        let version = kr.iter().find(|d| d.message.contains("'version'")).unwrap();
        assert_eq!((version.line, version.column), (4, 1));
        assert_eq!(version.level, crate::diagnostics::DiagnosticLevel::Info);
        assert!(
            version.fixes[0].safe,
            "moving plain data under metadata is safe"
        );
        let tags = kr.iter().find(|d| d.message.contains("'tags'")).unwrap();
        assert!(!tags.fixes[0].safe, "deleting a list is unsafe");

        // Safe mode: version moves under metadata, tags stay, YAML parses.
        let safe = apply_fixes(content, &diagnostics, true);
        let value = frontmatter_of(&safe);
        assert_eq!(value["metadata"]["version"], "1.0.0");
        assert_eq!(value["metadata"]["commit"], "abc");
        assert!(value.get("version").is_none());
        assert_eq!(value["tags"][1], "brazil");

        // Unsafe runs converge to a clean, parseable skill.
        let fixed = fix_until_stable(content, false);
        let value = frontmatter_of(&fixed);
        assert_eq!(value["metadata"]["version"], "1.0.0");
        assert!(value.get("tags").is_none());
        assert!(kiro_diags(&fixed).iter().all(|d| d.rule != "KR-SK-001"));
    }

    #[test]
    fn test_kiro_fix_creates_metadata_once_per_run() {
        let content = "---\nname: a\ndescription: b\nversion: 2.0.0\nstatus: verified\ntriggers:\n  - x\n  - y\n---\nBody\n";
        let diagnostics = kiro_diags(content);
        assert_eq!(
            diagnostics.iter().filter(|d| d.rule == "KR-SK-001").count(),
            3
        );
        let once = apply_fixes(content, &diagnostics, true);
        let value = frontmatter_of(&once);
        // Two creating fixes share a group: exactly one metadata block appears.
        assert_eq!(value["metadata"].as_mapping().unwrap().len(), 1);
        let fixed = fix_until_stable(content, true);
        let value = frontmatter_of(&fixed);
        assert_eq!(value["metadata"]["version"], "2.0.0");
        assert_eq!(value["metadata"]["status"], "verified");
        assert_eq!(
            value["triggers"][0], "x",
            "safe mode never deletes the list"
        );
    }

    #[test]
    fn test_kiro_fix_quotes_numbers_and_handles_fields_after_metadata() {
        let content = "---\nname: a\ndescription: b\nmetadata:\n    author: me\nversion: 1.0\nenabled: true # flag\n---\nBody\n";
        let fixed = fix_until_stable(content, true);
        let value = frontmatter_of(&fixed);
        assert_eq!(value["metadata"]["version"], "1.0");
        assert_eq!(value["metadata"]["enabled"], "true");
        assert_eq!(value["metadata"]["author"], "me");
        assert!(fixed.contains("    version: \"1.0\""), "{fixed}");
    }

    #[test]
    fn test_kiro_fix_falls_back_to_unsafe_delete() {
        for content in [
            // flow-style metadata cannot take a new line
            "---\nname: a\ndescription: b\nmetadata: {author: me}\nversion: 1.0.0\n---\nBody\n",
            // metadata already has the key
            "---\nname: a\ndescription: b\nversion: 1.0.0\nmetadata:\n  version: \"0.9\"\n---\nBody\n",
            // multi-line folded value
            "---\nname: a\ndescription: b\nversion: >\n  1.0.0\n---\nBody\n",
        ] {
            let diagnostics = kiro_diags(content);
            let kr: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "KR-SK-001")
                .collect();
            assert_eq!(kr.len(), 1, "{content}");
            assert_eq!(kr[0].fixes.len(), 1, "{content}");
            assert!(!kr[0].fixes[0].safe, "{content}");
            assert_eq!(apply_fixes(content, &diagnostics, true), content);
            let fixed = apply_fixes(content, &diagnostics, false);
            let value = frontmatter_of(&fixed);
            assert!(value.get("version").is_none(), "{fixed}");
            assert_eq!(value["name"], "a");
        }
    }

    #[test]
    fn test_kiro_last_entry_delete_keeps_closing_delimiter() {
        let content = "---\nname: a\ndescription: b\ntags:\n- x\n- y\n---\nBody\n";
        let diagnostics = kiro_diags(content);
        let fixed = apply_fixes(content, &diagnostics, false);
        assert_eq!(fixed, "---\nname: a\ndescription: b\n---\nBody\n");
    }

    #[test]
    fn test_kiro_other_client_fields_warn_and_move_unsafely() {
        let content = "---\nname: a\ndescription: b\nallowed-tools: Read, Grep\nargument-hint: \"[path]\"\n---\nBody\n";
        let diagnostics = kiro_diags(content);
        let kr: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "KR-SK-001")
            .collect();
        assert_eq!(
            kr.len(),
            2,
            "allowed-tools is not a Kiro field: {diagnostics:?}"
        );
        for d in &kr {
            assert_eq!(d.level, crate::diagnostics::DiagnosticLevel::Warning);
            assert!(!d.fixes[0].safe, "moving another client's field is unsafe");
        }
        assert!(
            kr[0].message.contains("restrict tools"),
            "{}",
            kr[0].message
        );
        assert_eq!(apply_fixes(content, &diagnostics, true), content);
        let fixed = fix_until_stable(content, false);
        let value = frontmatter_of(&fixed);
        assert_eq!(value["metadata"]["allowed-tools"], "Read, Grep");
        assert_eq!(value["metadata"]["argument-hint"], "[path]");
    }

    #[test]
    fn test_kiro_supported_and_nested_fields_not_flagged() {
        let content = "---\nname: a\ndescription: b\nlicense: MIT\ncompatibility: node\nmetadata:\n  model: x\n  version: \"1\"\n---\nBody\n";
        assert!(kiro_diags(content).is_empty());
    }

    #[test]
    fn test_kiro_unparseable_frontmatter_left_to_as_016() {
        let content = "---\nname: a\ndescription: b\nversion: [1\n---\nBody\n";
        assert!(kiro_diags(content).iter().all(|d| d.rule != "KR-SK-001"));
    }

    #[test]
    fn test_kiro_xp_sk_001_kept_when_kr_sk_001_disabled() {
        let content = make_skill("name: a\ndescription: b\nversion: 1.0.0", "Body");
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KR-SK-001".to_string()];
        let diagnostics = PerClientSkillValidator.validate(Path::new(KIRO_PATH), &content, &config);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "XP-SK-001");
    }

    #[test]
    fn test_kiro_messages_render() {
        let content =
            "---\nname: a\ndescription: b\nallowed-tools: Read\nmodel: x\nversion: 1\n---\nBody\n";
        let diagnostics = kiro_diags(content);
        assert_eq!(diagnostics.len(), 3);
        for d in &diagnostics {
            assert!(
                !d.message.contains("%{") && !d.message.starts_with("rules."),
                "{}",
                d.message
            );
            let suggestion = d.suggestion.as_deref().unwrap();
            assert!(!suggestion.contains("%{"), "{suggestion}");
        }
    }

    #[test]
    fn test_kiro_fixture_metadata_move() {
        let content = include_str!(
            "../../../../tests/fixtures/per_client_skills/.kiro/skills/metadata-move/SKILL.md"
        );
        let diagnostics = kiro_diags(content);
        let rules: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.line))
            .collect();
        assert_eq!(
            rules,
            [("KR-SK-001", 4), ("KR-SK-001", 5), ("KR-SK-001", 8)]
        );
        let safe = fix_until_stable(content, true);
        let value = frontmatter_of(&safe);
        assert_eq!(value["metadata"]["version"], "1.0.0");
        assert_eq!(value["argument-hint"], "[path] [--fix]");
        assert_eq!(value["tags"][0], "skill");
        let all = fix_until_stable(content, false);
        let value = frontmatter_of(&all);
        assert_eq!(value["metadata"]["argument-hint"], "[path] [--fix]");
        assert_eq!(value["metadata"]["author"], "agnix");
        assert!(value.get("tags").is_none());
    }

    #[test]
    fn test_roo_code_unsupported_disable_model() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\ndisable-model-invocation: false",
            "Body",
        );
        let diags = validate(".roo/skills/my-skill/SKILL.md", &content);
        let rc_diags: Vec<_> = diags.iter().filter(|d| d.rule == "RC-SK-001").collect();
        assert_eq!(rc_diags.len(), 1);
    }

    #[test]
    fn test_multiple_unsupported_fields() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus\ncontext: fork\nagent: general-purpose",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        // model, context, agent are all unsupported by Cursor
        assert_eq!(
            cr_diags.len(),
            3,
            "Expected 3 CR-SK-001 for model/context/agent, got {}",
            cr_diags.len()
        );
    }

    #[test]
    fn test_xp_sk_001_fires_for_non_claude() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "Expected 1 XP-SK-001 for extension field in non-Claude skill"
        );
    }

    #[test]
    fn test_xp_sk_001_does_not_fire_for_claude() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert!(
            xp_diags.is_empty(),
            "XP-SK-001 should not fire for Claude Code"
        );
    }

    #[test]
    fn test_xp_sk_001_fires_for_unknown() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate("SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "XP-SK-001 should fire for root-level skills with non-universal fields"
        );
    }

    #[test]
    fn test_indented_keys_not_flagged() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmetadata:\n  model: some-value\n  context: note",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(
            cr_diags.is_empty(),
            "Indented keys under metadata should not trigger per-client warnings, got {:?}",
            cr_diags
        );
    }

    #[test]
    fn test_no_frontmatter_no_diagnostics() {
        let content = "Just body content without frontmatter";
        let diags = validate(".cursor/skills/my-skill/SKILL.md", content);
        assert!(diags.is_empty());
    }

    #[test]
    fn test_fix_attached_for_unsupported_field() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diag = diags.iter().find(|d| d.rule == "CR-SK-001").unwrap();
        assert!(
            cr_diag.has_fixes(),
            "CR-SK-001 should have an auto-fix attached"
        );
        let fix = &cr_diag.fixes[0];
        assert!(fix.is_deletion(), "Fix should be a line deletion");
        assert!(fix.safe, "Fix should be safe");
    }

    #[test]
    fn test_disabled_rule_not_fired() {
        let content = make_skill("name: my-skill\ndescription: A test\nmodel: opus", "Body");
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["CR-SK-001".to_string()];
        let validator = PerClientSkillValidator;
        let diags = validator.validate(
            Path::new(".cursor/skills/my-skill/SKILL.md"),
            &content,
            &config,
        );
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert!(cr_diags.is_empty(), "Disabled rule should not fire");
    }

    #[test]
    fn test_line_numbers_correct() {
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmodel: opus",
            "Body content",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diag = diags.iter().find(|d| d.rule == "CR-SK-001").unwrap();
        // Line 1: ---
        // Line 2: (empty from frontmatter start)
        // Actually frontmatter content begins at byte 3 (after "---")
        // The frontmatter is "\nname: my-skill\ndescription: A test\nmodel: opus"
        // "model: opus" is on line 4 of the full content
        assert_eq!(cr_diag.line, 4, "model should be on line 4");
    }

    #[test]
    fn test_unknown_custom_field_flagged() {
        // A completely unknown field (not in UNIVERSAL_FIELDS or EXTENSION_FIELDS)
        // should be flagged for non-Claude clients
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let cr_diags: Vec<_> = diags.iter().filter(|d| d.rule == "CR-SK-001").collect();
        assert_eq!(
            cr_diags.len(),
            1,
            "Unknown custom field should trigger CR-SK-001"
        );
        assert!(cr_diags[0].message.contains("my-custom-field"));
    }

    #[test]
    fn test_unknown_custom_field_xp_fires() {
        // XP-SK-001 should fire for any non-universal field, including unknown ones
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate("SKILL.md", &content);
        let xp_diags: Vec<_> = diags.iter().filter(|d| d.rule == "XP-SK-001").collect();
        assert_eq!(
            xp_diags.len(),
            1,
            "XP-SK-001 should fire for unknown custom field"
        );
    }

    #[test]
    fn test_multiline_value_fix_marked_unsafe() {
        // A field with multi-line YAML value (next line is indented) should
        // have its auto-fix marked as unsafe since deleting only the key
        // line would leave orphaned indented content.
        let content = make_skill(
            "name: my-skill\ndescription: A test\nallowed-tools:\n  - Read\n  - Write\nmodel: opus",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        // 'model' is a single-line field -> safe fix
        let model_diag = diags
            .iter()
            .find(|d| d.rule == "CR-SK-001" && d.message.contains("model"))
            .expect("Expected CR-SK-001 for model");
        assert!(
            model_diag.fixes[0].safe,
            "Single-line field fix should be safe"
        );
    }

    #[test]
    fn test_multiline_value_with_list_fix_unsafe() {
        // 'context' key followed by indented lines -> unsafe fix
        let content = make_skill(
            "name: my-skill\ndescription: A test\ncontext:\n  - file1.rs\n  - file2.rs",
            "Body",
        );
        let diags = validate(".cursor/skills/my-skill/SKILL.md", &content);
        let ctx_diag = diags
            .iter()
            .find(|d| d.rule == "CR-SK-001" && d.message.contains("context"))
            .expect("Expected CR-SK-001 for context");
        assert!(
            !ctx_diag.fixes[0].safe,
            "Multi-line field fix should be unsafe"
        );
    }

    #[test]
    fn test_unknown_custom_field_not_flagged_for_claude() {
        // Claude Code accepts all fields, including unknown ones
        let content = make_skill(
            "name: my-skill\ndescription: A test\nmy-custom-field: value",
            "Body",
        );
        let diags = validate(".claude/skills/my-skill/SKILL.md", &content);
        assert!(
            diags.is_empty(),
            "Claude Code should not flag any fields, got {:?}",
            diags
        );
    }
}
