use crate::config::LintConfig;
use crate::fs::FileSystem;
use crate::parsers::frontmatter::{
    FrontmatterParts, check_yaml_depth, check_yaml_duplicate_top_level_keys,
};
use crate::pipeline::{
    compile_single_exclude_pattern, is_excluded_file, normalize_rel_path, should_prune_dir,
};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use super::{PathMatch, SkillFrontmatter, reference_path_regex};

pub(super) fn parse_frontmatter_fields(frontmatter: &str) -> Result<SkillFrontmatter, String> {
    if frontmatter.trim().is_empty() {
        return Ok(SkillFrontmatter::default());
    }
    check_yaml_depth(frontmatter).map_err(|e| e.to_string())?;
    check_yaml_duplicate_top_level_keys(frontmatter).map_err(|e| e.to_string())?;
    serde_yaml::from_str(frontmatter).map_err(|e| e.to_string())
}

pub(super) fn extract_reference_paths(body: &str) -> Vec<PathMatch> {
    let re = reference_path_regex();
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for m in re.find_iter(body) {
        #[allow(clippy::collapsible_if)]
        if let Some((trimmed, delta)) = trim_path_token_with_offset(m.as_str()) {
            if seen.insert(trimmed.clone()) {
                paths.push(PathMatch {
                    path: trimmed,
                    start: m.start() + delta,
                });
            }
        }
    }
    paths
}

pub(super) fn reference_path_too_deep(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut parts = normalized.split('/').filter(|part| !part.is_empty());
    let Some(prefix) = parts.next() else {
        return false;
    };

    // Check for bundled resource references. The spec scopes skill resources to
    // "`scripts/`, `references/`, or `assets/`" and states the depth rule over
    // file references generally - "Keep file references one level deep from
    // `SKILL.md`" - with `scripts/extract.py` used in its own example. Only the
    // `references*` spellings were inspected, so deep `scripts/` and `assets/`
    // paths were invisible to AS-013.
    if !prefix.eq_ignore_ascii_case("references")
        && !prefix.eq_ignore_ascii_case("reference")
        && !prefix.eq_ignore_ascii_case("refs")
        && !prefix.eq_ignore_ascii_case("scripts")
        && !prefix.eq_ignore_ascii_case("assets")
    {
        return false;
    }

    // Exclude git refs - they're not file references
    // Git refs look like: refs/remotes/..., refs/heads/..., refs/tags/...
    if prefix.eq_ignore_ascii_case("refs") {
        if let Some(second) = parts.next() {
            if second.eq_ignore_ascii_case("remotes")
                || second.eq_ignore_ascii_case("heads")
                || second.eq_ignore_ascii_case("tags")
                || second.eq_ignore_ascii_case("stash")
            {
                return false; // This is a git ref, not a file reference
            }
        }
        // Reset iterator for depth check
        let parts = normalized.split('/').filter(|part| !part.is_empty());
        return parts.skip(1).count() > 1;
    }

    parts.count() > 1
}

pub(super) fn trim_path_token(token: &str) -> &str {
    token
        .trim_start_matches(['(', '[', '{', '<', '"', '\''])
        .trim_end_matches(['.', ',', ';', ':', ')', ']', '}', '>', '"', '\''])
}

pub(super) fn trim_path_token_with_offset(token: &str) -> Option<(String, usize)> {
    let trimmed = trim_path_token(token);
    if trimmed.is_empty() {
        return None;
    }
    let offset = token.find(trimmed).unwrap_or(0);
    Some((trimmed.to_string(), offset))
}

pub(super) fn compute_line_starts(content: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in content.char_indices() {
        if ch == '\n' {
            starts.push(idx + 1);
        }
    }
    starts
}

