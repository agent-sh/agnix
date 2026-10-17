//! Codex CLI command rules, custom prompts and skill files.
//!
//! - CDX-RL-001, CDX-RL-002: `.rules` files (Starlark `prefix_rule(...)`
//!   calls) in a config layer's `rules/` folder.
//! - CDX-DEP-001: custom prompts in `~/.codex/prompts/`, deprecated in favor
//!   of skills.
//! - CX-SK-004: the repository skill catalog against Codex's initial skill
//!   list budget.
//! - CX-SK-006: `agents/openai.yaml` interface fields Codex ignores.
//! - CX-SK-007: auxiliary documentation inside a skill folder.

use crate::{
    config::PerFileLintConfig,
    diagnostics::Diagnostic,
    parsers::frontmatter::split_frontmatter,
    rules::config_span::{line_col, yaml_key_pos},
    rules::per_client_skill::{SkillClient, resolve_skill_client, shared_scope_includes},
    rules::{Validator, ValidatorMetadata},
};
use rust_i18n::t;
use std::path::{Component, Path, PathBuf};

// ============================================================================
// Shared helpers
// ============================================================================

/// The user's Codex home (`$CODEX_HOME`, else `~/.codex`), if known.
pub(crate) fn codex_home() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(home));
    }
    user_home().map(|home| home.join(".codex"))
}

#[cfg(feature = "filesystem")]
fn user_home() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(not(feature = "filesystem"))]
fn user_home() -> Option<PathBuf> {
    None
}

/// Whether `dir` is the user's Codex home directory. Symlinks resolve
/// through `fs`, so a mock or LSP file system answers the same way the rest
/// of the pipeline sees the tree.
pub(crate) fn is_codex_home(dir: &Path, fs: &dyn crate::fs::FileSystem) -> bool {
    codex_home().is_some_and(|home| same_dir(dir, &home, fs))
}

fn same_dir(a: &Path, b: &Path, fs: &dyn crate::fs::FileSystem) -> bool {
    a == b
        || match (fs.canonicalize(a), fs.canonicalize(b)) {
            (Ok(a), Ok(b)) => a == b,
            _ => false,
        }
}

// ============================================================================
// CDX-RL-001 / CDX-RL-002: .rules files
// ============================================================================

const RULES_RULE_IDS: &[&str] = &["CDX-RL-001", "CDX-RL-002"];

/// Commands that run arbitrary code (or reach the network or delete files)
/// when allowed outside the sandbox with no further prefix. This list is an
/// agnix heuristic; Codex documents only "prefer narrow rules over broad
/// access changes".
const BROAD_COMMANDS: &[&str] = &[
    "bash",
    "sh",
    "zsh",
    "fish",
    "dash",
    "ksh",
    "pwsh",
    "powershell",
    "python",
    "python3",
    "node",
    "deno",
    "bun",
    "ruby",
    "perl",
    "php",
    "sudo",
    "doas",
    "env",
    "xargs",
    "npx",
    "uvx",
    "curl",
    "wget",
    "rm",
];

/// Flags that turn a shell or interpreter into "run this code string".
const CODE_FLAGS: &[&str] = &["-c", "-lc", "-ic", "-e", "--eval", "-Command"];

#[derive(Debug, Clone, PartialEq)]
enum StarValue {
    Str(String),
    List(Vec<(usize, StarValue)>),
    Other,
}

#[derive(Debug)]
struct StarArg {
    name: String,
    name_offset: usize,
    value: StarValue,
    value_offset: usize,
}

#[derive(Debug)]
struct PrefixRuleCall {
    offset: usize,
    args: Vec<StarArg>,
}

impl PrefixRuleCall {
    fn arg(&self, name: &str) -> Option<&StarArg> {
        self.args.iter().rev().find(|arg| arg.name == name)
    }
}

/// A small reader for the Starlark subset that literal `prefix_rule(...)`
/// calls use: keyword arguments whose values are strings or lists of strings.
/// Anything else (variables, loops, function calls in values) is read as
/// `Other` and never reported.
struct StarReader<'a> {
    src: &'a str,
    pos: usize,
}

