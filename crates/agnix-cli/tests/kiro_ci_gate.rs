use assert_cmd::Command;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
struct ReposManifest {
    repos: Vec<RepoEntry>,
}

#[derive(Debug, Deserialize)]
struct RepoEntry {
    url: String,
    categories: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RulesIndex {
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    id: String,
}

fn workspace_root() -> &'static Path {
    use std::sync::OnceLock;

    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            let cargo_toml = ancestor.join("Cargo.toml");
            if let Ok(content) = fs::read_to_string(&cargo_toml)
                && (content.contains("[workspace]") || content.contains("[workspace."))
            {
                return ancestor.to_path_buf();
            }
        }
        panic!(
            "Failed to locate workspace root from CARGO_MANIFEST_DIR={}",
            manifest_dir.display()
        );
    })
    .as_path()
}

fn agnix() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("agnix");
    cmd.current_dir(workspace_root());
    cmd
}

fn normalize_repo_url(url: &str) -> Option<String> {
    let trimmed = url.trim().trim_end_matches('/');
    let prefix = "https://github.com/";
    if !trimmed.starts_with(prefix) {
        return None;
    }

    let mut parts = trimmed[prefix.len()..].split('/');
    let owner = parts.next()?;
    let repo = parts.next()?;
    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        return None;
    }

    Some(format!(
        "https://github.com/{}/{}",
        owner.to_ascii_lowercase(),
        repo.to_ascii_lowercase()
    ))
}

#[test]
#[ignore = "run via CI Kiro S-tier gate step"]
fn kiro_target_still_disables_claude_rules() {
    let temp_dir = tempfile::tempdir().unwrap();
    let steering_dir = temp_dir.path().join(".kiro").join("steering");
    fs::create_dir_all(&steering_dir).unwrap();
    let skills_dir = temp_dir
        .path()
        .join(".kiro")
        .join("skills")
        .join("deploy-prod");
    fs::create_dir_all(&skills_dir).unwrap();

    let steering_path = steering_dir.join("scope.md");
    let mut steering_file = fs::File::create(&steering_path).unwrap();
    // This intentionally violates KIRO-001.
    writeln!(
        steering_file,
        "---\ninclusion: ALWAYS\n---\nInvalid uppercase inclusion mode."
    )
    .unwrap();

    let skill_path = skills_dir.join("SKILL.md");
    let mut file = fs::File::create(&skill_path).unwrap();
    // This intentionally violates KR-SK-001.
    writeln!(
        file,
        "---\nname: deploy-prod\ndescription: Deploy to production\nmodel: haiku\n---\nDeploy the application"
    )
    .unwrap();

    let output = agnix()
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--format")
        .arg("json")
        .arg("--target")
        .arg("kiro")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("Invalid JSON output: {e}"));
    let diagnostics = json["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics missing in JSON output"));

    let has_cc_rule = diagnostics
        .iter()
        .any(|d| d["rule"].as_str().unwrap_or("").starts_with("CC-"));
    let has_non_cc_rule = diagnostics.iter().any(|d| {
        d["rule"]
            .as_str()
            .map(|rule| !rule.starts_with("CC-"))
            .unwrap_or(false)
    });
    let has_kiro_steering_rule = diagnostics.iter().any(|d| {
        d["rule"]
            .as_str()
            .map(|rule| rule.starts_with("KIRO-"))
            .unwrap_or(false)
    });
    let has_kiro_skill_rule = diagnostics.iter().any(|d| {
        d["rule"]
            .as_str()
            .map(|rule| rule.starts_with("KR-SK-"))
            .unwrap_or(false)
    });

    assert!(
        !has_cc_rule,
        "With --target kiro, CC-* rules should be disabled"
    );
    assert!(
        has_non_cc_rule,
        "With --target kiro, non-CC rules should still run"
    );
    assert!(
        has_kiro_steering_rule,
        "With --target kiro, KIRO-* steering rules should run"
    );
    assert!(
        has_kiro_skill_rule,
        "With --target kiro, KR-SK-* Kiro skill rules should run"
    );
}

#[test]
#[ignore = "run via CI Kiro S-tier gate step"]
fn readme_supported_tools_still_lists_kiro_surface() {
    let path = workspace_root().join("README.md");
    let readme = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

    let kiro_row = readme
        .lines()
        .find(|line| line.starts_with("| [Kiro](https://kiro.dev) |"))
        .expect("README.md Supported Tools table must include a Kiro row");

    assert!(
        kiro_row.contains("KIRO-\\*")
            && kiro_row.contains("KR-SK-\\*")
            && kiro_row.contains("KR-AG-\\*")
            && kiro_row.contains("KR-HK-\\*")
            && kiro_row.contains("KR-PW-\\*")
            && kiro_row.contains("KR-MCP-\\*"),
        "Kiro row must include KIRO-*, KR-SK-*, KR-AG-*, KR-HK-*, KR-PW-*, and KR-MCP-* prefixes"
    );
    assert!(
        kiro_row.contains(".kiro/steering/\\*\\*/\\*.md")
            && kiro_row.contains(".kiro/skills/\\*\\*/SKILL.md")
            && kiro_row.contains(".kiro/agents/\\*.json")
            && kiro_row.contains(".kiro/hooks/\\*.kiro.hook")
            && kiro_row.contains(".kiro/settings/mcp.json")
            && kiro_row.contains(".kiro/powers/\\*/POWER.md"),
        "Kiro row must include steering, skills, agents, hooks, mcp, and powers file surfaces"
    );
}

