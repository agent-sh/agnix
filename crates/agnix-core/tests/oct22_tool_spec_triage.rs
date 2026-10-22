//! Primary-source compatibility cases reviewed 2026-10-22.
use agnix_core::{LintConfig, ValidatorRegistry, validate_content};
use std::path::Path;

#[test]
fn claude_notification_accepts_plugin_notification_matcher() {
    // https://code.claude.com/docs/en/hooks#matcher-patterns
    // Plugin notifications are a documented Notification type, not a typo.
    for matcher in ["plugin_notification", "plugin_notification|idle_prompt"] {
        let content = serde_json::json!({"hooks": {"Notification": [{
            "matcher": matcher,
            "hooks": [{"type": "command", "command": "echo notification"}]
        }]}});
        let diagnostics = validate_content(
            Path::new(".claude/settings.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(
            diagnostics.iter().all(|d| d.rule != "CC-HK-025"),
            "{matcher}: {diagnostics:?}"
        );
    }
}

#[test]
fn opencode_210_accepts_experimental_todo_without_new_rule() {
    // The only config schema delta between v2.0.32 and v2.1.0 is
    // experimental.todo (packages/schema/src/config/experimental.ts).
    // Experimental keys are intentionally open, so no allowlist change is needed.
    let config = LintConfig::builder()
        .tool_versions(agnix_core::config::ToolVersions {
            opencode: Some("2.1.0".to_string()),
            ..Default::default()
        })
        .build()
        .unwrap();
    for enabled in [true, false] {
        for file in ["opencode.json", "opencode.jsonc"] {
            let diagnostics = validate_content(
                Path::new(file),
                &serde_json::json!({"experimental": {"todo": enabled}}).to_string(),
                &config,
                &ValidatorRegistry::with_defaults(),
            );
            assert!(diagnostics.is_empty(), "{file}: {diagnostics:?}");
        }
    }
}

#[test]
fn kiro_232_workflow_access_comes_from_agent_not_new_step_fields() {
    // https://kiro.dev/docs/workflows/authoring.md#step
    // Powers/MCP access belongs to the selected agent. The release does not
    // introduce a workflow-step access field or require a new KR-WF-010 rule.
    for agent in ["wf-planner", "wf-coder", "wf-design"] {
        let content = serde_json::json!({
            "name": "access-review",
            "steps": [{"type": "step", "id": "review", "agent": agent,
                "prompt": "Review the project"}]
        });
        let diagnostics = validate_content(
            Path::new(".kiro/workflows/access.workflow.json"),
            &content.to_string(),
            &LintConfig::default(),
            &ValidatorRegistry::with_defaults(),
        );
        assert!(diagnostics.is_empty(), "{agent}: {diagnostics:?}");
    }
}

#[test]
fn claude_notification_still_rejects_unknown_matcher_without_autofix() {
    let diagnostics = validate_content(
        Path::new(".claude/settings.json"),
        r#"{"hooks":{"Notification":[{"matcher":"plugin_notificaton","hooks":[{"type":"command","command":"echo notification"}]}]}}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    let diagnostic = diagnostics
        .iter()
        .find(|d| d.rule == "CC-HK-025")
        .expect("unknown notification type remains a warning");
    assert_eq!(diagnostic.level, agnix_core::DiagnosticLevel::Warning);
    assert!(diagnostic.fixes.is_empty());
}

#[test]
fn cursor_current_hook_contract_remains_accepted() {
    // https://cursor.com/docs/hooks.md - Hook configuration options.
    let diagnostics = validate_content(
        Path::new(".cursor/hooks.json"),
        r#"{"version":1,"hooks":{"beforeMCPExecution":[{"command":"echo check","matcher":".*","failClosed":true,"timeout":30}],"stop":[{"type":"prompt","prompt":"Verify the result","loop_limit":null}]}}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(diagnostics.is_empty(), "{diagnostics:?}");
}

#[test]
fn claude_current_plugin_component_fields_are_known() {
    // https://code.claude.com/docs/en/plugins-reference.md - Fields.
    let diagnostics = validate_content(
        Path::new(".claude-plugin/plugin.json"),
        r#"{"name":"review-demo","description":"Review demo","version":"1.0.0","types":"./index.d.ts","workflows":"./workflows","experimental":{"themes":"./themes","monitors":[],"evals":"./evals"},"metadata":{"owner":"team"},"defaultEnabled":true}"#,
        &LintConfig::default(),
        &ValidatorRegistry::with_defaults(),
    );
    assert!(
        diagnostics
            .iter()
            .all(|d| !matches!(d.rule.as_str(), "CC-PL-006" | "CC-PL-017")),
        "{diagnostics:?}"
    );
}
