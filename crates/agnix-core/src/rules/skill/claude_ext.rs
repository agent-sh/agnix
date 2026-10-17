//! Claude Code skill and legacy command checks added from the 2026-10 vendor
//! survey:
//!
//! - CC-SK-024: no `description` or `when_to_use`
//! - CC-SK-025: project skill listing over the listing budget (estimate)
//! - CC-SK-027: `name` or `paths` in a legacy `.claude/commands/` file
//! - CC-SK-028: `!` injection placeholder that Claude Code doesn't recognize,
//!   or a raw control character that stops every injected command
//! - CC-SK-030 / CC-SK-031: name or description that blocks upload to
//!   claude.ai or the Claude API (reserved words, XML tags)
//! - CC-SK-032: description not written in the third person
//! - CC-SK-033: vague skill name
//! - CC-SK-034: long bundled reference file without a table of contents
//! - CC-PL-020: unbraced `$CLAUDE_PLUGIN_ROOT` / `$CLAUDE_PLUGIN_DATA` in a
//!   plugin skill body
//! - CC-HK-0xx handler checks on skill frontmatter hooks

use super::*;
use crate::rules::hooks::{HookOrigin, check_hooks};

/// "Skill descriptions are cut short": the listing budget "scales at 1% of the
/// model's context window". The unit conversion is undocumented; agnix
/// assumes a 200,000-token window and 4 characters per token, so 1% is about
/// 8,000 characters. An agnix heuristic, reported at LOW.
const SKILL_LISTING_BUDGET_CHARS: usize = 8_000;
/// Per-skill cap in the listing: "the combined `description` and
/// `when_to_use` text is truncated at 1,536 characters".
const SKILL_LISTING_ENTRY_CAP: usize = 1_536;
/// skill-creator: "For large reference files (>300 lines), include a table of
/// contents". The platform best practices say 100; agnix uses 300 to keep
/// the check quiet.
const REFERENCE_TOC_LINES: usize = 300;
const MAX_LISTED_SKILLS: usize = 1000;

/// Vague names from the platform best practices "Naming conventions".
const VAGUE_SKILL_NAMES: &[&str] = &[
    "helper",
    "helpers",
    "util",
    "utils",
    "tool",
    "tools",
    "document",
    "documents",
    "data",
    "file",
    "files",
];

static_regex!(
    fn xml_tag_regex,
    r#"</?[A-Za-z][A-Za-z0-9_.:-]*(?:\s+[A-Za-z_:][A-Za-z0-9_.:-]*(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s"'<>=]+))?)*\s*/?>"#
);
static_regex!(fn unbraced_plugin_var_regex, r"\$CLAUDE_PLUGIN_(ROOT|DATA)\b");
static_regex!(fn toc_heading_regex, r"(?im)^\s{0,3}#{1,6}\s*(table of contents|contents|toc)\b");

/// Whether `path` is a legacy custom command file (`.claude/commands/**.md`).
pub(crate) fn is_claude_command_path(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        && crate::file_types::path_contains_consecutive_components(path, ".claude", "commands")
}

/// The first unbraced `$CLAUDE_PLUGIN_ROOT` or `$CLAUDE_PLUGIN_DATA` in a
/// Markdown body that Claude could run through the Bash tool: (byte offset,
/// spelling). Hook and other config examples get the variable from their own
/// environment, so these are skipped: a line that mentions hooks, a fenced
/// JSON block, and an inline code span holding only the variable name.
pub(crate) fn unbraced_plugin_var(body: &str) -> Option<(usize, &str)> {
    let mut offset = 0;
    let mut fence: Option<(char, bool)> = None; // (fence char, is a JSON block)
    for line in body.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let trimmed = line.trim_start();
        let fence_char = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        if let Some(c) = fence_char
            && trimmed.starts_with(&c.to_string().repeat(3))
        {
            match fence {
                Some((open, _)) if open == c => fence = None,
                Some(_) => {}
                None => {
                    let info = trimmed.trim_start_matches(c).trim().to_ascii_lowercase();
                    fence = Some((c, info.starts_with("json")));
                }
            }
            continue;
        }
        if fence.is_some_and(|(_, json)| json) || line.to_ascii_lowercase().contains("hook") {
            continue;
        }
        for m in unbraced_plugin_var_regex().find_iter(line) {
            let name_only = line[..m.start()].ends_with('`') && line[m.end()..].starts_with('`');
            if !name_only {
                return Some((start + m.start(), m.as_str()));
            }
        }
    }
    None
}

