//! PE-006 applies to native AGENTS.md when Claude Code is explicitly selected.
use agnix_core::{Diagnostic, DiagnosticLevel, LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

fn pe006(path: &str, content: &str, config: &str) -> Vec<Diagnostic> {
    let config = toml::from_str::<LintConfig>(config).expect("valid configuration");
    validate_content(
        Path::new(path),
        content,
        &config,
        &ValidatorRegistry::with_defaults(),
    )
    .into_iter()
    .filter(|diagnostic| diagnostic.rule == "PE-006")
    .collect()
}

#[test]
fn reporter_mixed_tools_enable_claude_advice_on_agents_md() {
    let findings = pe006(
        "AGENTS.md",
        "do not fart\n",
        r#"tools = ["claude-code", "opencode", "generic"]"#,
    );
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(findings[0].level, DiagnosticLevel::Warning);
    assert!(findings[0].message.contains("Claude"));
}

#[test]
fn explicit_claude_tools_and_legacy_target_enable_native_agents_md() {
    for config in [
        r#"tools = ["claude-code"]"#,
        r#"tools = ["CLAUDE-CODE"]"#,
        r#"tools = ["claude-code", "gemini-cli"]"#,
        r#"target = "ClaudeCode""#,
    ] {
        for path in [
            "AGENTS.md",
            "nested/AGENTS.md",
            ".claude/AGENTS.md",
            ".agents-like/AGENTS.md",
        ] {
            let findings = pe006(path, "Never use global variables.\n", config);
            assert_eq!(findings.len(), 1, "{path}, {config}: {findings:?}");
        }
    }
}

#[test]
fn shared_agents_md_without_a_claude_target_keeps_existing_behavior() {
    for config in [
        "",
        r#"tools = ["generic"]"#,
        r#"tools = ["opencode", "generic"]"#,
        r#"tools = ["gemini-cli"]"#,
        r#"target = "Codex""#,
        "target = \"ClaudeCode\"\ntools = [\"opencode\"]",
    ] {
        assert!(
            pe006("AGENTS.md", "Never use global variables.\n", config).is_empty(),
            "{config}"
        );
    }
}

#[test]
fn claude_target_does_not_claim_other_agents_variants_or_gemini_files() {
    for path in [
        "AGENTS.local.md",
        "AGENTS.override.md",
        "GEMINI.md",
        ".agents/checks/AGENTS.md",
        "nested/.agents/AGENTS.md",
    ] {
        assert!(
            pe006(
                path,
                "Never use global variables.\n",
                r#"tools = ["claude-code"]"#
            )
            .is_empty(),
            "{path}"
        );
    }
}

#[test]
fn explicit_disabling_and_prompt_category_are_respected() {
    for config in [
        "tools = [\"claude-code\"]\n[rules]\ndisabled_rules = [\"PE-006\"]",
        "tools = [\"claude-code\"]\n[rules]\nprompt_engineering = false",
        "tools = [\"claude-code\"]\n[[overrides]]\npaths = [\"AGENTS.md\"]\ndisabled_rules = [\"PE-006\"]",
    ] {
        assert!(
            pe006("AGENTS.md", "Never use global variables.\n", config).is_empty(),
            "{config}"
        );
    }
}

#[test]
fn alternatives_and_reasons_still_satisfy_the_rule() {
    for content in [
        "Never use global variables. Instead, pass function parameters.\n",
        "Never use global variables because tests share the same process.\n",
    ] {
        assert!(
            pe006("AGENTS.md", content, r#"tools = ["claude-code"]"#).is_empty(),
            "{content}"
        );
    }
}

#[test]
fn claude_memory_files_keep_existing_coverage() {
    for path in ["CLAUDE.md", "CLAUDE.local.md"] {
        assert_eq!(
            pe006(path, "Never use global variables.\n", "").len(),
            1,
            "{path}"
        );
    }
}
