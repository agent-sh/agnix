use super::*;
use crate::config::LintConfig;
use crate::fs::RealFileSystem;
use crate::schemas::skill::{VALID_EFFORT_LEVELS, VALID_MODEL_ALIASES};
use std::fs;

#[test]
fn test_valid_skill() {
    let content = r#"---
name: test-skill
description: Use when testing skill validation
---
Skill body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.is_empty());
}

#[test]
fn test_invalid_skill_name() {
    let content = r#"---
name: Test-Skill
description: Use when validating skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004_errors.len(), 1);
}

#[test]
fn test_as_001_missing_frontmatter() {
    let content = include_str!("../../../../../tests/fixtures/skills/missing-frontmatter/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_001_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
    assert_eq!(as_001_errors.len(), 1);
}

#[test]
fn test_as_002_missing_name() {
    let content = r#"---
description: Use when validating missing name
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_002_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-002").collect();
    assert_eq!(as_002_errors.len(), 1);
}

#[test]
fn test_as_003_missing_description() {
    let content = r#"---
name: test-skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
    assert_eq!(as_003_errors.len(), 1);
}

#[test]
fn test_as_004_invalid_name_format() {
    let content = r#"---
name: bad_name
description: Use when validating name format
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004_errors.len(), 1);
}

#[test]
fn test_as_004_plugin_prefixed_name_format_ok() {
    let content = r#"---
name: build-web-apps:frontend-app-builder
description: Use when validating plugin-prefixed skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_004_plugin_prefixed_name_uses_segment_length() {
    let content = r#"---
name: very-long-plugin-prefix-for-packaged-skills:frontend-app-builder-with-extra-context
description: Use when validating plugin-prefixed skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_004_plugin_prefixed_invalid_skill_segment() {
    let content = r#"---
name: build-web-apps:Frontend_App_Builder
description: Use when validating plugin-prefixed skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004_errors.len(), 1);
}

#[test]
fn test_as_017_name_directory_mismatch() {
    let content = r#"---
name: deploy-skill
description: Use when validating directory name matching
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new("code-review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let as_017_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-017").collect();
    assert_eq!(as_017_errors.len(), 1);
}

#[test]
fn test_as_017_name_directory_match_ok() {
    let content = r#"---
name: code-review
description: Use when validating directory name matching
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new("code-review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let as_017_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-017").collect();
    assert_eq!(as_017_errors.len(), 0);
}

#[test]
fn test_as_017_plugin_prefixed_name_matches_local_directory() {
    let content = r#"---
name: enhance:code-review
description: Use when validating directory name matching
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new("code-review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-017"));
}

#[test]
fn test_as_017_plugin_prefixed_name_still_checks_local_directory() {
    let content = r#"---
name: enhance:deploy-skill
description: Use when validating directory name matching
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new("code-review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let as_017_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-017").collect();
    assert_eq!(as_017_errors.len(), 1);
}

#[test]
fn test_as_008_description_too_long() {
    let long_description = "a".repeat(1025);
    let content = format!(
        "---\nname: test-skill\ndescription: {}\n---\nBody",
        long_description
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert_eq!(as_008_errors.len(), 1);
}

#[test]
fn test_as_008_description_counts_characters_not_bytes() {
    let description = "測".repeat(400);
    let content = format!(
        "---\nname: test-skill\ndescription: {}\n---\nBody",
        description
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-008"));
}

#[test]
fn test_as_008_description_empty_string() {
    let content = r#"---
name: test-skill
description: ""
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_003_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
    assert_eq!(as_003_errors.len(), 0);

    let as_008_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert_eq!(as_008_errors.len(), 1);
}

#[test]
fn test_as_009_angle_brackets_codex_only() {
    // AS-009 (angle brackets in description) is enforced ONLY by Codex
    // (its quick_validate.py rejects `<`/`>`). agentskills.io and Claude
    // Code impose no such restriction.
    let content = r#"---
name: test-skill
description: Use when validating <xml> tags
---
Body"#;
    let validator = SkillValidator;
    let cfg = LintConfig::default();

    // Codex skill (.agents/skills/) -> fires.
    let codex = validator.validate(
        Path::new(".agents/skills/test-skill/SKILL.md"),
        content,
        &cfg,
    );
    assert_eq!(
        codex.iter().filter(|d| d.rule == "AS-009").count(),
        1,
        "AS-009 should fire for a Codex skill"
    );

    // Claude skill (.claude/skills/) -> does NOT fire (reporter's false positive).
    let claude = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &cfg,
    );
    assert_eq!(
        claude.iter().filter(|d| d.rule == "AS-009").count(),
        0,
        "AS-009 must not fire for a Claude Code skill"
    );

    // Generic skill (no client marker) -> does NOT fire.
    let generic = validator.validate(Path::new("skills/test-skill/SKILL.md"), content, &cfg);
    assert_eq!(
        generic.iter().filter(|d| d.rule == "AS-009").count(),
        0,
        "AS-009 must not fire for a generic skill"
    );

    // Codex rejects ANY angle bracket, not just tag-shaped pairs: a lone `<`
    // must fire too (matches quick_validate.py's `"<" in description`).
    let lone = "---\nname: test-skill\ndescription: Use when a < b comparison holds\n---\nBody";
    let codex_lone =
        validator.validate(Path::new(".agents/skills/test-skill/SKILL.md"), lone, &cfg);
    assert_eq!(
        codex_lone.iter().filter(|d| d.rule == "AS-009").count(),
        1,
        "AS-009 must fire on a lone angle bracket for a Codex skill"
    );
}

#[test]
fn test_as_008_claude_allows_1536() {
    // agentskills.io caps description at 1024; Claude Code allows up to 1536.
    // Reproduces the reporter's false positive (#957): a 1025-char description
    // on a Claude skill was wrongly flagged.
    let desc_1025 = "a".repeat(1025);
    let content = format!("---\nname: test-skill\ndescription: {desc_1025}\n---\nBody");
    let validator = SkillValidator;

    // Claude skill via tools=["claude-code"] scope -> 1025 is fine.
    let mut claude_cfg = LintConfig::default();
    claude_cfg.set_tools(vec!["claude-code".to_string()]);
    let claude = validator.validate(
        Path::new("skills/test-skill/SKILL.md"),
        &content,
        &claude_cfg,
    );
    assert_eq!(
        claude.iter().filter(|d| d.rule == "AS-008").count(),
        0,
        "1025-char description must be accepted for a Claude Code skill"
    );

    // Generic (agentskills.io) -> 1025 exceeds the 1024 baseline, flagged.
    let generic = validator.validate(
        Path::new("skills/test-skill/SKILL.md"),
        &content,
        &LintConfig::default(),
    );
    assert_eq!(
        generic.iter().filter(|d| d.rule == "AS-008").count(),
        1,
        "1025-char description must be flagged for a generic skill"
    );

    // Claude still rejects beyond 1536.
    let desc_1537 = "a".repeat(1537);
    let content2 = format!("---\nname: test-skill\ndescription: {desc_1537}\n---\nBody");
    let claude_over = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        &content2,
        &LintConfig::default(),
    );
    assert_eq!(
        claude_over.iter().filter(|d| d.rule == "AS-008").count(),
        1,
        "1537-char description must be flagged even for Claude"
    );
}

