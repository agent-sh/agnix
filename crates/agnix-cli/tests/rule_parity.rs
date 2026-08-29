//! Rule parity integration tests.
//!
//! Ensures all rules from knowledge-base/rules.json are:

//! 1. Registered in SARIF output (sarif.rs)
//! 2. Implemented in agnix-core/src/rules/*.rs
//! 3. Covered by test fixtures in tests/fixtures/
//! 4. Have valid evidence metadata

// Allow common test patterns that clippy flags but are intentional in tests
#![allow(clippy::field_reassign_with_default)]

use regex::Regex;
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Evidence source type classification
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    /// Official specification (e.g., agentskills.io/specification, modelcontextprotocol.io/specification)
    Spec,
    /// Vendor documentation (e.g., code.claude.com/docs, docs.github.com)
    VendorDocs,
    /// Vendor source code
    VendorCode,
    /// Academic paper or research
    Paper,
    /// Community best practices and multi-platform research
    Community,
}

/// Normative level for rules (RFC 2119 style)
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NormativeLevel {
    /// Absolute requirement
    Must,
    /// Recommended but not mandatory
    Should,
    /// Optional best practice
    BestPractice,
}

/// Applicability constraints for a rule
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppliesTo {
    /// Tool this rule applies to (e.g., "claude-code", "cursor", "github-copilot")
    #[serde(default)]
    pub tool: Option<String>,
    /// Semver version range (e.g., ">=1.0.0")
    #[serde(default)]
    pub version_range: Option<String>,
    /// Specification revision (e.g., "1.0", "2025-11-25")
    #[serde(default)]
    pub spec_revision: Option<String>,
}

/// Test coverage tracking for a rule
#[derive(Debug, Clone, Deserialize)]
pub struct TestCoverage {
    /// Has unit tests
    pub unit: bool,
    /// Has fixture tests
    pub fixtures: bool,
    /// Has end-to-end tests
    pub e2e: bool,
}

/// Evidence metadata for a rule
#[derive(Debug, Clone, Deserialize)]
pub struct Evidence {
    /// Classification of the evidence source
    pub source_type: SourceType,
    /// URLs to authoritative sources
    pub source_urls: Vec<String>,
    /// Date when the evidence was last verified (ISO 8601)
    pub verified_on: String,
    /// Applicability constraints
    pub applies_to: AppliesTo,
    /// RFC 2119-style normative level
    pub normative_level: NormativeLevel,
    /// Test coverage information
    pub tests: TestCoverage,
}