pub(super) fn line_col_at(offset: usize, line_starts: &[usize]) -> (usize, usize) {
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

pub(super) fn frontmatter_key_line_col(
    parts: &FrontmatterParts,
    key: &str,
    line_starts: &[usize],
) -> (usize, usize) {
    let offset = frontmatter_key_offset(&parts.frontmatter, key)
        .map(|local| parts.frontmatter_start + local)
        .unwrap_or(parts.frontmatter_start);
    line_col_at(offset, line_starts)
}

pub(super) fn frontmatter_key_offset(frontmatter: &str, key: &str) -> Option<usize> {
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            // Check for CRLF or LF
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix(key) {
            if rest.trim_start().starts_with(':') {
                let column = line.len() - trimmed.len();
                return Some(offset + column);
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

/// Find the byte range of a YAML value for a given key in frontmatter.
/// Returns (start, end) byte offsets relative to the full content.
/// Handles both quoted and unquoted values.
pub(super) fn frontmatter_value_byte_range(
    _content: &str,
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    let frontmatter = &parts.frontmatter;
    let mut offset = 0usize;
    let bytes = frontmatter.as_bytes();

    for line in frontmatter.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') || trimmed.is_empty() {
            // Calculate actual byte length including newline characters
            let line_end = offset + line.len();
            if line_end < bytes.len() {
                if bytes[line_end] == b'\n' {
                    offset = line_end + 1; // LF
                } else if line_end + 1 < bytes.len()
                    && bytes[line_end] == b'\r'
                    && bytes[line_end + 1] == b'\n'
                {
                    offset = line_end + 2; // CRLF
                } else {
                    offset = line_end; // No newline (last line)
                }
            } else {
                offset = line_end; // End of string
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix(key) {
            if let Some(after_colon) = rest.trim_start().strip_prefix(':') {
                // Found the key, now find the value
                let leading_ws = line.len() - trimmed.len();
                let ws_after_key = rest.len() - rest.trim_start().len();
                let key_end = leading_ws + key.len() + ws_after_key + 1; // +1 for ':'

                let value_str = after_colon.trim_start();
                if value_str.is_empty() {
                    // No value on this line (might be multiline YAML)
                    return None;
                }

                // Calculate value start position in line
                let value_offset_in_line = key_end + (after_colon.len() - value_str.len());

                // Handle quoted values
                let (value_start, value_len) = if let Some(inner) = value_str.strip_prefix('"') {
                    // Double-quoted: find closing quote
                    let end_quote = inner.find('"')?;
                    (value_offset_in_line + 1, end_quote) // Skip opening quote
                } else if let Some(inner) = value_str.strip_prefix('\'') {
                    // Single-quoted: find closing quote
                    let end_quote = inner.find('\'')?;
                    (value_offset_in_line + 1, end_quote) // Skip opening quote
                } else {
                    // Unquoted value: take until end of line or comment
                    // Check for both " #" (space-hash) and "\t#" (tab-hash)
                    let value_end = value_str
                        .find(" #")
                        .or_else(|| value_str.find("\t#"))
                        .unwrap_or(value_str.len());
                    (value_offset_in_line, value_end)
                };

                let abs_start = parts.frontmatter_start + offset + value_start;
                let abs_end = abs_start + value_len;

                return Some((abs_start, abs_end));
            }
        }
        // Calculate actual byte length including newline characters
        let line_end = offset + line.len();
        if line_end < bytes.len() {
            if bytes[line_end] == b'\n' {
                offset = line_end + 1; // LF
            } else if line_end + 1 < bytes.len()
                && bytes[line_end] == b'\r'
                && bytes[line_end + 1] == b'\n'
            {
                offset = line_end + 2; // CRLF
            } else {
                offset = line_end; // No newline (last line)
            }
        } else {
            offset = line_end; // End of string
        }
    }
    None
}

/// Find the full line byte range for a frontmatter key.
/// Returns (line_start, line_end_exclusive) in full content byte offsets.
/// Includes the trailing '\n' when present.
pub(super) fn frontmatter_key_line_byte_range(
    content: &str,
    parts: &FrontmatterParts,
    key: &str,
) -> Option<(usize, usize)> {
    let local_key_start = frontmatter_key_offset(&parts.frontmatter, key)?;
    let abs_start = parts.frontmatter_start + local_key_start;
    if abs_start >= content.len() {
        return None;
    }

    let bytes = content.as_bytes();
    let mut end = abs_start;
    while end < content.len() && bytes[end] != b'\n' {
        end += 1;
    }
    if end < content.len() {
        end += 1;
    }

    Some((abs_start, end))
}

/// Name segments treated as placeholders rather than a real username
/// (case-insensitive). Matches with one of these names are not flagged.
pub(super) const USER_PATH_PLACEHOLDERS: &[&str] = &[
    "user",
    "username",
    "name",
    "you",
    "your-name",
    "yourname",
    "me",
    "myname",
    "someone",
    "example",
    "johndoe",
    "jdoe",
    "foo",
    "bar",
];

/// File extensions treated as bundled scripts (scanned in full, shebang included).
pub(super) const SCRIPT_EXTENSIONS: &[&str] = &[
    "sh", "bash", "zsh", "fish", "py", "rb", "pl", "lua", "js", "ts", "mjs",
];

/// `.md` filenames that are repo docs, not skill instructions - left out of scope.
pub(super) const SKIP_MD_FILENAMES: &[&str] = &[
    "readme.md",
    "changelog.md",
    "claude.md",
    "agents.md",
    "license.md",
];

/// A hardcoded user-home path found while scanning skill content.
pub(super) struct UserPathHit {
    /// Byte offset of the match start within the scanned region.
    pub offset: usize,
    /// The matched path prefix, e.g. `/Users/alice/`.
    pub text: String,
}

fn posix_user_path_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // No trailing slash required: the name class excludes `/`, so the match
    // ends at the username whether or not a path component follows. This also
    // catches a bare `/home/alice` at end of line.
    RE.get_or_init(|| {
        Regex::new(r"(?:/Users/|/home/)([A-Za-z0-9._-]+)")
            .expect("posix user path pattern must compile")
    })
}

fn windows_user_path_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"[Cc]:[\\/]Users[\\/]([A-Za-z0-9._-]+)")
            .expect("windows user path pattern must compile")
    })
}