#[test]
fn test_as_011_compatibility_too_long() {
    let long_compat = "b".repeat(501);
    let content = format!(
        "---\nname: test-skill\ndescription: Use when validating compatibility\ncompatibility: {}\n---\nBody",
        long_compat
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_011_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-011").collect();
    assert_eq!(as_011_errors.len(), 1);
}

#[test]
fn test_as_011_compatibility_counts_characters_not_bytes() {
    let compatibility = "測".repeat(500);
    let content = format!(
        "---\nname: test-skill\ndescription: Use when validating compatibility\ncompatibility: {}\n---\nBody",
        compatibility
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-011"));
}

#[test]
fn test_as_012_content_too_long() {
    let body = (0..501).map(|_| "line").collect::<Vec<_>>().join("\n");
    let content = format!(
        "---\nname: test-skill\ndescription: Use when validating content length\n---\n{}",
        body
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let as_012_warnings: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-012").collect();
    assert_eq!(as_012_warnings.len(), 1);
}

#[test]
fn test_as_013_reference_too_deep() {
    let content = include_str!("../../../../../tests/fixtures/skills/deep-reference/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
    assert_eq!(as_013_errors.len(), 1);
}

#[test]
fn test_as_013_reference_single_name_too_deep() {
    let content = r#"---
name: deep-reference
description: Use when validating deep references
---

See reference/deep/guide.md for details."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let as_013_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
    assert_eq!(as_013_errors.len(), 1);
}

#[test]
fn test_as_015_directory_size_exceeds() {
    use std::io::Write;

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_dir = temp_dir.path().join("big-skill");
    fs::create_dir_all(&skill_dir).unwrap();

    let skill_path = skill_dir.join("SKILL.md");
    let mut skill_file = fs::File::create(&skill_path).unwrap();
    writeln!(
        skill_file,
        "---\nname: big-skill\ndescription: Use when validating directory size\n---\nBody"
    )
    .unwrap();

    let big_file_path = skill_dir.join("big.bin");
    let big_payload = vec![0u8; 8 * 1024 * 1024 + 1];
    fs::write(&big_file_path, big_payload).unwrap();

    let content = fs::read_to_string(&skill_path).unwrap();
    let validator = SkillValidator;
    let diagnostics = validator.validate(&skill_path, &content, &LintConfig::default());

    let as_015_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-015").collect();
    assert_eq!(as_015_errors.len(), 1);
}

#[test]
fn test_cc_sk_006_dangerous_name_without_safety() {
    let content = r#"---
name: deploy-prod
description: Deploys to production
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should have an error for CC-SK-006
    let cc_sk_006_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(cc_sk_006_errors.len(), 1);
    assert_eq!(
        cc_sk_006_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_cc_sk_006_dangerous_name_with_safety() {
    let content = r#"---
name: deploy-prod
description: Deploys to production
disable-model-invocation: true
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have an error for CC-SK-006
    let cc_sk_006_errors: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(cc_sk_006_errors.len(), 0);
}

#[test]
fn test_cc_sk_006_covers_all_dangerous_names() {
    let dangerous_names = vec!["deploy", "ship", "publish", "delete", "release", "push"];

    for name in dangerous_names {
        let content = format!(
            r#"---
name: {}-prod
description: A dangerous skill
---
Body"#,
            name
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        // Should have an error for CC-SK-006
        let cc_sk_006_errors: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-006")
            .collect();

        assert_eq!(
            cc_sk_006_errors.len(),
            1,
            "Expected CC-SK-006 error for name: {}",
            name
        );
    }
}

#[test]
fn test_cc_sk_007_unrestricted_bash() {
    let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should have a warning for CC-SK-007
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 1);
    assert_eq!(
        cc_sk_007_warnings[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_cc_sk_007_scoped_bash_ok() {
    let content = r#"---
name: git-helper
description: Git operations helper
allowed-tools: Bash(git:*) Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have a warning for CC-SK-007 (scoped Bash is ok)
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 0);
}

#[test]
fn test_cc_sk_007_no_bash() {
    let content = r#"---
name: reader
description: File reader
allowed-tools: Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Should NOT have a warning for CC-SK-007 (no Bash at all)
    let cc_sk_007_warnings: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007_warnings.len(), 0);
}

// ===== CC-SK-007 Auto-fix Tests =====

#[test]
fn test_cc_sk_007_has_fix() {
    let content = r#"---
name: git-helper
description: Use when doing git operations
allowed-tools: Bash Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007.len(), 1);
    assert!(cc_sk_007[0].has_fixes());

    let fix = &cc_sk_007[0].fixes[0];
    assert_eq!(fix.replacement, "Bash(git:*)");
    assert!(!fix.safe); // Not safe, we don't know user's intended scope
}

#[test]
fn test_cc_sk_007_fix_correct_byte_position() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    assert_eq!(cc_sk_007.len(), 1);
    assert!(cc_sk_007[0].has_fixes());

    let fix = &cc_sk_007[0].fixes[0];

    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("Bash(git:*)"));
    assert!(!fixed.contains("allowed-tools: Bash "));
}

#[test]
fn test_cc_sk_007_multiple_bash_multiple_fixes() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash Read Bash
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Each Bash occurrence generates a warning
    assert_eq!(cc_sk_007.len(), 2);
    // Each should have a fix
    assert!(cc_sk_007[0].has_fixes());
    assert!(cc_sk_007[1].has_fixes());
}

#[test]
fn test_cc_sk_007_scoped_bash_no_fix() {
    let content = r#"---
name: helper
description: Use when helping
allowed-tools: Bash(git:*) Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Scoped Bash doesn't trigger the warning
    assert_eq!(cc_sk_007.len(), 0);
}

#[test]
fn test_find_plain_bash_positions() {
    let content = "allowed-tools: Bash Read Bash(git:*) Write Bash";
    let positions = find_plain_bash_positions(content, 0);

    // Should find 2: "Bash" at position 15 and "Bash" at position 43
    // But NOT "Bash(git:*)"
    assert_eq!(positions.len(), 2);
    assert_eq!(&content[positions[0].0..positions[0].1], "Bash");
    assert_eq!(&content[positions[1].0..positions[1].1], "Bash");
}

#[test]
fn test_find_plain_bash_positions_none() {
    let content = "allowed-tools: Bash(git:*) Bash(npm:*) Read";
    let positions = find_plain_bash_positions(content, 0);
    assert_eq!(positions.len(), 0);
}

#[test]
fn test_as_005_leading_hyphen() {
    let content = r#"---
name: -bad-name
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

    assert_eq!(as_005_errors.len(), 1);
    assert_eq!(
        as_005_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_as_005_trailing_hyphen() {
    let content = r#"---
name: bad-name-
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_005_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();

    assert_eq!(as_005_errors.len(), 1);
    assert_eq!(
        as_005_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_as_006_consecutive_hyphens() {
    let content = r#"---
name: bad--name
description: Use when testing validation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_006_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-006").collect();

    assert_eq!(as_006_errors.len(), 1);
    assert_eq!(
        as_006_errors[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

// ===== CC-SK-001: Invalid Model Value =====

#[test]
fn test_cc_sk_001_invalid_model() {
    let content = r#"---
name: test-skill
description: Use when testing
model: gpt-4
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_001: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-001")
        .collect();

    assert_eq!(cc_sk_001.len(), 1);
    assert_eq!(
        cc_sk_001[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_001[0].message.contains("gpt-4"));
}

#[test]
fn test_cc_sk_001_valid_models() {
    for model in &["sonnet", "opus", "haiku", "inherit"] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
model: {}
---
Body"#,
            model
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();

        assert_eq!(cc_sk_001.len(), 0, "Model '{}' should be valid", model);
    }
}

#[test]
fn test_cc_sk_001_no_model_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_001: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-001")
        .collect();

    assert_eq!(cc_sk_001.len(), 0);
}

// ===== CC-SK-002: Invalid Context Value =====

#[test]
fn test_cc_sk_002_invalid_context() {
    let content = r#"---
name: test-skill
description: Use when testing
context: split
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 1);
    assert_eq!(
        cc_sk_002[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_002[0].message.contains("split"));
}

#[test]
fn test_cc_sk_002_valid_context_fork() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 0);
}

#[test]
fn test_cc_sk_002_no_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();

    assert_eq!(cc_sk_002.len(), 0);
}

/// `context: fork` without `agent` is valid: "The `agent` field specifies
/// which subagent configuration to use... If omitted, uses `general-purpose`."
/// CC-SK-003 used to error here and shipped an autofix inserting
/// `agent: general-purpose`, restating a default the tool already applies.
#[test]
fn test_context_fork_without_agent_is_valid() {
    let content = r#"---
name: test-skill
description: Use when testing fork context without an explicit agent
context: fork
---
Run the test suite and summarize the failures."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.is_empty(),
        "`context: fork` defaults `agent` to general-purpose and must validate clean, got: {diagnostics:?}"
    );
}

// ===== CC-SK-004: Agent Without Context =====

#[test]
fn test_cc_sk_004_agent_without_context() {
    let content = r#"---
name: test-skill
description: Use when testing
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 1);
    assert_eq!(
        cc_sk_004[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
}

#[test]
fn test_cc_sk_004_agent_with_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 0);
}

#[test]
fn test_cc_sk_004_no_agent_no_context_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_004: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-004")
        .collect();

    assert_eq!(cc_sk_004.len(), 0);
}

// ===== CC-SK-005: Invalid Agent Type =====

#[test]
fn test_cc_sk_005_invalid_agent() {
    let content = r#"---
name: test-skill
description: Use when testing
context: fork
agent: CustomAgent
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 1);
    assert_eq!(
        cc_sk_005[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_005[0].message.contains("CustomAgent"));
}

#[test]
fn test_cc_sk_005_valid_agents() {
    for agent in &["Explore", "Plan", "general-purpose"] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(cc_sk_005.len(), 0, "Agent '{}' should be valid", agent);
    }
}

#[test]
fn test_cc_sk_005_valid_custom_agents() {
    // Custom agents in kebab-case should be valid
    for agent in &[
        "my-custom-agent",
        "code-review",
        "deploy-helper",
        "a",
        "agent123",
        "my-agent-v2",
    ] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            0,
            "Custom agent '{}' should be valid",
            agent
        );
    }
}

#[test]
fn test_cc_sk_005_rejects_invalid_agent_formats() {
    // Consolidated test for all invalid agent formats
    let invalid_agents = [
        ("MyAgent", "uppercase"),
        ("my_custom_agent", "underscore"),
        ("\"\"", "empty"),
        ("-custom-agent", "leading hyphen"),
        ("custom-agent-", "trailing hyphen"),
        ("custom--agent", "consecutive hyphens"),
        ("my@agent", "special char @"),
        ("agent!", "special char !"),
        ("test.agent", "special char ."),
        ("agent/name", "special char /"),
    ];

    for (agent, reason) in invalid_agents {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();

        assert_eq!(
            cc_sk_005.len(),
            1,
            "Agent '{}' ({}) should be rejected",
            agent,
            reason
        );
    }
}

#[test]
fn test_cc_sk_005_rejects_too_long_agent() {
    let long_agent = "a".repeat(65);
    let content = format!(
        r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
        long_agent
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 1, "Agent over 64 chars should be rejected");
}

#[test]
fn test_cc_sk_005_accepts_max_length_agent() {
    let max_agent = "a".repeat(64);
    let content = format!(
        r#"---
name: test-skill
description: Use when testing
context: fork
agent: {}
---
Body"#,
        max_agent
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(cc_sk_005.len(), 0, "Agent at 64 chars should be accepted");
}

#[test]
fn test_cc_sk_005_fixture_invalid_agent() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/invalid-agent/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(
        cc_sk_005.len(),
        1,
        "Invalid agent fixture should trigger CC-SK-005"
    );
}

#[test]
fn test_cc_sk_005_fixture_valid_custom_agent() {
    let content =
        include_str!("../../../../../tests/fixtures/valid/skills/with-custom-agent/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();

    assert_eq!(
        cc_sk_005.len(),
        0,
        "Valid custom agent fixture should pass CC-SK-005"
    );
}

// ===== CC-SK-008: Unknown Tool Name =====

#[test]
fn test_cc_sk_008_unknown_tool() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read Write UnknownTool
---
Body"#;

    let validator = SkillValidator;
    // CC-SK-008 is Claude-only; use a Claude skill path.
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 1);
    assert_eq!(
        cc_sk_008[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_008[0].message.contains("UnknownTool"));
}

#[test]
fn test_cc_sk_008_all_known_tools_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Write Edit Grep Glob Task WebFetch WebSearch AskUserQuestion TodoRead TodoWrite MultiTool NotebookEdit EnterPlanMode ExitPlanMode Skill StatusBarMessageTool SendMessageTool TaskOutput
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 0);
}

#[test]
fn test_cc_sk_008_scoped_tool_extracts_base_name() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash(git:*) Read Write
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 0);
}

#[test]
fn test_cc_sk_008_space_separated_scoped_bash_matchers_allow_internal_spaces() {
    let content = r#"---
name: commit
description: Stage and commit the current changes
allowed-tools: Bash(git add *) Bash(git commit *) Bash(git status *)
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/commit/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-SK-008"),
        "scoped Bash matchers with spaces must not trip CC-SK-008: {:?}",
        diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_cc_sk_008_comma_separated_scoped_matcher_allows_internal_spaces() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash(just playwright-cli:*), Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-SK-008"),
        "comma-separated scoped matchers with spaces must not trip CC-SK-008: {:?}",
        diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-008")
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_cc_sk_008_parameter_scoped_agent_tool_valid() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Agent(model:opus) Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        0,
        "Agent(model:opus) should be accepted as a scoped Claude tool"
    );
}

#[test]
fn test_cc_sk_008_multiple_unknown_tools() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool1 Read FakeTool2
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(cc_sk_008.len(), 2);
}

#[test]
fn test_cc_sk_008_scoped_unknown_tool() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: FakeTool(scope:*) Read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        1,
        "Should detect FakeTool as unknown even when scoped"
    );
    assert!(cc_sk_008[0].message.contains("FakeTool"));
}

// ===== CC-SK-009: Too Many Injections =====

#[test]
fn test_cc_sk_009_too_many_injections() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Current date: !`date`
Git status: !`git status`
Branch: !`git branch`
User: !`whoami`
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 1);
    assert_eq!(
        cc_sk_009[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
    assert!(cc_sk_009[0].message.contains("4"));
}

#[test]
fn test_cc_sk_009_exactly_three_injections_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Date: !`date`
Status: !`git status`
Branch: !`git branch`
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 0);
}

#[test]
fn test_cc_sk_009_no_injections_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
No dynamic injections here.
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_009: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();

    assert_eq!(cc_sk_009.len(), 0);
}

// ===== Edge Case Tests =====

#[test]
fn test_cc_sk_006_explicit_false_still_triggers() {
    let content = r#"---
name: deploy-prod
description: Use when deploying
disable-model-invocation: false
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();

    assert_eq!(
        cc_sk_006.len(),
        1,
        "Explicit false should still trigger CC-SK-006"
    );
}

#[test]
fn test_cc_sk_007_duplicate_bash_multiple_warnings() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash Read Bash
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_007: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-007")
        .collect();

    // Each plain "Bash" triggers warning (2 occurrences = 2 warnings)
    assert_eq!(
        cc_sk_007.len(),
        2,
        "Each Bash occurrence triggers a warning"
    );
}

#[test]
fn test_cc_sk_008_malformed_scope_no_panic() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Bash( Read Bash() Write
---
Body"#;

    let validator = SkillValidator;
    // Should not panic on malformed scope syntax
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Bash( extracts "Bash", which is known
    // Bash() extracts "Bash", which is known
    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        0,
        "Malformed scopes should extract base name correctly"
    );
}

#[test]
fn test_cc_sk_008_lowercase_tool_unknown() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: bash read
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    // Tool names are case-sensitive: bash != Bash
    assert_eq!(cc_sk_008.len(), 2, "lowercase tool names are unknown");
}

#[test]
fn test_cc_sk_008_mcp_tool_valid() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read mcp__memory__create_entities mcp__filesystem__read_file
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        0,
        "MCP tools with mcp__ prefix should be accepted in allowed-tools"
    );
}

#[test]
fn test_cc_sk_008_scoped_mcp_tool_valid() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read mcp__github__search_repositories(scope:*)
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        0,
        "Scoped MCP tools should be accepted in allowed-tools"
    );
}