impl<'a> StarReader<'a> {
    fn bytes(&self) -> &'a [u8] {
        self.src.as_bytes()
    }

    fn skip_trivia(&mut self) {
        while let Some(&byte) = self.bytes().get(self.pos) {
            match byte {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                b'\\' if self.bytes().get(self.pos + 1) == Some(&b'\n') => self.pos += 2,
                b'#' => {
                    while self.bytes().get(self.pos).is_some_and(|b| *b != b'\n') {
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        let quote = *self.bytes().get(self.pos)?;
        if quote != b'"' && quote != b'\'' {
            return None;
        }
        let triple = self.src[self.pos..].starts_with(if quote == b'"' { "\"\"\"" } else { "'''" });
        let delimiter_len = if triple { 3 } else { 1 };
        self.pos += delimiter_len;
        let mut out = String::new();
        loop {
            let rest = &self.src[self.pos..];
            let ch = rest.chars().next()?;
            if triple {
                if rest.starts_with(if quote == b'"' { "\"\"\"" } else { "'''" }) {
                    self.pos += 3;
                    return Some(out);
                }
            } else if ch as u32 == u32::from(quote) {
                self.pos += 1;
                return Some(out);
            } else if ch == '\n' {
                return None;
            }
            if ch == '\\' {
                self.pos += 1;
                let escaped = self.src[self.pos..].chars().next()?;
                self.pos += escaped.len_utf8();
                out.push(match escaped {
                    'n' => '\n',
                    't' => '\t',
                    other => other,
                });
                continue;
            }
            self.pos += ch.len_utf8();
            out.push(ch);
        }
    }

    fn value(&mut self, depth: usize) -> StarValue {
        self.skip_trivia();
        match self.bytes().get(self.pos) {
            Some(b'"' | b'\'') => match self.string() {
                Some(value) => StarValue::Str(value),
                None => StarValue::Other,
            },
            Some(b'[') if depth < 8 => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_trivia();
                    match self.bytes().get(self.pos) {
                        Some(b']') => {
                            self.pos += 1;
                            return StarValue::List(items);
                        }
                        Some(b',') => self.pos += 1,
                        Some(_) => {
                            let start = self.pos;
                            let item = self.value(depth + 1);
                            if self.pos == start {
                                return StarValue::Other;
                            }
                            items.push((start, item));
                        }
                        None => return StarValue::Other,
                    }
                }
            }
            _ => {
                // Skip one expression: up to a top-level `,` or `)`.
                let mut nesting = 0usize;
                while let Some(&byte) = self.bytes().get(self.pos) {
                    match byte {
                        b'(' | b'[' | b'{' => nesting += 1,
                        b')' | b']' | b'}' if nesting == 0 => break,
                        b')' | b']' | b'}' => nesting -= 1,
                        b',' if nesting == 0 => break,
                        b'"' | b'\'' => {
                            if self.string().is_none() {
                                self.pos += 1;
                            }
                            continue;
                        }
                        b'#' => {
                            self.skip_trivia();
                            continue;
                        }
                        _ => {}
                    }
                    self.pos += 1;
                }
                StarValue::Other
            }
        }
    }

    fn ident(&mut self) -> Option<(usize, &'a str)> {
        let start = self.pos;
        while self
            .bytes()
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            self.pos += 1;
        }
        (self.pos > start).then(|| (start, &self.src[start..self.pos]))
    }

    /// All literal `prefix_rule(...)` calls, outside strings and comments.
    fn prefix_rules(mut self) -> Vec<PrefixRuleCall> {
        let mut calls = Vec::new();
        while self.pos < self.src.len() {
            self.skip_trivia();
            let Some(&byte) = self.bytes().get(self.pos) else {
                break;
            };
            if byte == b'"' || byte == b'\'' {
                if self.string().is_none() {
                    self.pos += 1;
                }
                continue;
            }
            if !(byte.is_ascii_alphabetic() || byte == b'_') {
                self.pos += self.src[self.pos..]
                    .chars()
                    .next()
                    .map_or(1, char::len_utf8);
                continue;
            }
            let Some((offset, name)) = self.ident() else {
                self.pos += 1;
                continue;
            };
            self.skip_trivia();
            if name != "prefix_rule" || self.bytes().get(self.pos) != Some(&b'(') {
                continue;
            }
            self.pos += 1;
            let mut args = Vec::new();
            loop {
                self.skip_trivia();
                match self.bytes().get(self.pos) {
                    Some(b')') => {
                        self.pos += 1;
                        break;
                    }
                    Some(b',') => self.pos += 1,
                    Some(_) => {
                        let Some((name_offset, arg_name)) = self.ident() else {
                            // Positional or unreadable argument: skip it.
                            let before = self.pos;
                            self.value(0);
                            if self.pos == before {
                                self.pos += 1;
                            }
                            continue;
                        };
                        self.skip_trivia();
                        if self.bytes().get(self.pos) != Some(&b'=') {
                            continue;
                        }
                        self.pos += 1;
                        self.skip_trivia();
                        let value_offset = self.pos;
                        let value = self.value(0);
                        args.push(StarArg {
                            name: arg_name.to_string(),
                            name_offset,
                            value,
                            value_offset,
                        });
                    }
                    None => break,
                }
            }
            calls.push(PrefixRuleCall { offset, args });
        }
        calls
    }
}

