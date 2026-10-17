//! OpenCode release-line selection shared by the OpenCode validators.
//!
//! OpenCode ships two current lines that read the same files: OpenCode 1
//! (`opencode-ai`, docs at opencode.ai/docs) and OpenCode 2 (`@opencode/cli`,
//! released 2026-09-11, docs at opencode.ai/v2/docs). OpenCode 2 reads V1
//! configuration ("supported V1 and native V2 fields may coexist at the top
//! level") but ignores some V1 fields and does not read CLAUDE.md, while
//! OpenCode 1 does not know the native V2 keys. Validators therefore check
//! each file against the lines it targets:
//!
//! - `[tool_versions] opencode` pins one line: 1.x selects V1, 2.x or later V2.
//! - Unpinned, a file that uses native V2 fields (`agents`, `permissions`,
//!   `mcp.servers`, ...) targets V2; every other file targets both lines.
//!
//! Checks that only hold for one line run only when that line is the only
//! one targeted, so documented V1 configuration is never reported because
//! of V2 behavior and the reverse.

use crate::config::LintConfig;

/// An OpenCode release line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpenCodeLine {
    V1,
    V2,
}

/// The set of lines an OpenCode file is checked against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpenCodeLines {
    pub v1: bool,
    pub v2: bool,
}

impl OpenCodeLines {
    pub const BOTH: Self = Self { v1: true, v2: true };
    pub const V1_ONLY: Self = Self {
        v1: true,
        v2: false,
    };
    pub const V2_ONLY: Self = Self {
        v1: false,
        v2: true,
    };

    /// True when the file is checked against OpenCode 2 only.
    pub fn v2_only(self) -> bool {
        self == Self::V2_ONLY
    }

    /// True when the file is checked against OpenCode 1 only.
    pub fn v1_only(self) -> bool {
        self == Self::V1_ONLY
    }
}

/// The line selected by `[tool_versions] opencode`, or `None` when the
/// version is not pinned or has no leading major number.
pub(crate) fn pinned_line(config: &LintConfig) -> Option<OpenCodeLine> {
    let version = config.get_opencode_version()?.trim();
    let major: u64 = version
        .trim_start_matches(['v', 'V'])
        .split(['.', '-', '+'])
        .next()?
        .parse()
        .ok()?;
    Some(if major >= 2 {
        OpenCodeLine::V2
    } else {
        OpenCodeLine::V1
    })
}

/// The lines a file is checked against, given whether it uses native V2
/// fields. A pinned version always wins.
pub(crate) fn lines_for(config: &LintConfig, v2_markers: bool) -> OpenCodeLines {
    match pinned_line(config) {
        Some(OpenCodeLine::V1) => OpenCodeLines::V1_ONLY,
        Some(OpenCodeLine::V2) => OpenCodeLines::V2_ONLY,
        None if v2_markers => OpenCodeLines::V2_ONLY,
        None => OpenCodeLines::BOTH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ToolVersions;

    fn config_with(version: Option<&str>) -> LintConfig {
        LintConfig::builder()
            .tool_versions(ToolVersions {
                opencode: version.map(str::to_string),
                ..ToolVersions::default()
            })
            .build()
            .unwrap()
    }

    #[test]
    fn pinned_version_selects_line() {
        for (version, line) in [
            ("1.19.3", Some(OpenCodeLine::V1)),
            ("v1.0.0", Some(OpenCodeLine::V1)),
            ("2.0.32", Some(OpenCodeLine::V2)),
            ("2", Some(OpenCodeLine::V2)),
            ("3.1.0-beta.1", Some(OpenCodeLine::V2)),
            ("latest", None),
        ] {
            assert_eq!(pinned_line(&config_with(Some(version))), line, "{version}");
        }
        assert_eq!(pinned_line(&config_with(None)), None);
    }

    #[test]
    fn markers_select_lines_when_unpinned() {
        let config = config_with(None);
        assert_eq!(lines_for(&config, true), OpenCodeLines::V2_ONLY);
        assert_eq!(lines_for(&config, false), OpenCodeLines::BOTH);
    }

    #[test]
    fn pin_overrides_markers() {
        assert_eq!(
            lines_for(&config_with(Some("1.19.3")), true),
            OpenCodeLines::V1_ONLY
        );
        assert_eq!(
            lines_for(&config_with(Some("2.0.0")), false),
            OpenCodeLines::V2_ONLY
        );
        assert!(OpenCodeLines::V2_ONLY.v2_only());
        assert!(OpenCodeLines::V1_ONLY.v1_only());
        assert!(!OpenCodeLines::BOTH.v2_only());
    }
}