/// The Claude Code permissions reference documents `mcp__<server>` and
/// `mcp__<server>__*` as equivalent server-level MCP rules, so CC-SK-008 must
/// accept both. Regression for the CC-SK-008 half of upstream issue #1277.
#[test]
fn test_cc_sk_008_mcp_server_only_form_valid() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read, mcp__supabase-ssh, mcp__playwright, mcp__github__*
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert!(
        cc_sk_008.is_empty(),
        "server-only MCP form `mcp__<server>` must be accepted, got: {cc_sk_008:?}"
    );
}

/// The server segment names one configured server, so a glob there is still
/// rejected even though the server-only form is now accepted.
#[test]
fn test_cc_sk_008_mcp_glob_in_server_segment_rejected() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Read, mcp__supabase-*, mcp__*
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        2,
        "a glob in the server segment must still be rejected, got: {cc_sk_008:?}"
    );
}

/// `Workflow` and the other tools added to the built-in tools reference must
/// not be flagged as unknown. Regression for the `Workflow` false positive
/// reported in upstream issue #1277.
#[test]
fn test_cc_sk_008_recent_builtin_tools_accepted() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: Workflow, Artifact, ReportFindings, SendUserFile, EndConversation
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert!(
        cc_sk_008.is_empty(),
        "documented built-in tools must be accepted, got: {cc_sk_008:?}"
    );
}

#[test]
fn test_cc_sk_008_mcp_case_sensitive() {
    let content = r#"---
name: test-skill
description: Use when testing
allowed-tools: MCP__memory__create Mcp__test__tool
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_008: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-008")
        .collect();

    assert_eq!(
        cc_sk_008.len(),
        2,
        "MCP prefix is case-sensitive: MCP__ and Mcp__ should be rejected"
    );
}

#[test]
fn test_parse_error_handling() {
    let content = r#"---
name: test
description
invalid yaml
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();

    assert_eq!(
        parse_errors.len(),
        1,
        "Invalid YAML should produce parse error"
    );
}

// ===== Config Wiring Tests =====

#[test]
fn test_config_disabled_skills_category() {
    let mut config = LintConfig::default();
    config.rules_mut().skills = false;

    let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // No AS-* or CC-SK-* rules should fire when the skills category is disabled
    let skill_rules: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule.starts_with("AS-") || d.rule.starts_with("CC-SK-"))
        .collect();
    assert_eq!(skill_rules.len(), 0);
}

#[test]
fn test_config_disabled_specific_skill_rule() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["AS-005".to_string()];

    let content = r#"---
name: -bad-name
description: Missing trigger phrase
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // AS-005 should not fire when specifically disabled
    let as_005: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-005").collect();
    assert_eq!(as_005.len(), 0);

    // But other rules still fire - AS-004 (invalid name format) for "-bad-name".
    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
}

#[test]
fn test_config_cursor_target_disables_cc_sk_rules() {
    use crate::config::TargetTool;

    let mut config = LintConfig::default();
    config.set_target(TargetTool::Cursor);

    // Over the agentskills.io 1024 baseline so the generic AS-008 control fires.
    let long_desc = "a".repeat(1100);
    let content = format!("---\nname: deploy-prod\ndescription: {long_desc}\n---\nBody");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &config);

    // CC-SK rules (Claude-specific) do not fire for a Cursor target.
    assert_eq!(
        diagnostics.iter().filter(|d| d.rule == "CC-SK-006").count(),
        0
    );
    // A genuine agentskills.io rule (AS-008 length, 1024 baseline) still fires.
    assert_eq!(diagnostics.iter().filter(|d| d.rule == "AS-008").count(), 1);
}

#[test]
fn test_config_claude_code_target_enables_cc_sk_rules() {
    use crate::config::TargetTool;

    let mut config = LintConfig::default();
    config.set_target(TargetTool::ClaudeCode);

    let content = r#"---
name: deploy-prod
description: Use when deploying to production
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    // CC-SK-006 should fire for ClaudeCode target
    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();
    assert_eq!(cc_sk_006.len(), 1);
}

// ===== convert_to_kebab_case Tests =====

#[test]
fn test_convert_to_kebab_case_lowercase() {
    assert_eq!(convert_to_kebab_case("TestSkill"), "testskill");
}

#[test]
fn test_convert_to_kebab_case_underscores() {
    assert_eq!(convert_to_kebab_case("test_skill"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_mixed() {
    assert_eq!(convert_to_kebab_case("Test_Skill_Name"), "test-skill-name");
}

#[test]
fn test_convert_to_kebab_case_consecutive_hyphens() {
    assert_eq!(convert_to_kebab_case("test--skill"), "test-skill");
    assert_eq!(convert_to_kebab_case("test___skill"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_leading_trailing() {
    assert_eq!(convert_to_kebab_case("-test-skill-"), "test-skill");
    assert_eq!(convert_to_kebab_case("_test_skill_"), "test-skill");
}

#[test]
fn test_convert_to_kebab_case_invalid_chars() {
    assert_eq!(convert_to_kebab_case("test@skill!"), "testskill");
    assert_eq!(convert_to_kebab_case("test.skill"), "testskill");
}

#[test]
fn test_convert_to_kebab_case_truncate() {
    let long_name = "a".repeat(100);
    let result = convert_to_kebab_case(&long_name);
    assert!(result.len() <= 64);
    assert_eq!(result.len(), 64);
}

// ===== AS-004 Auto-fix Tests =====

#[test]
fn test_as_004_has_fix() {
    let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    assert_eq!(as_004[0].fixes[0].replacement, "test-skill");
}

#[test]
fn test_as_004_non_ascii_name_has_no_lossy_fix() {
    let content = r#"---
name: tëst_Skîll_😀_ทดสอบ
description: Use when testing non-ASCII skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(!as_004[0].has_fixes());
}

#[test]
fn test_as_004_fix_case_only_is_safe() {
    let content = r#"---
name: TestSkill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Case-only change (TestSkill -> testskill) is safe
    assert!(as_004[0].fixes[0].safe);
}

#[test]
fn test_as_004_fix_structural_is_unsafe() {
    let content = r#"---
name: Test_Skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Structural change (Test_Skill -> test-skill) is not safe
    assert!(!as_004[0].fixes[0].safe);
}

#[test]
fn test_as_004_fix_byte_position() {
    let content = r#"---
name: Bad_Name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());

    let fix = &as_004[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    assert!(fixed.contains("name: bad-name"));
}

#[test]
fn test_as_004_fix_quoted_value() {
    let content = r#"---
name: "Bad_Name"
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());

    let fix = &as_004[0].fixes[0];
    // Apply fix and verify
    let mut fixed = content.to_string();
    fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
    // The fix replaces the inner value, keeping quotes in place
    assert!(fixed.contains("bad-name"));
}

#[test]
fn test_as_004_no_fix_when_converts_to_empty() {
    // Name with only special characters should convert to empty string
    let content = r#"---
name: "!@#$%"
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    // Should have no fix since converted name would be empty
    assert!(!as_004[0].has_fixes());
}

#[test]
fn test_as_004_underscore_to_hyphen_is_unsafe() {
    // Test_Name -> test-name involves underscore replacement, should be unsafe
    let content = r#"---
name: test_name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
    assert!(as_004[0].has_fixes());
    // Underscore to hyphen is structural change, not safe
    assert!(!as_004[0].fixes[0].safe);
}

// ===== frontmatter_value_byte_range Tests =====

#[test]
fn test_frontmatter_value_byte_range_unquoted() {
    let content = r#"---
name: test-skill
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_double_quoted() {
    let content = r#"---
name: "test-skill"
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_single_quoted() {
    let content = r#"---
name: 'test-skill'
description: A test skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert!(range.is_some());
    let (start, end) = range.unwrap();
    assert_eq!(&content[start..end], "test-skill");
}

#[test]
fn test_frontmatter_value_byte_range_not_found() {
    let content = r#"---
name: test-skill
---
Body"#;
    let parts = split_frontmatter(content);
    let range = frontmatter_value_byte_range(content, &parts, "description");
    assert!(range.is_none());
}

#[test]
fn test_frontmatter_value_byte_range_exhaustive() {
    let content = "---
name: test-skill
description: \"Quoted description\"
empty:
  nested: value
unquoted_with_comment: value # this is a comment
quoted_with_comment: \"quoted value\" # this is also a comment
single_quoted: 'single value'
with_colon: \"value: with colon\"
  indented_key: indented_value
---
Body";
    let parts = split_frontmatter(content);

    // Unquoted
    let range = frontmatter_value_byte_range(content, &parts, "name");
    assert_eq!(&content[range.unwrap().0..range.unwrap().1], "test-skill");

    // Double quoted
    let range = frontmatter_value_byte_range(content, &parts, "description");
    assert_eq!(
        &content[range.unwrap().0..range.unwrap().1],
        "Quoted description"
    );

    // Empty (multiline/nested) - currently returns None as it only looks at the same line
    let range = frontmatter_value_byte_range(content, &parts, "empty");
    assert!(range.is_none());

    // Unquoted with comment
    let range = frontmatter_value_byte_range(content, &parts, "unquoted_with_comment");
    assert_eq!(&content[range.unwrap().0..range.unwrap().1], "value");

    // Quoted with comment
    let range = frontmatter_value_byte_range(content, &parts, "quoted_with_comment");
    assert_eq!(&content[range.unwrap().0..range.unwrap().1], "quoted value");

    // Single quoted
    let range = frontmatter_value_byte_range(content, &parts, "single_quoted");
    assert_eq!(&content[range.unwrap().0..range.unwrap().1], "single value");

    // With colon
    let range = frontmatter_value_byte_range(content, &parts, "with_colon");
    assert_eq!(
        &content[range.unwrap().0..range.unwrap().1],
        "value: with colon"
    );

    // Indented key
    let range = frontmatter_value_byte_range(content, &parts, "indented_key");
    assert_eq!(
        &content[range.unwrap().0..range.unwrap().1],
        "indented_value"
    );

    // CRLF
    let content_crlf = "---\r\nname: test-skill\r\ndescription: value\r\n---\r\nBody";
    let parts_crlf = split_frontmatter(content_crlf);
    let range = frontmatter_value_byte_range(content_crlf, &parts_crlf, "name");
    assert_eq!(
        &content_crlf[range.unwrap().0..range.unwrap().1],
        "test-skill"
    );

    // Malformed: unclosed quote
    let content_malformed = "---\nname: \"unclosed\ndescription: value\n---\nBody";
    let parts_malformed = split_frontmatter(content_malformed);
    let range = frontmatter_value_byte_range(content_malformed, &parts_malformed, "name");
    assert!(range.is_none());
}

#[test]
fn test_key_helpers_exhaustive() {
    let content = "---
name: test-skill
  indented_key: value
# comment
other: val
---";
    let parts = split_frontmatter(content);

    // frontmatter_key_offset
    // frontmatter starts after "---\n" (index 4), leading newline is stripped
    // parts.frontmatter = "name: test-skill\n  indented_key: value\n# comment\nother: val"
    assert_eq!(frontmatter_key_offset(&parts.frontmatter, "name"), Some(0));
    assert_eq!(
        frontmatter_key_offset(&parts.frontmatter, "indented_key"),
        Some(19)
    );

    // frontmatter_key_line_byte_range
    let range = frontmatter_key_line_byte_range(content, &parts, "name").unwrap();
    assert_eq!(&content[range.0..range.1], "name: test-skill\n");

    let range = frontmatter_key_line_byte_range(content, &parts, "other").unwrap();
    assert_eq!(&content[range.0..range.1], "other: val\n");

    // CRLF
    let content_crlf = "---\r\nname: test\r\nother: val\r\n---";
    let parts_crlf = split_frontmatter(content_crlf);
    let range = frontmatter_key_line_byte_range(content_crlf, &parts_crlf, "name").unwrap();
    assert_eq!(&content_crlf[range.0..range.1], "name: test\r\n");
}

// ===== directory_size_until tests =====

/// Helper to write N bytes to a file efficiently using a small buffer
fn write_bytes_to_file(path: &std::path::Path, num_bytes: usize) {
    use std::io::Write;
    let mut file = fs::File::create(path).expect("Failed to create test file");
    let buffer = [0u8; 8192];
    let mut remaining = num_bytes;
    while remaining > 0 {
        let to_write = remaining.min(buffer.len());
        file.write_all(&buffer[..to_write])
            .expect("Failed to write test data");
        remaining -= to_write;
    }
}

#[test]
fn test_directory_size_until_short_circuits() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    // Create 10 files of 1MB each (10MB total)
    for i in 0..10 {
        let file_path = temp_dir.path().join(format!("file_{:02}.bin", i));
        write_bytes_to_file(&file_path, 1024 * 1024);
    }

    // With a 2MB limit, should short-circuit and return > 2MB
    let size = directory_size_until(temp_dir.path(), 2 * 1024 * 1024, &fs);
    assert!(size > 2 * 1024 * 1024, "Size should exceed 2MB limit");
    // Should not have scanned all 10MB (short-circuited after exceeding limit).
    // Upper bound is 3MB because: directory iteration order is unspecified,
    // so we may read up to 2 full 1MB files before the check triggers on the 3rd.
    assert!(
        size <= 3 * 1024 * 1024,
        "Size {} should be <= 3MB (short-circuited)",
        size
    );
}

#[test]
fn test_directory_size_until_accurate_under_limit() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    // Create 2 files of 1KB each (2KB total)
    for i in 0..2 {
        let file_path = temp_dir.path().join(format!("file_{}.bin", i));
        write_bytes_to_file(&file_path, 1024);
    }

    // With a 1MB limit, should return exact size
    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &fs);
    assert_eq!(size, 2048);
}

#[test]
fn test_directory_size_until_handles_empty_directory() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let fs = RealFileSystem;

    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &fs);
    assert_eq!(size, 0);
}

#[test]
fn test_directory_size_until_nested_directories() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let real_fs = RealFileSystem;

    // Create nested structure: root/sub1/sub2 with files at each level
    let sub1 = temp_dir.path().join("sub1");
    let sub2 = sub1.join("sub2");
    fs::create_dir_all(&sub2).expect("Failed to create nested directories");

    // 1KB at root, 2KB in sub1, 3KB in sub2 = 6KB total
    write_bytes_to_file(&temp_dir.path().join("root.bin"), 1024);
    write_bytes_to_file(&sub1.join("sub1.bin"), 2048);
    write_bytes_to_file(&sub2.join("sub2.bin"), 3072);

    let size = directory_size_until(temp_dir.path(), 1024 * 1024, &real_fs);
    assert_eq!(size, 6144, "Should sum files across all nested directories");
}

#[test]
fn test_directory_size_until_nested_short_circuits() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let real_fs = RealFileSystem;

    // Create nested structure with large files
    let sub1 = temp_dir.path().join("sub1");
    let sub2 = sub1.join("sub2");
    fs::create_dir_all(&sub2).expect("Failed to create nested directories");

    // 1MB at each level = 3MB total, with 2MB limit should short-circuit
    write_bytes_to_file(&temp_dir.path().join("root.bin"), 1024 * 1024);
    write_bytes_to_file(&sub1.join("sub1.bin"), 1024 * 1024);
    write_bytes_to_file(&sub2.join("sub2.bin"), 1024 * 1024);

    let size = directory_size_until(temp_dir.path(), 2 * 1024 * 1024, &real_fs);
    assert!(size > 2 * 1024 * 1024, "Should exceed limit");
    assert!(
        size <= 3 * 1024 * 1024,
        "Should short-circuit before scanning all"
    );
}