fn first_token_alternatives(value: &StarValue) -> Vec<&str> {
    match value {
        StarValue::Str(token) => vec![token.as_str()],
        StarValue::List(items) => items
            .iter()
            .filter_map(|(_, item)| match item {
                StarValue::Str(token) => Some(token.as_str()),
                _ => None,
            })
            .collect(),
        StarValue::Other => Vec::new(),
    }
}

/// The broad command a pattern allows, if it is one token from
/// `BROAD_COMMANDS` or such a command followed only by a code flag.
fn broad_command(pattern: &StarValue) -> Option<String> {
    let StarValue::List(tokens) = pattern else {
        return None;
    };
    let first = tokens.first()?;
    let broad: Vec<&str> = first_token_alternatives(&first.1)
        .into_iter()
        .filter(|token| {
            let name = Path::new(token)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(token);
            BROAD_COMMANDS.contains(&name)
        })
        .collect();
    let command = *broad.first()?;
    match tokens.len() {
        1 => Some(command.to_string()),
        2 => {
            let flags = first_token_alternatives(&tokens[1].1);
            flags
                .iter()
                .any(|flag| CODE_FLAGS.contains(flag))
                .then(|| format!("{command} {}", flags[0]))
        }
        _ => None,
    }
}

pub struct CodexRulesValidator;

impl Validator for CodexRulesValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULES_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let rl_001 = config.is_rule_enabled("CDX-RL-001");
        let rl_002 = config.is_rule_enabled("CDX-RL-002");
        if !rl_001 && !rl_002 {
            return diagnostics;
        }
        let reader = StarReader {
            src: content,
            pos: 0,
        };
        for call in reader.prefix_rules() {
            let decision = call.arg("decision");
            let decision_value = match decision.map(|arg| &arg.value) {
                None => Some("allow"),
                Some(StarValue::Str(value)) => Some(value.as_str()),
                Some(_) => None,
            };
            if rl_001 && decision_value == Some("forbidden") {
                let justified = call
                    .arg("justification")
                    .is_some_and(|arg| match &arg.value {
                        StarValue::Str(text) => !text.trim().is_empty(),
                        _ => true,
                    });
                if !justified {
                    let offset = decision.map_or(call.offset, |arg| arg.name_offset);
                    let (line, column) = line_col(content, offset);
                    diagnostics.push(
                        Diagnostic::info(
                            path.to_path_buf(),
                            line,
                            column,
                            "CDX-RL-001",
                            t!("rules.cdx_rl_001.message"),
                        )
                        .with_suggestion(t!("rules.cdx_rl_001.suggestion")),
                    );
                }
            }
            if rl_002
                && decision_value == Some("allow")
                && let Some(pattern) = call.arg("pattern")
                && let Some(command) = broad_command(&pattern.value)
            {
                let (line, column) = line_col(content, pattern.value_offset);
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        column,
                        "CDX-RL-002",
                        t!("rules.cdx_rl_002.message", command = command.as_str()),
                    )
                    .with_suggestion(t!("rules.cdx_rl_002.suggestion")),
                );
            }
        }
        diagnostics
    }
}

// ============================================================================
// CDX-DEP-001: custom prompts
// ============================================================================

pub struct CodexPromptValidator;

impl Validator for CodexPromptValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: &["CDX-DEP-001"],
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        _content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        if !config.is_rule_enabled("CDX-DEP-001") {
            return Vec::new();
        }
        // Path components from `.codex` on: `.codex/prompts/<file>` is
        // top-level, anything deeper is never scanned.
        let components: Vec<&str> = path
            .components()
            .filter_map(|c| match c {
                Component::Normal(name) => name.to_str(),
                _ => None,
            })
            .collect();
        let Some(index) = components
            .windows(2)
            .rposition(|pair| pair[0] == ".codex" && pair[1] == "prompts")
        else {
            return Vec::new();
        };
        let nested = components.len() > index + 3;
        let dot_codex: PathBuf = path
            .ancestors()
            .find(|dir| dir.file_name().and_then(|n| n.to_str()) == Some(".codex"))
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let in_home = is_codex_home(&dot_codex, config.fs().as_ref());
        let message = if nested {
            t!("rules.cdx_dep_001.nested")
        } else if !in_home && !dot_codex.as_os_str().is_empty() && codex_home().is_some() {
            t!("rules.cdx_dep_001.not_home")
        } else {
            t!("rules.cdx_dep_001.message")
        };
        let level: fn(PathBuf, usize, usize, &str, String) -> Diagnostic = if nested {
            Diagnostic::warning
        } else {
            Diagnostic::info
        };
        vec![
            level(path.to_path_buf(), 1, 1, "CDX-DEP-001", message.to_string())
                .with_suggestion(t!("rules.cdx_dep_001.suggestion")),
        ]
    }
}