fn is_placeholder_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    USER_PATH_PLACEHOLDERS.contains(&lower.as_str())
}

/// Find hardcoded user-home paths (`/Users/<name>/`, `/home/<name>/`,
/// `C:\Users\<name>\`) in `content`, skipping placeholder names. Angle-bracket
/// (`<name>`), template (`${...}`, `{{...}}`), and env-var (`$HOME`) forms never
/// match the name character class, so they are skipped implicitly.
pub(super) fn find_hardcoded_user_paths(content: &str) -> Vec<UserPathHit> {
    let mut hits = Vec::new();
    for re in [posix_user_path_regex(), windows_user_path_regex()] {
        for caps in re.captures_iter(content) {
            let whole = caps.get(0).expect("group 0 always present");
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or_default();
            if is_placeholder_name(name) {
                continue;
            }
            hits.push(UserPathHit {
                offset: whole.start(),
                text: whole.as_str().to_string(),
            });
        }
    }
    hits.sort_by_key(|h| h.offset);
    hits
}

/// Whether a bundled file is a script: a `#!` shebang on the first line (any
/// extension), or a recognized script extension.
pub(super) fn is_script_path(path: &Path, content: &str) -> bool {
    if content.starts_with("#!") {
        return true;
    }
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some(ext) if SCRIPT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str())
    )
}

pub(super) fn directory_size_until(
    path: &Path,
    max_bytes: u64,
    fs: &dyn FileSystem,
    config: &LintConfig,
) -> u64 {
    let exclude_patterns = config
        .exclude()
        .iter()
        .chain(config.files_config().exclude.iter())
        .filter_map(|pattern| compile_single_exclude_pattern(pattern).ok())
        .collect::<Vec<_>>();
    let root_dir = if exclude_patterns.is_empty() {
        None
    } else {
        config.root_dir().map(|root| root.as_path())
    };

    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = match fs.read_dir(&current) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        for entry in entries {
            if entry.metadata.is_symlink {
                continue;
            }
            let relative_path = root_dir.map(|root| normalize_rel_path(&entry.path, root));
            if entry.metadata.is_dir {
                if relative_path
                    .as_deref()
                    .is_some_and(|path| should_prune_dir(path, &exclude_patterns))
                {
                    continue;
                }
                stack.push(entry.path.clone());
                continue;
            }
            if entry.metadata.is_file {
                if relative_path
                    .as_deref()
                    .is_some_and(|path| is_excluded_file(path, &exclude_patterns))
                {
                    continue;
                }
                total = total.saturating_add(entry.metadata.len);
                if total > max_bytes {
                    return total;
                }
            }
        }
    }
    total
}
