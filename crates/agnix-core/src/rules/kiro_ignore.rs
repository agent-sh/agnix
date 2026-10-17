//! Kiro ignore files (KR-IGN-001).
//!
//! `.kiroignore` (workspace root or any subdirectory) and the global
//! `~/.kiro/settings/kiroignore` use gitignore syntax; Kiro V3 evaluates them
//! with gitignore semantics. As in git, "You cannot re-include a file if a
//! parent directory is excluded. For example, if you ignore `secrets/`,
//! adding `!secrets/public.txt` won't work." (https://kiro.dev/docs/kiroignore/).
//! KR-IGN-001 reports such negations.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    rules::{Validator, ValidatorMetadata},
};
use regex::Regex;
use rust_i18n::t;
use std::path::Path;

const RULE_IDS: &[&str] = &["KR-IGN-001"];

pub struct KiroIgnoreValidator;

impl Validator for KiroIgnoreValidator {
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
        if !config.is_rule_enabled("KR-IGN-001") {
            return Vec::new();
        }
        let patterns = parse_patterns(content);
        let mut diagnostics = Vec::new();
        for pattern in &patterns {
            if !pattern.negated {
                continue;
            }
            let Some(ancestors) = literal_ancestors(&pattern.glob) else {
                continue;
            };
            // Check the shortest ancestor first: once a directory is excluded,
            // nothing below it can be re-included.
            let blocker = ancestors.iter().find_map(|dir| {
                excluding_pattern(&patterns, dir).map(|blocker| (dir.clone(), blocker))
            });
            if let Some((dir, blocker)) = blocker {
                let excluded_by = &patterns[blocker];
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        pattern.line,
                        pattern.column,
                        "KR-IGN-001",
                        t!(
                            "rules.kr_ign_001.message",
                            pattern = pattern.raw.as_str(),
                            dir = dir.as_str(),
                            excluded_by = excluded_by.raw.as_str(),
                            line = excluded_by.line
                        ),
                    )
                    .with_suggestion(t!(
                        "rules.kr_ign_001.suggestion",
                        excluded_by = excluded_by.raw.as_str(),
                        dir = dir.as_str()
                    )),
                );
            }
        }
        diagnostics
    }
}

/// One gitignore pattern line.
struct Pattern {
    /// The line as written (trimmed), for messages.
    raw: String,
    line: usize,
    column: usize,
    negated: bool,
    /// The pattern without `!`, the leading `/` and the trailing `/`.
    glob: String,
    /// Matches a `/`-separated path relative to the ignore file. A trailing
    /// `/` (directories only) needs no handling: only directories are tested.
    regex: Option<Regex>,
}

/// Trailing spaces are ignored unless escaped with a backslash.
fn trim_trailing_spaces(line: &str) -> &str {
    let mut end = line.len();
    while end > 0 && line.as_bytes()[end - 1] == b' ' {
        if end >= 2 && line.as_bytes()[end - 2] == b'\\' {
            break;
        }
        end -= 1;
    }
    &line[..end]
}

fn parse_patterns(content: &str) -> Vec<Pattern> {
    let mut patterns = Vec::new();
    for (index, raw_line) in content.lines().enumerate() {
        let line = trim_trailing_spaces(raw_line.trim_end_matches('\r'));
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let column = raw_line.len() - raw_line.trim_start().len() + 1;
        let mut text = line;
        let negated = text.starts_with('!');
        // Drop the `!` of a negation, or the backslash of an escaped leading
        // `!` or `#`.
        if negated || text.starts_with("\\!") || text.starts_with("\\#") {
            text = &text[1..];
        }
        let text = text.trim_end_matches('/');
        // A `/` other than a trailing one anchors the pattern to the
        // directory of the ignore file.
        let anchored = text.contains('/');
        let glob = text.trim_start_matches('/').to_string();
        if glob.is_empty() {
            continue;
        }
        let regex = glob_regex(&glob, anchored);
        patterns.push(Pattern {
            raw: line.trim().to_string(),
            line: index + 1,
            column,
            negated,
            glob,
            regex,
        });
    }
    patterns
}

/// gitignore glob to a regex over a `/`-separated path relative to the
/// ignore file's directory. An unanchored pattern matches at any depth.
fn glob_regex(glob: &str, anchored: bool) -> Option<Regex> {
    let mut out = String::from("^");
    if !anchored {
        out.push_str("(?:.*/)?");
    }
    let bytes = glob.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        match c {
            '*' if glob[i..].starts_with("**") => {
                let at_start = i == 0 || bytes[i - 1] == b'/';
                let at_end = i + 2 == bytes.len() || bytes[i + 2] == b'/';
                if at_start && at_end {
                    if i + 2 == bytes.len() {
                        // `dir/**`: everything inside.
                        out.push_str(".*");
                    } else {
                        // `**/` or `a/**/b`: zero or more directories.
                        out.push_str("(?:.*/)?");
                        i += 1; // skip the `/` after `**`
                    }
                } else {
                    out.push_str("[^/]*");
                }
                i += 2;
                continue;
            }
            '*' => out.push_str("[^/]*"),
            '?' => out.push_str("[^/]"),
            '[' => {
                let close = glob[i + 1..].find(']').map(|j| i + 1 + j);
                match close {
                    Some(close) if close > i + 1 => {
                        let class = &glob[i + 1..close];
                        out.push('[');
                        let class = class
                            .strip_prefix('!')
                            .map_or_else(|| class.to_string(), |rest| format!("^{rest}"));
                        out.push_str(&class.replace('\\', "\\\\"));
                        out.push(']');
                        i = close + 1;
                        continue;
                    }
                    _ => out.push_str("\\["),
                }
            }
            '\\' if i + 1 < bytes.len() => {
                i += 1;
                out.push_str(&regex::escape(&glob[i..i + 1]));
            }
            _ => out.push_str(&regex::escape(&c.to_string())),
        }
        i += 1;
    }
    out.push('$');
    Regex::new(&out).ok()
}