#[test]
fn test_as_015_boundary_exactly_8mb() {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let skill_dir = temp_dir.path().join("skill");
    fs::create_dir_all(&skill_dir).expect("Failed to create skill directory");

    // Create SKILL.md
    let skill_path = skill_dir.join("SKILL.md");
    fs::write(&skill_path, "---\nname: boundary-test\n---\nBody")
        .expect("Failed to write SKILL.md");

    // Create file that brings total to exactly 8MB (minus SKILL.md size)
    let skill_md_size = fs::metadata(&skill_path)
        .expect("Failed to read SKILL.md metadata")
        .len() as usize;
    let target_size = 8 * 1024 * 1024 - skill_md_size;
    write_bytes_to_file(&skill_dir.join("data.bin"), target_size);

    let validator = SkillValidator;
    let content = fs::read_to_string(&skill_path).expect("Failed to read SKILL.md content");
    let diagnostics = validator.validate(&skill_path, &content, &LintConfig::default());

    // Exactly 8MB should NOT trigger AS-015 (uses > not >=)
    let as_015_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-015").collect();
    assert!(
        as_015_errors.is_empty(),
        "Exactly 8MB should not trigger AS-015, but got: {:?}",
        as_015_errors
    );
}

// ===== Additional AS-016 Parse Error Tests =====

#[test]
fn test_as_001_missing_closing_delimiter_treated_as_no_frontmatter() {
    // When opening --- exists but closing --- is missing, the entire content
    // is treated as body without frontmatter, triggering AS-001
    let content = r#"---
name: test
description: A test skill
Missing closing delimiter
Body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
    assert_eq!(as_001.len(), 1);

    // Should NOT be treated as parse error since no frontmatter was detected
    let as_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(as_016.is_empty());
}

#[test]
fn test_as_016_invalid_yaml_colon_in_value() {
    let content = r#"---
name: test:value:with:colons
description: A test skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // This should parse OK as YAML handles colons in values
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(parse_errors.is_empty());
}

#[test]
fn test_as_016_invalid_yaml_tabs() {
    let content = "---\nname: test\n\tdescription: bad indent\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Tabs in YAML can cause parse errors
    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert_eq!(
        parse_errors.len(),
        1,
        "Tab indentation should cause parse error"
    );
}

#[test]
fn test_as_016_valid_yaml_no_error() {
    let content = r#"---
name: valid-skill
description: A properly formatted skill
model: sonnet
---
Body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert!(parse_errors.is_empty());
}

#[test]
fn test_as_016_duplicate_frontmatter_key() {
    let content = r#"---
name: first-skill
description: Use when validating duplicate frontmatter keys
name: second-skill
---
Body content"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert_eq!(
        parse_errors.len(),
        1,
        "duplicate top-level key should produce one AS-016 diagnostic"
    );
    assert!(
        parse_errors[0]
            .message
            .contains("Duplicate YAML frontmatter key 'name'"),
        "expected duplicate key message, got: {}",
        parse_errors[0].message
    );
}

#[test]
fn test_as_016_metadata_requires_string_keys_and_values() {
    let validator = SkillValidator;
    let valid = r#"---
name: test-skill
description: Use when validating Agent Skills metadata
metadata:
  owner: platform
---
Body"#;
    let valid_diagnostics = validator.validate(
        Path::new("skills/test-skill/SKILL.md"),
        valid,
        &LintConfig::default(),
    );
    assert!(
        valid_diagnostics
            .iter()
            .all(|diagnostic| diagnostic.rule != "AS-016")
    );

    for invalid in [
        r#"---
name: test-skill
description: Use when validating Agent Skills metadata
metadata:
  priority: 1
---
Body"#,
        r#"---
name: test-skill
description: Use when validating Agent Skills metadata
metadata:
  owner:
    team: platform
---
Body"#,
        r#"---
name: test-skill
description: Use when validating Agent Skills metadata
metadata:
  1: platform
---
Body"#,
        r#"---
name: test-skill
description: Use when validating Agent Skills metadata
metadata: platform
---
Body"#,
    ] {
        let diagnostics = validator.validate(
            Path::new("skills/test-skill/SKILL.md"),
            invalid,
            &LintConfig::default(),
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.rule == "AS-016")
                .count(),
            1,
            "non-string metadata must fail frontmatter parsing"
        );
    }
}

#[test]
fn test_as_016_allowed_tools_yaml_list_no_parse_error() {
    // Reproduces #957: `allowed-tools` as a YAML list previously failed to
    // deserialize (Option<String>) and tripped AS-016. Claude Code accepts a
    // list; agentskills.io documents a space-separated string. Both shapes
    // must now parse without an AS-016 error.
    let list_form = r#"---
name: valid-skill
description: Use when reviewing code for issues
allowed-tools:
  - Read
  - Grep
  - Glob
---
Body content"#;
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), list_form, &LintConfig::default());
    assert!(
        diagnostics.iter().all(|d| d.rule != "AS-016"),
        "YAML-list allowed-tools must not trip AS-016, got: {:?}",
        diagnostics
            .iter()
            .filter(|d| d.rule == "AS-016")
            .collect::<Vec<_>>()
    );
    // The joined tools (Read Grep Glob) are all known, so no CC-SK-008 either.
    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-SK-008"),
        "valid tools in list form must not trip CC-SK-008"
    );

    // String form still parses cleanly too.
    let string_form = r#"---
name: valid-skill
description: Use when reviewing code for issues
allowed-tools: Read Grep Glob
---
Body content"#;
    let string_diags =
        validator.validate(Path::new("test.md"), string_form, &LintConfig::default());
    assert!(string_diags.iter().all(|d| d.rule != "AS-016"));
}

#[test]
fn test_as_016_disabled() {
    let mut config = LintConfig::default();
    config.rules_mut().disabled_rules = vec!["AS-016".to_string()];

    let content = r#"---
name: test
description
invalid yaml syntax
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &config);

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-016"));
}

// ===== Additional edge case tests for comprehensive coverage =====

#[test]
fn test_as_001_empty_file() {
    let content = "";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-001"));
}

#[test]
fn test_as_001_only_body_no_frontmatter() {
    let content = "This is just body content without any frontmatter.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-001"));
}

#[test]
fn test_as_002_no_name_field() {
    let content = r#"---
description: A test skill without name field
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Missing name field should trigger AS-002
    assert!(diagnostics.iter().any(|d| d.rule == "AS-002"));
}

#[test]
fn test_as_004_whitespace_only_name() {
    let content = r#"---
name: "   "
description: A test skill
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Whitespace-only name should trigger AS-004 (invalid format)
    assert!(diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_003_whitespace_description() {
    let content = r#"---
name: test-skill
description: "   "
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    // Whitespace-only description should be treated as short (AS-008)
    let as_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-008").collect();
    assert!(
        !as_008.is_empty(),
        "Whitespace description should trigger AS-008"
    );
}

#[test]
fn test_as_004_uppercase_in_name() {
    let content = r#"---
name: TestSkill
description: Use when testing skill names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-004").collect();
    assert_eq!(as_004.len(), 1);
}

#[test]
fn test_as_004_valid_lowercase_hyphen_name() {
    let content = r#"---
name: valid-skill-name
description: Use when testing valid names
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "AS-004"));
}

#[test]
fn test_as_011_exactly_500_chars() {
    let long_compat = "a".repeat(500);
    let content = format!(
        "---\nname: test\ndescription: Use when testing\ncompatibility: {}\n---\nBody",
        long_compat
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    // Exactly 500 should be OK (limit is >500)
    assert!(!diagnostics.iter().any(|d| d.rule == "AS-011"));
}

#[test]
fn test_as_011_501_chars_triggers() {
    let long_compat = "a".repeat(501);
    let content = format!(
        "---\nname: test\ndescription: Use when testing\ncompatibility: {}\n---\nBody",
        long_compat
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "AS-011"));
}

