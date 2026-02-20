//! Tests that verify internal modules are accessible at `pub(crate)` scope.
//!
//! These tests live at the crate root (not inside any submodule) so they can
//! only compile if the items under test are at least `pub(crate)`.

use std::path::{Path, PathBuf};

use crate::backend::Backend;
use crate::backend::helpers::{create_error_diagnostic, normalize_path};
use crate::backend::revalidation::MAX_CONFIG_REVALIDATION_CONCURRENCY;
use crate::diagnostic_mapper::to_lsp_diagnostic;
use crate::position::byte_to_position;

#[test]
fn backend_new_test_creates_valid_instance() {
    let backend = Backend::new_test();
    // Access a pub(crate) field to prove it is reachable from the crate root.
    let _config = backend.config.load();
}

#[test]
fn helpers_normalize_path_accessible() {
    let path = PathBuf::from("/a/b/../c");
    let normalized = normalize_path(&path);
    assert_eq!(normalized, PathBuf::from("/a/c"));
}

#[test]
fn helpers_create_error_diagnostic_accessible() {
    let diag = create_error_diagnostic("test::code", "something went wrong".to_string());
    assert_eq!(
        diag.code,
        Some(tower_lsp::lsp_types::NumberOrString::String(
            "test::code".to_string()
        ))
    );
    assert_eq!(diag.message, "something went wrong");
    assert_eq!(diag.severity, Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR));
}

#[test]
fn revalidation_concurrency_constants_accessible() {
    assert_eq!(MAX_CONFIG_REVALIDATION_CONCURRENCY, 8);
}

#[test]
fn backend_is_project_level_trigger_accessible() {
    assert!(Backend::is_project_level_trigger(Path::new("CLAUDE.md")));
    assert!(Backend::is_project_level_trigger(Path::new(".agnix.toml")));
    assert!(!Backend::is_project_level_trigger(Path::new("README.md")));
}

#[test]
fn position_module_accessible() {
    let pos = byte_to_position("hello\nworld", 6);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 0);
}

#[test]
fn diagnostic_mapper_accessible() {
    let core_diag = agnix_core::Diagnostic {
        level: agnix_core::DiagnosticLevel::Warning,
        message: "test warning".to_string(),
        file: PathBuf::from("test.md"),
        line: 3,
        column: 5,
        rule: "AS-001".to_string(),
        suggestion: None,
        fixes: vec![],
        assumption: None,
        metadata: None,
    };
    let lsp_diag = to_lsp_diagnostic(&core_diag);
    assert_eq!(
        lsp_diag.severity,
        Some(tower_lsp::lsp_types::DiagnosticSeverity::WARNING)
    );
    assert_eq!(lsp_diag.range.start.line, 2);
    assert_eq!(lsp_diag.range.start.character, 4);
}