/// The plugin root of a plugin skill (`<root>/.claude-plugin/plugin.json`
/// above the SKILL.md).
fn plugin_root_of(path: &Path, fs: &dyn crate::fs::FileSystem) -> Option<PathBuf> {
    path.parent()?
        .ancestors()
        .find(|dir| fs.is_file(&dir.join(".claude-plugin").join("plugin.json")))
        .map(Path::to_path_buf)
}

impl ValidationContext<'_> {
    fn body(&self) -> &str {
        if self.parts.body_start <= self.content.len() {
            &self.content[self.parts.body_start..]
        } else {
            ""
        }
    }

    fn yaml_field(&mut self, key: &str) -> Option<serde_yaml::Value> {
        self.parsed_frontmatter_yaml()?
            .get(key)
            .filter(|v| !v.is_null())
            .cloned()
    }

    /// CC-SK-027: "a Markdown file in `.claude/commands/` is the older format
    /// and still works. It supports the same frontmatter except `name` and
    /// `paths`. ... Prefer a skill for new work". One note per file; when the
    /// file sets `name` or `paths`, the note says they have no effect.
    pub(super) fn validate_cc_command_note(&mut self) {
        if !self.config.is_rule_enabled("CC-SK-027") {
            return;
        }
        let ignored: Vec<&str> = if self.parts.has_frontmatter && self.parts.has_closing {
            ["name", "paths"]
                .into_iter()
                .filter(|field| self.yaml_field(field).is_some())
                .collect()
        } else {
            Vec::new()
        };
        let stem = self
            .path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("name")
            .to_string();
        let (line, col, message) = match ignored.first() {
            Some(field) => {
                let (line, col) = self.frontmatter_key_line_col(field);
                (
                    line,
                    col,
                    t!(
                        "rules.cc_sk_027.ignored_fields",
                        fields = ignored.join(" and ")
                    ),
                )
            }
            None => (1, 1, t!("rules.cc_sk_027.message")),
        };
        self.diagnostics.push(
            Diagnostic::info(self.path.to_path_buf(), line, col, "CC-SK-027", message)
                .with_suggestion(t!("rules.cc_sk_027.suggestion", name = stem.as_str())),
        );
    }

    /// CC-SK-024: "All fields are optional. Only `description` is
    /// recommended so Claude knows when to use the skill" and "If omitted,
    /// uses the first non-empty line of the markdown content."
    pub(super) fn validate_cc_description_presence(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-024")
            || frontmatter.disable_model_invocation == Some(true)
        {
            return;
        }
        let has_description = frontmatter
            .description
            .as_deref()
            .is_some_and(|d| !d.trim().is_empty());
        let has_when = self
            .yaml_field("when_to_use")
            .and_then(|v| v.as_str().map(|s| !s.trim().is_empty()))
            .unwrap_or(false);
        if has_description || has_when {
            return;
        }
        let (line, col) = self.line_col_at(self.parts.frontmatter_start);
        self.diagnostics.push(
            Diagnostic::info(
                self.path.to_path_buf(),
                line,
                col,
                "CC-SK-024",
                t!("rules.cc_sk_024.message"),
            )
            .with_suggestion(t!("rules.cc_sk_024.suggestion")),
        );
    }

    /// CC-SK-030 / CC-SK-031: the claude.ai and Claude API upload limits:
    /// `name` "no XML tags, no reserved words ("anthropic", "claude")" and
    /// `description` "no XML tags". Claude Code itself loads such skills, so
    /// these are LOW notes for skills that may be shared.
    pub(super) fn validate_cc_upload_constraints(&mut self, frontmatter: &SkillFrontmatter) {
        let name = frontmatter
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| derived_claude_skill_name(self.path));
        if self.config.is_rule_enabled("CC-SK-030") {
            let lower = name.to_ascii_lowercase();
            if let Some(word) = ["anthropic", "claude"]
                .into_iter()
                .find(|w| lower.contains(w))
                && !lower.starts_with("anthropic-skills")
            {
                let (line, col) = self.frontmatter_key_line_col("name");
                self.diagnostics.push(
                    Diagnostic::info(
                        self.path.to_path_buf(),
                        line,
                        col,
                        "CC-SK-030",
                        t!("rules.cc_sk_030.message", name = name.as_str(), word = word),
                    )
                    .with_suggestion(t!("rules.cc_sk_030.suggestion")),
                );
            }
        }
        if self.config.is_rule_enabled("CC-SK-031") {
            for (field, value) in [
                ("name", Some(name.as_str())),
                ("description", frontmatter.description.as_deref()),
            ] {
                let Some(value) = value else { continue };
                if let Some(tag) = xml_tag_regex().find(value) {
                    let (line, col) = self.frontmatter_key_line_col(field);
                    self.diagnostics.push(
                        Diagnostic::info(
                            self.path.to_path_buf(),
                            line,
                            col,
                            "CC-SK-031",
                            t!("rules.cc_sk_031.message", field = field, tag = tag.as_str()),
                        )
                        .with_suggestion(t!("rules.cc_sk_031.suggestion")),
                    );
                }
            }
        }
    }

    /// CC-SK-032: "Always write in third person. The description is injected
    /// into the system prompt, and inconsistent point-of-view can cause
    /// discovery problems."
    pub(super) fn validate_cc_description_style(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-032") {
            return;
        }
        let Some(description) = frontmatter.description.as_deref().map(str::trim) else {
            return;
        };
        let lower = description.to_ascii_lowercase();
        let first_person = ["i ", "i'm ", "i'll ", "i can ", "i will ", "i help "];
        let second_person = [
            "you can ",
            "you should ",
            "this skill helps you",
            "lets you ",
        ];
        let Some(opening) = first_person
            .iter()
            .chain(second_person.iter())
            .find(|p| lower.starts_with(**p))
        else {
            return;
        };
        let (line, col) = self.frontmatter_key_line_col("description");
        self.diagnostics.push(
            Diagnostic::info(
                self.path.to_path_buf(),
                line,
                col,
                "CC-SK-032",
                t!("rules.cc_sk_032.message", opening = opening.trim()),
            )
            .with_suggestion(t!("rules.cc_sk_032.suggestion")),
        );
    }

    /// CC-SK-033: "Vague names: `helper`, `utils`, `tools`" and "Overly
    /// generic: `documents`, `data`, `files`".
    pub(super) fn validate_cc_vague_name(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("CC-SK-033") {
            return;
        }
        let name = frontmatter
            .name
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| derived_claude_skill_name(self.path));
        let leaf = name
            .rsplit(':')
            .next()
            .unwrap_or(&name)
            .to_ascii_lowercase();
        if !VAGUE_SKILL_NAMES.contains(&leaf.as_str()) {
            return;
        }
        let (line, col) = self.frontmatter_key_line_col("name");
        self.diagnostics.push(
            Diagnostic::info(
                self.path.to_path_buf(),
                line,
                col,
                "CC-SK-033",
                t!("rules.cc_sk_033.message", name = name.as_str()),
            )
            .with_suggestion(t!("rules.cc_sk_033.suggestion")),
        );
    }

    /// CC-SK-028: "The inline form is only recognized when `!` appears at the
    /// start of a line or immediately after whitespace. If `!` follows another
    /// character, as in `KEY=!`cmd``, the placeholder is left as literal text
    /// and the command does not run", and a raw control character other than
    /// tab or newline in any `!` command means none of them runs.
    pub(super) fn validate_cc_injected_commands(&mut self) {
        if !self.config.is_rule_enabled("CC-SK-028") {
            return;
        }
        let body_start = self.parts.body_start.min(self.content.len());
        let body = self.body();
        let masked = crate::parsers::markdown::mask_code_spans(body);
        let mut unrecognized = None;
        let mut has_injection = body
            .lines()
            .any(|line| line.trim_start().starts_with("```!"));
        for (pos, _) in body.match_indices("!`") {
            // The `!` must be prose: masked text keeps it, code turns it to a space.
            if masked.as_bytes().get(pos) != Some(&b'!') {
                continue;
            }
            let before = body[..pos].chars().next_back();
            match before {
                None => has_injection = true,
                Some(c) if c.is_whitespace() => has_injection = true,
                Some(_) => {
                    if unrecognized.is_none() {
                        unrecognized = Some(pos);
                    }
                }
            }
        }
        if let Some(pos) = unrecognized {
            let line_start = body[..pos].rfind('\n').map_or(0, |i| i + 1);
            let line_end = body[pos..].find('\n').map_or(body.len(), |i| pos + i);
            let snippet: String = body[line_start..line_end].trim().chars().take(60).collect();
            let (line, col) = self.line_col_at(body_start + pos);
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "CC-SK-028",
                    t!("rules.cc_sk_028.unrecognized", snippet = snippet.as_str()),
                )
                .with_suggestion(t!("rules.cc_sk_028.unrecognized_suggestion")),
            );
        }
        if has_injection
            && let Some(offset) = self
                .content
                .char_indices()
                .find(|(_, c)| c.is_control() && !matches!(c, '\t' | '\n' | '\r'))
                .map(|(i, _)| i)
        {
            let (line, col) = self.line_col_at(offset);
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "CC-SK-028",
                    t!("rules.cc_sk_028.control_character"),
                )
                .with_suggestion(t!("rules.cc_sk_028.control_character_suggestion")),
            );
        }
    }

    /// CC-PL-020: "The variables aren't present in the environment of
    /// commands Claude runs through the Bash tool ... In skill, command, and
    /// agent content, write the `${...}` reference in the Markdown body
    /// instead, and Claude Code substitutes the path inline".
    pub(super) fn validate_cc_plugin_vars(&mut self) {
        if !self.config.is_rule_enabled("CC-PL-020")
            || plugin_root_of(self.path, self.config.fs().as_ref()).is_none()
        {
            return;
        }
        let body_start = self.parts.body_start.min(self.content.len());
        let Some((pos, spelling)) = unbraced_plugin_var(self.body()) else {
            return;
        };
        let spelling = spelling.to_string();
        let braced = format!("${{{}}}", &spelling[1..]);
        let (line, col) = self.line_col_at(body_start + pos);
        self.diagnostics.push(
            Diagnostic::warning(
                self.path.to_path_buf(),
                line,
                col,
                "CC-PL-020",
                t!("rules.cc_pl_020.message", spelling = spelling.as_str()),
            )
            .with_suggestion(t!("rules.cc_pl_020.suggestion", braced = braced.as_str())),
        );
    }

    /// CC-HK-0xx handler checks on skill frontmatter hooks, reported at the
    /// `hooks` key.
    pub(super) fn validate_cc_frontmatter_hooks(&mut self) {
        let Some(hooks) = self.yaml_field("hooks") else {
            return;
        };
        let Ok(serde_json::Value::Object(events)) = serde_json::to_value(&hooks) else {
            return;
        };
        let (line, col) = self.frontmatter_key_line_col("hooks");
        for finding in check_hooks(&events, HookOrigin::Frontmatter, None, self.config) {
            let make = match finding.level {
                crate::diagnostics::DiagnosticLevel::Error => Diagnostic::error,
                crate::diagnostics::DiagnosticLevel::Warning => Diagnostic::warning,
                crate::diagnostics::DiagnosticLevel::Info => Diagnostic::info,
            };
            self.diagnostics.push(
                make(
                    self.path.to_path_buf(),
                    line,
                    col,
                    finding.rule,
                    finding.message,
                )
                .with_suggestion(finding.suggestion),
            );
        }
    }

    /// CC-SK-025: the project's `.claude/skills` listing is estimated over the
    /// listing budget. The first skill directory (sorted) reports it.
    pub(super) fn validate_cc_listing_budget(&mut self) {
        if !self.config.is_rule_enabled("CC-SK-025") {
            return;
        }
        let Some(skill_dir) = self.path.parent() else {
            return;
        };
        let Some(skills_dir) = skill_dir.parent() else {
            return;
        };
        if skills_dir.file_name().is_none_or(|n| n != "skills")
            || skills_dir
                .parent()
                .and_then(Path::file_name)
                .is_none_or(|n| n != ".claude")
        {
            return;
        }
        let fs = self.config.fs();
        let Ok(entries) = fs.read_dir(skills_dir) else {
            return;
        };
        let mut skill_files: Vec<PathBuf> = entries
            .iter()
            .filter(|e| e.metadata.is_dir && !e.metadata.is_symlink)
            .map(|e| e.path.join("SKILL.md"))
            .filter(|p| fs.is_file(p))
            .take(MAX_LISTED_SKILLS)
            .collect();
        skill_files.sort();
        if skill_files.first().map(PathBuf::as_path) != Some(self.path) {
            return;
        }
        let mut total = 0usize;
        let mut listed = 0usize;
        for file in &skill_files {
            let text = if file.as_path() == self.path {
                self.content.to_string()
            } else {
                match fs.read_to_string(file) {
                    Ok(t) => t,
                    Err(_) => continue,
                }
            };
            let parts = split_frontmatter(&text);
            let yaml: serde_yaml::Value =
                serde_yaml::from_str(&parts.frontmatter).unwrap_or(serde_yaml::Value::Null);
            let disabled = yaml.get("disable-model-invocation").is_some_and(|v| {
                v.as_bool() == Some(true)
                    || v.as_str().is_some_and(|s| {
                        matches!(s.to_ascii_lowercase().as_str(), "true" | "yes" | "on" | "1")
                    })
            });
            if disabled {
                continue;
            }
            let name = yaml
                .get("name")
                .and_then(|v| v.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| derived_claude_skill_name(file));
            let text_len = ["description", "when_to_use"]
                .iter()
                .filter_map(|k| yaml.get(*k).and_then(|v| v.as_str()))
                .map(|s| s.chars().count())
                .sum::<usize>()
                .min(SKILL_LISTING_ENTRY_CAP);
            total += name.chars().count() + text_len;
            listed += 1;
        }
        if total <= SKILL_LISTING_BUDGET_CHARS {
            return;
        }
        self.diagnostics.push(
            Diagnostic::info(
                self.path.to_path_buf(),
                1,
                1,
                "CC-SK-025",
                t!(
                    "rules.cc_sk_025.message",
                    chars = total,
                    count = listed,
                    budget = SKILL_LISTING_BUDGET_CHARS
                ),
            )
            .with_suggestion(t!("rules.cc_sk_025.suggestion")),
        );
    }

    /// CC-SK-034: "For large reference files (>300 lines), include a table of
    /// contents" (skill-creator). Reported on the reference file itself.
    pub(super) fn validate_cc_reference_toc(&mut self) {
        if !self.config.is_rule_enabled("CC-SK-034") {
            return;
        }
        let Some(skill_dir) = self.path.parent() else {
            return;
        };
        let fs = self.config.fs();
        let mut targets: Vec<String> =
            crate::parsers::markdown::extract_markdown_links(self.body())
                .into_iter()
                .map(|link| link.url)
                .collect();
        targets.extend(
            extract_reference_paths(self.body())
                .into_iter()
                .map(|p| p.path),
        );
        let mut seen = std::collections::HashSet::new();
        for target in targets {
            let target = target.split('#').next().unwrap_or_default().to_string();
            if !target.to_ascii_lowercase().ends_with(".md")
                || target.contains("://")
                || target.starts_with('/')
                || target.split(['/', '\\']).any(|seg| seg == "..")
            {
                continue;
            }
            let file = skill_dir.join(target.trim_start_matches("./"));
            if !seen.insert(file.clone()) || !fs.is_file(&file) {
                continue;
            }
            let Ok(text) = fs.read_to_string(&file) else {
                continue;
            };
            let lines = text.lines().count();
            if lines <= REFERENCE_TOC_LINES {
                continue;
            }
            let head: String = text.lines().take(40).collect::<Vec<_>>().join("\n");
            let anchor_links = head.lines().filter(|l| l.contains("](#")).count();
            if toc_heading_regex().is_match(&head) || anchor_links >= 3 {
                continue;
            }
            self.diagnostics.push(
                Diagnostic::info(
                    file.clone(),
                    1,
                    1,
                    "CC-SK-034",
                    t!(
                        "rules.cc_sk_034.message",
                        lines = lines,
                        threshold = REFERENCE_TOC_LINES
                    ),
                )
                .with_suggestion(t!("rules.cc_sk_034.suggestion")),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_path_detection() {
        assert!(is_claude_command_path(Path::new(
            "p/.claude/commands/deploy.md"
        )));
        assert!(is_claude_command_path(Path::new(
            ".claude/commands/frontend/component.md"
        )));
        assert!(!is_claude_command_path(Path::new("p/commands/deploy.md")));
        assert!(!is_claude_command_path(Path::new(
            ".claude/skills/x/SKILL.md"
        )));
    }

    #[test]
    fn unbraced_plugin_variable() {
        assert_eq!(
            unbraced_plugin_var("run bash $CLAUDE_PLUGIN_ROOT/x.sh"),
            Some((9, "$CLAUDE_PLUGIN_ROOT"))
        );
        assert_eq!(unbraced_plugin_var("run ${CLAUDE_PLUGIN_ROOT}/x.sh"), None);
        assert_eq!(unbraced_plugin_var("$CLAUDE_PLUGIN_ROOTS"), None);
        // Found inside a shell fence, after an earlier skipped line.
        assert_eq!(
            unbraced_plugin_var("intro\n```bash\nnode $CLAUDE_PLUGIN_DATA/x\n```\n"),
            Some((19, "$CLAUDE_PLUGIN_DATA"))
        );
    }

    #[test]
    fn unbraced_plugin_variable_skips_hook_and_config_examples() {
        // Hook commands get the variable from their environment.
        assert_eq!(
            unbraced_plugin_var("Test with `echo $CLAUDE_PLUGIN_ROOT` in hook scripts"),
            None
        );
        // A name-only mention documents the variable.
        assert_eq!(
            unbraced_plugin_var("- `$CLAUDE_PLUGIN_ROOT` - Plugin directory"),
            None
        );
        // JSON config examples are not run through the Bash tool.
        assert_eq!(
            unbraced_plugin_var("```json\n{\"command\": \"$CLAUDE_PLUGIN_ROOT/run.sh\"}\n```\n"),
            None
        );
        // A JavaScript block Claude may run still counts.
        assert!(
            unbraced_plugin_var("```javascript\nconst root = '$CLAUDE_PLUGIN_ROOT';\n```\n")
                .is_some()
        );
    }

    #[test]
    fn xml_tags() {
        assert!(xml_tag_regex().is_match("Use <example> tags"));
        assert!(xml_tag_regex().is_match("</thinking>"));
        assert!(!xml_tag_regex().is_match("a < b and c > d"));
        assert!(!xml_tag_regex().is_match("x<5"));
        assert!(xml_tag_regex().is_match(r#"<example type="a">"#));
        // A prose placeholder is not tag-shaped.
        assert!(!xml_tag_regex().is_match("Pass <the problem, stated in full, or a path>"));
    }
}

#[cfg(test)]
mod validator_tests {
    use super::super::SkillValidator;
    use crate::config::LintConfig;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel};
    use crate::rules::Validator;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    fn write(root: &Path, rel: &str, content: &str) -> std::path::PathBuf {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
        path
    }

    fn check(path: &Path) -> Vec<Diagnostic> {
        let content = fs::read_to_string(path).unwrap();
        SkillValidator.validate(path, &content, &LintConfig::default())
    }

    fn validate(path: &str, content: &str) -> Vec<Diagnostic> {
        SkillValidator.validate(Path::new(path), content, &LintConfig::default())
    }

    fn count(diagnostics: &[Diagnostic], id: &str) -> usize {
        diagnostics.iter().filter(|d| d.rule == id).count()
    }

    #[test]
    fn cc_sk_027_command_file_gets_one_migration_note() {
        let diagnostics = validate(
            ".claude/commands/deploy.md",
            "---\nname: ship\npaths: src/**\ndescription: Deploy the app\n---\nDeploy.\n",
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-027")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].level, DiagnosticLevel::Info);
        assert_eq!(hits[0].line, 2);
        assert!(hits[0].message.contains("name and paths"));
        assert!(
            hits[0]
                .suggestion
                .as_deref()
                .unwrap()
                .contains(".claude/skills/deploy/SKILL.md")
        );
        // No deep validation: neither Agent Skills spec rules nor the skill
        // field rules run on command files (CC-SK-006 would match "deploy").
        assert!(
            diagnostics.iter().all(|d| d.rule == "CC-SK-027"),
            "{diagnostics:?}"
        );
        // A command file without frontmatter gets the plain note.
        let diagnostics = validate(".claude/commands/review.md", "Review the diff.\n");
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].rule, "CC-SK-027");
    }

    #[test]
    fn cc_sk_006_commit_and_send_keywords() {
        for name in ["commit", "send-slack-message"] {
            let diagnostics = validate(
                &format!(".claude/skills/{name}/SKILL.md"),
                &format!(
                    "---\nname: {name}\ndescription: Use when you need to {name}\n---\nRun it.\n"
                ),
            );
            let hit = diagnostics
                .iter()
                .find(|d| d.rule == "CC-SK-006")
                .expect(name);
            assert_eq!(hit.level, DiagnosticLevel::Warning);
        }
    }

    #[test]
    fn cc_sk_024_no_description() {
        let diagnostics = validate(
            ".claude/skills/x/SKILL.md",
            "---\nname: x\n---\nDo things.\n",
        );
        assert_eq!(count(&diagnostics, "CC-SK-024"), 1);
        for fm in [
            "name: x\ndescription: Does x. Use when x is needed.",
            "name: x\nwhen_to_use: When x is needed",
            "name: x\ndisable-model-invocation: true",
        ] {
            let diagnostics = validate(
                ".claude/skills/x/SKILL.md",
                &format!("---\n{fm}\n---\nDo things.\n"),
            );
            assert_eq!(count(&diagnostics, "CC-SK-024"), 0, "{fm}");
        }
    }

    #[test]
    fn cc_sk_028_injected_commands() {
        let diagnostics = validate(
            ".claude/skills/x/SKILL.md",
            "---\nname: x\ndescription: Shows status. Use when asked for status.\n---\nBranch: !`git branch --show-current`\nKEY=!`echo hi`\n`KEY=!`cmd`` in code is fine\n",
        );
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-028")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert_eq!(hits[0].line, 6);
        let diagnostics = validate(
            ".claude/skills/x/SKILL.md",
            "---\nname: x\ndescription: Shows status. Use when asked.\n---\nBranch: !`git status`\nbell \u{7} here\n",
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.rule == "CC-SK-028" && d.message.contains("control character")),
            "{diagnostics:?}"
        );
    }

    #[test]
    fn cc_sk_030_031_upload_constraints() {
        let diagnostics = validate(
            ".claude/skills/claude-tools/SKILL.md",
            "---\nname: claude-tools\ndescription: Formats <output> blocks. Use when formatting.\n---\nGo.\n",
        );
        assert_eq!(count(&diagnostics, "CC-SK-030"), 1, "{diagnostics:?}");
        assert_eq!(count(&diagnostics, "CC-SK-031"), 1, "{diagnostics:?}");
        assert!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "CC-SK-030" || d.rule == "CC-SK-031")
                .all(|d| d.level == DiagnosticLevel::Info)
        );
    }

    #[test]
    fn cc_sk_032_033_style() {
        let diagnostics = validate(
            ".claude/skills/helper/SKILL.md",
            "---\nname: helper\ndescription: I can help you process Excel files\n---\nGo.\n",
        );
        assert_eq!(count(&diagnostics, "CC-SK-032"), 1, "{diagnostics:?}");
        assert_eq!(count(&diagnostics, "CC-SK-033"), 1, "{diagnostics:?}");
        let diagnostics = validate(
            ".claude/skills/processing-pdfs/SKILL.md",
            "---\nname: processing-pdfs\ndescription: Extracts text from PDF files. Use when working with PDFs.\n---\nGo.\n",
        );
        assert_eq!(
            count(&diagnostics, "CC-SK-032") + count(&diagnostics, "CC-SK-033"),
            0
        );
    }

    #[test]
    fn cc_sk_025_listing_budget() {
        let temp = TempDir::new().unwrap();
        let long = "x".repeat(1500);
        let mut first = None;
        for i in 0..7 {
            let path = write(
                temp.path(),
                &format!(".claude/skills/s{i}/SKILL.md"),
                &format!("---\nname: s{i}\ndescription: {long}\n---\nBody\n"),
            );
            first.get_or_insert(path);
        }
        let first = first.unwrap();
        assert_eq!(count(&check(&first), "CC-SK-025"), 1);
        let other = temp.path().join(".claude/skills/s3/SKILL.md");
        assert_eq!(count(&check(&other), "CC-SK-025"), 0);
    }

    #[test]
    fn cc_sk_034_reference_toc() {
        let temp = TempDir::new().unwrap();
        let skill = write(
            temp.path(),
            ".claude/skills/finance/SKILL.md",
            "---\nname: finance\ndescription: Answers finance questions. Use when asked about revenue.\n---\nSee [the schema](reference/finance.md) and [api](reference/api.md).\n",
        );
        let long: String = (0..400).map(|i| format!("line {i}\n")).collect();
        write(
            temp.path(),
            ".claude/skills/finance/reference/finance.md",
            &long,
        );
        write(
            temp.path(),
            ".claude/skills/finance/reference/api.md",
            &format!("# API\n\n## Contents\n- a\n{long}"),
        );
        let diagnostics = check(&skill);
        let hits: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-034")
            .collect();
        assert_eq!(hits.len(), 1, "{diagnostics:?}");
        assert!(hits[0].file.ends_with("reference/finance.md"));
    }

    #[test]
    fn cc_pl_020_plugin_skill_body() {
        let temp = TempDir::new().unwrap();
        write(
            temp.path(),
            ".claude-plugin/plugin.json",
            r#"{"name": "p"}"#,
        );
        let skill = write(
            temp.path(),
            "skills/run/SKILL.md",
            "---\nname: run\ndescription: Runs the tool. Use when asked to run it.\n---\nRun `bash $CLAUDE_PLUGIN_ROOT/scripts/run.sh`.\n",
        );
        let diagnostics = check(&skill);
        assert_eq!(count(&diagnostics, "CC-PL-020"), 1, "{diagnostics:?}");
        let fixed = write(
            temp.path(),
            "skills/ok/SKILL.md",
            "---\nname: ok\ndescription: Runs the tool. Use when asked to run it.\n---\nRun `bash \"${CLAUDE_PLUGIN_ROOT}\"/scripts/run.sh`.\n",
        );
        assert_eq!(count(&check(&fixed), "CC-PL-020"), 0);
    }

    #[test]
    fn skill_frontmatter_hooks_get_handler_checks() {
        let diagnostics = validate(
            ".claude/skills/x/SKILL.md",
            "---\nname: x\ndescription: Does x. Use when needed.\nhooks:\n  PermissionRequest:\n    - hooks:\n        - type: command\n          command: ./scripts/approve.sh\n---\nGo.\n",
        );
        assert_eq!(count(&diagnostics, "CC-HK-035"), 1, "{diagnostics:?}");
        // Relative script paths in frontmatter hooks are the vendor's own
        // example, so the CC-HK-030 relative-path note is skipped there.
        assert_eq!(count(&diagnostics, "CC-HK-030"), 0, "{diagnostics:?}");
    }
}