#[test]
fn test_as_012_exactly_500_lines_ok() {
    let body_lines = (0..470)
        .map(|i| format!("Line {}", i))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "---\nname: test\ndescription: Use when testing line limits\n---\n{}",
        body_lines
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), &content, &LintConfig::default());

    // Around 500 lines should be OK
    assert!(!diagnostics.iter().any(|d| d.rule == "AS-012"));
}

#[test]
fn test_cc_sk_001_all_valid_models() {
    // Must match VALID_MODEL_ALIASES plus claude-* prefixed IDs
    let valid_models = VALID_MODEL_ALIASES;

    for model in valid_models {
        let content = format!(
            "---\nname: test\ndescription: Use when testing models\nmodel: {}\n---\nBody",
            model
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_001: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-001")
            .collect();
        assert!(
            cc_sk_001.is_empty(),
            "Model '{}' should be valid but got CC-SK-001",
            model
        );
    }
}

#[test]
fn test_cc_sk_001_invalid_model_exhaustive() {
    let content = r#"---
name: test
description: Use when testing
model: invalid-model
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-001"));
}

#[test]
fn test_cc_sk_002_fork_context_valid() {
    // Context can only be "fork", and requires an agent field
    let content = r#"---
name: test
description: Use when testing contexts
context: fork
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_002: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-002")
        .collect();
    assert!(
        cc_sk_002.is_empty(),
        "Context 'fork' with agent should be valid"
    );
}

#[test]
fn test_cc_sk_002_invalid_context_exhaustive() {
    let content = r#"---
name: test
description: Use when testing contexts
context: invalid
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-002"));
}

#[test]
fn test_cc_sk_004_agent_without_context_exhaustive() {
    let content = r#"---
name: test
description: Use when testing
agent: general-purpose
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(diagnostics.iter().any(|d| d.rule == "CC-SK-004"));
}

#[test]
fn test_cc_sk_005_builtin_agents_valid() {
    // Must match BUILTIN_AGENTS constant in skill/mod.rs
    let builtin_agents = BUILTIN_AGENTS;

    for agent in builtin_agents {
        let content = format!(
            "---\nname: test\ndescription: Use when testing\ncontext: fork\nagent: {}\n---\nBody",
            agent
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-005")
            .collect();
        assert!(
            cc_sk_005.is_empty(),
            "Built-in agent '{}' should be valid",
            agent
        );
    }
}

#[test]
fn test_cc_sk_005_custom_kebab_agent_valid() {
    let content = r#"---
name: test
description: Use when testing
context: fork
agent: my-custom-agent
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    assert!(!diagnostics.iter().any(|d| d.rule == "CC-SK-005"));
}

// ===== Additional auto-fix coverage for new rules =====

#[test]
fn test_as_005_has_safe_fix() {
    let content = r#"---
name: -bad-name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let as_005 = diagnostics
        .iter()
        .find(|d| d.rule == "AS-005")
        .expect("AS-005 should be reported");

    assert!(as_005.has_fixes());
    let fix = &as_005.fixes[0];
    assert_eq!(fix.replacement, "bad-name");
    assert!(fix.safe);
}

#[test]
fn test_as_006_has_safe_fix() {
    let content = r#"---
name: bad--name
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let as_006 = diagnostics
        .iter()
        .find(|d| d.rule == "AS-006")
        .expect("AS-006 should be reported");

    assert!(as_006.has_fixes());
    let fix = &as_006.fixes[0];
    assert_eq!(fix.replacement, "bad-name");
    assert!(fix.safe);
}

#[test]
fn test_cc_sk_001_has_unsafe_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
model: gpt-4
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_001 = diagnostics
        .iter()
        .find(|d| d.rule == "CC-SK-001")
        .expect("CC-SK-001 should be reported");

    assert!(cc_sk_001.has_fixes());
    let fix = &cc_sk_001.fixes[0];
    assert_eq!(fix.replacement, "sonnet");
    assert!(!fix.safe);
}

#[test]
fn test_cc_sk_002_has_unsafe_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
context: split
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_002 = diagnostics
        .iter()
        .find(|d| d.rule == "CC-SK-002")
        .expect("CC-SK-002 should be reported");

    assert!(cc_sk_002.has_fixes());
    let fix = &cc_sk_002.fixes[0];
    assert_eq!(fix.replacement, "fork");
    assert!(!fix.safe);
}

#[test]
fn test_cc_sk_004_has_insert_or_replace_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
agent: Explore
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_004 = diagnostics
        .iter()
        .find(|d| d.rule == "CC-SK-004")
        .expect("CC-SK-004 should be reported");

    assert!(cc_sk_004.has_fixes());
    let fix = &cc_sk_004.fixes[0];
    assert!(fix.replacement.contains("context: fork") || fix.replacement == "fork");
    assert!(!fix.safe);
}

// ===== CC-SK-010: Invalid Hooks in Skill Frontmatter =====

#[test]
fn test_cc_sk_010_invalid_hook_event() {
    let content = r#"---
name: hooks-skill
description: Use when testing hooks
hooks:
  InvalidEvent:
    - type: command
      command: echo hello
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert_eq!(cc_sk_010.len(), 1);
    assert_eq!(
        cc_sk_010[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    assert!(cc_sk_010[0].message.contains("InvalidEvent"));
}

#[test]
fn test_cc_sk_010_valid_hook_event() {
    let content = r#"---
name: hooks-skill
description: Use when testing hooks
hooks:
  PreToolUse:
    - type: command
      command: echo pre
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert_eq!(cc_sk_010.len(), 0);
}

#[test]
fn test_cc_sk_010_no_hooks_field_ok() {
    let content = r#"---
name: simple-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert_eq!(cc_sk_010.len(), 0);
}

#[test]
fn test_cc_sk_010_hooks_not_mapping() {
    let content = r#"---
name: hooks-skill
description: Use when testing hooks
hooks: not-a-mapping
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert_eq!(cc_sk_010.len(), 1);
    assert!(cc_sk_010[0].message.contains("must be a mapping"));
}

#[test]
fn test_cc_sk_010_fixture_invalid() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/invalid-hooks/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert!(
        !cc_sk_010.is_empty(),
        "Invalid hooks fixture should trigger CC-SK-010"
    );
}

#[test]
fn test_cc_sk_010_fixture_valid() {
    let content = include_str!("../../../../../tests/fixtures/valid/skills/with-hooks/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_010: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-010")
        .collect();

    assert_eq!(
        cc_sk_010.len(),
        0,
        "Valid hooks fixture should not trigger CC-SK-010"
    );
}

// ===== CC-SK-011: Unreachable Skill =====

#[test]
fn test_cc_sk_011_unreachable() {
    let content = r#"---
name: unreachable
description: Use when testing unreachable
user-invocable: false
disable-model-invocation: true
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_011: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-011")
        .collect();

    assert_eq!(cc_sk_011.len(), 1);
    assert_eq!(
        cc_sk_011[0].level,
        crate::diagnostics::DiagnosticLevel::Error
    );
    // Should have an unsafe auto-fix that deletes disable-model-invocation line
    assert!(cc_sk_011[0].has_fixes(), "CC-SK-011 should have auto-fix");
    let fix = &cc_sk_011[0].fixes[0];
    assert!(!fix.safe, "CC-SK-011 fix should be unsafe");
    assert!(
        fix.replacement.is_empty(),
        "CC-SK-011 fix should be a deletion"
    );
    // Verify the fix deletes the disable-model-invocation line
    let deleted = &content[fix.start_byte..fix.end_byte];
    assert!(
        deleted.contains("disable-model-invocation"),
        "Fix should target the disable-model-invocation line, got: {:?}",
        deleted
    );
}

#[test]
fn test_cc_sk_011_user_invocable_true_ok() {
    let content = r#"---
name: invocable
description: Use when testing
user-invocable: true
disable-model-invocation: true
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_011: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-011")
        .collect();

    assert_eq!(cc_sk_011.len(), 0);
}

#[test]
fn test_cc_sk_011_model_invocation_false_ok() {
    let content = r#"---
name: invocable
description: Use when testing
user-invocable: false
disable-model-invocation: false
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_011: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-011")
        .collect();

    assert_eq!(cc_sk_011.len(), 0);
}

#[test]
fn test_cc_sk_011_defaults_ok() {
    // Default: user-invocable=true, disable-model-invocation=false
    let content = r#"---
name: default-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_011: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-011")
        .collect();

    assert_eq!(cc_sk_011.len(), 0);
}

#[test]
fn test_cc_sk_011_fixture() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/unreachable-skill/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_011: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-011")
        .collect();

    assert_eq!(
        cc_sk_011.len(),
        1,
        "Unreachable skill fixture should trigger CC-SK-011"
    );
}

// ===== CC-SK-012: Argument Hint Without $ARGUMENTS =====

