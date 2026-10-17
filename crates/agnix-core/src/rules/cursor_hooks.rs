//! Cursor project hook checks beyond the schema (CUR-010 to CUR-019 live in
//! `rules::cursor`): CUR-022 (script path relative to the wrong base), CUR-032
//! (MCP permission hook that fails open) and CUR-033 (hooks cloud agents do
//! not run).
//!
//! "Project hooks run from the **project root**, so use
//! `.cursor/hooks/format.sh` (not `./hooks/format.sh`)", while user hooks
//! (`~/.cursor/hooks.json`) run from `~/.cursor/`.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    rules::config_json::{JsonCx, key_span, node_span, str_field},
    schemas::kiro_json::{JsonNode, parse_json_document},
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

/// Hooks that "don't apply to cloud agents".
const NOT_IN_CLOUD_AGENTS: &[&str] = &[
    "sessionStart",
    "sessionEnd",
    "beforeMCPExecution",
    "afterMCPExecution",
    "beforeTabFileRead",
    "afterTabFileEdit",
    "workspaceOpen",
];

/// The project root of a project hook file (`<project>/.cursor/hooks.json`),
/// or `None` for the user file `~/.cursor/hooks.json` and bare relative paths.
fn project_root(path: &Path) -> Option<PathBuf> {
    let dot_cursor = path.parent()?;
    let root = dot_cursor.parent()?;
    if root.as_os_str().is_empty() {
        return None;
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"));
    if home.is_some_and(|home| Path::new(&home) == root) {
        return None;
    }
    Some(root.to_path_buf())
}

/// The first word of a command when it is a relative path (`./x.sh`,
/// `hooks/x.sh`, `.cursor/hooks/x`): it has a `/`, does not start with `/`,
/// `~` or a variable, and has no shell syntax.
fn relative_path_token(command: &str) -> Option<&str> {
    let first = command.split_whitespace().next()?;
    let ok = first.contains('/')
        && !first.starts_with('/')
        && !first.starts_with('~')
        && !first.contains([
            '$', '`', '"', '\'', '*', '?', '(', ')', ';', '&', '|', '<', '>',
        ]);
    ok.then_some(first)
}

pub(crate) fn validate_hook_extras(
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut cx = JsonCx::new(path, content, config);
    let Some(root_dir) = project_root(path) else {
        return cx.diagnostics;
    };
    let Ok(document) = parse_json_document(content, false) else {
        return cx.diagnostics;
    };
    let Some(events) = document
        .root
        .value
        .get("hooks")
        .and_then(|m| m.value.value.as_object())
    else {
        return cx.diagnostics;
    };
    let cloud_agents = config
        .fs()
        .is_file(&root_dir.join(".cursor").join("environment.json"));

    for event in events {
        let name = event.key.as_str();
        if cloud_agents && cx.enabled("CUR-033") && NOT_IN_CLOUD_AGENTS.contains(&name) {
            cx.push(
                DiagnosticLevel::Info,
                "CUR-033",
                key_span(event),
                t!("rules.cur_033.event", event = name),
                t!("rules.cur_033.suggestion"),
            );
        }
        let Some(entries) = event.value.value.as_array() else {
            continue;
        };
        for entry in entries {
            if entry.value.as_object().is_none() {
                continue;
            }
            let kind = str_field(entry, "type").unwrap_or("command");
            if kind == "prompt" {
                if cloud_agents && cx.enabled("CUR-033") {
                    let span = entry.value.get("type").map_or(node_span(entry), key_span);
                    cx.push(
                        DiagnosticLevel::Info,
                        "CUR-033",
                        span,
                        t!("rules.cur_033.prompt", event = name),
                        t!("rules.cur_033.suggestion"),
                    );
                }
                continue;
            }
            // "Set `failClosed: true` on the hook definition to block on those
            // failures too. This is recommended for security-critical
            // `beforeMCPExecution` hooks."
            if name == "beforeMCPExecution"
                && cx.enabled("CUR-032")
                && entry
                    .value
                    .get("failClosed")
                    .and_then(|m| m.value.value.as_bool())
                    != Some(true)
            {
                cx.push(
                    DiagnosticLevel::Info,
                    "CUR-032",
                    node_span(entry),
                    t!("rules.cur_032.message"),
                    t!("rules.cur_032.suggestion"),
                );
            }
            if cx.enabled("CUR-022")
                && let Some(member) = entry.value.get("command")
            {
                check_script_base(&mut cx, content, &member.value, &root_dir, config);
            }
        }
    }
    cx.diagnostics
}

fn check_script_base(
    cx: &mut JsonCx<'_>,
    content: &str,
    node: &JsonNode,
    root: &Path,
    config: &PerFileLintConfig<'_>,
) {
    let Some(command) = node.value.as_str() else {
        return;
    };
    let Some(token) = relative_path_token(command) else {
        return;
    };
    let fs = config.fs();
    let bare = token.trim_start_matches("./");
    if fs.exists(&root.join(bare)) {
        return;
    }
    let under_cursor = Path::new(".cursor").join(bare);
    if !bare.starts_with(".cursor/") && fs.exists(&root.join(&under_cursor)) {
        let fixed = format!(".cursor/{bare}");
        let mut diagnostic_fix = None;
        // Replace the path in place when the JSON string starts with it
        // verbatim (no escapes before it).
        let text_start = node.start + 1;
        if content
            .get(text_start..text_start + token.len())
            .is_some_and(|raw| raw == token)
        {
            diagnostic_fix = Some(Fix::replace(
                text_start,
                text_start + token.len(),
                fixed.clone(),
                t!("rules.cur_022.fix", path = fixed.as_str()),
                false,
            ));
        }
        cx.push(
            DiagnosticLevel::Error,
            "CUR-022",
            node_span(node),
            t!(
                "rules.cur_022.missing",
                path = token,
                fixed = fixed.as_str()
            ),
            t!("rules.cur_022.suggestion", fixed = fixed.as_str()),
        );
        if let (Some(fix), Some(last)) = (diagnostic_fix, cx.diagnostics.last_mut()) {
            last.fixes.push(fix);
        }
        return;
    }
    if (token.starts_with("./hooks/") || token.starts_with("hooks/"))
        && !fs.is_dir(&root.join("hooks"))
    {
        cx.push(
            DiagnosticLevel::Warning,
            "CUR-022",
            node_span(node),
            t!("rules.cur_022.user_style", path = token),
            t!("rules.cur_022.user_style_suggestion"),
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::config::LintConfig;
    use crate::diagnostics::{Diagnostic, DiagnosticLevel};
    use crate::rules::Validator;
    use crate::rules::cursor::CursorValidator;

    fn project() -> tempfile::TempDir {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join(".cursor/hooks")).unwrap();
        std::fs::write(temp.path().join(".cursor/hooks/format.sh"), "#!/bin/bash\n").unwrap();
        temp
    }

    fn validate(root: &std::path::Path, content: &str) -> Vec<Diagnostic> {
        let path = root.join(".cursor/hooks.json");
        CursorValidator.validate(&path, content, &LintConfig::default())
    }

    fn rule<'a>(diagnostics: &'a [Diagnostic], id: &str) -> Vec<&'a Diagnostic> {
        diagnostics.iter().filter(|d| d.rule == id).collect()
    }

    #[test]
    fn cur_022_project_hook_paths() {
        let temp = project();
        let content = r#"{
  "version": 1,
  "hooks": {
    "afterFileEdit": [
      { "command": "./hooks/format.sh --all" },
      { "command": ".cursor/hooks/format.sh" },
      { "command": "./hooks/missing.sh" },
      { "command": "npx prettier --write ." },
      { "command": "bun run scripts/check.ts" }
    ]
  }
}"#;
        let diagnostics = validate(temp.path(), content);
        let cur_022 = rule(&diagnostics, "CUR-022");
        assert_eq!(cur_022.len(), 2, "{cur_022:#?}");
        let error = cur_022
            .iter()
            .find(|d| d.level == DiagnosticLevel::Error)
            .unwrap();
        assert_eq!(error.line, 5);
        let fix = &error.fixes[0];
        assert!(!fix.safe);
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("\".cursor/hooks/format.sh --all\""));
        assert!(serde_json::from_str::<serde_json::Value>(&fixed).is_ok());
        assert!(
            rule(&validate(temp.path(), &fixed), "CUR-022")
                .iter()
                .all(|d| d.line != 5)
        );
        let warning = cur_022
            .iter()
            .find(|d| d.level == DiagnosticLevel::Warning)
            .unwrap();
        assert_eq!(warning.line, 7);
    }

    #[test]
    fn cur_032_and_cur_033() {
        let temp = project();
        let content = r#"{
  "version": 1,
  "hooks": {
    "beforeMCPExecution": [{ "command": ".cursor/hooks/format.sh" }, { "command": ".cursor/hooks/format.sh", "failClosed": true }],
    "beforeShellExecution": [{ "type": "prompt", "prompt": "Is this command safe?" }],
    "afterFileEdit": [{ "command": ".cursor/hooks/format.sh" }]
  }
}"#;
        let diagnostics = validate(temp.path(), content);
        assert_eq!(rule(&diagnostics, "CUR-032").len(), 1);
        // Without .cursor/environment.json the project does not use cloud agents.
        assert!(rule(&diagnostics, "CUR-033").is_empty());
        std::fs::write(temp.path().join(".cursor/environment.json"), "{}").unwrap();
        let diagnostics = validate(temp.path(), content);
        let cur_033 = rule(&diagnostics, "CUR-033");
        assert_eq!(cur_033.len(), 2, "{cur_033:#?}");
        assert!(cur_033.iter().all(|d| d.level == DiagnosticLevel::Info));
    }
}
