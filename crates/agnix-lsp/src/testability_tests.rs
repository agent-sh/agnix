//! Tests that verify internal modules are accessible at `pub(crate)` scope.
//!
//! These tests live at the crate root (not inside any submodule) so they can
//! only compile if the items under test are at least `pub(crate)`.

use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use crate::backend::Backend;
use crate::backend::helpers::{create_error_diagnostic, normalize_path};
use crate::backend::revalidation::{
    MAX_CONFIG_REVALIDATION_CONCURRENCY, config_revalidation_concurrency, for_each_bounded,
};
use crate::diagnostic_mapper::to_lsp_diagnostic;
use crate::position::byte_to_position;

#[test]
fn backend_new_test_creates_valid_instance() {
    let backend = Backend::new_test();
    // Access several pub(crate) fields to prove all are reachable from the crate root.
    let _config = backend.config.load();
    assert!(backend.registry.total_validator_count() > 0);
    assert_eq!(backend.config_generation.load(Ordering::SeqCst), 0);
    assert_eq!(
        backend.project_validation_generation.load(Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn backend_fields_accessible() {
    let backend = Backend::new_test();
    // Verify async-accessed fields are reachable from the crate root.
    assert!(backend.documents.read().await.is_empty());
    assert!(backend.project_level_diagnostics.read().await.is_empty());
    assert!(backend.project_diagnostics_uris.read().await.is_empty());
    assert!(backend.workspace_root.read().await.is_none());
    assert!(backend.workspace_root_canonical.read().await.is_none());
}

#[test]
fn helpers_normalize_path_accessible() {
    let path = PathBuf::from("/a/b/../c");
    let normalized = normalize_path(&path);
    assert_eq!(normalized, PathBuf::from("/a/c"));
}

#[test]
fn helpers_normalize_path_root_guard() {
    // Traversal above root is silently dropped per the normalize_path contract.
    let path = PathBuf::from("/../etc/passwd");
    let normalized = normalize_path(&path);
    assert!(!normalized.to_string_lossy().starts_with("/.."));
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
    assert_eq!(
        diag.severity,
        Some(tower_lsp::lsp_types::DiagnosticSeverity::ERROR)
    );
}

#[test]
fn revalidation_concurrency_accessible() {
    // config_revalidation_concurrency returns a value within expected bounds.
    assert_eq!(config_revalidation_concurrency(0), 0);
    let n = config_revalidation_concurrency(4);
    assert!(n >= 1);
    assert!(n <= MAX_CONFIG_REVALIDATION_CONCURRENCY);
}

#[tokio::test]
async fn revalidation_for_each_bounded_accessible() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let errors = for_each_bounded(0..4usize, 2, move |_i| {
        let c = Arc::clone(&counter_clone);
        async move {
            c.fetch_add(1, AtomicOrdering::SeqCst);
        }
    })
    .await;
    assert!(errors.is_empty());
    assert_eq!(counter.load(AtomicOrdering::SeqCst), 4);
}

#[test]
fn backend_is_project_level_trigger_accessible() {
    assert!(Backend::is_project_level_trigger(Path::new("CLAUDE.md")));
    assert!(Backend::is_project_level_trigger(Path::new(".agnix.toml")));
    assert!(!Backend::is_project_level_trigger(Path::new("README.md")));
}

#[tokio::test]
async fn backend_get_document_content_accessible() {
    let backend = Backend::new_test();
    let uri = tower_lsp::lsp_types::Url::parse("file:///test.md").unwrap();
    // Before inserting, content should be None.
    assert!(backend.get_document_content(&uri).await.is_none());
}

#[tokio::test]
async fn events_handle_did_close_accessible() {
    use std::sync::Arc;
    use tower_lsp::lsp_types::{DidCloseTextDocumentParams, TextDocumentIdentifier};
    let backend = Backend::new_test();
    let uri = tower_lsp::lsp_types::Url::parse("file:///test.md").unwrap();
    // Insert then close a document to prove handle_did_close is accessible from outside the backend module.
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::new("content".to_string()));
    let params = DidCloseTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
    };
    backend.handle_did_close(params).await;
    assert!(backend.documents.read().await.get(&uri).is_none());
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