#[test]
fn test_cc_sk_012_hint_without_arguments() {
    let content = r#"---
name: hint-skill
description: Use when testing hints
argument-hint: <file-path>
---
Process the given file."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();

    assert_eq!(cc_sk_012.len(), 1);
    assert_eq!(
        cc_sk_012[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_cc_sk_012_hint_with_arguments_ok() {
    let content = r#"---
name: hint-skill
description: Use when testing hints
argument-hint: <file-path>
---
Process the file specified in $ARGUMENTS."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();

    assert_eq!(cc_sk_012.len(), 0);
}

#[test]
fn test_cc_sk_012_no_hint_ok() {
    let content = r#"---
name: no-hint
description: Use when testing
---
Body without $ARGUMENTS."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();

    assert_eq!(cc_sk_012.len(), 0);
}

#[test]
fn test_cc_sk_012_fixture_invalid() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/argument-hint-no-args/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();

    assert_eq!(
        cc_sk_012.len(),
        1,
        "Argument hint without $ARGUMENTS fixture should trigger CC-SK-012"
    );
}

#[test]
fn test_cc_sk_012_fixture_valid() {
    let content =
        include_str!("../../../../../tests/fixtures/valid/skills/with-argument-hint/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();

    assert_eq!(
        cc_sk_012.len(),
        0,
        "Valid argument hint fixture should not trigger CC-SK-012"
    );
}

// ===== CC-SK-016: Indexed $ARGUMENTS[n] Without argument-hint =====

#[test]
fn test_cc_sk_016_indexed_args_without_hint() {
    let content = r#"---
name: indexed-args
description: Use when validating indexed arguments
---
Process path: $ARGUMENTS[0]"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_016: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-016")
        .collect();

    assert_eq!(cc_sk_016.len(), 1);
    assert_eq!(
        cc_sk_016[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_cc_sk_016_indexed_args_with_hint_ok() {
    let content = r#"---
name: indexed-args
description: Use when validating indexed arguments
argument-hint: <path>
---
Process path: $ARGUMENTS[0]"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_016: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-016")
        .collect();

    assert_eq!(cc_sk_016.len(), 0);
}

// ===== CC-SK-017: Unknown Frontmatter Field =====

#[test]
fn test_cc_sk_017_unknown_frontmatter_field() {
    let content = r#"---
name: test-skill
description: Use when validating unknown frontmatter fields
desription: typo field
---
Body"#;

    let validator = SkillValidator;
    // CC-SK-017 is Claude-only; use a Claude skill path.
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let cc_sk_017: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-017")
        .collect();

    assert_eq!(cc_sk_017.len(), 1);
    assert!(cc_sk_017[0].message.contains("desription"));
}

#[test]
fn test_cc_sk_017_claude_only_and_new_fields() {
    let validator = SkillValidator;

    // `when_to_use` and `arguments` are documented Claude skill fields - no
    // longer flagged (#957).
    let documented = r#"---
name: test-skill
description: Use when reviewing
when_to_use: reviewing a pull request
arguments: optional args
---
Body"#;
    let claude = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        documented,
        &LintConfig::default(),
    );
    assert!(
        claude.iter().all(|d| d.rule != "CC-SK-017"),
        "documented fields when_to_use/arguments must not trip CC-SK-017"
    );

    // CC-SK-017 is Claude-only: an unknown field on a non-Claude (Codex) skill
    // is not judged against Claude's field set.
    let unknown_field = r#"---
name: test-skill
description: Use when reviewing
desription: typo
---
Body"#;
    let codex = validator.validate(
        Path::new(".agents/skills/test-skill/SKILL.md"),
        unknown_field,
        &LintConfig::default(),
    );
    assert!(
        codex.iter().all(|d| d.rule != "CC-SK-017"),
        "CC-SK-017 must not fire on a non-Claude skill"
    );
}

#[test]
fn test_cc_sk_017_claude_v2_1_186_frontmatter_aliases() {
    let validator = SkillValidator;
    let content = r#"---
name: test-skill
description: Use when validating Claude skill metadata aliases
display-name: Test Skill
display_name: Test Skill
displayName: Test Skill
default-enabled: true
default_enabled: true
defaultEnabled: true
fallback: test-skill
metadata:
  display-name: Nested metadata is intentionally unchecked
---
Body"#;

    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );
    assert!(
        diagnostics.iter().all(|d| d.rule != "CC-SK-017"),
        "Claude v2.1.186 frontmatter aliases must not trip CC-SK-017, got {diagnostics:?}"
    );
}

#[test]
fn test_cc_sk_008_powershell_known_and_claude_only() {
    let validator = SkillValidator;

    // PowerShell is now a known Claude tool (#957) - no CC-SK-008.
    let pwsh = r#"---
name: test-skill
description: Use when running scripts
allowed-tools: Bash PowerShell Read
---
Body"#;
    let claude = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        pwsh,
        &LintConfig::default(),
    );
    assert!(
        claude.iter().all(|d| d.rule != "CC-SK-008"),
        "PowerShell must be recognized as a known tool"
    );

    // CC-SK-008 is Claude-only: an unknown tool on a Codex skill is not judged
    // against Claude's tool set.
    let unknown = r#"---
name: test-skill
description: Use when running scripts
allowed-tools: Read TotallyUnknownTool
---
Body"#;
    let codex = validator.validate(
        Path::new(".agents/skills/test-skill/SKILL.md"),
        unknown,
        &LintConfig::default(),
    );
    assert!(
        codex.iter().all(|d| d.rule != "CC-SK-008"),
        "CC-SK-008 must not fire on a non-Claude skill"
    );
}

#[test]
fn test_cc_sk_family_suppressed_for_known_non_claude_client() {
    // The whole CC-SK family is gated at the dispatch: an invalid Claude model
    // value (CC-SK-001) fires for Claude/unscoped skills but is suppressed for
    // a skill owned by another known tool (Codex `.agents/skills/`).
    let content = r#"---
name: test-skill
description: Use when testing model validation
model: not-a-real-model
---
Body"#;
    let validator = SkillValidator;

    // Unscoped (Unknown) -> CC-SK-001 still fires (can't rule out Claude).
    let generic = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    assert!(
        generic.iter().any(|d| d.rule == "CC-SK-001"),
        "CC-SK-001 should fire for an unscoped skill"
    );

    // Codex skill -> CC-SK-001 suppressed (not judged by Claude rules).
    let codex = validator.validate(
        Path::new(".agents/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );
    assert!(
        codex.iter().all(|d| d.rule != "CC-SK-001"),
        "CC-SK-001 must not fire on a Codex skill"
    );
}

#[test]
fn test_cc_sk_gate_respects_ambiguous_multi_tool_scope() {
    // An explicit multi-tool scope that excludes claude-code rules Claude out,
    // so CC-SK must not fire even though the path has no client marker (both
    // "unscoped" and "ambiguous multi-tool" resolve to Unknown).
    let content = r#"---
name: test-skill
description: Use when testing model validation
model: not-a-real-model
---
Body"#;
    let validator = SkillValidator;

    let mut multi_non_claude = LintConfig::default();
    multi_non_claude.set_tools(vec!["codex".to_string(), "cursor".to_string()]);
    let diags = validator.validate(Path::new("test.md"), content, &multi_non_claude);
    assert!(
        diags.iter().all(|d| d.rule != "CC-SK-001"),
        "CC-SK-001 must not fire when tools scope excludes claude-code"
    );

    // The same scope WITH claude-code keeps Claude rules on.
    let mut with_claude = LintConfig::default();
    with_claude.set_tools(vec!["claude-code".to_string(), "cursor".to_string()]);
    let diags2 = validator.validate(Path::new("test.md"), content, &with_claude);
    assert!(
        diags2.iter().any(|d| d.rule == "CC-SK-001"),
        "CC-SK-001 should fire when claude-code is in the tools scope"
    );
}

#[test]
fn test_cc_sk_017_known_frontmatter_field_ok() {
    let content = r#"---
name: test-skill
description: Use when validating known frontmatter fields
hooks:
  PreToolUse:
    - type: command
      command: echo pre
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_017: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-017")
        .collect();

    assert_eq!(cc_sk_017.len(), 0);
}

// ===== CC-SK-013: Fork Context Without Actionable Instructions =====

#[test]
fn test_cc_sk_013_fork_without_instructions() {
    let content = r#"---
name: ref-skill
description: Use when looking up docs
context: fork
agent: general-purpose
---
This is a reference document about the API.
It describes the system architecture.
The data models are documented here."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(cc_sk_013.len(), 1);
    assert_eq!(
        cc_sk_013[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
}

#[test]
fn test_cc_sk_013_fork_with_instructions_ok() {
    let content = r#"---
name: build-skill
description: Use when building the project
context: fork
agent: general-purpose
---
Run the build command and check for errors.
Create a report of the results."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(cc_sk_013.len(), 0);
}

#[test]
fn test_cc_sk_013_no_fork_ok() {
    let content = r#"---
name: ref-skill
description: Use when looking up docs
---
This is just reference content without imperative verbs."#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(cc_sk_013.len(), 0);
}

#[test]
fn test_cc_sk_013_fixture_invalid() {
    let content =
        include_str!("../../../../../tests/fixtures/invalid/skills/fork-no-instructions/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(
        cc_sk_013.len(),
        1,
        "Fork without instructions fixture should trigger CC-SK-013"
    );
}

#[test]
fn test_cc_sk_013_fixture_valid() {
    let content =
        include_str!("../../../../../tests/fixtures/valid/skills/fork-with-instructions/SKILL.md");

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(
        cc_sk_013.len(),
        0,
        "Fork with instructions fixture should not trigger CC-SK-013"
    );
}

// ===== CC-SK-014: Invalid disable-model-invocation Type =====

#[test]
fn test_cc_sk_014_string_true() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: "true"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert!(cc_sk_014.is_empty());
    assert!(diagnostics.iter().all(|d| d.rule != "AS-016"));
}

#[test]
fn test_cc_sk_014_string_false() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: "false"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert!(cc_sk_014.is_empty());
}

#[test]
fn test_cc_sk_014_boolean_true_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: true
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert_eq!(cc_sk_014.len(), 0);
}

#[test]
fn test_cc_sk_014_quoted_alias_needs_no_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: "true"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert!(cc_sk_014.is_empty());
}

#[test]
fn test_cc_sk_014_invalid_alias_is_reported_without_guessing_a_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: "maybe"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert_eq!(cc_sk_014.len(), 1);
    assert!(!cc_sk_014[0].has_fixes());
}

#[test]
fn test_cc_sk_014_single_quoted() {
    let content = "---\nname: test-skill\ndescription: Use when testing\ndisable-model-invocation: 'true'\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert!(cc_sk_014.is_empty());
}

// ===== CC-SK-015: Invalid user-invocable Type =====

#[test]
fn test_cc_sk_015_string_true() {
    let content = r#"---
name: test-skill
description: Use when testing
user-invocable: "true"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert!(cc_sk_015.is_empty());
}

#[test]
fn test_cc_sk_015_string_false() {
    let content = r#"---
name: test-skill
description: Use when testing
user-invocable: "false"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert!(cc_sk_015.is_empty());
}

#[test]
fn test_cc_sk_015_boolean_false_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
user-invocable: false
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert_eq!(cc_sk_015.len(), 0);
}

#[test]
fn test_cc_sk_015_quoted_alias_needs_no_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
user-invocable: "false"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert!(cc_sk_015.is_empty());
}

#[test]
fn test_cc_sk_015_invalid_alias_is_reported_without_guessing_a_fix() {
    let content = r#"---
name: test-skill
description: Use when testing
user-invocable: "maybe"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert_eq!(cc_sk_015.len(), 1);
    assert!(!cc_sk_015[0].has_fixes());
}

#[test]
fn test_cc_sk_014_fixture() {
    let content = include_str!(
        "../../../../../tests/fixtures/invalid/skills/string-boolean-disable/SKILL.md"
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert_eq!(
        cc_sk_014.len(),
        1,
        "String boolean disable fixture should trigger CC-SK-014"
    );
}

#[test]
fn test_cc_sk_015_fixture() {
    let content = include_str!(
        "../../../../../tests/fixtures/invalid/skills/string-boolean-invocable/SKILL.md"
    );

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("SKILL.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert_eq!(
        cc_sk_015.len(),
        1,
        "String boolean invocable fixture should trigger CC-SK-015"
    );
}

// ===== CC-SK-014/015: Inline comment handling =====

#[test]
fn test_cc_sk_014_with_inline_comment() {
    let content = "---\nname: test-skill\ndescription: Use when testing\ndisable-model-invocation: \"true\" # some comment\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();

    assert!(cc_sk_014.is_empty());
}

#[test]
fn test_cc_sk_015_with_inline_comment() {
    let content = "---\nname: test-skill\ndescription: Use when testing\nuser-invocable: \"false\" # override default\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_015: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-015")
        .collect();

    assert!(cc_sk_015.is_empty());
}

// ===== CC-SK-014/015: No AS-016 false positive =====

#[test]
fn test_cc_sk_014_does_not_produce_as_016() {
    let content = r#"---
name: test-skill
description: Use when testing
disable-model-invocation: "true"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let as_016: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    let cc_sk_014: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-014")
        .collect();
    assert!(cc_sk_014.is_empty());
    assert!(as_016.is_empty());
}

#[test]
fn test_claude_boolean_aliases_are_case_insensitive() {
    for value in ["YES", "no", "On", "OFF", "1", "0"] {
        let content = format!(
            "---\nname: test-skill\ndescription: Use when testing\ndisable-model-invocation: {value}\nuser-invocable: '{value}'\n---\nBody"
        );
        let diagnostics = SkillValidator.validate(
            Path::new(".claude/skills/test-skill/SKILL.md"),
            &content,
            &LintConfig::default(),
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| !matches!(d.rule.as_str(), "CC-SK-014" | "CC-SK-015" | "AS-016")),
            "alias {value} should parse cleanly: {diagnostics:?}"
        );
    }
}

#[test]
fn test_fork_background_false_is_a_known_valid_field() {
    let content = "---\nname: test-skill\ndescription: Use when testing\ncontext: fork\nbackground: false\n---\nRun the review.";
    let diagnostics = SkillValidator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );
    assert!(
        diagnostics
            .iter()
            .all(|d| !matches!(d.rule.as_str(), "CC-SK-017" | "AS-016")),
        "background should be accepted: {diagnostics:?}"
    );
}

// ===== CC-SK-013: Empty body with fork context =====

#[test]
fn test_cc_sk_013_empty_body_fork() {
    let content = "---\nname: test-skill\ndescription: Use when testing\ncontext: fork\n---\n";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(
        cc_sk_013.len(),
        1,
        "Empty body with fork context should trigger CC-SK-013"
    );
}

#[test]
fn test_cc_sk_013_whitespace_only_body_fork() {
    let content =
        "---\nname: test-skill\ndescription: Use when testing\ncontext: fork\n---\n   \n  \n";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_013: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-013")
        .collect();

    assert_eq!(
        cc_sk_013.len(),
        1,
        "Whitespace-only body with fork context should trigger CC-SK-013"
    );
}