// ============================================================================
// CX-SK-004 / CX-SK-006 / CX-SK-007: Codex skills
// ============================================================================

const SKILL_RULE_IDS: &[&str] = &["CX-SK-004", "CX-SK-006", "CX-SK-007"];

/// "By default, this list uses at most 2% of the model's context window, or
/// 8,000 characters when the context window is unknown."
const CATALOG_FALLBACK_CHARS: usize = 8_000;
/// Codex scans skill roots up to this depth (`MAX_SCAN_DEPTH`).
const MAX_SKILL_SCAN_DEPTH: usize = 6;

/// Auxiliary documentation the skill-creator guidance says to leave out.
const AUXILIARY_DOCS: &[&str] = &[
    "readme.md",
    "readme",
    "readme.txt",
    "changelog.md",
    "changes.md",
    "install.md",
    "installation.md",
];

pub struct CodexSkillValidator;

impl Validator for CodexSkillValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: SKILL_RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if filename == "openai.yaml" {
            return if config.is_rule_enabled("CX-SK-006") {
                validate_openai_yaml(path, content, config)
            } else {
                Vec::new()
            };
        }
        // `.agents/skills` is shared: these Codex notes apply whenever Codex
        // is one of the readers in scope, not only when it is the only one.
        let codex_reads = match resolve_skill_client(path, config) {
            SkillClient::Codex => true,
            SkillClient::Shared => shared_scope_includes(config, SkillClient::Codex),
            _ => false,
        };
        if filename != "SKILL.md" || !codex_reads {
            return Vec::new();
        }
        let mut diagnostics = Vec::new();
        if config.is_rule_enabled("CX-SK-007") {
            auxiliary_docs(path, config, &mut diagnostics);
        }
        if config.is_rule_enabled("CX-SK-004") {
            catalog_budget(path, config, &mut diagnostics);
        }
        diagnostics
    }
}

fn auxiliary_docs(path: &Path, config: &PerFileLintConfig<'_>, out: &mut Vec<Diagnostic>) {
    let Some(dir) = path.parent() else {
        return;
    };
    let Ok(entries) = config.fs().read_dir(dir) else {
        return;
    };
    let mut found: Vec<String> = entries
        .iter()
        .filter_map(|entry| entry.path.file_name()?.to_str().map(str::to_string))
        .filter(|name| AUXILIARY_DOCS.contains(&name.to_ascii_lowercase().as_str()))
        .collect();
    found.sort();
    if found.is_empty() {
        return;
    }
    out.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            1,
            "CX-SK-007",
            t!("rules.cx_sk_007.message", files = found.join(", ")),
        )
        .with_suggestion(t!("rules.cx_sk_007.suggestion")),
    );
}

/// The `skills` directory that roots a Codex skill (`.agents/skills`,
/// `.codex/skills`).
fn skills_root(path: &Path) -> Option<&Path> {
    path.ancestors().skip(1).find(|dir| {
        dir.file_name().and_then(|n| n.to_str()) == Some("skills")
            && dir
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                .is_some_and(|owner| owner == ".agents" || owner == ".codex")
    })
}

fn collect_skill_files(
    dir: &Path,
    depth: usize,
    config: &PerFileLintConfig<'_>,
    out: &mut Vec<PathBuf>,
) {
    if depth > MAX_SKILL_SCAN_DEPTH || out.len() > 2_000 {
        return;
    }
    let Ok(mut entries) = config.fs().read_dir(dir) else {
        return;
    };
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    for entry in entries {
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if name == "SKILL.md" && depth > 0 {
            out.push(entry.path.clone());
            // A skill directory is a leaf: Codex does not look for skills
            // inside another skill.
            return;
        }
    }
    let Ok(mut entries) = config.fs().read_dir(dir) else {
        return;
    };
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    for entry in entries {
        let name = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if matches!(name, "node_modules" | ".git") {
            continue;
        }
        if config.fs().is_dir(&entry.path) {
            collect_skill_files(&entry.path, depth + 1, config, out);
        }
    }
}

