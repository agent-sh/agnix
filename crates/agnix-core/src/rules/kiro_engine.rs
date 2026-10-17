//! Kiro CLI agent-engine selection shared by the Kiro validators.
//!
//! Kiro CLI 2.x ships two agent engines in one binary: V2, the 2.x default,
//! and V3, the harness shared with Kiro IDE 1.0, opt-in since CLI 2.8.0 and
//! the default from CLI 3.0. The same file can load in both, and some
//! conditions are fatal in only one of them: a missing `name` skips a V2
//! agent but not a V3 one, while an absolute `file://` prompt fails only in
//! V3. Validators therefore check each file against the engines it targets:
//!
//! - `[tool_versions] kiro` pins one engine: 2.x selects V2, 3.x or later V3.
//! - Unpinned, a file with V3-only markers (Markdown agents, `permissions`,
//!   array hooks, standalone hook files) targets V3, one with V2-only markers
//!   (`toolsSettings`, `allowedTools`, `toolAliases`, object-form hooks,
//!   `timeout_ms`) targets V2, and a file with both or neither targets both.

use crate::config::LintConfig;

/// A Kiro CLI agent engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum KiroEngine {
    V2,
    V3,
}

/// The set of engines a Kiro file is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KiroEngines {
    pub v2: bool,
    pub v3: bool,
}

impl KiroEngines {
    pub const BOTH: Self = Self { v2: true, v3: true };
    pub const V2_ONLY: Self = Self {
        v2: true,
        v3: false,
    };
    pub const V3_ONLY: Self = Self {
        v2: false,
        v3: true,
    };

    pub fn includes(self, engine: KiroEngine) -> bool {
        match engine {
            KiroEngine::V2 => self.v2,
            KiroEngine::V3 => self.v3,
        }
    }

    /// Returns true if `engine` is the only engine in the set, so a condition
    /// fatal in that engine breaks the file for every engine it targets.
    pub fn only(self, engine: KiroEngine) -> bool {
        self == Self::from(engine)
    }
}

impl From<KiroEngine> for KiroEngines {
    fn from(engine: KiroEngine) -> Self {
        match engine {
            KiroEngine::V2 => Self::V2_ONLY,
            KiroEngine::V3 => Self::V3_ONLY,
        }
    }
}

/// The engine selected by `[tool_versions] kiro`, or `None` when the version
/// is not pinned or has no leading major number.
pub(crate) fn pinned_engine(config: &LintConfig) -> Option<KiroEngine> {
    let version = config.get_kiro_version()?.trim();
    let major: u64 = version
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .next()?
        .parse()
        .ok()?;
    Some(if major >= 3 {
        KiroEngine::V3
    } else {
        KiroEngine::V2
    })
}

/// The engines a file is checked against, given whether it has V2-only and
/// V3-only markers. A pinned version always wins.
pub(crate) fn engines_for(config: &LintConfig, v2_markers: bool, v3_markers: bool) -> KiroEngines {
    if let Some(engine) = pinned_engine(config) {
        return engine.into();
    }
    match (v2_markers, v3_markers) {
        (true, false) => KiroEngines::V2_ONLY,
        (false, true) => KiroEngines::V3_ONLY,
        _ => KiroEngines::BOTH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ToolVersions;

    fn config_with(version: Option<&str>) -> LintConfig {
        LintConfig::builder()
            .tool_versions(ToolVersions {
                kiro: version.map(str::to_string),
                ..ToolVersions::default()
            })
            .build()
            .unwrap()
    }

    #[test]
    fn pinned_version_selects_engine() {
        for (version, engine) in [
            ("2.32.1", Some(KiroEngine::V2)),
            ("v2.8.0-nightly.3", Some(KiroEngine::V2)),
            ("3.0.0", Some(KiroEngine::V3)),
            ("3", Some(KiroEngine::V3)),
            ("4.1", Some(KiroEngine::V3)),
            ("latest", None),
        ] {
            assert_eq!(
                pinned_engine(&config_with(Some(version))),
                engine,
                "{version}"
            );
        }
        assert_eq!(pinned_engine(&config_with(None)), None);
    }

    #[test]
    fn markers_select_engines_when_unpinned() {
        let config = config_with(None);
        assert_eq!(engines_for(&config, true, false), KiroEngines::V2_ONLY);
        assert_eq!(engines_for(&config, false, true), KiroEngines::V3_ONLY);
        assert_eq!(engines_for(&config, true, true), KiroEngines::BOTH);
        assert_eq!(engines_for(&config, false, false), KiroEngines::BOTH);
    }

    #[test]
    fn pin_overrides_markers() {
        let config = config_with(Some("3.0.0"));
        assert_eq!(engines_for(&config, true, false), KiroEngines::V3_ONLY);
        assert!(KiroEngines::V3_ONLY.only(KiroEngine::V3));
        assert!(!KiroEngines::BOTH.only(KiroEngine::V3));
    }
}