// ===== CC-SK-005 auto-fix tests =====

#[test]
fn test_cc_sk_005_autofix_invalid_agent() {
    let content = "---\nname: my-skill\ndescription: Use when testing\ncontext: fork\nagent: INVALID_AGENT\n---\nRun the tests.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();
    assert_eq!(cc_sk_005.len(), 1);
    assert!(
        cc_sk_005[0].has_fixes(),
        "CC-SK-005 should have auto-fix for invalid agent"
    );
    let fix = &cc_sk_005[0].fixes[0];
    assert!(!fix.safe, "CC-SK-005 fix should be unsafe");
    assert_eq!(
        fix.replacement, "general-purpose",
        "Fix should replace with 'general-purpose'"
    );
    // Verify the fix targets the correct bytes
    let target = &content[fix.start_byte..fix.end_byte];
    assert_eq!(target, "INVALID_AGENT");
}

#[test]
fn test_cc_sk_005_no_fix_for_valid_agent() {
    // Valid built-in agent should not trigger CC-SK-005
    let content = "---\nname: my-skill\ndescription: Use when testing\ncontext: fork\nagent: general-purpose\n---\nRun the tests.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_005: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-005")
        .collect();
    assert!(cc_sk_005.is_empty());
}

// ===== AS-016 suggestion test =====

#[test]
fn test_as_016_has_suggestion() {
    let content = "---\n invalid: [yaml\n---\ncontent";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let parse_errors: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-016").collect();
    assert_eq!(parse_errors.len(), 1);
    assert!(
        parse_errors[0].suggestion.is_some(),
        "AS-016 should have a suggestion"
    );
    assert!(
        parse_errors[0]
            .suggestion
            .as_ref()
            .unwrap()
            .contains("YAML frontmatter syntax"),
        "AS-016 suggestion should mention YAML frontmatter syntax"
    );
}

// ===== CC-SK-006 auto-fix tests =====

#[test]
fn test_cc_sk_006_has_fix() {
    // Dangerous name "deploy" without disable-model-invocation
    let content = "---\nname: deploy-app\ndescription: Use when deploying the app\n---\nDeploy the application.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_006: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-006")
        .collect();
    assert_eq!(cc_sk_006.len(), 1);
    assert!(cc_sk_006[0].has_fixes(), "CC-SK-006 should have auto-fix");
    let fix = &cc_sk_006[0].fixes[0];
    assert!(
        !fix.safe,
        "CC-SK-006 fix should be unsafe (changes runtime behavior)"
    );
    assert!(
        fix.replacement.contains("disable-model-invocation: true"),
        "Fix should insert disable-model-invocation: true"
    );
}

// ===== CC-SK-012 auto-fix tests =====

#[test]
fn test_cc_sk_012_has_fix() {
    // Has argument-hint but body lacks $ARGUMENTS
    let content = "---\nname: greet-user\ndescription: Use when greeting users\nargument-hint: Name of person to greet\n---\nGreet the user warmly.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let cc_sk_012: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();
    assert_eq!(cc_sk_012.len(), 1);
    assert!(cc_sk_012[0].has_fixes(), "CC-SK-012 should have auto-fix");
    let fix = &cc_sk_012[0].fixes[0];
    assert!(
        !fix.safe,
        "CC-SK-012 fix should be unsafe (appends to body)"
    );
    assert!(
        fix.replacement.contains("$ARGUMENTS"),
        "Fix should append $ARGUMENTS"
    );
}

// ===== AS-001 auto-fix tests =====

#[test]
fn test_as_001_has_fix() {
    let content = "Some skill content without frontmatter.";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let as_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-001").collect();
    assert_eq!(as_001.len(), 1);
    assert!(as_001[0].has_fixes(), "AS-001 should have auto-fix");
    let fix = &as_001[0].fixes[0];
    assert!(!fix.safe, "AS-001 fix should be unsafe");
    assert!(
        fix.replacement.contains("---"),
        "Fix should insert frontmatter block"
    );
}

// ===== AS-002 auto-fix tests =====

#[test]
fn test_as_002_has_fix() {
    let content = "---\ndescription: Use when testing\n---\nBody";
    let validator = SkillValidator;
    let diagnostics =
        validator.validate(Path::new("test-skill.md"), content, &LintConfig::default());
    let as_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-002").collect();
    assert_eq!(as_002.len(), 1);
    assert!(as_002[0].has_fixes(), "AS-002 should have auto-fix");
    let fix = &as_002[0].fixes[0];
    assert!(!fix.safe, "AS-002 fix should be unsafe");
    assert!(
        fix.replacement.contains("name:"),
        "Fix should insert name field"
    );
}

// ===== AS-003 auto-fix tests =====

#[test]
fn test_as_003_has_fix() {
    let content = "---\nname: test-skill\n---\nBody";
    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());
    let as_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-003").collect();
    assert_eq!(as_003.len(), 1);
    assert!(as_003[0].has_fixes(), "AS-003 should have auto-fix");
    let fix = &as_003[0].fixes[0];
    assert!(!fix.safe, "AS-003 fix should be unsafe");
    assert!(
        fix.replacement.contains("description:"),
        "Fix should insert description placeholder"
    );
}

// ===== AS-009 auto-fix tests =====

#[test]
fn test_as_009_has_fix() {
    // AS-009 is Codex-only, so use a Codex skill path to trigger it.
    let content = "---\nname: test-skill\ndescription: <b>Use when testing</b>\n---\nBody";
    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".agents/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );
    let as_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-009").collect();
    assert_eq!(as_009.len(), 1);
    assert!(
        as_009[0].has_fixes(),
        "AS-009 should have auto-fix to strip XML tags"
    );
    let fix = &as_009[0].fixes[0];
    assert!(!fix.safe, "AS-009 fix should be unsafe");
    assert!(
        !fix.replacement.contains('<'),
        "Fix should strip XML tags from description"
    );
}

// ===== CC-SK-018: Invalid effort value =====

#[test]
fn test_cc_sk_018_invalid_effort() {
    let content = r#"---
name: test-skill
description: Use when testing effort validation
effort: turbo
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_018: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-018")
        .collect();

    assert_eq!(cc_sk_018.len(), 1);
    assert_eq!(
        cc_sk_018[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
    assert!(cc_sk_018[0].message.contains("turbo"));
}

#[test]
fn test_cc_sk_018_valid_effort_values() {
    for effort in VALID_EFFORT_LEVELS {
        let content = format!(
            "---\nname: test-skill\ndescription: Use when testing effort\neffort: {}\n---\nBody",
            effort
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_018: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-018")
            .collect();

        assert_eq!(cc_sk_018.len(), 0, "Effort '{}' should be valid", effort);
    }
}

#[test]
fn test_cc_sk_018_no_effort_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_018: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-018")
        .collect();

    assert_eq!(cc_sk_018.len(), 0);
}

// ===== CC-SK-019: Invalid paths format =====

#[test]
fn test_cc_sk_019_empty_paths() {
    let content =
        "---\nname: test-skill\ndescription: Use when testing paths\npaths: \"\"\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_019: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-019")
        .collect();

    assert_eq!(cc_sk_019.len(), 1);
    assert_eq!(
        cc_sk_019[0].level,
        crate::diagnostics::DiagnosticLevel::Info
    );
}

#[test]
fn test_cc_sk_019_whitespace_only_paths() {
    let content =
        "---\nname: test-skill\ndescription: Use when testing paths\npaths: \"   \"\n---\nBody";

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_019: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-019")
        .collect();

    assert_eq!(cc_sk_019.len(), 1);
    assert_eq!(
        cc_sk_019[0].level,
        crate::diagnostics::DiagnosticLevel::Info
    );
}

#[test]
fn test_cc_sk_019_valid_paths() {
    let content = r#"---
name: test-skill
description: Use when testing paths
paths: "src/**/*.ts"
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_019: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-019")
        .collect();

    assert_eq!(cc_sk_019.len(), 0);
}

// ===== CC-SK-020: Invalid shell value =====

#[test]
fn test_cc_sk_020_invalid_shell() {
    let content = r#"---
name: test-skill
description: Use when testing shell validation
shell: zsh
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_020: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-020")
        .collect();

    assert_eq!(cc_sk_020.len(), 1);
    assert_eq!(
        cc_sk_020[0].level,
        crate::diagnostics::DiagnosticLevel::Warning
    );
    assert!(cc_sk_020[0].message.contains("zsh"));
}

#[test]
fn test_cc_sk_020_valid_shell_values() {
    for shell in &["bash", "powershell"] {
        let content = format!(
            "---\nname: test-skill\ndescription: Use when testing shell\nshell: {}\n---\nBody",
            shell
        );

        let validator = SkillValidator;
        let diagnostics =
            validator.validate(Path::new("test.md"), &content, &LintConfig::default());

        let cc_sk_020: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "CC-SK-020")
            .collect();

        assert_eq!(cc_sk_020.len(), 0, "Shell '{}' should be valid", shell);
    }
}

#[test]
fn test_cc_sk_020_no_shell_ok() {
    let content = r#"---
name: test-skill
description: Use when testing
---
Body"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(Path::new("test.md"), content, &LintConfig::default());

    let cc_sk_020: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-020")
        .collect();

    assert_eq!(cc_sk_020.len(), 0);
}

// ===== CC-SK-021: Hardcoded User Directory Path =====

#[test]
fn test_find_hardcoded_user_paths_posix_and_windows() {
    let hits =
        find_hardcoded_user_paths("see /Users/alice/x/y and C:\\Users\\bob.smith\\AppData here");
    let texts: Vec<&str> = hits.iter().map(|h| h.text.as_str()).collect();
    assert_eq!(texts, vec!["/Users/alice", "C:\\Users\\bob.smith"]);
}

#[test]
fn test_find_hardcoded_user_paths_home() {
    let hits = find_hardcoded_user_paths("data at /home/bob/data.csv");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].text, "/home/bob");
}

#[test]
fn test_find_hardcoded_user_paths_without_trailing_slash() {
    // A bare username with no trailing path component is still flagged.
    let hits = find_hardcoded_user_paths("home is /home/alice");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].text, "/home/alice");
}

#[test]
fn test_is_script_path_unknown_ext_with_shebang() {
    // Shebang wins over an unrecognized extension.
    assert!(is_script_path(Path::new("a/tool.custom"), "#!/bin/sh\n"));
    assert!(!is_script_path(Path::new("a/data.custom"), "not a script"));
}

#[test]
fn test_find_hardcoded_user_paths_skips_placeholders() {
    // Generic placeholder names, angle-bracket, template, and env-var forms.
    let content =
        "/Users/user/ /home/example/ /Users/foo/ /Users/<name>/ /home/${USER}/ /home/{{name}}/";
    assert!(find_hardcoded_user_paths(content).is_empty());
}

#[test]
fn test_is_script_path() {
    assert!(is_script_path(Path::new("a/run.py"), ""));
    assert!(is_script_path(Path::new("a/run.sh"), ""));
    assert!(!is_script_path(Path::new("a/notes.md"), "x"));
    assert!(!is_script_path(Path::new("a/data.txt"), "x"));
    // extensionless: only a script when it starts with a shebang
    assert!(is_script_path(
        Path::new("a/runner"),
        "#!/usr/bin/env bash\n"
    ));
    assert!(!is_script_path(Path::new("a/runner"), "plain text\n"));
}

fn cc_sk_021_for(skill_dir: &Path) -> Vec<Diagnostic> {
    let skill_path = skill_dir.join("SKILL.md");
    let content = fs::read_to_string(&skill_path).unwrap();
    SkillValidator
        .validate(&skill_path, &content, &LintConfig::default())
        .into_iter()
        .filter(|d| d.rule == "CC-SK-021")
        .collect()
}

