//! Agent Skills checks that apply to every client (AS-013 reference chains,
//! AS-020..AS-024, AS-026, AS-027) and the cross-client skill hygiene rules
//! XP-SK-003 and XP-SK-004. AS-025 (duplicate names across skill roots) is a
//! project-level check in `rules/project_level.rs`.
//!
//! Sources: agentskills.io specification ("Should describe both what the
//! skill does and when to use it", "Instructions (< 5000 tokens
//! recommended)", "Keep file references one level deep from SKILL.md"),
//! skill-creation best practices and using-scripts pages, Codex
//! quick_validate.py and Gemini validate_skill.cjs (TODO placeholders),
//! Gemini skills best practices and Cline config docs (secrets), Anthropic
//! skill best practices and VS Code agent skills (forward slashes).

use super::helpers::{compute_line_starts, extract_reference_paths, line_col_at};
use super::{SkillFrontmatter, ValidationContext};
use crate::diagnostics::Diagnostic;
use crate::parsers::frontmatter::split_frontmatter;
use crate::rules::per_client_skill::SkillClient;
use regex::Regex;
use rust_i18n::t;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// SKILL.md body size above which AS-021 reports, in estimated tokens. The
/// spec recommends instructions "< 5000 tokens"; the estimate (characters /
/// 4) is an agnix heuristic.
const BODY_TOKEN_LIMIT: usize = 5000;

fn regex(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("skill pattern must compile"))
}

/// Phrases that tell an agent when to use a skill (AS-020).
fn trigger_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex(
        &RE,
        r"(?i)\b(?:use (?:this|it|when|whenever|for|if|after|before|to|on|in|with)|when\b|whenever|if (?:the user|you|a user|asked)|for (?:tasks|requests|questions)\b|trigger|invoke|activat|appl(?:y|ies) (?:to|when)|asks? (?:for|to|about)|mentions?\b|requests?\b)",
    )
}

/// Unfinished scaffold placeholders (AS-027), as Codex's quick_validate.py
/// matches them: `[TODO: ...]` on its own line.
fn todo_line_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex(&RE, r"^[ ]{0,3}\[TODO:[^\n]*\][ \t]*$")
}

/// A generic pointer to bundled docs (AS-026).
fn generic_pointer_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex(
        &RE,
        r"(?i)\bsee (?:the )?(?:`)?(?:references?|docs)/?(?:`)?(?: (?:folder|directory|dir))?(?: for (?:more )?(?:details|info(?:rmation)?|more))?\s*[.)]?\s*$",
    )
}

/// Credentials in skill files (XP-SK-003): well-known token formats and
/// private key blocks.
fn secret_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex(
        &RE,
        r"\b(?:ghp_[A-Za-z0-9]{30,}|gho_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{40,}|glpat-[A-Za-z0-9_-]{20,}|sk-ant-[A-Za-z0-9_-]{20,}|sk-proj-[A-Za-z0-9_-]{20,}|xox[bpas]-[A-Za-z0-9-]{20,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35})\b|-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----",
    )
}

/// A Windows-style path into a skill's bundled directories (XP-SK-004).
fn backslash_path_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    regex(
        &RE,
        r"(?i)\b(?:scripts|references?|assets|templates|examples|resources)\\[A-Za-z0-9_.\\-]+",
    )
}