fn skill_metadata_len(content: &str) -> usize {
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return 0;
    }
    let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter) else {
        return 0;
    };
    ["name", "description"]
        .iter()
        .filter_map(|key| yaml.get(*key).and_then(serde_yaml::Value::as_str))
        .map(|text| {
            text.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .count()
        })
        .sum()
}

fn catalog_budget(path: &Path, config: &PerFileLintConfig<'_>, out: &mut Vec<Diagnostic>) {
    let Some(root) = skills_root(path) else {
        return;
    };
    let mut files = Vec::new();
    collect_skill_files(root, 0, config, &mut files);
    // Report once per root, on its first skill.
    if files.first().map(PathBuf::as_path) != Some(path) {
        return;
    }
    let display_base = root.parent().and_then(Path::parent).unwrap_or(root);
    let total: usize = files
        .iter()
        .map(|file| {
            let shown = file.strip_prefix(display_base).unwrap_or(file);
            let metadata = config
                .fs()
                .read_to_string(file)
                .map(|text| skill_metadata_len(&text))
                .unwrap_or(0);
            metadata + shown.to_string_lossy().chars().count()
        })
        .sum();
    if total <= CATALOG_FALLBACK_CHARS {
        return;
    }
    out.push(
        Diagnostic::info(
            path.to_path_buf(),
            1,
            1,
            "CX-SK-004",
            t!(
                "rules.cx_sk_004.message",
                count = files.len(),
                chars = total,
                limit = CATALOG_FALLBACK_CHARS
            ),
        )
        .with_suggestion(t!("rules.cx_sk_004.suggestion")),
    );
}

/// Whether a skill directory is part of a plugin (Codex lets icon paths with
/// `..` resolve into the plugin's shared `assets/`).
fn plugin_root_for(skill_dir: &Path, config: &PerFileLintConfig<'_>) -> Option<PathBuf> {
    skill_dir.ancestors().skip(1).find_map(|dir| {
        let fs = config.fs();
        (fs.is_file(&dir.join(".codex-plugin").join("plugin.json"))
            || fs.is_file(&dir.join(".claude-plugin").join("plugin.json"))
            || fs.is_file(&dir.join("plugin.json")))
        .then(|| dir.to_path_buf())
    })
}

fn lexically_normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Why Codex ignores an icon path, mirroring `resolve_asset_path`.
fn icon_problem(
    value: &str,
    skill_dir: &Path,
    config: &PerFileLintConfig<'_>,
) -> Option<&'static str> {
    if value.is_empty() {
        return None;
    }
    let path = Path::new(value);
    if path.is_absolute() || value.starts_with('/') || value.starts_with('\\') {
        return Some("it must be a relative path under assets/");
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => normalized.push(part),
            Component::ParentDir => {
                let Some(plugin_root) = plugin_root_for(skill_dir, config) else {
                    return Some("'..' is allowed only to reach a plugin's shared assets/");
                };
                let assets = lexically_normalize(&plugin_root.join("assets"));
                let resolved = lexically_normalize(&skill_dir.join(path));
                return (!resolved.starts_with(&assets))
                    .then_some("a path with '..' must resolve under the plugin's assets/");
            }
            _ => return Some("it must be a relative path under assets/"),
        }
    }
    match normalized.components().next() {
        Some(Component::Normal(first)) if first == "assets" => None,
        _ => Some("it must be under the skill's assets/ directory"),
    }
}

/// 1-based line of a YAML key at the given nesting path, for example
/// `["interface", "display_name"]`. Block mappings only; flow mappings and
/// unusual layouts fall back to the caller's default.
fn yaml_key_line(content: &str, path: &[&str]) -> Option<usize> {
    yaml_key_pos(content, path).map(|(line, _)| line)
}

