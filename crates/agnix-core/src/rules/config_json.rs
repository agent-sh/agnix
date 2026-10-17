//! Span-located diagnostics for the JSON configuration validators of GitHub
//! Copilot, Cursor and Cline (hook files, CLI configuration, allowlists,
//! plugin manifests and MCP settings).
//!
//! Each check reports at the key or value it is about, using the source spans
//! of [`crate::schemas::kiro_json`]'s span-preserving tree.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    schemas::kiro_json::{JsonMember, JsonNode, LineIndex},
};
use std::path::Path;

/// Collects diagnostics for one JSON file.
pub(crate) struct JsonCx<'a> {
    pub path: &'a Path,
    lines: LineIndex<'a>,
    pub config: &'a PerFileLintConfig<'a>,
    pub diagnostics: Vec<Diagnostic>,
}

impl<'a> JsonCx<'a> {
    pub fn new(path: &'a Path, content: &'a str, config: &'a PerFileLintConfig<'a>) -> Self {
        Self {
            path,
            lines: LineIndex::new(content),
            config,
            diagnostics: Vec::new(),
        }
    }

    pub fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    /// Push a diagnostic covering `start..end` of the source.
    pub fn push(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        (start, end): (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        let (line, column) = self.lines.line_col(start);
        let (end_line, end_column) = self.lines.line_col(end.max(start));
        let path = self.path.to_path_buf();
        let message = message.into();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics.push(
            diagnostic
                .with_span(end_line, end_column)
                .with_suggestion(suggestion),
        );
    }

    /// Push a diagnostic at a line and column (for syntax errors).
    pub fn push_at(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        (line, column): (usize, usize),
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) {
        let path = self.path.to_path_buf();
        let message = message.into();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics
            .push(diagnostic.with_suggestion(suggestion));
    }
}

/// The span of a member's key.
pub(crate) fn key_span(member: &JsonMember) -> (usize, usize) {
    (member.key_start, member.key_end)
}

/// The span of a node.
pub(crate) fn node_span(node: &JsonNode) -> (usize, usize) {
    (node.start, node.end)
}

/// The string value of member `key` of an object node, if it is a string.
pub(crate) fn str_field<'n>(node: &'n JsonNode, key: &str) -> Option<&'n str> {
    node.value.get(key).and_then(|m| m.value.value.as_str())
}
