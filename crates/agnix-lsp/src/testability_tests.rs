//! Tests that verify internal modules are accessible at `pub(crate)` scope.
//!
//! These tests live at the crate root (not inside any submodule) so they can
//! only compile if the items under test are at least `pub(crate)`.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::backend::Backend;
use crate::backend::helpers::{create_error_diagnostic, normalize_path};
use crate::backend::revalidation::{
    MAX_CONFIG_REVALIDATION_CONCURRENCY, config_revalidation_concurrency, for_each_bounded,
};
use crate::diagnostic_mapper::to_lsp_diagnostic;
use crate::position::byte_to_position;

// ===== Backend struct fields =====

#[test]
fn backend_new_test_creates_valid_instance() {
    let backend = Backend::new_test();
    // Access several pub(crate) fields to prove all are reachable from the crate root.
    let _config = backend.config.load();
    assert!(backend.registry.total_validator_count() > 0);
    assert_eq!(backend.config_generation.load(Ordering::Relaxed), 0);
    assert_eq!(
        backend
            .project_validation_generation
            .load(Ordering::Relaxed),
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

// ===== helpers module =====

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
    assert_eq!(normalized, PathBuf::from("/etc/passwd"));
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

// ===== revalidation module =====

#[test]
fn revalidation_concurrency_accessible() {
    // config_revalidation_concurrency returns a value within expected bounds.
    assert_eq!(config_revalidation_concurrency(0), 0);
    let n = config_revalidation_concurrency(4);
    assert!(n >= 1);
    assert!(n <= MAX_CONFIG_REVALIDATION_CONCURRENCY);
    // Upper-bound clamping: very large document count is still within cap.
    assert!(config_revalidation_concurrency(usize::MAX) <= MAX_CONFIG_REVALIDATION_CONCURRENCY);
}

#[tokio::test]
async fn revalidation_for_each_bounded_accessible() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let errors = for_each_bounded(0..4usize, 2, move |_i| {
        let c = Arc::clone(&counter_clone);
        async move {
            c.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;
    assert!(errors.is_empty());
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn revalidation_for_each_bounded_panic_path() {
    // Verify that a panicking task's JoinError is collected rather than propagated.
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);
    let errors = for_each_bounded(0..3usize, 2, move |i| {
        let c = Arc::clone(&counter_clone);
        async move {
            if i == 1 {
                panic!("intentional panic in test");
            }
            c.fetch_add(1, Ordering::SeqCst);
        }
    })
    .await;
    assert_eq!(errors.len(), 1);
    // The two non-panicking items (0 and 2) should still have incremented the counter.
    assert_eq!(counter.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn revalidation_should_publish_diagnostics_accessible() {
    use tower_lsp::lsp_types::Url;
    let backend = Backend::new_test();
    let uri = Url::parse("file:///CLAUDE.md").unwrap();
    let content = Arc::new("content".to_string());

    // No document present: returns false when expected_content is set.
    assert!(
        !backend
            .should_publish_diagnostics(&uri, None, Some(&content))
            .await
    );

    // Insert document; now the correct Arc reference should return true.
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::clone(&content));
    assert!(
        backend
            .should_publish_diagnostics(&uri, None, Some(&content))
            .await
    );

    // Mismatched Arc (different allocation) should return false.
    let different_content = Arc::new("content".to_string());
    assert!(
        !backend
            .should_publish_diagnostics(&uri, None, Some(&different_content))
            .await
    );

    // Stale config generation should return false.
    assert!(
        !backend
            .should_publish_diagnostics(&uri, Some(999), None)
            .await
    );

    // Both guards disabled (None, None): always returns true regardless of document presence.
    assert!(backend.should_publish_diagnostics(&uri, None, None).await);
    backend.documents.write().await.remove(&uri);
    assert!(backend.should_publish_diagnostics(&uri, None, None).await);
}

#[tokio::test]
async fn revalidation_handle_did_change_configuration_accessible() {
    use tower_lsp::lsp_types::{DidChangeConfigurationParams, LSPAny};
    let backend = Backend::new_test();
    // Empty settings JSON: should not panic and no workspace to revalidate.
    let params = DidChangeConfigurationParams {
        settings: LSPAny::Null,
    };
    backend.handle_did_change_configuration(params).await;
}

// ===== helpers module (continued) =====

#[test]
fn backend_is_project_level_trigger_accessible() {
    assert!(Backend::is_project_level_trigger(Path::new("CLAUDE.md")));
    assert!(Backend::is_project_level_trigger(Path::new(".agnix.toml")));
    assert!(!Backend::is_project_level_trigger(Path::new("README.md")));
}

#[tokio::test]
async fn events_handle_did_open_accessible() {
    use tower_lsp::lsp_types::{DidOpenTextDocumentParams, TextDocumentItem, Url};
    let backend = Backend::new_test();
    let uri = Url::parse("file:///CLAUDE.md").unwrap();
    // CRLF content should be normalized to LF in the document cache.
    let crlf_content = "line1\r\nline2\r\n".to_string();
    let params = DidOpenTextDocumentParams {
        text_document: TextDocumentItem {
            uri: uri.clone(),
            language_id: "markdown".to_string(),
            version: 1,
            text: crlf_content,
        },
    };
    backend.handle_did_open(params).await;
    let stored = backend.documents.read().await.get(&uri).cloned();
    assert!(stored.is_some());
    assert_eq!(stored.unwrap().as_ref(), "line1\nline2\n");
}

#[tokio::test]
async fn events_handle_did_change_accessible() {
    use tower_lsp::lsp_types::{
        DidChangeTextDocumentParams, TextDocumentContentChangeEvent, Url,
        VersionedTextDocumentIdentifier,
    };
    let backend = Backend::new_test();
    let uri = Url::parse("file:///CLAUDE.md").unwrap();
    // Pre-insert initial content.
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::new("old content".to_string()));
    // Change to CRLF content - should be normalized.
    let params = DidChangeTextDocumentParams {
        text_document: VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: 2,
        },
        content_changes: vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new\r\ncontent\r\n".to_string(),
        }],
    };
    backend.handle_did_change(params).await;
    let stored = backend.documents.read().await.get(&uri).cloned();
    assert!(stored.is_some());
    assert_eq!(stored.unwrap().as_ref(), "new\ncontent\n");
}

#[tokio::test]
async fn events_handle_did_save_accessible() {
    use tower_lsp::lsp_types::{DidSaveTextDocumentParams, TextDocumentIdentifier, Url};
    let backend = Backend::new_test();
    // Use a non-project-level-trigger URI to avoid spawning a background validation task.
    let uri = Url::parse("file:///skill.yml").unwrap();
    backend.documents.write().await.insert(
        uri.clone(),
        Arc::new("# Agent\nname: my-agent\n".to_string()),
    );
    let params = DidSaveTextDocumentParams {
        text_document: TextDocumentIdentifier { uri: uri.clone() },
        text: None,
    };
    // Should not panic; diagnostics are published to the disconnected test client.
    backend.handle_did_save(params).await;
}

#[tokio::test]
async fn events_handle_did_close_accessible() {
    use tower_lsp::lsp_types::{DidCloseTextDocumentParams, TextDocumentIdentifier, Url};
    let backend = Backend::new_test();
    let uri = Url::parse("file:///test.md").unwrap();
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

// ===== helpers module (continued): get_document_content =====

#[tokio::test]
async fn backend_get_document_content_accessible() {
    let backend = Backend::new_test();
    let uri = tower_lsp::lsp_types::Url::parse("file:///test.md").unwrap();
    // Cache miss.
    assert!(backend.get_document_content(&uri).await.is_none());
    // Cache hit.
    let content = Arc::new("hello".to_string());
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::clone(&content));
    let retrieved = backend.get_document_content(&uri).await;
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().as_ref(), "hello");
}

// ===== position module =====

#[test]
fn position_module_accessible() {
    let pos = byte_to_position("hello\nworld", 6);
    assert_eq!(pos.line, 1);
    assert_eq!(pos.character, 0);
}

// ===== diagnostic_mapper module =====

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