fn validate_openai_yaml(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let warn = |line: usize, message: String, diagnostics: &mut Vec<Diagnostic>| {
        diagnostics.push(
            Diagnostic::warning(path.to_path_buf(), line, 1, "CX-SK-006", message)
                .with_suggestion(t!("rules.cx_sk_006.suggestion")),
        );
    };
    if content.trim().is_empty() {
        return diagnostics;
    }
    let yaml: serde_yaml::Value = match serde_yaml::from_str(content) {
        Ok(yaml) => yaml,
        Err(error) => {
            let line = error.location().map_or(1, |loc| loc.line().max(1));
            warn(
                line,
                t!("rules.cx_sk_006.parse_error", error = error.to_string()).to_string(),
                &mut diagnostics,
            );
            return diagnostics;
        }
    };
    if yaml.is_null() {
        return diagnostics;
    }
    let Some(root) = yaml.as_mapping() else {
        warn(
            1,
            t!("rules.cx_sk_006.root_type").to_string(),
            &mut diagnostics,
        );
        return diagnostics;
    };
    let skill_dir = path
        .parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new("."));
    let get = |map: &serde_yaml::Mapping, key: &str| {
        map.get(serde_yaml::Value::String(key.into())).cloned()
    };

    // Whole-file failures first: a type Codex cannot deserialize drops every
    // field of the file.
    let mut type_errors = Vec::new();
    if let Some(interface) = get(root, "interface") {
        match interface.as_mapping() {
            Some(interface) => {
                for field in [
                    "display_name",
                    "short_description",
                    "icon_small",
                    "icon_large",
                    "brand_color",
                    "default_prompt",
                ] {
                    if let Some(value) = get(interface, field)
                        && !value.is_string()
                        && !value.is_null()
                    {
                        type_errors.push((vec!["interface", field], "a string"));
                    }
                }
            }
            None if !interface.is_null() => type_errors.push((vec!["interface"], "a mapping")),
            None => {}
        }
    }
    if let Some(policy) = get(root, "policy") {
        match policy.as_mapping() {
            Some(policy) => {
                if let Some(value) = get(policy, "allow_implicit_invocation")
                    && !value.is_bool()
                    && !value.is_null()
                {
                    type_errors.push((vec!["policy", "allow_implicit_invocation"], "a boolean"));
                }
            }
            None if !policy.is_null() => type_errors.push((vec!["policy"], "a mapping")),
            None => {}
        }
    }
    let mut dependency_tools = Vec::new();
    if let Some(dependencies) = get(root, "dependencies") {
        match dependencies.as_mapping() {
            Some(dependencies) => match get(dependencies, "tools") {
                Some(serde_yaml::Value::Sequence(tools)) => dependency_tools = tools,
                Some(serde_yaml::Value::Null) | None => {}
                Some(_) => type_errors.push((vec!["dependencies", "tools"], "a list")),
            },
            None if !dependencies.is_null() => {
                type_errors.push((vec!["dependencies"], "a mapping"));
            }
            None => {}
        }
    }
    if !type_errors.is_empty() {
        for (field_path, expected) in type_errors {
            let line = yaml_key_line(content, &field_path).unwrap_or(1);
            warn(
                line,
                t!(
                    "rules.cx_sk_006.type_error",
                    field = field_path.join("."),
                    expected = expected
                )
                .to_string(),
                &mut diagnostics,
            );
        }
        return diagnostics;
    }

    // Field-level failures: Codex drops the field and keeps the rest.
    if let Some(interface) = get(root, "interface").and_then(|v| v.as_mapping().cloned()) {
        for (field, max) in [
            ("display_name", 64usize),
            ("short_description", 1024),
            ("default_prompt", 1024),
        ] {
            let Some(value) = get(&interface, field).and_then(|v| v.as_str().map(str::to_string))
            else {
                continue;
            };
            let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
            let reason = if normalized.is_empty() {
                Some(t!("rules.cx_sk_006.empty").to_string())
            } else if normalized.chars().count() > max {
                Some(
                    t!(
                        "rules.cx_sk_006.too_long",
                        length = normalized.chars().count(),
                        max = max
                    )
                    .to_string(),
                )
            } else {
                None
            };
            if let Some(reason) = reason {
                let line = yaml_key_line(content, &["interface", field]).unwrap_or(1);
                warn(
                    line,
                    t!(
                        "rules.cx_sk_006.ignored",
                        field = format!("interface.{field}"),
                        reason = reason
                    )
                    .to_string(),
                    &mut diagnostics,
                );
            }
        }
        if let Some(color) =
            get(&interface, "brand_color").and_then(|v| v.as_str().map(str::to_string))
        {
            let color = color.trim();
            let valid = color.len() == 7
                && color.starts_with('#')
                && color[1..].chars().all(|c| c.is_ascii_hexdigit());
            if !valid {
                let line = yaml_key_line(content, &["interface", "brand_color"]).unwrap_or(1);
                warn(
                    line,
                    t!(
                        "rules.cx_sk_006.ignored",
                        field = "interface.brand_color",
                        reason = t!("rules.cx_sk_006.bad_color", value = color).to_string()
                    )
                    .to_string(),
                    &mut diagnostics,
                );
            }
        }
        for field in ["icon_small", "icon_large"] {
            let Some(value) = get(&interface, field).and_then(|v| v.as_str().map(str::to_string))
            else {
                continue;
            };
            if let Some(reason) = icon_problem(&value, skill_dir, config) {
                let line = yaml_key_line(content, &["interface", field]).unwrap_or(1);
                warn(
                    line,
                    t!(
                        "rules.cx_sk_006.ignored",
                        field = format!("interface.{field}"),
                        reason = reason
                    )
                    .to_string(),
                    &mut diagnostics,
                );
            }
        }
    }
    for (index, tool) in dependency_tools.iter().enumerate() {
        let missing: Vec<&str> = ["type", "value"]
            .into_iter()
            .filter(|key| {
                tool.get(*key)
                    .and_then(serde_yaml::Value::as_str)
                    .is_none_or(|text| text.split_whitespace().next().is_none())
            })
            .collect();
        if !missing.is_empty() {
            let line = yaml_key_line(content, &["dependencies", "tools"]).unwrap_or(1);
            warn(
                line,
                t!(
                    "rules.cx_sk_006.dependency",
                    index = index + 1,
                    fields = missing.join(" and ")
                )
                .to_string(),
                &mut diagnostics,
            );
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn rules_file(content: &str) -> Vec<Diagnostic> {
        CodexRulesValidator.validate(
            Path::new(".codex/rules/default.rules"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn documented_prefix_rule_is_clean() {
        let content = r#"# Prompt before running commands with the prefix `gh pr view` outside the sandbox.
prefix_rule(
    # The prefix to match.
    pattern = ["gh", "pr", "view"],
    decision = "prompt",
    justification = "Viewing PRs is allowed with approval",
    match = [
        "gh pr view 7888",
    ],
    not_match = [
        "gh pr --repo openai/codex view 7888",
    ],
)
"#;
        assert!(rules_file(content).is_empty());
    }

    #[test]
    fn forbidden_without_justification_and_broad_allows() {
        let content = r#"prefix_rule(pattern = ["rm", "-rf"], decision = "forbidden")
prefix_rule(pattern = ["git", "push"], decision = "forbidden", justification = "Ask a maintainer")
prefix_rule(pattern = ["bash"])
prefix_rule(pattern = [["python3", "node"], "-c"], decision = "allow")
prefix_rule(pattern = ["bash"], decision = "prompt")
prefix_rule(pattern = ["git", "status"])
x = "prefix_rule(pattern = ['sh'])"
"#;
        let diagnostics = rules_file(content);
        let summary: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.line))
            .collect();
        assert_eq!(
            summary,
            [("CDX-RL-001", 1), ("CDX-RL-002", 3), ("CDX-RL-002", 4)]
        );
        assert!(diagnostics[2].message.contains("python3 -c"));
    }

    #[test]
    fn prompt_files_are_deprecated_and_nested_ones_never_load() {
        let validate = |path: &str| {
            CodexPromptValidator.validate(Path::new(path), "Body", &LintConfig::default())
        };
        let top = validate("/x/.codex/prompts/draftpr.md");
        assert_eq!(top.len(), 1);
        assert_eq!(top[0].rule, "CDX-DEP-001");
        let nested = validate("/x/.codex/prompts/team/draftpr.md");
        assert_eq!(
            nested[0].level,
            crate::diagnostics::DiagnosticLevel::Warning
        );
    }

    #[test]
    fn same_dir_resolves_symlinks_through_the_injected_file_system() {
        // Neither path exists on disk; only the mock knows the symlink.
        let fs = MockFileSystem::new();
        fs.add_dir("/agnix-mock-home/.codex");
        fs.add_symlink("/agnix-mock-links/codex", "/agnix-mock-home/.codex");
        let link = Path::new("/agnix-mock-links/codex");
        let home = Path::new("/agnix-mock-home/.codex");
        assert!(same_dir(link, home, &fs));
        assert!(!same_dir(link, home, &crate::fs::RealFileSystem));
        assert!(!same_dir(Path::new("/agnix-mock-other"), home, &fs));
    }

    fn skill(name: &str, description_len: usize) -> String {
        format!(
            "---\nname: {name}\ndescription: {}\n---\nBody\n",
            "d".repeat(description_len)
        )
    }

    #[test]
    fn catalog_budget_reports_once_per_root() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_dir("/r/.agents/skills");
        for index in 0..12 {
            fs.add_dir(format!("/r/.agents/skills/s{index:02}"));
            fs.add_file(
                format!("/r/.agents/skills/s{index:02}/SKILL.md"),
                skill(&format!("s{index:02}"), 700),
            );
        }
        let mut config = LintConfig::default();
        config.set_fs(fs.clone());
        let first = CodexSkillValidator.validate(
            Path::new("/r/.agents/skills/s00/SKILL.md"),
            &skill("s00", 700),
            &config,
        );
        assert_eq!(first.len(), 1, "{first:?}");
        assert_eq!(first[0].rule, "CX-SK-004");
        let other = CodexSkillValidator.validate(
            Path::new("/r/.agents/skills/s05/SKILL.md"),
            &skill("s05", 700),
            &config,
        );
        assert!(other.is_empty());

        let small = Arc::new(MockFileSystem::new());
        small.add_dir("/r/.agents/skills");
        small.add_dir("/r/.agents/skills/a");
        small.add_file("/r/.agents/skills/a/SKILL.md", skill("a", 100));
        let mut config = LintConfig::default();
        config.set_fs(small);
        assert!(
            CodexSkillValidator
                .validate(
                    Path::new("/r/.agents/skills/a/SKILL.md"),
                    &skill("a", 100),
                    &config
                )
                .is_empty()
        );
    }

    #[test]
    fn auxiliary_docs_in_skill_folder() {
        let fs = Arc::new(MockFileSystem::new());
        fs.add_dir("/r/.agents/skills");
        fs.add_dir("/r/.agents/skills/a");
        fs.add_file("/r/.agents/skills/a/SKILL.md", skill("a", 10));
        fs.add_file("/r/.agents/skills/a/README.md", "x");
        fs.add_file("/r/.agents/skills/a/LICENSE.txt", "x");
        let mut config = LintConfig::default();
        config.set_fs(fs);
        let diagnostics = CodexSkillValidator.validate(
            Path::new("/r/.agents/skills/a/SKILL.md"),
            &skill("a", 10),
            &config,
        );
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].rule, "CX-SK-007");
        assert!(diagnostics[0].message.contains("README.md"));

        // `.agents/skills` is shared: a scope with Codex among its tools still
        // gets the note, a scope without Codex does not.
        for (tools, expected) in [(vec!["codex", "amp"], 1), (vec!["amp"], 0)] {
            config.set_tools(tools.iter().map(|t| t.to_string()).collect());
            let diagnostics = CodexSkillValidator.validate(
                Path::new("/r/.agents/skills/a/SKILL.md"),
                &skill("a", 10),
                &config,
            );
            assert_eq!(diagnostics.len(), expected, "{tools:?}: {diagnostics:?}");
        }
    }

    fn openai_yaml(content: &str) -> Vec<Diagnostic> {
        CodexSkillValidator.validate(
            Path::new("/r/.agents/skills/a/agents/openai.yaml"),
            content,
            &LintConfig::default(),
        )
    }

    #[test]
    fn openai_yaml_documented_example_is_clean() {
        let content = r##"interface:
  display_name: "Optional user-facing name"
  short_description: "Optional user-facing description"
  icon_small: "./assets/small-400px.png"
  icon_large: "./assets/large-logo.svg"
  brand_color: "#3B82F6"
  default_prompt: "Optional surrounding prompt to use the skill with"

dependencies:
  tools:
    - type: "mcp"
      value: "github"
      description: "GitHub MCP server"

policy:
  allow_implicit_invocation: true
"##;
        assert!(
            openai_yaml(content).is_empty(),
            "{:?}",
            openai_yaml(content)
        );
        assert!(openai_yaml("policy:\n  allow_implicit_invocation: false\n").is_empty());
        assert!(openai_yaml("").is_empty());
    }

    #[test]
    fn openai_yaml_ignored_fields() {
        let long = "x".repeat(65);
        let content = format!(
            "interface:\n  display_name: \"{long}\"\n  brand_color: \"blue\"\n  icon_small: \"/abs/icon.png\"\n  icon_large: \"icons/big.png\"\ndependencies:\n  tools:\n    - type: mcp\n"
        );
        let diagnostics = openai_yaml(&content);
        let lines: Vec<_> = diagnostics.iter().map(|d| d.line).collect();
        assert_eq!(lines, [2, 3, 4, 5, 7], "{diagnostics:?}");
        assert!(diagnostics.iter().all(|d| d.rule == "CX-SK-006"));
        let whole_file = openai_yaml("policy:\n  allow_implicit_invocation: \"false\"\n");
        assert_eq!(whole_file.len(), 1);
        assert!(whole_file[0].message.contains("allow_implicit_invocation"));
        assert_eq!(openai_yaml("interface: [").len(), 1);
    }
}