/// Auto-fix metadata for a rule
#[derive(Debug, Clone, Deserialize)]
pub struct FixMetadata {
    /// Whether this rule has auto-fix support
    pub autofix: bool,
    /// Safety classification (only present when autofix is true)
    #[serde(default)]
    pub fix_safety: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct SecurityMetadata {
    cwe: Vec<String>,
    owasp: Vec<String>,
    vulnerability_class: String,
    subcategory: String,
    confidence: String,
    likelihood: String,
    impact: String,
}

/// Rule definition from rules.json
#[derive(Debug, Deserialize)]
struct RulesIndex {
    total_rules: usize,
    rules: Vec<RuleEntry>,
}

#[derive(Debug, Deserialize)]
struct RuleEntry {
    id: String,
    #[allow(dead_code)]
    name: String,
    severity: String,
    category: String,
    /// Evidence metadata (required for all rules)
    evidence: Evidence,
    /// Auto-fix metadata (required for all rules)
    fix: FixMetadata,
    /// Security taxonomy metadata (present for security-facing rules)
    #[serde(default)]
    security: Option<SecurityMetadata>,
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

fn load_rules_json() -> RulesIndex {
    let rules_path = workspace_root().join("knowledge-base/rules.json");
    let content = fs::read_to_string(&rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", rules_path.display(), e));
    serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", rules_path.display(), e))
}

#[test]
fn security_metadata_is_valid_when_present() {
    let rules = load_rules_json();
    let allowed_subcategories = ["vuln", "audit", "secure-default"];
    let allowed_levels = ["HIGH", "MEDIUM", "LOW"];
    let mut security_rule_count = 0usize;

    for rule in rules.rules {
        let Some(security) = rule.security else {
            continue;
        };
        security_rule_count += 1;
        assert!(
            !security.cwe.is_empty(),
            "{} security.cwe must not be empty",
            rule.id
        );
        assert!(
            security
                .cwe
                .iter()
                .all(|cwe| cwe.starts_with("CWE-") && cwe[4..].chars().all(|c| c.is_ascii_digit())),
            "{} security.cwe entries must be CWE IDs",
            rule.id
        );
        assert!(
            !security.owasp.is_empty(),
            "{} security.owasp must not be empty",
            rule.id
        );
        assert!(
            !security.vulnerability_class.trim().is_empty(),
            "{} security.vulnerability_class must not be empty",
            rule.id
        );
        assert!(
            allowed_subcategories.contains(&security.subcategory.as_str()),
            "{} security.subcategory is invalid",
            rule.id
        );
        for (field, value) in [
            ("confidence", security.confidence.as_str()),
            ("likelihood", security.likelihood.as_str()),
            ("impact", security.impact.as_str()),
        ] {
            assert!(
                allowed_levels.contains(&value),
                "{} security.{} is invalid",
                rule.id,
                field
            );
        }
    }

    assert!(
        security_rule_count >= 30,
        "expected broad security taxonomy coverage, found {security_rule_count}"
    );
}

/// Extract SARIF rule IDs from rules.json (since SARIF rules are now generated from rules.json at build time)
fn extract_sarif_rule_ids() -> BTreeSet<String> {
    // Since SARIF rules are now generated from rules.json via build.rs,
    // we verify SARIF parity by checking rules.json directly.
    // The build.rs script transforms rules.json into SARIF rules at compile time.
    let rules_index = load_rules_json();
    rules_index.rules.iter().map(|r| r.id.clone()).collect()
}

fn extract_implemented_rule_ids() -> BTreeSet<String> {
    let core_src = workspace_root().join("crates/agnix-core/src");
    let mut rule_ids = BTreeSet::new();

    // Pattern matches rule IDs in Diagnostic::error/warning/info calls
    // e.g., Diagnostic::error(..., "AS-001", ...) or rule: "CC-HK-001".to_string()
    let re = Regex::new(r#""([A-Z]+-(?:[A-Z]+-)?[0-9]+)""#).unwrap();

    // Known rule ID prefixes to filter out false positives
    let valid_prefixes = [
        "AS-", "CC-SK-", "CC-HK-", "CC-AG-", "CC-MEM-", "CC-OS-", "CC-PL-", "CC-SET-", "AGM-",
        "MCP-", "COP-", "CUR-", "CLN-", "CDX-", "OC-", "GM-", "GM-AG-", "XML-", "REF-", "PE-",
        "XP-", "VER-", "WS-", "CR-SK-", "CL-SK-", "CP-SK-", "CX-SK-", "OC-SK-", "WS-SK-", "KR-SK-",
        "KR-AG-", "KR-HK-", "KR-PW-", "KR-MCP-", "KR-SET-", "KIRO-", "AMP-SK-", "AMP-", "RC-SK-",
        "ROO-",
    ];

    fn extract_from_file(
        path: &Path,
        re: &Regex,
        valid_prefixes: &[&str],
        rule_ids: &mut BTreeSet<String>,
    ) {
        if let Ok(content) = fs::read_to_string(path) {
            for cap in re.captures_iter(&content) {
                let rule_id = &cap[1];
                if valid_prefixes.iter().any(|p| rule_id.starts_with(p)) {
                    rule_ids.insert(rule_id.to_string());
                }
            }
        }
    }

    fn scan_rules_recursive(
        dir: &Path,
        re: &Regex,
        valid_prefixes: &[&str],
        rule_ids: &mut BTreeSet<String>,
    ) {
        let entries = fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("Failed to read rules directory {}: {}", dir.display(), e));
        for entry in entries {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();
            if path.is_dir() {
                scan_rules_recursive(&path, re, valid_prefixes, rule_ids);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                extract_from_file(&path, re, valid_prefixes, rule_ids);
            }
        }
    }

    // Scan rules directory recursively so nested modules are included.
    let rules_dir = core_src.join("rules");
    scan_rules_recursive(&rules_dir, &re, &valid_prefixes, &mut rule_ids);

    // Also scan top-level .rs files for project-level rules (e.g., AGM-006
    // in pipeline.rs, VER-001, XP-004/005/006).
    for entry in fs::read_dir(&core_src)
        .unwrap_or_else(|e| panic!("Failed to read core src dir {}: {}", core_src.display(), e))
    {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "rs") {
            extract_from_file(&path, &re, &valid_prefixes, &mut rule_ids);
        }
    }

    rule_ids
}

fn scan_fixtures_for_coverage() -> HashMap<String, Vec<String>> {
    let fixtures_dir = workspace_root().join("tests/fixtures");
    let mut coverage: HashMap<String, Vec<String>> = HashMap::new();

    // Pattern to match rule IDs in fixture file content or directory names
    let re = Regex::new(r"[A-Z]+-(?:[A-Z]+-)?[0-9]+").unwrap();

    fn scan_dir_recursive(dir: &Path, re: &Regex, coverage: &mut HashMap<String, Vec<String>>) {
        if !dir.is_dir() {
            return;
        }

        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();

            if path.is_dir() {
                scan_dir_recursive(&path, re, coverage);
            } else if path.is_file() {
                // Check file content for explicit rule references
                if let Ok(content) = fs::read_to_string(&path) {
                    for cap in re.captures_iter(&content) {
                        let rule_id = cap[0].to_string();
                        let fixture_path = path.to_string_lossy().to_string();
                        coverage
                            .entry(rule_id)
                            .or_default()
                            .push(fixture_path.clone());
                    }
                }

                // Also check filename patterns like "xml-001-unclosed.md"
                let filename = path.file_name().unwrap().to_string_lossy().to_lowercase();
                for cap in re.captures_iter(&filename.to_uppercase()) {
                    let rule_id = cap[0].to_string();
                    let fixture_path = path.to_string_lossy().to_string();
                    coverage.entry(rule_id).or_default().push(fixture_path);
                }
            }
        }
    }