#[test]
#[ignore = "run via CI Kiro S-tier gate step"]
fn project_memory_marks_kiro_as_s_tier() {
    for relative in ["CLAUDE.md", "AGENTS.md"] {
        let path = workspace_root().join(relative);
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));

        let s_tier_line = content
            .lines()
            .find(|line| line.starts_with("- **S** (test always):"))
            .unwrap_or_else(|| panic!("{relative} must define an S-tier support line"));
        let b_tier_line = content
            .lines()
            .find(|line| line.starts_with("- **B** (test if time permits):"))
            .unwrap_or_else(|| panic!("{relative} must define a B-tier support line"));

        assert!(
            s_tier_line.contains("Kiro CLI"),
            "{relative} must list Kiro CLI in S tier"
        );
        assert!(
            !b_tier_line.contains("Kiro CLI"),
            "{relative} must not list Kiro CLI in B tier"
        );
    }
}

#[test]
#[ignore = "run via CI Kiro S-tier gate step"]
fn kiro_rules_still_documented_in_validation_rules() {
    let rules_path = workspace_root().join("knowledge-base/rules.json");
    let rules_content = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rules_path.display(), e));
    let index: RulesIndex = serde_json::from_str(&rules_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", rules_path.display(), e));

    let validation_rules_path = workspace_root().join("knowledge-base/VALIDATION-RULES.md");
    let validation_content = fs::read_to_string(&validation_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", validation_rules_path.display(), e));

    let kiro_rules: Vec<String> = index
        .rules
        .iter()
        .filter(|rule| {
            rule.id.starts_with("KIRO-")
                || rule.id.starts_with("KR-SK-")
                || rule.id.starts_with("KR-AG-")
                || rule.id.starts_with("KR-HK-")
                || rule.id.starts_with("KR-PW-")
                || rule.id.starts_with("KR-MCP-")
        })
        .map(|rule| rule.id.clone())
        .collect();
    assert!(
        !kiro_rules.is_empty(),
        "Expected at least one KIRO-/KR-SK-/KR-AG-/KR-HK-/KR-PW-/KR-MCP- rule in rules.json"
    );

    let missing_rules: Vec<String> = kiro_rules
        .iter()
        .filter(|rule| {
            let patterns = [
                format!("<a id=\"{}\"></a>", rule.to_ascii_lowercase()),
                format!("### {} ", rule),
                format!("### {}[", rule),
            ];
            !patterns
                .iter()
                .any(|pattern| validation_content.contains(pattern))
        })
        .cloned()
        .collect();
    assert!(
        missing_rules.is_empty(),
        "Kiro rules missing from VALIDATION-RULES.md:\n{}",
        missing_rules.join(", ")
    );
}

#[test]
#[ignore = "run via CI Kiro S-tier gate step"]
fn real_world_manifest_has_explicit_kiro_coverage() {
    let path = workspace_root().join("tests/real-world/repos.yaml");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let manifest: ReposManifest = serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));

    let kiro_entries: Vec<&RepoEntry> = manifest
        .repos
        .iter()
        .filter(|repo| repo.categories.iter().any(|category| category == "kiro"))
        .collect();

    let required_kiro_repos = [
        "https://github.com/Theadd/kiro-agents",
        "https://github.com/awsdataarchitect/kiro-best-practices",
        "https://github.com/dereknguyen269/derek-power",
        "https://github.com/cremich/promptz",
    ];

    assert!(
        kiro_entries.len() >= required_kiro_repos.len(),
        "tests/real-world/repos.yaml must include at least {} explicit 'kiro' categorized repos, found {}",
        required_kiro_repos.len(),
        kiro_entries.len()
    );

    let repo_url_re =
        Regex::new(r"^https://github\.com/[^/]+/[^/]+/?$").expect("regex must compile");
    assert!(
        kiro_entries
            .iter()
            .all(|repo| repo_url_re.is_match(&repo.url)),
        "All explicit 'kiro' category entries must be valid GitHub owner/repo URLs"
    );

    let normalized_kiro_repo_urls: HashSet<String> = kiro_entries
        .iter()
        .map(|repo| normalize_repo_url(&repo.url))
        .collect::<Option<HashSet<String>>>()
        .expect("All explicit 'kiro' repo URLs must normalize to owner/repo form");
    let missing_required: Vec<&str> = required_kiro_repos
        .iter()
        .copied()
        .filter(|url| {
            let normalized_required =
                normalize_repo_url(url).expect("required Kiro repo URL constants must normalize");
            !normalized_kiro_repo_urls.contains(&normalized_required)
        })
        .collect();
    assert!(
        missing_required.is_empty(),
        "Missing required explicit Kiro real-world repos in tests/real-world/repos.yaml:\n{}",
        missing_required.join("\n")
    );
}
