//! Amp skill checks (AMP-005, AMP-008, AMP-010, AMP-SK-002, AMP-SK-003, and
//! AMP-006 on skill MCP servers).
//!
//! Runs on SKILL.md and on a skill's sibling `mcp.json`
//! (`FileType::AmpSkillMcp`) below Amp's local skill roots: `.agents/skills/`,
//! `.claude/skills/`, `~/.config/agents/skills/`, `~/.agents/skills/` and
//! `~/.config/amp/skills/` (ampcode.com/docs/customize/skills, "Skill
//! Sources and Precedence"). Other SKILL.md files return early. The skill
//! frontmatter field allowlist (AMP-SK-001) lives in `per_client_skill`.

use super::amp_mcp::{McpOrigin, Sink, check_servers, type_name, yaml_node};
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    file_types::{AmpSkillLocation, amp_skill_location},
    fs::FileSystem,
    parsers::frontmatter::{normalize_line_endings, split_frontmatter},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::{JsonNode, parse_json_document},
};
use rust_i18n::t;
use serde_yaml::Value as YamlValue;
use std::path::{Path, PathBuf};

/// AMP-006 is owned by `AmpValidator` and also emitted here.
const RULE_IDS: &[&str] = &["AMP-005", "AMP-008", "AMP-010", "AMP-SK-002", "AMP-SK-003"];

/// "Amp searches local skill directories recursively, up to five directories
/// below each skills root."
const MAX_SKILL_DEPTH: usize = 5;

pub struct AmpSkillValidator;

impl Validator for AmpSkillValidator {
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
        let Some(location) = amp_skill_location(path) else {
            return Vec::new();
        };
        let fs = config.fs();
        if location.is_claude_root() && claude_skills_disabled(location.root, fs.as_ref()) {
            return Vec::new();
        }
        match path.file_name().and_then(|name| name.to_str()) {
            Some("SKILL.md") => validate_skill_md(path, content, config, &location),
            Some("mcp.json") if location.depth == 1 => {
                validate_skill_mcp_json(path, content, config)
            }
            _ => Vec::new(),
        }
    }
}

/// `amp.skills.disableClaudeCodeSkills` is true in the workspace settings
/// next to a `.claude/skills/` root (or the user settings, for
/// `~/.claude/skills/`): "Disable loading skills from Claude Code directories
/// (`.claude/skills/`, `~/.claude/skills/`, and `~/.claude/plugins/cache/`)".
fn claude_skills_disabled(root: &Path, fs: &dyn FileSystem) -> bool {
    let Some(project) = root.parent().and_then(Path::parent) else {
        return false;
    };
    [
        project.join(".amp").join("settings.json"),
        project.join(".amp").join("settings.jsonc"),
        project.join(".config").join("amp").join("settings.json"),
        project.join(".config").join("amp").join("settings.jsonc"),
    ]
    .iter()
    .filter(|settings| fs.is_file(settings))
    .filter_map(|settings| fs.read_to_string(settings).ok())
    .filter_map(|text| parse_json_document(&text, true).ok())
    .any(|doc| {
        doc.root
            .value
            .get("amp.skills.disableClaudeCodeSkills")
            .and_then(|member| member.value.value.as_bool())
            == Some(true)
    })
}

/// SKILL.md frontmatter as a node tree, or `None` without valid mapping
/// frontmatter (other skill rules report those problems).
fn skill_frontmatter(content: &str) -> Option<JsonNode> {
    let parts = split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return None;
    }
    let yaml: YamlValue = serde_yaml::from_str(&parts.frontmatter).ok()?;
    if !yaml.is_mapping() {
        return None;
    }
    let start = parts.frontmatter_start;
    let range = start..start + parts.frontmatter.len();
    Some(yaml_node(content, &yaml, range, (start, start)))
}

/// The non-null `mcpServers` frontmatter value of a SKILL.md.
fn declares_mcp_servers(frontmatter: &JsonNode) -> bool {
    frontmatter
        .value
        .get("mcpServers")
        .is_some_and(|member| !member.value.value.is_null())
}

fn frontmatter_name(frontmatter: &JsonNode) -> Option<String> {
    frontmatter
        .value
        .get("name")
        .and_then(|member| member.value.value.as_str())
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
}