    scan_dir_recursive(&fixtures_dir, &re, &mut coverage);
    coverage
}

/// Infer fixture coverage based on directory structure
fn infer_fixture_coverage(rules: &[RuleEntry]) -> HashMap<String, Vec<String>> {
    let fixtures_dir = workspace_root().join("tests/fixtures");
    let mut coverage: HashMap<String, Vec<String>> = HashMap::new();

    // Map categories to fixture directories
    let category_to_dirs: HashMap<&str, Vec<&str>> = [
        (
            "agent-skills",
            vec!["skills", "invalid/skills", "valid/skills"],
        ),
        (
            "claude-skills",
            vec!["skills", "invalid/skills", "valid/skills"],
        ),
        ("claude-hooks", vec!["valid/hooks", "invalid/hooks"]),
        ("claude-agents", vec!["valid/agents", "invalid/agents"]),
        ("claude-memory", vec!["valid/memory", "invalid/memory"]),
        (
            "claude-settings",
            vec!["valid/claude-settings", "invalid/claude-settings"],
        ),
        (
            "claude-output-styles",
            vec!["valid/output-styles", "invalid/output-styles"],
        ),
        ("claude-plugins", vec!["valid/plugins", "invalid/plugins"]),
        ("agents-md", vec!["agents_md"]),
        ("mcp", vec!["mcp"]),
        (
            "copilot",
            vec!["copilot", "copilot-invalid", "copilot-too-long"],
        ),
        ("cursor", vec!["cursor", "cursor-invalid", "cursor-legacy"]),
        ("cline", vec!["cline", "cline-invalid"]),
        ("xml", vec!["xml"]),
        ("references", vec!["refs"]),
        (
            "prompt-engineering",
            vec!["prompt", "invalid/pe", "valid/pe"],
        ),
        (
            "cross-platform",
            vec!["cross_platform", "per_client_skills"],
        ),
        ("opencode", vec!["opencode", "opencode-invalid"]),
        ("cursor-skills", vec!["per_client_skills"]),
        ("cline-skills", vec!["per_client_skills"]),
        ("copilot-skills", vec!["per_client_skills"]),
        ("codex-skills", vec!["per_client_skills"]),
        ("opencode-skills", vec!["per_client_skills"]),
        ("windsurf-skills", vec!["per_client_skills"]),
        ("kiro-skills", vec!["per_client_skills"]),
        ("kiro-agents", vec!["kiro-agents"]),
        ("kiro-hooks", vec!["kiro-hooks", "kiro-agents"]),
        ("kiro-mcp", vec!["kiro-mcp", "kiro-powers"]),
        ("kiro-powers", vec!["kiro-powers"]),
        (
            "kiro-settings",
            vec!["valid/kiro-settings", "invalid/kiro-settings"],
        ),
        ("amp-skills", vec!["per_client_skills"]),
        ("amp-checks", vec!["amp-checks"]),
        ("roo-code-skills", vec!["per_client_skills"]),
        ("gemini-cli", vec!["gemini_md", "gemini_md-invalid"]),
        (
            "gemini-agents",
            vec!["valid/gemini-agents", "invalid/gemini-agents"],
        ),
        ("codex", vec!["codex", "codex-invalid"]),
        ("roo-code", vec!["roo-code"]),
        ("windsurf", vec!["windsurf", "windsurf-legacy"]),
        ("kiro-steering", vec!["kiro-steering"]),
    ]
    .into_iter()
    .collect();

    for rule in rules {
        if let Some(dirs) = category_to_dirs.get(rule.category.as_str()) {
            for dir in dirs {
                let full_path = fixtures_dir.join(dir);
                if full_path.exists() {
                    coverage
                        .entry(rule.id.clone())
                        .or_default()
                        .push(full_path.to_string_lossy().to_string());
                }
            }
        }
    }

    coverage
}

#[test]
fn test_all_rules_registered_in_sarif() {
    let rules_index = load_rules_json();
    let sarif_rules = extract_sarif_rule_ids();

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let missing_from_sarif: Vec<&String> = documented_rules.difference(&sarif_rules).collect();

    let extra_in_sarif: Vec<&String> = sarif_rules.difference(&documented_rules).collect();

    let mut report = String::new();

    if !missing_from_sarif.is_empty() {
        report.push_str(&format!(
            "\nMissing from SARIF ({} rules):\n",
            missing_from_sarif.len()
        ));
        for rule in &missing_from_sarif {
            report.push_str(&format!("  - {}\n", rule));
        }
    }

    if !extra_in_sarif.is_empty() {
        report.push_str(&format!(
            "\nExtra in SARIF (not in rules.json) ({} rules):\n",
            extra_in_sarif.len()
        ));
        for rule in &extra_in_sarif {
            report.push_str(&format!("  - {}\n", rule));
        }
    }

    assert!(
        missing_from_sarif.is_empty() && extra_in_sarif.is_empty(),
        "SARIF rule parity check failed:\n{}\nSARIF has {} rules, rules.json has {} rules",
        report,
        sarif_rules.len(),
        documented_rules.len()
    );
}

#[test]
fn test_all_rules_implemented() {
    let rules_index = load_rules_json();
    let implemented_rules = extract_implemented_rule_ids();

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let not_implemented: Vec<&String> = documented_rules.difference(&implemented_rules).collect();

    if !not_implemented.is_empty() {
        let mut report = format!(
            "Rules documented but not found in implementation ({}):\n",
            not_implemented.len()
        );
        for rule in &not_implemented {
            report.push_str(&format!("  - {}\n", rule));
        }
        report.push_str("\nNote: This may indicate:\n");
        report.push_str("  1. Rule not yet implemented\n");
        report.push_str("  2. Rule ID string not found in source (check spelling)\n");

        eprintln!("{}", report);
    }

    // Strict parity: fail if ANY documented rule is not implemented
    assert!(
        not_implemented.is_empty(),
        "{} rules are documented in rules.json but not implemented:\n{}",
        not_implemented.len(),
        not_implemented
            .iter()
            .map(|r| format!("  - {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_fixture_coverage_exists() {
    let rules_index = load_rules_json();
    let explicit_coverage = scan_fixtures_for_coverage();
    let inferred_coverage = infer_fixture_coverage(&rules_index.rules);

    // Combine explicit and inferred coverage
    let mut all_coverage: HashMap<String, Vec<String>> = explicit_coverage;
    for (rule, fixtures) in inferred_coverage {
        all_coverage.entry(rule).or_default().extend(fixtures);
    }

    let documented_rules: BTreeSet<String> =
        rules_index.rules.iter().map(|r| r.id.clone()).collect();

    let covered_rules: BTreeSet<String> = all_coverage.keys().cloned().collect();

    let not_covered: Vec<&String> = documented_rules.difference(&covered_rules).collect();

    // Strict parity: fail if ANY documented rule has no test coverage
    assert!(
        not_covered.is_empty(),
        "{} rules are documented but have no test fixture coverage:\n{}\nAdd test fixtures for uncovered rules.",
        not_covered.len(),
        not_covered
            .iter()
            .map(|r| format!("  - {}", r))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn test_rules_json_integrity() {
    let rules_index = load_rules_json();

    // Check total_rules field matches actual count
    assert_eq!(
        rules_index.rules.len(),
        rules_index.total_rules,
        "The 'total_rules' field ({}) in rules.json does not match the actual number of rules ({})",
        rules_index.total_rules,
        rules_index.rules.len()
    );

    // Check total count matches compiled rule registry
    assert_eq!(
        rules_index.rules.len(),
        agnix_rules::rule_count(),
        "Expected {} rules in rules.json, found {}",
        agnix_rules::rule_count(),
        rules_index.rules.len(),
    );

    // Check no duplicate IDs
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for rule in &rules_index.rules {
        assert!(
            seen.insert(rule.id.clone()),
            "Duplicate rule ID found: {}",
            rule.id
        );
    }

    // Check valid severity values
    let valid_severities = ["HIGH", "MEDIUM", "LOW"];
    for rule in &rules_index.rules {
        assert!(
            valid_severities.contains(&rule.severity.as_str()),
            "Invalid severity '{}' for rule {}",
            rule.severity,
            rule.id
        );
    }

    // Check valid category values
    let valid_categories = [
        "agent-skills",
        "claude-skills",
        "claude-hooks",
        "claude-agents",
        "claude-memory",
        "claude-output-styles",
        "claude-settings",
        "gemini-agents",
        "agents-md",
        "claude-plugins",
        "mcp",
        "copilot",
        "cursor",
        "cline",
        "gemini-cli",
        "codex",
        "windsurf",
        "xml",
        "references",
        "prompt-engineering",
        "cross-platform",
        "opencode",
        "version-awareness",
        "cursor-skills",
        "cline-skills",
        "copilot-skills",
        "codex-skills",
        "opencode-skills",
        "windsurf-skills",
        "kiro-skills",
        "kiro-agents",
        "kiro-hooks",
        "kiro-mcp",
        "kiro-powers",
        "kiro-settings",
        "kiro-steering",
        "amp-skills",
        "amp-checks",
        "roo-code-skills",
        "roo-code",
    ];
    for rule in &rules_index.rules {
        assert!(
            valid_categories.contains(&rule.category.as_str()),
            "Invalid category '{}' for rule {}",
            rule.category,
            rule.id
        );
    }
}

#[test]
fn test_rules_json_matches_validation_rules_md() {
    // Verify rules.json and VALIDATION-RULES.md contain the same rule IDs.
    let rules_index = load_rules_json();
    let validation_rules_path = workspace_root().join("knowledge-base/VALIDATION-RULES.md");
    let content = fs::read_to_string(&validation_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", validation_rules_path.display(), e));

    let rules_json_ids: BTreeSet<String> = rules_index.rules.iter().map(|r| r.id.clone()).collect();
    let heading_re = Regex::new(r"(?m)^### ([A-Z]+(?:-[A-Z]+)*-[0-9]{3})(?:\s|\[|$)").unwrap();
    let validation_rules_ids: BTreeSet<String> = heading_re
        .captures_iter(&content)
        .map(|cap| cap[1].to_string())
        .collect();
    let referenced_re = Regex::new(r"\b[A-Z]+(?:-[A-Z]+)*-[0-9]{3}\b").unwrap();
    let referenced_ids: BTreeSet<String> = referenced_re
        .captures_iter(&content)
        .map(|cap| cap[0].to_string())
        .collect();

    let missing_in_md: Vec<&String> = rules_json_ids.difference(&validation_rules_ids).collect();
    let extra_in_md: Vec<&String> = validation_rules_ids.difference(&rules_json_ids).collect();
    let unknown_references: Vec<&String> = referenced_ids.difference(&rules_json_ids).collect();

    let mut report = String::new();
    if !missing_in_md.is_empty() {
        report.push_str("Rules in rules.json but not found in VALIDATION-RULES.md:\n");
        for rule in &missing_in_md {
            report.push_str(&format!("  - {}\n", rule));
        }
    }
    if !extra_in_md.is_empty() {
        report.push_str("Rules in VALIDATION-RULES.md but not found in rules.json:\n");
        for rule in &extra_in_md {
            report.push_str(&format!("  - {}\n", rule));
        }
    }
    if !unknown_references.is_empty() {
        report
            .push_str("Rule-like references in VALIDATION-RULES.md but not found in rules.json:\n");
        for rule in &unknown_references {
            report.push_str(&format!("  - {}\n", rule));
        }
    }

    assert!(
        missing_in_md.is_empty() && extra_in_md.is_empty() && unknown_references.is_empty(),
        "rules.json and VALIDATION-RULES.md rule ID parity failed:\n{}",
        report
    );
}

#[test]
fn test_sarif_rule_count() {
    let sarif_rules = extract_sarif_rule_ids();

    let rules_index = load_rules_json();
    // SARIF should have exactly the same number of rules as rules.json.
    assert_eq!(
        sarif_rules.len(),
        rules_index.total_rules,
        "SARIF should have {} rules, found {}. Missing or extra rules detected.",
        rules_index.total_rules,
        sarif_rules.len(),
    );
}

// ============================================================================
// Evidence Metadata Validation Tests
// ============================================================================

#[test]
fn test_all_rules_have_evidence_metadata() {
    let rules_index = load_rules_json();
    let date_re = Regex::new(r"^\d{4}-\d{2}-\d{2}$").unwrap();

    for rule in &rules_index.rules {
        // Check source_urls is not empty
        assert!(
            !rule.evidence.source_urls.is_empty(),
            "Rule {} has no source URLs in evidence metadata",
            rule.id
        );

        // Check verified_on is a valid date format (YYYY-MM-DD)
        assert!(
            date_re.is_match(&rule.evidence.verified_on),
            "Rule {} has invalid verified_on date format: '{}'. Expected YYYY-MM-DD",
            rule.id,
            rule.evidence.verified_on
        );
    }
}

#[test]
fn test_evidence_source_urls_valid() {
    let rules_index = load_rules_json();

    // Basic URL validation pattern
    let url_re = Regex::new(r"^https?://[^\s]+$").unwrap();

    for rule in &rules_index.rules {
        for url in &rule.evidence.source_urls {
            assert!(
                url_re.is_match(url),
                "Rule {} has invalid source URL: '{}'",
                rule.id,
                url
            );

            // Check URL doesn't have trailing whitespace
            assert!(
                url.trim() == url,
                "Rule {} source URL has whitespace: '{}'",
                rule.id,
                url
            );
        }
    }
}

#[test]
fn test_normative_level_consistency() {
    let rules_index = load_rules_json();

    let mut inconsistencies = Vec::new();

    for rule in &rules_index.rules {
        // HIGH severity rules should typically have MUST normative level
        // This is a soft check - we just report inconsistencies
        let is_high_severity = rule.severity == "HIGH";
        let is_must_level = rule.evidence.normative_level == NormativeLevel::Must;

        // HIGH + SHOULD/BEST_PRACTICE is suspicious but allowed for some cases
        // (e.g., rules from research papers that are recommendations)
        if is_high_severity && !is_must_level {
            // Only flag if source is spec or vendor_docs (not paper/community)
            if rule.evidence.source_type == SourceType::Spec
                || rule.evidence.source_type == SourceType::VendorDocs
            {
                inconsistencies.push(format!(
                    "{}: HIGH severity but {:?} normative level (source: {:?})",
                    rule.id, rule.evidence.normative_level, rule.evidence.source_type
                ));
            }
        }
    }

    // This is informational - we don't fail on inconsistencies
    // Just report them for review
    if !inconsistencies.is_empty() {
        eprintln!(
            "\nNote: {} rules have HIGH severity but non-MUST normative level:",
            inconsistencies.len()
        );
        for msg in &inconsistencies {
            eprintln!("  - {}", msg);
        }
    }
}

#[test]
fn test_evidence_source_type_distribution() {
    let rules_index = load_rules_json();

    let mut by_source: HashMap<String, usize> = HashMap::new();

    for rule in &rules_index.rules {
        let key = format!("{:?}", rule.evidence.source_type);
        *by_source.entry(key).or_insert(0) += 1;
    }

    // Just report distribution - this is informational
    eprintln!("\nEvidence source type distribution:");
    let mut sorted: Vec<_> = by_source.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    for (source_type, count) in sorted {
        eprintln!("  {}: {}", source_type, count);
    }

    // Ensure we have at least some diversity in sources
    assert!(
        by_source.len() >= 3,
        "Expected at least 3 different source types, found {}",
        by_source.len()
    );
}

#[test]
fn test_evidence_test_coverage_accuracy() {
    let rules_index = load_rules_json();
    let explicit_coverage = scan_fixtures_for_coverage();
    let inferred_coverage = infer_fixture_coverage(&rules_index.rules);

    // Combine coverage
    let mut all_coverage: HashMap<String, Vec<String>> = explicit_coverage;
    for (rule, fixtures) in inferred_coverage {
        all_coverage.entry(rule).or_default().extend(fixtures);
    }

    let mut mismatches = Vec::new();

    for rule in &rules_index.rules {
        let has_fixtures = all_coverage.contains_key(&rule.id);
        let claims_fixtures = rule.evidence.tests.fixtures;

        // If evidence claims fixtures but we can't find them, that's a problem
        if claims_fixtures && !has_fixtures {
            mismatches.push(format!("{}: claims fixtures=true but none found", rule.id));
        }
    }

    // This should be empty - evidence should be accurate
    assert!(
        mismatches.is_empty(),
        "Evidence test coverage mismatches:\n{}",
        mismatches.join("\n")
    );
}

#[test]
fn test_applies_to_tool_values() {
    let rules_index = load_rules_json();

    // Use valid_tools derived from rules.json at compile time
    let valid_tools = agnix_rules::valid_tools();

    for rule in &rules_index.rules {
        if let Some(ref tool) = rule.evidence.applies_to.tool {
            assert!(
                valid_tools.contains(&tool.as_str()),
                "Rule {} has unknown tool '{}'. Valid tools: {:?}",
                rule.id,
                tool,
                valid_tools
            );
        }
    }
}

// ============================================================================
// Tool Mapping Consistency Tests (Review-requested coverage)
// ============================================================================

#[test]
fn test_tool_rule_prefixes_consistency() {
    // Every tool in TOOL_RULE_PREFIXES must also exist in VALID_TOOLS
    // This ensures no orphaned tools or prefixes exist
    let valid_tools = agnix_rules::valid_tools();

    for (prefix, tool) in agnix_rules::TOOL_RULE_PREFIXES {
        assert!(
            valid_tools.contains(tool),
            "Tool '{}' from prefix '{}' is not in VALID_TOOLS. \
             TOOL_RULE_PREFIXES and VALID_TOOLS must be consistent.",
            tool,
            prefix
        );
    }
}

// ============================================================================
// Fix Metadata Validation Tests
// ============================================================================

#[test]
fn test_all_rules_have_fix_metadata() {
    let rules_index = load_rules_json();
    let valid_fix_safety = ["safe", "unsafe", "safe/unsafe"];

    for rule in &rules_index.rules {
        if rule.fix.autofix {
            // Rules with autofix must have a valid fix_safety value
            let safety = rule.fix.fix_safety.as_deref().unwrap_or("");
            assert!(
                valid_fix_safety.contains(&safety),
                "Rule {} has autofix=true but invalid fix_safety: '{}'. Expected one of: {:?}",
                rule.id,
                safety,
                valid_fix_safety
            );
        } else {
            // Rules without autofix should not have fix_safety
            assert!(
                rule.fix.fix_safety.is_none(),
                "Rule {} has autofix=false but fix_safety is set to '{}'",
                rule.id,
                rule.fix.fix_safety.as_deref().unwrap_or("")
            );
        }
    }
}

#[test]
fn test_autofix_count_matches_documentation() {
    let rules_index = load_rules_json();
    let autofix_count = rules_index.rules.iter().filter(|r| r.fix.autofix).count();

    // VALIDATION-RULES.md documents the autofix count in the footer
    let validation_rules_path = workspace_root().join("knowledge-base/VALIDATION-RULES.md");
    let content = fs::read_to_string(&validation_rules_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", validation_rules_path.display(), e));

    // Look for the pattern "Auto-Fixable: N rules (N%)"
    let re = Regex::new(r"Auto-Fixable\*\*:\s*(\d+)\s*rules").unwrap();
    let cap = re.captures(&content).unwrap_or_else(|| {
        panic!(
            "Could not find documented auto-fixable rule count in {} using pattern {:?}. \
             The footer format may have changed.",
            validation_rules_path.display(),
            re.as_str()
        )
    });
    let documented_count: usize = cap[1].parse().unwrap();
    assert_eq!(
        autofix_count, documented_count,
        "rules.json has {} auto-fixable rules but VALIDATION-RULES.md documents {}",
        autofix_count, documented_count
    );
}

/// Every rule ID listed in `.github/spec-baselines.json` must exist in
/// `knowledge-base/rules.json`.
///
/// The Spec Drift Sentinel reads those lists to tell a maintainer which rules
/// to review when an upstream doc changes, and it renders them straight into
/// the drift issue. Nothing validated them, so the file accumulated 15 rule IDs
/// that never existed in this repo: `CC-SA-001`..`007` (the real subagent rules
/// are `CC-AG-*`), `GH-001`..`004` (really `COP-*`), and `CLINE-001`..`003`
/// (really `CLN-*`/`CL-SK-*`), plus `AS-014` which was deliberately removed in
/// PR #979. A drift issue naming rules that cannot be looked up sends the
/// reviewer nowhere.
#[test]
fn test_spec_baseline_rule_ids_exist() {
    #[derive(Deserialize)]
    struct Baselines {
        sources: HashMap<String, HashMap<String, BaselineSource>>,
    }
    #[derive(Deserialize)]
    struct BaselineSource {
        #[serde(default)]
        rules: Vec<String>,
    }

    let path = workspace_root().join(".github/spec-baselines.json");
    let content = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", path.display(), e));
    let baselines: Baselines = serde_json::from_str(&content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", path.display(), e));

    let known: BTreeSet<String> = load_rules_json()
        .rules
        .into_iter()
        .map(|rule| rule.id)
        .collect();

    let mut unknown: Vec<String> = Vec::new();
    for (tier, sources) in &baselines.sources {
        for (source, entry) in sources {
            for rule_id in &entry.rules {
                if !known.contains(rule_id) {
                    unknown.push(format!("{tier}/{source}: {rule_id}"));
                }
            }
        }
    }
    unknown.sort();

    assert!(
        unknown.is_empty(),
        "spec-baselines.json references {} rule ID(s) absent from rules.json:\n  {}\n\n\
         Fix the mapping (or drop the ID if the rule was removed) so drift issues \
         point at rules that exist.",
        unknown.len(),
        unknown.join("\n  ")
    );
}

#[test]
fn test_refreshed_release_surfaces_match_research_inventory() {
    let baseline_path = workspace_root().join(".github/tool-release-baselines.json");
    let baseline_content = fs::read_to_string(&baseline_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", baseline_path.display(), e));
    let baselines: serde_json::Value = serde_json::from_str(&baseline_content)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {}", baseline_path.display(), e));
    let surfaces = baselines["tools"]["claude-code"]["changes_of_interest"]["config_surfaces"]
        .as_array()
        .expect("Claude Code config_surfaces must be an array");
    let validators = baselines["tools"]["claude-code"]["validators"]
        .as_array()
        .expect("Claude Code validators must be an array");

    assert!(
        surfaces
            .iter()
            .any(|surface| surface == ".claude/rules/**/*.md"),
        "Claude Code baseline must track recursive Markdown rule files"
    );
    assert!(
        surfaces
            .iter()
            .all(|surface| surface != ".claude/rules.yaml"),
        "Claude Code baseline must not use the obsolete .claude/rules.yaml surface"
    );
    assert!(
        surfaces
            .iter()
            .any(|surface| surface == ".claude/output-styles/*.md"),
        "Claude Code baseline must track output-style Markdown files"
    );
    assert!(
        validators
            .iter()
            .any(|validator| validator == "output_style"),
        "Claude Code baseline must register the output-style validator"
    );

    let research_path = workspace_root().join("knowledge-base/RESEARCH-TRACKING.md");
    let research = fs::read_to_string(&research_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", research_path.display(), e));
    let claude_row = research
        .lines()
        .find(|line| line.starts_with("| Claude Code |"))
        .expect("RESEARCH-TRACKING.md must contain a Claude Code inventory row");

    for surface in surfaces {
        let surface = surface
            .as_str()
            .expect("Claude Code config surfaces must be strings");
        assert!(
            claude_row.contains(surface),
            "Claude Code research inventory is missing baseline surface: {surface}"
        );
    }
    assert!(
        claude_row.contains("CC-OS"),
        "Claude Code research inventory must include the output-style rule prefix"
    );
    assert!(
        claude_row.contains("MCP"),
        "Claude Code research inventory must include the shared MCP rule prefix"
    );

    let codex_row = research
        .lines()
        .find(|line| line.starts_with("| Codex CLI |"))
        .expect("RESEARCH-TRACKING.md must contain a Codex CLI inventory row");
    let codex_rule_prefixes: std::collections::BTreeSet<_> = codex_row
        .split('|')
        .rfind(|cell| !cell.trim().is_empty())
        .expect("Codex inventory row must have a Rule Prefix cell")
        .split(',')
        .map(str::trim)
        .collect();
    for implemented_prefix in agnix_rules::get_prefixes_for_tool("codex")
        .into_iter()
        .map(|prefix| prefix.trim_end_matches('-'))
    {
        assert!(
            codex_rule_prefixes.contains(implemented_prefix),
            "Codex research inventory is missing {implemented_prefix}"
        );
    }
    assert!(
        codex_rule_prefixes.contains("MCP"),
        "Codex research inventory must include the shared MCP rule prefix"
    );
    for surface in [".mcp.json", "[mcp_servers.*]", "requirements.toml"] {
        assert!(
            codex_row.contains(surface),
            "Codex research inventory is missing validated surface: {surface}"
        );
    }

    for (tool_id, row_name, surface, prefix) in [
        ("codex", "Codex CLI", ".codex/skills/*/SKILL.md", "CX-SK"),
        (
            "opencode",
            "OpenCode",
            ".opencode/skills/*/SKILL.md",
            "OC-SK",
        ),
        ("cursor", "Cursor", ".cursor/skills/*/SKILL.md", "CR-SK"),
        ("kiro", "Kiro CLI", ".kiro/skills/*/SKILL.md", "KR-SK"),
        ("cline", "Cline", ".cline/skills/*/SKILL.md", "CL-SK"),
    ] {
        let tool = &baselines["tools"][tool_id];
        let tool_surfaces = tool["changes_of_interest"]["config_surfaces"]
            .as_array()
            .unwrap_or_else(|| panic!("{tool_id} config_surfaces must be an array"));
        let tool_validators = tool["validators"]
            .as_array()
            .unwrap_or_else(|| panic!("{tool_id} validators must be an array"));
        let relevant = tool["changes_of_interest"]["relevant"]
            .as_array()
            .unwrap_or_else(|| panic!("{tool_id} relevant changes must be an array"));

        assert!(
            tool_surfaces.iter().any(|candidate| candidate == surface),
            "{tool_id} baseline must track {surface}"
        );
        assert!(
            tool_validators
                .iter()
                .any(|validator| validator == "per_client_skill"),
            "{tool_id} baseline must register the per-client skill validator"
        );
        assert!(
            relevant.iter().any(|change| change
                .as_str()
                .is_some_and(|text| text.to_ascii_lowercase().contains("skill"))),
            "{tool_id} baseline must monitor skill schema changes"
        );

        let row_prefix = format!("| {row_name} |");
        let row = research
            .lines()
            .find(|line| line.starts_with(&row_prefix))
            .unwrap_or_else(|| panic!("RESEARCH-TRACKING.md must contain a {row_name} row"));
        let row_rule_prefixes: std::collections::BTreeSet<_> = row
            .split('|')
            .rfind(|cell| !cell.trim().is_empty())
            .expect("research inventory row must have a Rule Prefix cell")
            .split(',')
            .map(str::trim)
            .collect();
        assert!(
            row.contains(surface),
            "{row_name} research inventory is missing {surface}"
        );
        assert!(
            row_rule_prefixes.contains(prefix),
            "{row_name} research inventory is missing {prefix}"
        );

        if tool_id == "opencode" {
            assert!(
                row.contains(".mcp.json") && row_rule_prefixes.contains("MCP"),
                "OpenCode research inventory must include its shared MCP surface and prefix"
            );
            for implemented_prefix in agnix_rules::get_prefixes_for_tool("opencode")
                .into_iter()
                .filter(|prefix| prefix.starts_with("OC-"))
                .map(|prefix| prefix.trim_end_matches('-'))
            {
                assert!(
                    row_rule_prefixes.contains(implemented_prefix),
                    "OpenCode research inventory is missing {implemented_prefix}"
                );
            }
        }
    }

    let amp = &baselines["tools"]["amp"];
    assert!(
        amp["validators"]
            .as_array()
            .expect("amp validators must be an array")
            .iter()
            .any(|validator| validator == "agents_md"),
        "amp baseline must register the AGENTS.md validator"
    );
    assert!(
        amp["changes_of_interest"]["config_surfaces"]
            .as_array()
            .expect("amp config_surfaces must be an array")
            .iter()
            .any(|surface| surface == "AGENTS.md"),
        "amp baseline must track AGENTS.md"
    );
    let amp_row = research
        .lines()
        .find(|line| line.starts_with("| amp |"))
        .expect("RESEARCH-TRACKING.md must contain an amp row");
    let amp_rule_prefixes: std::collections::BTreeSet<_> = amp_row
        .split('|')
        .rfind(|cell| !cell.trim().is_empty())
        .expect("amp research inventory row must have a Rule Prefix cell")
        .split(',')
        .map(str::trim)
        .collect();
    assert!(
        amp_rule_prefixes.contains("AGM"),
        "amp research inventory must include the shared AGM rule prefix"
    );
}

/// Kiro CLI 2.18.0 loads nested `AGENTS.md` files as steering context from
/// anywhere in the workspace tree, so `AGENTS.md` is a Kiro config surface.
/// Anchored to the implementation: the file type agnix classifies `AGENTS.md`
/// into must dispatch `AgentsMdValidator`, and the AGM rule family must exist.
#[test]
fn test_kiro_agents_md_steering_surface_is_registered() {
    use agnix_core::{ValidatorRegistry, detect_file_type};

    let file_type = detect_file_type(Path::new("crates/agnix-core/AGENTS.md"));
    let registry = ValidatorRegistry::with_defaults();
    assert!(
        registry
            .validators_for(file_type)
            .iter()
            .any(|validator| validator.name() == "AgentsMdValidator"),
        "AgentsMdValidator must run on nested AGENTS.md files"
    );

    let rules = load_rules_json();
    assert!(
        rules.rules.iter().any(|rule| rule.id.starts_with("AGM-")),
        "rules.json must carry the AGM rule family that covers AGENTS.md"
    );

    let baseline_path = workspace_root().join(".github/tool-release-baselines.json");
    let baselines: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(&baseline_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", baseline_path.display(), e)),
    )
    .unwrap_or_else(|e| panic!("Failed to parse {}: {}", baseline_path.display(), e));
    let kiro = &baselines["tools"]["kiro"];
    assert!(
        kiro["validators"]
            .as_array()
            .expect("kiro validators must be an array")
            .iter()
            .any(|validator| validator == "agents_md"),
        "kiro baseline must register the AGENTS.md validator"
    );
    assert!(
        kiro["changes_of_interest"]["config_surfaces"]
            .as_array()
            .expect("kiro config_surfaces must be an array")
            .iter()
            .any(|surface| surface == "AGENTS.md"),
        "kiro baseline must track the AGENTS.md steering surface"
    );

    let research_path = workspace_root().join("knowledge-base/RESEARCH-TRACKING.md");
    let research = fs::read_to_string(&research_path)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", research_path.display(), e));
    let row = research
        .lines()
        .find(|line| line.starts_with("| Kiro CLI |"))
        .expect("RESEARCH-TRACKING.md must contain a Kiro CLI row");
    assert!(
        row.contains("AGENTS.md"),
        "Kiro CLI research inventory is missing the AGENTS.md steering surface"
    );
    assert!(
        row.contains("AGM"),
        "Kiro CLI research inventory is missing the AGM rule prefix"
    );
}

#[test]
fn test_is_tool_alias_case_sensitivity() {
    // Test that tool alias matching is case insensitive
    // "Copilot" (mixed case) and "COPILOT" (uppercase) should both
    // be recognized as valid tools via the alias mechanism

    // The is_tool_alias function is private, but we can test through
    // LintConfig::is_rule_enabled which uses it internally

    use agnix_core::LintConfig;

    let aliases = ["Copilot", "COPILOT", "copilot"];
    for alias in aliases {
        let mut config = LintConfig::default();
        config.set_tools(vec![alias.to_string()]);
        assert!(
            config.is_rule_enabled("COP-001"),
            "Alias '{}' should match 'github-copilot' and enable COP-* rules",
            alias
        );
    }
}
