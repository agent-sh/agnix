//! Watch mode implementation for continuous validation

use notify_debouncer_mini::{
    DebounceEventResult, DebouncedEventBatch, new_debouncer, notify::RecursiveMode,
};
use rust_i18n::t;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::time::Duration;

/// What one debouncer notification means for the watch loop.
#[derive(Debug, PartialEq, Eq)]
enum WatchAction {
    /// Run validation again.
    Revalidate,
    /// Nothing that validation reads has changed.
    Nothing,
}

/// Map a debouncer notification to a loop action.
///
/// `Rescan` is handled before the path filter: the watcher sends it when events
/// were missed, it carries no paths, and it applies to every watched path.
/// Notification kinds the debouncer adds later also revalidate: one extra run
/// costs little, stale output costs more.
fn action_for(batch: &DebouncedEventBatch) -> WatchAction {
    match batch {
        DebouncedEventBatch::Events(events) => {
            if events.iter().any(|e| is_relevant_file(&e.path)) {
                WatchAction::Revalidate
            } else {
                WatchAction::Nothing
            }
        }
        // `Rescan` names no path, and so does any kind added later.
        _ => WatchAction::Revalidate,
    }
}

/// Run validation in watch mode, re-running on file changes
pub fn watch_and_validate<F>(path: &Path, mut validate_fn: F) -> anyhow::Result<()>
where
    F: FnMut() -> anyhow::Result<bool>,
{
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })?;

    // Initial validation
    println!("{}\n", t!("cli.watch_starting"));
    let _ = validate_fn();

    // Set up file watcher
    let (tx, rx) = channel::<DebounceEventResult>();

    let mut debouncer = new_debouncer(Duration::from_millis(500), tx)?;
    debouncer.watcher().watch(path, RecursiveMode::Recursive)?;

    // Watch loop
    while running.load(Ordering::SeqCst) {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Ok(batch)) => {
                if action_for(&batch) == WatchAction::Revalidate {
                    clear_screen();
                    println!("{}\n", t!("cli.watch_changes_detected"));
                    let _ = validate_fn();
                }
            }
            Ok(Err(e)) => {
                eprintln!("{}", t!("cli.watch_error", error = format!("{:?}", e)));
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                // Continue watching
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                break;
            }
        }
    }

    println!("\n{}", t!("cli.watch_stopped"));
    Ok(())
}

fn is_relevant_file(path: &Path) -> bool {
    let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let is_cursor_rule = matches!(
        agnix_core::detect_file_type(path),
        agnix_core::FileType::CursorRule
    );
    let parent = path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str());

    let is_codex_config = parent == Some(".codex")
        && matches!(
            filename,
            "config.toml" | "config.json" | "config.yaml" | "config.yml"
        );

    // Check for relevant filenames
    matches!(
        filename,
        "SKILL.md"
            | "CLAUDE.md"
            | "CLAUDE.local.md"
            | "AGENTS.md"
            | "AGENTS.local.md"
            | "AGENTS.override.md"
            | "GEMINI.md"
            | "GEMINI.local.md"
            | "settings.json"
            | "settings.local.json"
            | "plugin.json"
            | "copilot-instructions.md"
        | ".agnix.toml"
    ) || extension == "mcp"
        || is_codex_config
        || filename.ends_with(".mcp.json")
        || extension.eq_ignore_ascii_case("mdc")
        || is_cursor_rule
        || filename.ends_with(".instructions.md")
        // Also watch for agent files
        || (extension == "md"
            && path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("agents"))
}

fn clear_screen() {
    // ANSI escape code to clear screen and move cursor to top
    print!("\x1B[2J\x1B[1;1H");
    let _ = std::io::Write::flush(&mut std::io::stdout());
}

#[cfg(test)]
mod tests {
    use super::{DebouncedEventBatch, WatchAction, action_for, is_relevant_file};
    use notify_debouncer_mini::{
        DebouncedEvent, DebouncedEventKind, new_debouncer, notify::RecursiveMode,
    };
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    #[test]
    fn codex_config_files_are_relevant() {
        assert!(is_relevant_file(Path::new(".codex/config.toml")));
        assert!(is_relevant_file(Path::new(".codex/config.json")));
        assert!(is_relevant_file(Path::new(".codex/config.yaml")));
        assert!(is_relevant_file(Path::new(".codex/config.yml")));
    }

    #[test]
    fn non_codex_config_files_are_not_relevant_by_name() {
        assert!(!is_relevant_file(Path::new("configs/config.yaml")));
        assert!(!is_relevant_file(Path::new("configs/config.json")));
    }

    #[test]
    fn cursor_rule_files_are_relevant() {
        for path in [
            ".cursor/rules/test.md",
            ".cursor/rules/test.MD",
            ".cursor/rules/test.mdc",
            ".cursor/rules/test.MDC",
        ] {
            assert!(is_relevant_file(Path::new(path)));
        }

        assert!(!is_relevant_file(Path::new("docs/test.md")));
    }

    fn events(paths: &[&str]) -> DebouncedEventBatch {
        DebouncedEventBatch::Events(
            paths
                .iter()
                .map(|path| DebouncedEvent::new(PathBuf::from(path), DebouncedEventKind::Any))
                .collect(),
        )
    }

    #[test]
    fn relevant_change_revalidates() {
        assert_eq!(
            action_for(&events(&["skills/lint/SKILL.md"])),
            WatchAction::Revalidate
        );
        assert_eq!(
            action_for(&events(&["notes.txt", ".cursor/rules/x.mdc"])),
            WatchAction::Revalidate
        );
    }

    #[test]
    fn irrelevant_or_empty_batch_does_not_revalidate() {
        assert_eq!(
            action_for(&events(&["README.md", "target/debug/agnix"])),
            WatchAction::Nothing
        );
        assert_eq!(action_for(&events(&[])), WatchAction::Nothing);
    }

    #[test]
    fn rescan_revalidates_without_paths() {
        // The debouncer sends Rescan when events were missed. It names no
        // path, so the path filter must not apply to it.
        assert_eq!(
            action_for(&DebouncedEventBatch::Rescan),
            WatchAction::Revalidate
        );
    }

    // Runs the real watcher backend of the host. A full CI run covers inotify,
    // FSEvents and ReadDirectoryChangesW.
    #[test]
    fn debouncer_reports_a_written_relevant_file() {
        let dir = tempfile::tempdir().unwrap();
        let skill = dir.path().join("SKILL.md");
        std::fs::write(&skill, "a").unwrap();

        let (tx, rx) = std::sync::mpsc::channel::<super::DebounceEventResult>();
        let mut debouncer = new_debouncer(Duration::from_millis(50), tx).unwrap();
        debouncer
            .watcher()
            .watch(dir.path(), RecursiveMode::Recursive)
            .unwrap();
        std::fs::write(&skill, "b").unwrap();

        let deadline = Instant::now() + Duration::from_secs(15);
        let mut written = false;
        while let Some(left) = deadline.checked_duration_since(Instant::now()) {
            match rx.recv_timeout(left) {
                Ok(Ok(batch)) => {
                    if matches!(
                        batch,
                        DebouncedEventBatch::Events(ref events)
                            if events.iter().any(|event| event.path == skill)
                    ) {
                        written = true;
                        break;
                    }
                }
                Ok(Err(_)) => {}
                Err(_) => break,
            }
        }
        assert!(written, "no debounced event for the written file");
    }
}