/// Interactive input in a bundled script (AS-022).
fn interactive_patterns() -> &'static [(Regex, &'static str)] {
    static RE: OnceLock<Vec<(Regex, &'static str)>> = OnceLock::new();
    RE.get_or_init(|| {
        [
            (r"\bread\s+(?:-[a-zA-Z]*\s+)*-[a-zA-Z]*p\b", "read -p"),
            (r"\bread\b[^\n]*</dev/tty", "read from /dev/tty"),
            (r"^\s*select\s+\w+\s+in\b", "select"),
            (r#"\binput\(\s*[rfu]?["']"#, "input()"),
            (r"\bgetpass\.getpass\(", "getpass()"),
            (r"\bclick\.(?:prompt|confirm)\(", "click.prompt()"),
            (r"\bquestionary\.\w+\(", "questionary"),
            (r"\.question\(", "readline question()"),
            (
                r#"(?:require\(|from\s+)["'](?:inquirer|@inquirer/[\w-]+|prompts|enquirer)["']"#,
                "an interactive prompt library",
            ),
            (r"(?i)\bRead-Host\b", "Read-Host"),
        ]
        .into_iter()
        .map(|(pattern, label)| (Regex::new(pattern).expect("interactive pattern"), label))
        .collect()
    })
}

/// A line that only runs the prompt when a terminal is attached.
fn is_tty_guard(line: &str) -> bool {
    line.contains("-t 0")
        || line.contains("-t 1")
        || line.contains("isatty")
        || line.contains("isTTY")
        || line.contains("[Environment]::UserInteractive")
}

/// File extensions treated as bundled scripts.
const SCRIPT_EXTENSIONS: &[&str] = &[
    "sh", "bash", "zsh", "fish", "py", "rb", "pl", "lua", "js", "ts", "mjs", "cjs", "ps1",
];

fn is_script(path: &Path, content: &str) -> bool {
    content.starts_with("#!")
        || path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| SCRIPT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

/// Normalize a reference relative to the skill root.
fn normalize_reference(path: &str) -> String {
    let path = path.replace('\\', "/");
    let path = path
        .strip_prefix("${CLAUDE_SKILL_DIR}/")
        .unwrap_or(&path)
        .to_string();
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
}

/// Text of `content` inside code spans and fences only; everything else is
/// blanked (line breaks kept), so offsets match `content`.
fn code_only(content: &str) -> String {
    let masked = crate::parsers::markdown::mask_code_spans(content);
    let bytes: Vec<u8> = content
        .bytes()
        .zip(masked.bytes())
        .map(|(orig, mask)| {
            if orig == b'\n' || orig == b'\r' || orig != mask {
                // A line break, or a code byte (masking replaced it).
                orig
            } else {
                b' '
            }
        })
        .collect();
    // Whole characters are either kept or blanked, so this is valid UTF-8.
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Package names a `package.json` at `dir` depends on.
fn package_json_dependencies(dir: &Path, fs: &dyn crate::fs::FileSystem) -> HashSet<String> {
    let Ok(text) = fs.read_to_string(&dir.join("package.json")) else {
        return HashSet::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return HashSet::new();
    };
    [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ]
    .iter()
    .filter_map(|key| json.get(*key).and_then(|v| v.as_object()))
    .flat_map(|deps| deps.keys().cloned())
    .collect()
}

/// The package name of an npm spec (`@scope/pkg@1.0.0` -> `@scope/pkg`).
fn npm_package_name(spec: &str) -> &str {
    let at = if let Some(rest) = spec.strip_prefix('@') {
        rest.find('@').map(|i| i + 1)
    } else {
        spec.find('@')
    };
    at.map_or(spec, |at| &spec[..at])
}

/// Packages whose command name differs from the package name.
const NPM_BIN_PACKAGES: &[(&str, &str)] = &[
    ("tsc", "typescript"),
    ("tsserver", "typescript"),
    ("ng", "@angular/cli"),
    ("nuxi", "nuxt"),
    ("vue-cli-service", "@vue/cli-service"),
    ("playwright", "@playwright/test"),
    ("biome", "@biomejs/biome"),
];

/// True when the runner resolves `package` to a local install: a dependency
/// in a nearby package.json, or a binary in `node_modules/.bin`.
fn is_local_npm_package(
    package: &str,
    dependencies: &HashSet<String>,
    dirs: &[&Path],
    fs: &dyn crate::fs::FileSystem,
) -> bool {
    let name = npm_package_name(package);
    dependencies.contains(name)
        || NPM_BIN_PACKAGES
            .iter()
            .any(|(bin, pkg)| *bin == name && dependencies.contains(*pkg))
        || dirs
            .iter()
            .any(|dir| fs.exists(&dir.join("node_modules").join(".bin").join(name)))
}

impl ValidationContext<'_> {
    /// The SKILL.md body and its byte offset in the file.
    fn body(&self) -> (usize, &str) {
        let start = self.parts.body_start.min(self.content.len());
        (start, &self.content[start..])
    }

    /// AS-013: keep references one level deep. Follows each bundled Markdown
    /// file SKILL.md references and reports one that links a further bundled
    /// file SKILL.md does not reference, the chain the spec warns about
    /// ("Avoid deeply nested reference chains"). Directory depth alone
    /// (`references/api/errors.md`) is one level of reference and fine.
    pub(super) fn validate_reference_chains(&mut self) {
        if !self.config.is_rule_enabled("AS-013") {
            return;
        }
        let Some(dir) = self.path.parent().map(Path::to_path_buf) else {
            return;
        };
        let (body_start, body) = self.body();
        let first_level = extract_reference_paths(body);
        let direct: HashSet<String> = first_level
            .iter()
            .map(|m| normalize_reference(&m.path))
            .collect();
        let fs = self.config.fs();
        let mut found = Vec::new();
        for reference in &first_level {
            let relative = normalize_reference(&reference.path);
            if !relative.to_ascii_lowercase().ends_with(".md") {
                continue;
            }
            let file = dir.join(&relative);
            let Ok(text) = fs.read_to_string(&file) else {
                continue;
            };
            let file_dir = Path::new(&relative)
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let mut nested: Vec<String> = extract_reference_paths(&text)
                .into_iter()
                .map(|m| normalize_reference(&m.path))
                .collect();
            for link in crate::parsers::markdown::extract_markdown_links(&text) {
                let url = link.url.split('#').next().unwrap_or_default();
                if url.is_empty() || url.contains("://") || url.starts_with('/') {
                    continue;
                }
                if url.to_ascii_lowercase().ends_with(".md") {
                    nested.push(normalize_reference(&format!("{file_dir}/{url}")));
                }
            }
            if let Some(deeper) = nested.into_iter().find(|n| {
                *n != relative
                    && !direct.contains(n)
                    && !n.is_empty()
                    && !n.starts_with("..")
                    && fs.is_file(&dir.join(n))
            }) {
                found.push((body_start + reference.start, relative, deeper));
            }
        }
        for (offset, relative, deeper) in found {
            let (line, col) = self.line_col_at(offset);
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-013",
                    t!(
                        "rules.as_013.message",
                        path = relative.as_str(),
                        nested = deeper.as_str()
                    ),
                )
                .with_suggestion(t!("rules.as_013.suggestion", nested = deeper.as_str())),
            );
        }
    }

    /// AS-020, AS-021, AS-024, AS-026, AS-027 on SKILL.md, and AS-022,
    /// AS-023, XP-SK-003, XP-SK-004 on the skill and its bundled files.
    pub(super) fn validate_portable_rules(&mut self, frontmatter: &SkillFrontmatter) {
        self.validate_description_trigger(frontmatter);
        self.validate_body_tokens();
        self.validate_todo_placeholders(frontmatter);
        self.validate_generic_pointers();
        self.validate_unpinned_runners();
        self.validate_backslash_paths();
        self.validate_bundled_files();
    }

    /// AS-020: description says what the skill does but not when to use it.
    fn validate_description_trigger(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("AS-020")
            || frontmatter.disable_model_invocation == Some(true)
        {
            return;
        }
        let Some(description) = frontmatter.description.as_deref().map(str::trim) else {
            return;
        };
        if description.is_empty() {
            return;
        }
        // Claude Code's `when_to_use` is the trigger text.
        if self.client == SkillClient::ClaudeCode
            && frontmatter
                .when_to_use
                .as_deref()
                .is_some_and(|w| !w.trim().is_empty())
        {
            return;
        }
        // The marker list is English; skip text that is mostly not ASCII.
        let letters = description.chars().filter(|c| c.is_alphabetic()).count();
        let ascii_letters = description
            .chars()
            .filter(|c| c.is_ascii_alphabetic())
            .count();
        if letters == 0 || ascii_letters * 2 < letters {
            return;
        }
        if trigger_pattern().is_match(description) {
            return;
        }
        let (line, col) = self.frontmatter_key_line_col("description");
        self.diagnostics.push(
            Diagnostic::info(
                self.path.to_path_buf(),
                line,
                col,
                "AS-020",
                t!("rules.as_020.message"),
            )
            .with_suggestion(t!("rules.as_020.suggestion")),
        );
    }

    /// AS-021: SKILL.md body over the spec's 5,000-token recommendation.
    /// AS-012 (500 lines) already covers a long file, so this reports only
    /// bodies under 500 lines that are still large (wide tables, code).
    fn validate_body_tokens(&mut self) {
        if !self.config.is_rule_enabled("AS-021") {
            return;
        }
        let (body_start, body) = self.body();
        if body.lines().count() > 500 && self.config.is_rule_enabled("AS-012") {
            return;
        }
        let tokens = body.chars().count() / 4;
        if tokens <= BODY_TOKEN_LIMIT {
            return;
        }
        let (line, col) = self.line_col_at(body_start);
        self.diagnostics.push(
            Diagnostic::warning(
                self.path.to_path_buf(),
                line,
                col,
                "AS-021",
                t!(
                    "rules.as_021.message",
                    tokens = tokens,
                    limit = BODY_TOKEN_LIMIT
                ),
            )
            .with_suggestion(t!("rules.as_021.suggestion")),
        );
    }

    /// AS-027: unfinished `[TODO: ...]` scaffold placeholders.
    fn validate_todo_placeholders(&mut self, frontmatter: &SkillFrontmatter) {
        if !self.config.is_rule_enabled("AS-027") {
            return;
        }
        if frontmatter
            .description
            .as_deref()
            .is_some_and(|d| d.trim_start().starts_with("[TODO:"))
        {
            let (line, col) = self.frontmatter_key_line_col("description");
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-027",
                    t!("rules.as_027.description"),
                )
                .with_suggestion(t!("rules.as_027.suggestion")),
            );
        }
        let (body_start, body) = self.body();
        let masked = crate::parsers::markdown::mask_code_spans(body);
        let mut offset = 0usize;
        let mut hits = Vec::new();
        for line in masked.split_inclusive('\n') {
            let text = line.trim_end_matches(['\n', '\r']);
            if todo_line_pattern().is_match(text) {
                hits.push(body_start + offset + (text.len() - text.trim_start().len()));
            }
            offset += line.len();
        }
        for hit in hits {
            let (line, col) = self.line_col_at(hit);
            self.diagnostics.push(
                Diagnostic::warning(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-027",
                    t!("rules.as_027.instructions"),
                )
                .with_suggestion(t!("rules.as_027.suggestion")),
            );
        }
    }

    /// AS-026: a generic pointer to bundled docs with no load condition.
    fn validate_generic_pointers(&mut self) {
        if !self.config.is_rule_enabled("AS-026") {
            return;
        }
        let (body_start, body) = self.body();
        let masked = crate::parsers::markdown::mask_code_spans(body);
        let mut offset = 0usize;
        let mut hits = Vec::new();
        for line in masked.split_inclusive('\n') {
            let text = line.trim_end_matches(['\n', '\r']);
            if let Some(m) = generic_pointer_pattern().find(text) {
                // The sentence holding the pointer: from the last sentence end.
                let sentence_start = text[..m.start()]
                    .rfind(['.', '!', '?', ';'])
                    .map_or(0, |i| i + 1);
                let sentence = text[sentence_start..m.end()].to_ascii_lowercase();
                let conditional = ["if ", "when ", "before ", "after ", "unless ", "while "]
                    .iter()
                    .any(|word| sentence.contains(word));
                if !conditional {
                    hits.push(body_start + offset + m.start());
                }
            }
            offset += line.len();
        }
        for hit in hits {
            let (line, col) = self.line_col_at(hit);
            self.diagnostics.push(
                Diagnostic::info(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-026",
                    t!("rules.as_026.message"),
                )
                .with_suggestion(t!("rules.as_026.suggestion")),
            );
        }
    }

    /// AS-024: a package-runner command in SKILL.md code without a pinned
    /// version ("Pin versions (e.g., `npx eslint@9.0.0`)"). Packages the
    /// skill's or the project's package.json depends on run the local copy
    /// and are skipped; so is `go run` with `@latest`, which the page accepts.
    fn validate_unpinned_runners(&mut self) {
        if !self.config.is_rule_enabled("AS-024") {
            return;
        }
        let (body_start, body) = self.body();
        let code = code_only(body);
        let mut local: HashSet<String> = HashSet::new();
        let fs = self.config.fs();
        let mut dirs: Vec<&Path> = Vec::new();
        if let Some(dir) = self.path.parent() {
            local.extend(package_json_dependencies(dir, fs.as_ref()));
            dirs.push(dir);
        }
        if let Some(root) = self.config.root_dir() {
            local.extend(package_json_dependencies(root, fs.as_ref()));
            dirs.push(root.as_path());
        }
        static RUNNER: OnceLock<Regex> = OnceLock::new();
        let runner = regex(
            &RUNNER,
            r"\b(?:npx|bunx|pnpx|pnpm\s+dlx|yarn\s+dlx|uvx|uv\s+tool\s+run|pipx\s+run|go\s+run)\s[^\n`|;&]*",
        );
        let mut hits = Vec::new();
        for m in runner.find_iter(&code) {
            let argv: Vec<String> = m.as_str().split_whitespace().map(str::to_string).collect();
            let package = if argv.first().is_some_and(|a| a == "go") {
                // `go run module/path` without `@version`; `.` and `./x` are local.
                argv.iter()
                    .skip(2)
                    .find(|a| !a.starts_with('-'))
                    .filter(|module| {
                        module.contains('/')
                            && !module.starts_with('.')
                            && !module.starts_with('/')
                            && !module.contains('@')
                    })
                    .cloned()
            } else {
                crate::rules::mcp_servers::unpinned_package(&argv)
                    .map(|found| found.package)
                    .filter(|package| !is_local_npm_package(package, &local, &dirs, fs.as_ref()))
            };
            if let Some(package) = package {
                hits.push((body_start + m.start(), package));
            }
        }
        for (offset, package) in hits {
            let (line, col) = self.line_col_at(offset);
            self.diagnostics.push(
                Diagnostic::info(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-024",
                    t!("rules.as_024.message", package = package.as_str()),
                )
                .with_suggestion(t!("rules.as_024.suggestion")),
            );
        }
    }

    /// XP-SK-004: a Windows-style path into a bundled directory in SKILL.md.
    /// Fences marked as PowerShell or batch show Windows commands and are
    /// skipped.
    fn validate_backslash_paths(&mut self) {
        if !self.config.is_rule_enabled("XP-SK-004") {
            return;
        }
        let (body_start, body) = self.body();
        let mut hits = Vec::new();
        let mut offset = 0usize;
        let mut windows_fence = false;
        let mut in_fence = false;
        for line in body.split_inclusive('\n') {
            let text = line.trim_end_matches(['\n', '\r']);
            let trimmed = text.trim_start();
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                if in_fence {
                    in_fence = false;
                    windows_fence = false;
                } else {
                    in_fence = true;
                    let info = trimmed
                        .trim_start_matches(['`', '~'])
                        .trim()
                        .to_ascii_lowercase();
                    windows_fence = matches!(
                        info.as_str(),
                        "powershell" | "ps1" | "pwsh" | "bat" | "batch" | "cmd" | "dos"
                    );
                }
            } else if !windows_fence {
                for m in backslash_path_pattern().find_iter(text) {
                    hits.push((body_start + offset + m.start(), m.as_str().to_string()));
                }
            }
            offset += line.len();
        }
        for (hit, path) in hits {
            let (line, col) = self.line_col_at(hit);
            self.diagnostics.push(
                Diagnostic::info(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "XP-SK-004",
                    t!("rules.xp_sk_004.message", path = path.as_str()),
                )
                .with_suggestion(t!(
                    "rules.xp_sk_004.suggestion",
                    fixed = path.replace('\\', "/")
                )),
            );
        }
    }

    /// Files inside the skill directory, skipping hidden directories,
    /// `node_modules` and excluded paths. Empty when the skill is not on disk.
    fn bundled_files(&self) -> Vec<PathBuf> {
        let Some(dir) = self.path.parent() else {
            return Vec::new();
        };
        let fs = self.config.fs();
        if !fs.is_file(self.path) {
            return Vec::new();
        }
        let patterns = self
            .config
            .exclude()
            .iter()
            .chain(self.config.files_config().exclude.iter())
            .filter_map(|pattern| crate::pipeline::compile_single_exclude_pattern(pattern).ok())
            .collect::<Vec<_>>();
        let root = self.config.root_dir().map(PathBuf::as_path).unwrap_or(dir);
        let mut stack = vec![dir.to_path_buf()];
        let mut files = Vec::new();
        while let Some(current) = stack.pop() {
            let Ok(entries) = fs.read_dir(&current) else {
                continue;
            };
            for entry in entries {
                if entry.metadata.is_symlink {
                    continue;
                }
                let relative = crate::pipeline::normalize_rel_path(&entry.path, root);
                if entry.metadata.is_dir {
                    let hidden = entry
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|s| s.starts_with('.') || s == "node_modules");
                    if !hidden && !crate::pipeline::should_prune_dir(&relative, &patterns) {
                        stack.push(entry.path.clone());
                    }
                } else if entry.metadata.is_file
                    && !crate::pipeline::is_excluded_file(&relative, &patterns)
                {
                    files.push(entry.path.clone());
                }
            }
        }
        files.sort();
        files
    }

    /// AS-022 (interactive scripts), AS-023 (bundled files SKILL.md never
    /// mentions) and XP-SK-003 (secrets) over the bundled files.
    fn validate_bundled_files(&mut self) {
        let interactive = self.config.is_rule_enabled("AS-022");
        let unreferenced = self.config.is_rule_enabled("AS-023");
        let secrets = self.config.is_rule_enabled("XP-SK-003");
        if !interactive && !unreferenced && !secrets {
            return;
        }
        let Some(dir) = self.path.parent().map(Path::to_path_buf) else {
            return;
        };

        // XP-SK-003 in SKILL.md itself.
        if secrets {
            let content = self.content;
            self.report_secrets(self.path.to_path_buf(), content);
        }

        let files = self.bundled_files();
        if files.is_empty() {
            return;
        }
        let fs = self.config.fs();
        let mut texts: Vec<(PathBuf, String)> = Vec::new();
        for file in &files {
            if file == self.path {
                continue;
            }
            let text_like = file.extension().and_then(|e| e.to_str()).is_none_or(|ext| {
                let ext = ext.to_ascii_lowercase();
                ext == "md"
                    || ext == "txt"
                    || ext == "json"
                    || ext == "yaml"
                    || ext == "yml"
                    || ext == "toml"
                    || ext == "env"
                    || SCRIPT_EXTENSIONS.contains(&ext.as_str())
            });
            if !text_like {
                continue;
            }
            if let Ok(text) = fs.read_to_string(file) {
                texts.push((file.clone(), text));
            }
        }

        for (file, text) in &texts {
            let relative = file.strip_prefix(&dir).unwrap_or(file);
            let in_scripts = relative
                .components()
                .next()
                .is_some_and(|c| c.as_os_str() == "scripts");
            if interactive && in_scripts && is_script(file, text) {
                self.report_interactive(file, text);
            }
            if secrets {
                self.report_secrets(file.clone(), text);
            }
        }

        if unreferenced {
            self.report_unreferenced(&dir, &files, &texts);
        }
    }

    fn report_secrets(&mut self, file: PathBuf, text: &str) {
        let line_starts = compute_line_starts(text);
        let hits: Vec<usize> = secret_pattern()
            .find_iter(text)
            .map(|m| m.start())
            .collect();
        for hit in hits {
            let (line, col) = line_col_at(hit, &line_starts);
            self.diagnostics.push(
                Diagnostic::warning(
                    file.clone(),
                    line,
                    col,
                    "XP-SK-003",
                    t!("rules.xp_sk_003.message"),
                )
                .with_suggestion(t!("rules.xp_sk_003.suggestion")),
            );
        }
    }

    fn report_interactive(&mut self, file: &Path, text: &str) {
        let lines: Vec<&str> = text.lines().collect();
        let mut hits = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let code = line.trim_start();
            if code.starts_with('#') || code.starts_with("//") {
                continue;
            }
            let guarded =
                is_tty_guard(line) || idx.checked_sub(1).is_some_and(|i| is_tty_guard(lines[i]));
            if guarded {
                continue;
            }
            if let Some((_, label)) = interactive_patterns()
                .iter()
                .find(|(pattern, _)| pattern.is_match(line))
            {
                hits.push((idx + 1, *label));
            }
        }
        for (line, label) in hits {
            self.diagnostics.push(
                Diagnostic::warning(
                    file.to_path_buf(),
                    line,
                    1,
                    "AS-022",
                    t!(
                        "rules.as_022.message",
                        script = file
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or_default(),
                        pattern = label
                    ),
                )
                .with_suggestion(t!("rules.as_022.suggestion")),
            );
        }
    }

    /// AS-023: a file directly under `scripts/` or `references/` that neither
    /// SKILL.md nor another bundled Markdown or script file mentions by path
    /// or name.
    fn report_unreferenced(&mut self, dir: &Path, files: &[PathBuf], texts: &[(PathBuf, String)]) {
        let mut mentions = String::from(self.content);
        for (_, text) in texts {
            mentions.push('\n');
            mentions.push_str(text);
        }
        let mut reported = Vec::new();
        for file in files {
            let Ok(relative) = file.strip_prefix(dir) else {
                continue;
            };
            let parts: Vec<&str> = relative
                .components()
                .filter_map(|c| c.as_os_str().to_str())
                .collect();
            let [folder, name] = parts.as_slice() else {
                continue;
            };
            if !matches!(*folder, "scripts" | "references") {
                continue;
            }
            if name.starts_with('_')
                || name.starts_with('.')
                || name.eq_ignore_ascii_case("README.md")
            {
                continue;
            }
            let stem = Path::new(name)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(name);
            // Other files mention it by name, or import it as a module.
            let own_text = texts
                .iter()
                .find(|(path, _)| path == file)
                .map(|(_, text)| text.as_str())
                .unwrap_or_default();
            let mentioned_elsewhere = |needle: &str| {
                let total = mentions.matches(needle).count();
                let own = own_text.matches(needle).count();
                total > own
            };
            if mentioned_elsewhere(name) || (stem.len() >= 3 && mentioned_elsewhere(stem)) {
                continue;
            }
            reported.push(format!("{folder}/{name}"));
        }
        let (line, col) = self.line_col_at(self.parts.body_start.min(self.content.len()));
        for path in reported {
            self.diagnostics.push(
                Diagnostic::info(
                    self.path.to_path_buf(),
                    line,
                    col,
                    "AS-023",
                    t!("rules.as_023.message", path = path.as_str()),
                )
                .with_suggestion(t!("rules.as_023.suggestion", path = path.as_str())),
            );
        }
    }
}