const CLEAN_SKILL: &str = "---\nname: demo\ndescription: Use when demoing the rule for testing here.\n---\nRun `cargo build` from the repo root.\n";

#[test]
fn test_cc_sk_021_flags_bundled_md() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(dir.join("SKILL.md"), CLEAN_SKILL).unwrap();
    fs::write(
        dir.join("references/bad.md"),
        "# ref\nConfig at /Users/alice/projects/cfg.json\n",
    )
    .unwrap();

    let diags = cc_sk_021_for(&dir);
    assert_eq!(diags.len(), 1, "{diags:?}");
    assert!(diags[0].file.ends_with("references/bad.md"));
}

#[test]
fn test_cc_sk_021_flags_skill_md_itself() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: demo\ndescription: Use when demoing the rule for testing here.\n---\nRun /Users/alice/build.sh now.\n",
    )
    .unwrap();

    let diags = cc_sk_021_for(&dir);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].file.ends_with("SKILL.md"));
}

#[test]
fn test_cc_sk_021_flags_script_shebang_and_body() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(dir.join("SKILL.md"), CLEAN_SKILL).unwrap();
    fs::write(
        dir.join("scripts/bad.sh"),
        "#!/Users/alice/.venv/bin/python\n# fixture: /home/bob/data.csv\n",
    )
    .unwrap();

    let diags = cc_sk_021_for(&dir);
    assert_eq!(diags.len(), 2, "{diags:?}");
    assert!(diags.iter().all(|d| d.file.ends_with("scripts/bad.sh")));
}

#[test]
fn test_cc_sk_021_flags_extensionless_shebang() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(dir.join("SKILL.md"), CLEAN_SKILL).unwrap();
    fs::write(
        dir.join("scripts/runner"),
        "#!/Users/carol/bin/run\necho hi\n",
    )
    .unwrap();

    let diags = cc_sk_021_for(&dir);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].file.ends_with("scripts/runner"));
}

#[test]
fn test_cc_sk_021_flags_python_comment() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("bundled")).unwrap();
    fs::write(dir.join("SKILL.md"), CLEAN_SKILL).unwrap();
    fs::write(
        dir.join("bundled/example.py"),
        "import os  # path /Users/alice/data\n",
    )
    .unwrap();

    let diags = cc_sk_021_for(&dir);
    assert_eq!(diags.len(), 1);
    assert!(diags[0].file.ends_with("bundled/example.py"));
}

#[test]
fn test_cc_sk_021_all_three_sources() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::create_dir_all(dir.join("references")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: demo\ndescription: Use when demoing the rule for testing here.\n---\nRun /Users/alice/build.sh now.\n",
    )
    .unwrap();
    fs::write(dir.join("references/bad.md"), "ref /home/bob/x\n").unwrap();
    fs::write(dir.join("scripts/bad.sh"), "#!/Users/carol/run\n").unwrap();

    let diags = cc_sk_021_for(&dir);
    let files: std::collections::BTreeSet<String> = diags
        .iter()
        .map(|d| d.file.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(diags.len(), 3, "{diags:?}");
    assert_eq!(files.len(), 3);
}

#[test]
fn test_cc_sk_021_good_placeholders_clean() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(dir.join("scripts")).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        "---\nname: demo\ndescription: Use when demoing the rule for testing here.\n---\nReplace `/Users/<name>/` with your home. Logs in `$HOME/.cache/`. Template `/home/${USER}/c/`.\n",
    )
    .unwrap();
    fs::write(
        dir.join("scripts/ok.sh"),
        "#!/usr/bin/env bash\noutput=\"${HOME}/.cache/app\"\ncd \"$(git rev-parse --show-toplevel)\"\n",
    )
    .unwrap();

    assert!(cc_sk_021_for(&dir).is_empty());
}

#[test]
fn test_cc_sk_021_ignores_files_outside_skill_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("demo");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("SKILL.md"), CLEAN_SKILL).unwrap();
    // Sibling of the skill dir, not bundled with the skill.
    fs::write(tmp.path().join("build.sh"), "#!/Users/alice/run\n").unwrap();

    assert!(cc_sk_021_for(&dir).is_empty());
}

/// CC-SK-009 counted raw `` !` `` occurrences, which was wrong twice over:
/// inert `` KEY=!`cmd` `` forms were counted (the doc says a `!` following
/// another character stays literal), and ` ```! ` fenced blocks were not counted
/// at all.
#[test]
fn test_cc_sk_009_ignores_inert_inline_injections() {
    let content = r#"---
name: test-skill
description: Use when testing inert injection forms
---
Run the setup with these values.

KEY1=!`date`
KEY2=!`whoami`
KEY3=!`pwd`
KEY4=!`hostname`
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();
    assert!(
        hits.is_empty(),
        "`KEY=!`cmd`` is literal text and must not count as an injection, got: {hits:?}"
    );
}

/// A ` ```! ` fenced block runs each line as a command, so its lines count.
#[test]
fn test_cc_sk_009_counts_shell_fence_lines() {
    let content = r#"---
name: test-skill
description: Use when testing fenced shell injection counting
---
Check the environment.

```!
node --version
npm --version
git status --short
uname -a
```
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "four commands in a ```! fence exceed the limit of 3, got: {diagnostics:?}"
    );
}

/// A plain fenced block is inert and must not contribute counts.
#[test]
fn test_cc_sk_009_ignores_plain_fence() {
    let content = r#"---
name: test-skill
description: Use when testing that plain fences are inert
---
Example output:

```bash
!`one`
!`two`
!`three`
!`four`
```
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-009")
        .collect();
    assert!(
        hits.is_empty(),
        "a plain ```bash fence does not execute and must not be counted, got: {hits:?}"
    );
}

/// Genuine inline injections past the limit still fire.
#[test]
fn test_cc_sk_009_still_flags_real_inline_injections() {
    let content = r#"---
name: test-skill
description: Use when testing that real inline injections are counted
---
Current state: !`date` and !`whoami` and !`pwd` and !`hostname`
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().any(|d| d.rule == "CC-SK-009"),
        "four recognized inline injections must exceed the limit, got: {diagnostics:?}"
    );
}

/// The substitution table documents four argument forms. CC-SK-012 only looked
/// for the literal `$ARGUMENTS`, so a body using `$0` or a declared `$name` was
/// falsely reported as ignoring its own arguments.
#[test]
fn test_cc_sk_012_accepts_positional_shorthand() {
    let content = r#"---
name: test-skill
description: Use when testing positional shorthand recognition
argument-hint: "[issue-number] [branch]"
---
Fix issue $0 on branch $1.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();
    assert!(
        hits.is_empty(),
        "`$0`/`$1` are documented shorthand for $ARGUMENTS[N], got: {hits:?}"
    );
}

/// `$name` placeholders declared in the `arguments` frontmatter list also count.
#[test]
fn test_cc_sk_012_accepts_named_arguments() {
    let content = r#"---
name: test-skill
description: Use when testing named argument recognition
argument-hint: "[issue] [branch]"
arguments: issue branch
---
Fix $issue on $branch.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-012")
        .collect();
    assert!(
        hits.is_empty(),
        "`$issue`/`$branch` are declared in `arguments` and must count, got: {hits:?}"
    );
}

/// A body that references no argument at all still fires.
#[test]
fn test_cc_sk_012_still_flags_body_with_no_arguments() {
    let content = r#"---
name: test-skill
description: Use when testing that a body ignoring arguments is flagged
argument-hint: "[issue-number]"
---
Do the thing with no argument reference at all.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().any(|d| d.rule == "CC-SK-012"),
        "a body referencing no arguments must still be flagged, got: {diagnostics:?}"
    );
}

/// CC-SK-016 covers the `$N` shorthand too, since it is documented as
/// equivalent to `$ARGUMENTS[N]`.
#[test]
fn test_cc_sk_016_flags_positional_shorthand_without_hint() {
    let content = r#"---
name: test-skill
description: Use when testing shorthand without an argument hint
---
Fix issue $0 for the reporter.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().any(|d| d.rule == "CC-SK-016"),
        "`$0` without argument-hint has the same gap as $ARGUMENTS[0], got: {diagnostics:?}"
    );
}

/// `${CLAUDE_SKILL_DIR}`-style variables and dollar amounts must not be
/// mistaken for positional arguments.
#[test]
fn test_cc_sk_016_ignores_non_positional_dollar_forms() {
    let content = r#"---
name: test-skill
description: Use when testing that other dollar forms are not positional
---
Run ${CLAUDE_SKILL_DIR}/scripts/run.sh and report the $500 total for item x$3y.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/test-skill/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics
        .iter()
        .filter(|d| d.rule == "CC-SK-016")
        .collect();
    assert!(
        hits.is_empty(),
        "`${{CLAUDE_SKILL_DIR}}`, `$500`, and `x$3y` are not positional arguments, got: {hits:?}"
    );
}

/// AS-013 only inspected `references/`-prefixed paths, so deep `scripts/` and
/// `assets/` paths were invisible - even though the spec scopes skill resources
/// to "`scripts/`, `references/`, or `assets/`" and uses `scripts/extract.py` in
/// its own example.
#[test]
fn test_as_013_covers_scripts_and_assets() {
    for path in [
        "scripts/deep/nested/extract.py",
        "assets/deep/nested/logo.png",
    ] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing deep resource reference detection
---
Run the helper at {path} to continue.
"#
        );

        let validator = SkillValidator;
        let diagnostics = validator.validate(
            Path::new(".claude/skills/test-skill/SKILL.md"),
            &content,
            &LintConfig::default(),
        );

        assert!(
            diagnostics.iter().any(|d| d.rule == "AS-013"),
            "deep path '{path}' must be reported by AS-013, got: {diagnostics:?}"
        );
    }
}

/// One level deep stays clean for the newly covered prefixes.
#[test]
fn test_as_013_allows_one_level_scripts_and_assets() {
    for path in ["scripts/extract.py", "assets/logo.png"] {
        let content = format!(
            r#"---
name: test-skill
description: Use when testing shallow resource references stay clean
---
Run the helper at {path} to continue.
"#
        );

        let validator = SkillValidator;
        let diagnostics = validator.validate(
            Path::new(".claude/skills/test-skill/SKILL.md"),
            &content,
            &LintConfig::default(),
        );

        let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-013").collect();
        assert!(
            hits.is_empty(),
            "'{path}' is one level deep and must not be flagged, got: {hits:?}"
        );
    }
}

/// For a Claude Code skill, `name` is a display label - "the command still comes
/// from the directory or file name" - and a plugin skill's `name` deliberately
/// replaces the last command segment. AS-017 used to reject the reference's own
/// `name: fancy` in `skills/review/` example.
#[test]
fn test_as_017_scoped_out_for_claude_code_skills() {
    let content = r#"---
name: fancy
description: Use when testing that a Claude Code display label is allowed
---
Body with instructions to run the task.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".claude/skills/review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    let hits: Vec<_> = diagnostics.iter().filter(|d| d.rule == "AS-017").collect();
    assert!(
        hits.is_empty(),
        "`name` is a display label for Claude Code skills and must not trip AS-017, got: {hits:?}"
    );
}

/// The agentskills.io baseline still requires the match, so a skill owned by
/// another client keeps the rule.
#[test]
fn test_as_017_still_enforced_for_non_claude_clients() {
    let content = r#"---
name: mismatched
description: Use when testing that the baseline still requires a directory match
---
Body with instructions to run the task.
"#;

    let validator = SkillValidator;
    let diagnostics = validator.validate(
        Path::new(".opencode/skills/review/SKILL.md"),
        content,
        &LintConfig::default(),
    );

    assert!(
        diagnostics.iter().any(|d| d.rule == "AS-017"),
        "the agentskills.io baseline requires name==directory, got: {diagnostics:?}"
    );
}