fn validate_skill_md(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
    location: &AmpSkillLocation<'_>,
) -> Vec<Diagnostic> {
    let mut sink = Sink::new(path, content, config);
    let fs = config.fs();

    if location.depth > MAX_SKILL_DEPTH {
        sink.push(
            DiagnosticLevel::Warning,
            "AMP-SK-003",
            (0, 0),
            t!(
                "rules.amp_sk_003.message",
                depth = location.depth,
                root = location.root.display().to_string()
            )
            .to_string(),
            t!("rules.amp_sk_003.suggestion").to_string(),
        );
    }

    let Some(frontmatter) = skill_frontmatter(content) else {
        return sink.diagnostics;
    };

    if let Some(servers) = frontmatter.value.get("mcpServers") {
        let span = (servers.key_start, servers.key_end);
        match servers.value.value.as_object() {
            Some(entries) => check_servers(&mut sink, entries, McpOrigin::SkillFrontmatter),
            None if servers.value.value.is_null() => {}
            None => sink.push(
                DiagnosticLevel::Warning,
                "AMP-010",
                span,
                t!(
                    "rules.amp_010.frontmatter_not_mapping",
                    found = type_name(&servers.value)
                )
                .to_string(),
                t!("rules.amp_010.suggestion").to_string(),
            ),
        }
        if declares_mcp_servers(&frontmatter)
            && path
                .parent()
                .is_some_and(|dir| fs.is_file(&dir.join("mcp.json")))
        {
            sink.push(
                DiagnosticLevel::Warning,
                "AMP-005",
                span,
                t!("rules.amp_005.message").to_string(),
                t!("rules.amp_005.suggestion").to_string(),
            );
        }
    }

    if location.is_claude_root() {
        check_masked_by_agents_copy(
            &mut sink,
            path,
            content,
            &frontmatter,
            location,
            fs.as_ref(),
        );
    }

    sink.diagnostics
}

/// AMP-SK-002: "Amp uses the first skill with a given frontmatter `name`",
/// and its order puts `.agents/skills/` (4) before `.claude/skills/` (5). A
/// `.claude/skills/<dir>/SKILL.md` whose `name` equals that of
/// `.agents/skills/<dir>/SKILL.md` next to it is masked. Byte-identical
/// copies are deliberate per-tool copies, and a `.claude` stub that points
/// at the `.agents` copy (`.agents/skills/<dir>/SKILL.md` in its text) wants
/// Amp to use that copy; neither is reported.
fn check_masked_by_agents_copy(
    sink: &mut Sink<'_, '_>,
    path: &Path,
    content: &str,
    frontmatter: &JsonNode,
    location: &AmpSkillLocation<'_>,
    fs: &dyn FileSystem,
) {
    let Some(name) = frontmatter_name(frontmatter) else {
        return;
    };
    let Some(relative) = path
        .parent()
        .and_then(|dir| dir.strip_prefix(location.root).ok())
    else {
        return;
    };
    let Some(project) = location.root.parent().and_then(Path::parent) else {
        return;
    };
    let other: PathBuf = project
        .join(".agents")
        .join("skills")
        .join(relative)
        .join("SKILL.md");
    if !fs.is_file(&other) {
        return;
    }
    let Ok(raw) = fs.read_to_string(&other) else {
        return;
    };
    let other_content = normalize_line_endings(&raw);
    let pointer = format!(
        ".agents/skills/{}/SKILL.md",
        relative.to_string_lossy().replace('\\', "/")
    );
    if other_content == content || content.contains(&pointer) {
        return;
    }
    let Some(other_name) = skill_frontmatter(&other_content).and_then(|fm| frontmatter_name(&fm))
    else {
        return;
    };
    if other_name != name {
        return;
    }
    let span = frontmatter
        .value
        .get("name")
        .map_or((0, 0), |member| (member.key_start, member.key_end));
    sink.push(
        DiagnosticLevel::Info,
        "AMP-SK-002",
        span,
        t!(
            "rules.amp_sk_002.message",
            name = name.as_str(),
            other = other.display().to_string()
        )
        .to_string(),
        t!("rules.amp_sk_002.suggestion").to_string(),
    );
}