/// Name of a skill: frontmatter `name`, else its directory name (AS-025).
pub(crate) fn skill_name(skill_md: &Path, content: &str) -> Option<String> {
    let parts = split_frontmatter(content);
    let declared = (parts.has_frontmatter && parts.has_closing)
        .then(|| serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter).ok())
        .flatten()
        .and_then(|yaml| {
            yaml.get("name")
                .and_then(|n| n.as_str())
                .map(|n| n.trim().to_string())
        })
        .filter(|n| !n.is_empty());
    declared.or_else(|| {
        skill_md
            .parent()
            .and_then(Path::file_name)
            .and_then(|n| n.to_str())
            .map(str::to_string)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_reference_strips_prefixes() {
        assert_eq!(normalize_reference("./references/a.md"), "references/a.md");
        assert_eq!(
            normalize_reference("${CLAUDE_SKILL_DIR}/scripts/x.py"),
            "scripts/x.py"
        );
        assert_eq!(
            normalize_reference("references/sub/../b.md"),
            "references/b.md"
        );
        assert_eq!(normalize_reference("references\\a.md"), "references/a.md");
    }

    #[test]
    fn code_only_keeps_code() {
        let text = "Run `npx foo` then\n```\nuvx bar\n```\ndone";
        let code = code_only(text);
        assert!(code.contains("npx foo"));
        assert!(code.contains("uvx bar"));
        assert!(!code.contains("Run"));
        assert!(!code.contains("done"));
        assert_eq!(code.len(), text.len());
    }

    #[test]
    fn trigger_markers() {
        for text in [
            "Use when reviewing pull requests",
            "Formats code. Use this skill for Rust files.",
            "Invoked whenever the user asks about deploys",
            "Generates release notes when a tag is pushed",
        ] {
            assert!(trigger_pattern().is_match(text), "{text}");
        }
        for text in [
            "Helps with PDFs.",
            "Release note generator",
            "Formats Rust code",
        ] {
            assert!(!trigger_pattern().is_match(text), "{text}");
        }
    }

    #[test]
    fn generic_pointer() {
        assert!(generic_pointer_pattern().is_match("See references/ for details."));
        assert!(generic_pointer_pattern().is_match("see the docs folder"));
        assert!(
            !generic_pointer_pattern().is_match("See references/api-errors.md for status codes.")
        );
    }

    #[test]
    fn secrets_and_backslashes() {
        assert!(secret_pattern().is_match("token: ghp_abcdefghijklmnopqrstuvwxyz0123456789"));
        assert!(secret_pattern().is_match("-----BEGIN OPENSSH PRIVATE KEY-----"));
        assert!(!secret_pattern().is_match("token: ${GITHUB_TOKEN}"));
        assert!(backslash_path_pattern().is_match("run scripts\\setup.py"));
        assert!(!backslash_path_pattern().is_match("my\\_file.md"));
    }

    #[test]
    fn interactive_input_patterns() {
        let hit = |line: &str| interactive_patterns().iter().any(|(p, _)| p.is_match(line));
        assert!(hit("read -p \"Continue? \" answer"));
        assert!(hit("name = input(\"Name: \")"));
        assert!(hit("pw = getpass.getpass()"));
        assert!(hit("$x = Read-Host 'Value'"));
        assert!(!hit("read line"));
        assert!(!hit("data = sys.stdin.read()"));
        assert!(!hit("value = input()"));
    }
}
