//! CDX-AG-010: the Codex instruction chain exceeds `project_doc_max_bytes`.
//!
//! "Codex skips empty files and stops adding files once the combined size
//! reaches the limit defined by `project_doc_max_bytes` (32 KiB by default).
//! ... Raise the limit or split instructions across nested directories when
//! you hit the cap." (learn.chatgpt.com/docs/agent-configuration/agents-md)
//!
//! Chains are built by `project_level::codex_instruction_chains` (one per
//! leaf directory, at most one file per directory, `AGENTS.override.md`
//! before `AGENTS.md`). Each over-cap chain is reported once, on its largest
//! file, which is the one edit that fixes every chain sharing it. A chain of
//! one file, or a chain whose largest file is over the cap by itself, is left
//! to CDX-AG-009, which reports that file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rust_i18n::t;

use crate::config::LintConfig;
use crate::diagnostics::Diagnostic;
use crate::rules::project_level::codex_instruction_chains;
use crate::schemas::cross_platform::CODEX_BYTE_LIMIT;

pub(crate) fn chain_diagnostics(
    instruction_file_paths: &[PathBuf],
    config: &LintConfig,
    root_dir: &Path,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if !config.is_rule_enabled("CDX-AG-010") {
        return diagnostics;
    }
    let chains = codex_instruction_chains(instruction_file_paths, root_dir, config.fs().as_ref());
    let mut reported: BTreeMap<PathBuf, (usize, usize)> = BTreeMap::new();
    for chain in &chains {
        let mut sizes: Vec<(PathBuf, usize)> = Vec::with_capacity(chain.len());
        for path in chain {
            if let Ok(content) = config.fs().read_to_string(path) {
                sizes.push((path.clone(), content.len()));
            }
        }
        let total: usize = sizes.iter().map(|(_, size)| size).sum();
        if total <= CODEX_BYTE_LIMIT {
            continue;
        }
        if let [only] = chain.as_slice()
            && config.for_path(only).is_rule_enabled("CDX-AG-009")
        {
            continue;
        }
        let Some((culprit, culprit_size)) = sizes
            .iter()
            .max_by_key(|(path, size)| (*size, std::cmp::Reverse(path.components().count())))
        else {
            continue;
        };
        // The largest file alone is over the cap: CDX-AG-009 already reports
        // that file, and trimming it is the same edit this rule would ask
        // for, so the chain is not reported a second time on it.
        let culprit_is_agents_md = culprit
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| matches!(name, "AGENTS.md" | "AGENTS.override.md"));
        if *culprit_size > CODEX_BYTE_LIMIT
            && culprit_is_agents_md
            && config.for_path(culprit).is_rule_enabled("CDX-AG-009")
        {
            continue;
        }
        reported
            .entry(culprit.clone())
            .and_modify(|entry| {
                if total > entry.0 {
                    *entry = (total, chain.len());
                }
            })
            .or_insert((total, chain.len()));
    }
    for (path, (total, count)) in reported {
        if config.for_path(&path).is_rule_enabled("CDX-AG-010") {
            diagnostics.push(
                Diagnostic::warning(
                    path,
                    1,
                    1,
                    "CDX-AG-010",
                    t!(
                        "rules.cdx_ag_010.message",
                        bytes = total,
                        count = count,
                        limit = CODEX_BYTE_LIMIT
                    ),
                )
                .with_suggestion(t!("rules.cdx_ag_010.suggestion")),
            );
        }
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystem;
    use std::sync::Arc;

    /// A chain whose largest file is over the cap by itself got CDX-AG-009
    /// and CDX-AG-010 on the same file; CDX-AG-009 reports it alone.
    #[test]
    fn chain_with_an_oversized_file_is_left_to_cdx_ag_009() {
        let fs = MockFileSystem::new();
        fs.add_file("/repo/AGENTS.md", "a".repeat(100));
        fs.add_file("/repo/pkg/AGENTS.md", "b".repeat(CODEX_BYTE_LIMIT + 10));
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        let paths = vec![
            PathBuf::from("/repo/AGENTS.md"),
            PathBuf::from("/repo/pkg/AGENTS.md"),
        ];
        assert!(chain_diagnostics(&paths, &config, Path::new("/repo")).is_empty());
        // With CDX-AG-009 disabled the chain is reported again.
        config.rules_mut().disabled_rules = vec!["CDX-AG-009".to_string()];
        let diagnostics = chain_diagnostics(&paths, &config, Path::new("/repo"));
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].file, PathBuf::from("/repo/pkg/AGENTS.md"));
    }

    #[test]
    fn chain_over_the_cap_is_reported_on_its_largest_file() {
        let fs = MockFileSystem::new();
        fs.add_file("/repo/AGENTS.md", "a".repeat(20_000));
        fs.add_file("/repo/pkg/AGENTS.md", "b".repeat(15_000));
        fs.add_file("/repo/other/AGENTS.md", "c".repeat(1_000));
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        let paths = vec![
            PathBuf::from("/repo/AGENTS.md"),
            PathBuf::from("/repo/other/AGENTS.md"),
            PathBuf::from("/repo/pkg/AGENTS.md"),
        ];
        let diagnostics = chain_diagnostics(&paths, &config, Path::new("/repo"));
        assert_eq!(diagnostics.len(), 1, "{diagnostics:?}");
        assert_eq!(diagnostics[0].file, PathBuf::from("/repo/AGENTS.md"));
        assert!(diagnostics[0].message.contains("35000"));
    }
}