/// A skill's sibling `mcp.json`: an object of server name to entry
/// (ampcode.com/docs/customize/skills, "MCP Servers in Skills"). Parsed
/// leniently as JSONC; whether Amp accepts comments here is undocumented.
fn validate_skill_mcp_json(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    if !["AMP-006", "AMP-008", "AMP-010"]
        .iter()
        .any(|rule| config.is_rule_enabled(rule))
    {
        return Vec::new();
    }
    let mut sink = Sink::new(path, content, config);
    let doc = match parse_json_document(content, true) {
        Ok(doc) => doc,
        Err(error) => {
            if config.is_rule_enabled("AMP-010") {
                sink.diagnostics.push(
                    Diagnostic::error(
                        path.to_path_buf(),
                        error.line,
                        error.column,
                        "AMP-010",
                        t!("rules.amp_010.parse_error", error = error.message.as_str()),
                    )
                    .with_suggestion(t!("rules.amp_010.suggestion")),
                );
            }
            return sink.diagnostics;
        }
    };
    let Some(servers) = doc.root.value.as_object() else {
        sink.push(
            DiagnosticLevel::Error,
            "AMP-010",
            (doc.root.start, doc.root.end),
            t!(
                "rules.amp_010.root_not_object",
                found = type_name(&doc.root)
            )
            .to_string(),
            t!("rules.amp_010.suggestion").to_string(),
        );
        return sink.diagnostics;
    };
    // "If both are present, Amp uses `mcpServers` and ignores `mcp.json`":
    // the servers of an ignored file are not checked (AMP-005 reports it on
    // SKILL.md).
    let fs = config.fs();
    let ignored = path
        .parent()
        .map(|dir| dir.join("SKILL.md"))
        .filter(|skill| fs.is_file(skill))
        .and_then(|skill| fs.read_to_string(&skill).ok())
        .and_then(|raw| skill_frontmatter(&normalize_line_endings(&raw)))
        .is_some_and(|frontmatter| declares_mcp_servers(&frontmatter));
    if !ignored {
        check_servers(&mut sink, servers, McpOrigin::SkillFile);
    }
    sink.diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    fn config_with(files: &[(&str, &str)]) -> LintConfig {
        let fs = Arc::new(MockFileSystem::new());
        for (path, content) in files {
            fs.add_file(*path, *content);
            for dir in Path::new(path).ancestors().skip(1) {
                fs.add_dir(dir);
            }
        }
        let mut config = LintConfig::default();
        config.set_fs(fs);
        config
    }

    fn run(path: &str, content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        AmpSkillValidator.validate(Path::new(path), content, config)
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<(&str, DiagnosticLevel, usize)> {
        let mut out: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.level, d.line))
            .collect();
        out.sort_by(|a, b| a.0.cmp(b.0).then(a.2.cmp(&b.2)));
        out
    }

    const SKILL: &str = "---\nname: linear\ndescription: Manages Linear issues. Use when filing an issue.\n---\n\nUse the linear tools.\n";

    // The two vendor examples (skills page, "MCP Servers in Skills").
    const LOCAL_EXAMPLE: &str = r#"{
	"chrome-devtools": {
		"command": "npx",
		"args": ["-y", "chrome-devtools-mcp@latest"],
		"includeTools": ["navigate_*", "take_screenshot", "click", "fill*"]
	}
}"#;
    const REMOTE_EXAMPLE: &str = r#"{
	"linear": {
		"url": "https://mcp.linear.app/sse",
		"includeTools": ["list_issues", "create_issue", "update_issue"]
	}
}"#;

    #[test]
    fn vendor_examples_are_clean() {
        let config = config_with(&[]);
        for content in [LOCAL_EXAMPLE, REMOTE_EXAMPLE] {
            let diagnostics = run(".agents/skills/x/mcp.json", content, &config);
            assert!(diagnostics.is_empty(), "{diagnostics:?}");
        }
        let skill = "---\nname: linear-ops\ndescription: Manage Linear issues. Use when the user asks about Linear tickets.\nmcpServers:\n  linear:\n    url: https://mcp.linear.app/mcp\n    includeTools: [\"list_issues\"]\n---\n\nUse the linear tools.\n";
        assert!(run(".agents/skills/linear-ops/SKILL.md", skill, &config).is_empty());
    }

    #[test]
    fn outside_amp_roots_is_skipped() {
        let config = config_with(&[]);
        let skill =
            "---\nname: x\ndescription: y\nmcpServers:\n  a:\n    command: npx\n---\nbody\n";
        assert!(run("skills/x/SKILL.md", skill, &config).is_empty());
        assert!(run(".cursor/skills/x/SKILL.md", skill, &config).is_empty());
        assert!(!run(".claude/skills/x/SKILL.md", skill, &config).is_empty());
    }

    #[test]
    fn amp_005_both_sources() {
        let skill = "---\nname: linear\ndescription: d\nmcpServers:\n  linear:\n    url: https://mcp.linear.app/mcp\n    includeTools: [list_issues]\n---\nbody\n";
        let config = config_with(&[("/p/.agents/skills/linear/mcp.json", REMOTE_EXAMPLE)]);
        let diagnostics = run("/p/.agents/skills/linear/SKILL.md", skill, &config);
        assert_eq!(
            rules(&diagnostics),
            [("AMP-005", DiagnosticLevel::Warning, 4)]
        );
        assert_eq!(diagnostics[0].column, 1);
        // The ignored mcp.json is not checked further.
        let config = config_with(&[("/p/.agents/skills/linear/SKILL.md", skill)]);
        let ignored = r#"{"linear": {"url": "https://mcp.linear.app/sse"}}"#;
        assert!(run("/p/.agents/skills/linear/mcp.json", ignored, &config).is_empty());
        // Without the sibling, no AMP-005.
        let config = config_with(&[]);
        assert!(run("/p/.agents/skills/linear/SKILL.md", skill, &config).is_empty());
    }

    #[test]
    fn amp_008_missing_include_tools() {
        let config = config_with(&[]);
        let json = r#"{"linear": {"url": "https://mcp.linear.app/sse"}, "bad": 3}"#;
        let diagnostics = run(".agents/skills/linear/mcp.json", json, &config);
        assert_eq!(
            rules(&diagnostics),
            [
                ("AMP-008", DiagnosticLevel::Info, 1),
                ("AMP-010", DiagnosticLevel::Warning, 1)
            ]
        );
        let skill = "---\nname: x\ndescription: d\nmcpServers:\n  pw:\n    command: npx\n    args: [\"-y\", \"@playwright/mcp\"]\n---\nbody\n";
        let diagnostics = run(".agents/skills/x/SKILL.md", skill, &config);
        assert_eq!(rules(&diagnostics), [("AMP-008", DiagnosticLevel::Info, 5)]);
    }

    #[test]
    fn amp_006_id_token_placement() {
        let config = config_with(&[]);
        let json = r#"{
  "ok": {"url": "https://mcp.example.com/mcp", "headers": {"Authorization": "Bearer ${amp:id-token}"}, "includeTools": ["a"]},
  "in-url": {"url": "https://x.dev/${amp:id-token}", "includeTools": ["a"]},
  "plain-http": {"url": "http://x.dev/mcp", "headers": {"A": "${amp:id-token}"}, "includeTools": ["a"]},
  "var-url": {"url": "${SRC}/mcp", "headers": {"A": "${amp:id-token}"}, "includeTools": ["a"]},
  "oauth": {"url": "https://x.dev", "headers": {"A": "${amp:id-token}"}, "oauth": {"clientId": "c", "authUrl": "a", "tokenUrl": "t"}, "includeTools": ["a"]},
  "local": {"command": "run ${amp:id-token}", "args": ["--t", "${amp:id-token}"], "env": {"T": "${amp:id-token}"}, "includeTools": ["a"]}
}"#;
        let diagnostics = run(".agents/skills/x/mcp.json", json, &config);
        let lines: Vec<_> = rules(&diagnostics)
            .into_iter()
            .map(|(rule, level, line)| {
                assert_eq!((rule, level), ("AMP-006", DiagnosticLevel::Error));
                line
            })
            .collect();
        assert_eq!(lines, [3, 4, 6, 7, 7, 7], "{diagnostics:?}");
        let skill = "---\nname: x\ndescription: d\nmcpServers:\n  local:\n    command: npx\n    includeTools: [a]\n    env:\n      TOKEN: \"${amp:id-token}\"\n---\nbody\n";
        let diagnostics = run(".agents/skills/x/SKILL.md", skill, &config);
        assert_eq!(
            rules(&diagnostics),
            [("AMP-006", DiagnosticLevel::Error, 9)]
        );
        assert_eq!(diagnostics[0].column, 14);
    }

    #[test]
    fn amp_010_shapes() {
        let config = config_with(&[]);
        let parse = run(".agents/skills/x/mcp.json", "{\"a\": ", &config);
        assert_eq!(rules(&parse), [("AMP-010", DiagnosticLevel::Error, 1)]);
        let root = run(".agents/skills/x/mcp.json", "[]", &config);
        assert_eq!(rules(&root), [("AMP-010", DiagnosticLevel::Error, 1)]);
        let json = r#"{
  "none": {"includeTools": ["a"]},
  "both": {"command": "x", "url": "https://x", "includeTools": ["a"]},
  "types": {"command": ["x"], "args": "-y", "env": {"A": 1}, "includeTools": "a", "transport": "ws"}
}"#;
        let diagnostics = run(".agents/skills/x/mcp.json", json, &config);
        assert_eq!(
            rules(&diagnostics)
                .iter()
                .filter(|(rule, ..)| *rule == "AMP-010")
                .count(),
            7,
            "{diagnostics:?}"
        );
        // JSONC is accepted without a diagnostic.
        let jsonc =
            "{\n  // browser\n  \"b\": {\"command\": \"npx\", \"includeTools\": [\"a\"],},\n}";
        assert!(run(".agents/skills/x/mcp.json", jsonc, &config).is_empty());
        let skill = "---\nname: x\ndescription: d\nmcpServers: [a]\n---\nbody\n";
        let diagnostics = run(".agents/skills/x/SKILL.md", skill, &config);
        assert_eq!(
            rules(&diagnostics),
            [("AMP-010", DiagnosticLevel::Warning, 4)]
        );
    }

    #[test]
    fn amp_sk_002_masked_claude_copy() {
        let agents =
            "---\nname: deploy\ndescription: Deploys. Use when deploying.\n---\n\nAgents copy.\n";
        let claude =
            "---\nname: deploy\ndescription: Deploys. Use when deploying.\n---\n\nClaude copy.\n";
        let config = config_with(&[("/p/.agents/skills/deploy/SKILL.md", agents)]);
        let diagnostics = run("/p/.claude/skills/deploy/SKILL.md", claude, &config);
        assert_eq!(
            rules(&diagnostics),
            [("AMP-SK-002", DiagnosticLevel::Info, 2)]
        );
        assert!(
            diagnostics[0]
                .message
                .replace('\\', "/")
                .contains("/p/.agents/skills/deploy/SKILL.md")
        );
        // Byte-identical copies are deliberate.
        assert!(run("/p/.claude/skills/deploy/SKILL.md", agents, &config).is_empty());
        // A stub that sends the reader to the .agents copy is deliberate.
        let stub = "---\nname: deploy\ndescription: Deploys. Use when deploying.\n---\n\nRead `.agents/skills/deploy/SKILL.md` and follow it.\n";
        assert!(run("/p/.claude/skills/deploy/SKILL.md", stub, &config).is_empty());
        // Different names are different skills.
        let renamed = claude.replace("name: deploy", "name: deploy-claude");
        assert!(run("/p/.claude/skills/deploy/SKILL.md", &renamed, &config).is_empty());
        // The .agents copy itself is not reported.
        assert!(run("/p/.agents/skills/deploy/SKILL.md", agents, &config).is_empty());
        // Amp skips Claude Code skill directories when told to.
        let config = config_with(&[
            ("/p/.agents/skills/deploy/SKILL.md", agents),
            (
                "/p/.amp/settings.json",
                "{\"amp.skills.disableClaudeCodeSkills\": true}",
            ),
        ]);
        assert!(run("/p/.claude/skills/deploy/SKILL.md", claude, &config).is_empty());
    }

    #[test]
    fn amp_sk_003_depth_boundary() {
        let config = config_with(&[]);
        let five = run(".agents/skills/a/b/c/d/e/SKILL.md", SKILL, &config);
        assert!(five.is_empty(), "{five:?}");
        let six = run(".agents/skills/a/b/c/d/e/f/SKILL.md", SKILL, &config);
        assert_eq!(rules(&six), [("AMP-SK-003", DiagnosticLevel::Warning, 1)]);
        assert!(six[0].message.contains('6'));
        let home = run(
            "/home/u/.config/amp/skills/a/b/c/d/e/f/g/SKILL.md",
            SKILL,
            &config,
        );
        assert_eq!(rules(&home), [("AMP-SK-003", DiagnosticLevel::Warning, 1)]);
    }
}