/// The directories a negation names literally: for `!secrets/keys/a.txt`,
/// `secrets` and `secrets/keys`. Only anchored patterns name a directory;
/// the chain stops at the first segment with a wildcard.
fn literal_ancestors(glob: &str) -> Option<Vec<String>> {
    let segments: Vec<&str> = glob.split('/').collect();
    if segments.len() < 2 {
        return None;
    }
    let mut ancestors = Vec::new();
    for (index, segment) in segments[..segments.len() - 1].iter().enumerate() {
        if segment.is_empty() || segment.contains(['*', '?', '[', '\\']) {
            break;
        }
        ancestors.push(segments[..=index].join("/"));
    }
    (!ancestors.is_empty()).then_some(ancestors)
}

/// Index of the pattern that leaves directory `dir` excluded (the last
/// matching pattern, if it is not a negation), or `None`.
fn excluding_pattern(patterns: &[Pattern], dir: &str) -> Option<usize> {
    let mut last = None;
    for (index, pattern) in patterns.iter().enumerate() {
        if pattern.regex.as_ref().is_some_and(|re| re.is_match(dir)) {
            last = Some(index);
        }
    }
    let index = last?;
    (!patterns[index].negated).then_some(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let config = LintConfig::default();
        let path = Path::new(".kiroignore");
        KiroIgnoreValidator.validate_per_file(path, content, &config.for_path(path))
    }

    #[test]
    fn documented_example_is_reported() {
        // kiro.dev/docs/kiroignore "Pattern syntax".
        let diagnostics = validate("secrets/\n!secrets/public.txt\n");
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.rule, "KR-IGN-001");
        assert_eq!((diagnostic.line, diagnostic.column), (2, 1));
        assert!(diagnostic.message.contains("!secrets/public.txt"));
        assert!(diagnostic.message.contains("secrets/"));
    }

    #[test]
    fn documented_examples_that_work_are_not_reported() {
        // The docs' own setup and secrets examples.
        assert!(
            validate(
                "# Secrets\n.env\n.env.*\n!.env.example\n*.pem\n*.key\n\nsecrets/\nprivate/\n"
            )
            .is_empty()
        );
        assert!(validate(".env\n.env.local\n.env.production\n\n# Keep the template\n!.env.example\n*.pem\ncredentials/\n").is_empty());
    }

    #[test]
    fn excluded_parents_in_every_form() {
        for content in [
            "/secrets\n!secrets/public.txt\n",
            "secrets\n!/secrets/public.txt\n",
            "**/secrets\n!secrets/public.txt\n",
            "build/\n!build/keep/a.txt\n",
            "build/keep/\n!build/keep/a.txt\n",
            "node_modules\n!node_modules/pkg/index.js\n",
            "data/*/\n!data/exports/keep.csv\n",
        ] {
            assert_eq!(validate(content).len(), 1, "{content}");
        }
    }

    #[test]
    fn negations_that_can_work_are_not_reported() {
        for content in [
            // Only the contents are excluded, not the directory.
            "secrets/*\n!secrets/public.txt\n",
            "secrets/**\n!secrets/public.txt\n",
            // The directory is re-included first.
            "secrets/\n!secrets/\n!secrets/public.txt\n",
            // No parent named literally.
            "secrets/\n!public.txt\n",
            "secrets/\n!**/public.txt\n",
            "*.log\n!logs/keep.log\n",
            // Anchored elsewhere.
            "/a/secrets/\n!secrets/public.txt\n",
            "\\!important\n!important\n",
        ] {
            assert!(validate(content).is_empty(), "{content}");
        }
    }

    #[test]
    fn rule_can_be_disabled() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["KR-IGN-001".to_string()];
        let path = Path::new(".kiroignore");
        assert!(
            KiroIgnoreValidator
                .validate_per_file(path, "secrets/\n!secrets/a\n", &config.for_path(path))
                .is_empty()
        );
    }

    #[test]
    fn glob_translation() {
        let m = |glob: &str, anchored: bool, path: &str| {
            glob_regex(glob, anchored).unwrap().is_match(path)
        };
        assert!(m("secrets", false, "a/secrets"));
        assert!(m("secrets", false, "secrets"));
        assert!(!m("a/secrets", true, "b/a/secrets"));
        assert!(m("**/build", true, "x/y/build"));
        assert!(m("a/**/b", true, "a/b"));
        assert!(m("a/**/b", true, "a/x/y/b"));
        assert!(m("*.l?g", false, "x/app.log"));
        assert!(m("[!a]b", false, "cb"));
        assert!(!m("[!a]b", false, "ab"));
        assert!(m("dir/**", true, "dir/x/y"));
    }
}
