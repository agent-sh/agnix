use super::*;
use tower_lsp::LspService;

/// Test that Backend::new creates a valid Backend instance.
/// We verify this by creating a service and checking initialize returns proper capabilities.
#[tokio::test]
async fn test_backend_new_creates_valid_instance() {
    let (service, _socket) = LspService::new(Backend::new);

    // The service was created successfully, meaning Backend::new worked
    // We can verify by calling initialize
    let init_params = InitializeParams::default();
    let result = service.inner().initialize(init_params).await;

    assert!(result.is_ok());
}

/// Test that initialize() returns correct server capabilities.
#[tokio::test]
async fn test_initialize_returns_correct_capabilities() {
    let (service, _socket) = LspService::new(Backend::new);

    let init_params = InitializeParams::default();
    let result = service.inner().initialize(init_params).await;

    let init_result = result.expect("initialize should succeed");

    // Verify text document sync capability
    match init_result.capabilities.text_document_sync {
        Some(TextDocumentSyncCapability::Kind(kind)) => {
            assert_eq!(kind, TextDocumentSyncKind::FULL);
        }
        _ => panic!("Expected FULL text document sync capability"),
    }

    assert!(
        init_result.capabilities.completion_provider.is_some(),
        "Expected completion provider capability"
    );

    // Verify server info
    let server_info = init_result
        .server_info
        .expect("server_info should be present");
    assert_eq!(server_info.name, "agnix-lsp");
    assert!(server_info.version.is_some());
}

#[tokio::test]
async fn test_completion_returns_skill_frontmatter_candidates() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    let content = "---\nna\n---\n";
    std::fs::write(&skill_path, content).unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let completion = service
        .inner()
        .completion(CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 1,
                    character: 1,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: None,
        })
        .await
        .unwrap();

    let items = match completion {
        Some(CompletionResponse::Array(items)) => items,
        _ => panic!("Expected completion items"),
    };
    assert!(items.iter().any(|item| item.label == "name"));
}

/// Test that shutdown() returns Ok.
#[tokio::test]
async fn test_shutdown_returns_ok() {
    let (service, _socket) = LspService::new(Backend::new);

    let result = service.inner().shutdown().await;
    assert!(result.is_ok());
}

/// Test validation error diagnostic has correct code.
/// We test the diagnostic structure directly since we can't easily mock the validation.
#[test]
fn test_validation_error_diagnostic_structure() {
    // Simulate what validate_file returns on validation error
    let error_message = "Failed to parse file";
    let diagnostic = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(
            "agnix::validation-error".to_string(),
        )),
        code_description: None,
        source: Some("agnix".to_string()),
        message: format!("Validation error: {}", error_message),
        related_information: None,
        tags: None,
        data: None,
    };

    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String(
            "agnix::validation-error".to_string()
        ))
    );
    assert_eq!(diagnostic.source, Some("agnix".to_string()));
    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
    assert!(diagnostic.message.contains("Validation error:"));
}

/// Test internal error diagnostic has correct code.
#[test]
fn test_internal_error_diagnostic_structure() {
    // Simulate what validate_file returns on panic/internal error
    let error_message = "task panicked";
    let diagnostic = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String("agnix::internal-error".to_string())),
        code_description: None,
        source: Some("agnix".to_string()),
        message: format!("Internal error: {}", error_message),
        related_information: None,
        tags: None,
        data: None,
    };

    assert_eq!(
        diagnostic.code,
        Some(NumberOrString::String("agnix::internal-error".to_string()))
    );
    assert_eq!(diagnostic.source, Some("agnix".to_string()));
    assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
    assert!(diagnostic.message.contains("Internal error:"));
}

/// Test that invalid URIs are identified correctly.
/// Non-file URIs should fail to_file_path().
#[test]
fn test_invalid_uri_detection() {
    // Non-file URIs should fail to_file_path()
    let http_uri = Url::parse("http://example.com/file.md").unwrap();
    assert!(http_uri.to_file_path().is_err());

    let data_uri = Url::parse("data:text/plain;base64,SGVsbG8=").unwrap();
    assert!(data_uri.to_file_path().is_err());

    // File URIs should succeed - use platform-appropriate path
    #[cfg(windows)]
    let file_uri = Url::parse("file:///C:/tmp/test.md").unwrap();
    #[cfg(not(windows))]
    let file_uri = Url::parse("file:///tmp/test.md").unwrap();
    assert!(file_uri.to_file_path().is_ok());
}

/// Test validate_file with a valid file returns diagnostics.
#[tokio::test]
async fn test_validate_file_valid_skill() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create a valid skill file
    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill

This is a valid skill.
"#,
    )
    .unwrap();

    // We can't directly call validate_file since it's private,
    // but we can verify the validation logic works through did_open
    // The Backend will log messages to the client
    let uri = Url::from_file_path(&skill_path).unwrap();

    // Call did_open which triggers validate_and_publish internally
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(), // Content is read from file
            },
        })
        .await;

    // If we get here without panicking, the validation completed
}

/// Test validate_file with an invalid skill file.
#[tokio::test]
async fn test_validate_file_invalid_skill() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create an invalid skill file (invalid name with spaces)
    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: Invalid Name With Spaces
version: 1.0.0
model: sonnet
---

# Invalid Skill

This skill has an invalid name.
"#,
    )
    .unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Call did_open which triggers validation
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Validation should complete and publish diagnostics
}

/// Test did_save triggers validation.
#[tokio::test]
async fn test_did_save_triggers_validation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
    )
    .unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Call did_save which triggers validate_and_publish
    service
        .inner()
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: None,
        })
        .await;

    // Validation should complete without error
}

/// Test did_save on a project-level trigger file starts project-level revalidation.
#[tokio::test]
async fn test_did_save_project_trigger_starts_project_revalidation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let agents_path = temp_dir.path().join("AGENTS.md");
    std::fs::write(&agents_path, "# Root AGENTS").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let before = service
        .inner()
        .project_validation_generation
        .load(Ordering::SeqCst);

    let uri = Url::from_file_path(&agents_path).unwrap();
    service
        .inner()
        .did_save(DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: None,
        })
        .await;

    let mut observed_increment = false;
    for _ in 0..40 {
        let current = service
            .inner()
            .project_validation_generation
            .load(Ordering::SeqCst);
        if current > before {
            observed_increment = true;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

    assert!(
        observed_increment,
        "did_save on AGENTS.md should trigger project-level revalidation"
    );
}

/// Test did_close clears diagnostics.
#[tokio::test]
async fn test_did_close_clears_diagnostics() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test").unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Call did_close which publishes empty diagnostics
    service
        .inner()
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        })
        .await;

    // Should complete without error
}

/// Test initialized() completes without error.
#[tokio::test]
async fn test_initialized_completes() {
    let (service, _socket) = LspService::new(Backend::new);

    // Call initialized
    service.inner().initialized(InitializedParams {}).await;

    // Should complete without error (logs a message to client)
}

/// Test validate_and_publish with non-file URI is handled gracefully.
/// Since validate_and_publish is private, we test the URI validation logic directly.
#[tokio::test]
async fn test_non_file_uri_handled_gracefully() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create a non-file URI (http://)
    let http_uri = Url::parse("http://example.com/test.md").unwrap();

    // Call did_open with non-file URI
    // This should be handled gracefully (log warning and return early)
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: http_uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Should complete without panic
}

/// Test validation with non-existent file.
#[tokio::test]
async fn test_validate_nonexistent_file() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create a URI for a file that doesn't exist
    let temp_dir = tempfile::tempdir().unwrap();
    let nonexistent_path = temp_dir.path().join("nonexistent.md");
    let uri = Url::from_file_path(&nonexistent_path).unwrap();

    // Call did_open - should handle missing file gracefully
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Should complete without panic (will publish error diagnostic)
}

/// Test server info contains version from Cargo.toml.
#[tokio::test]
async fn test_server_info_version() {
    let (service, _socket) = LspService::new(Backend::new);

    let init_params = InitializeParams::default();
    let result = service.inner().initialize(init_params).await.unwrap();

    let server_info = result.server_info.unwrap();
    let version = server_info.version.unwrap();

    // Version should be a valid semver string
    assert!(!version.is_empty());
    // Should match the crate version pattern (e.g., "0.1.0")
    assert!(version.contains('.'));
}

/// Test that initialize captures workspace root from root_uri.
#[tokio::test]
async fn test_initialize_captures_workspace_root() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    let init_params = InitializeParams {
        root_uri: Some(root_uri),
        ..Default::default()
    };

    let result = service.inner().initialize(init_params).await;
    assert!(result.is_ok());

    // The workspace root should now be set (we can't directly access it,
    // but the test verifies initialize handles root_uri without error)
}

/// Test that initialize loads config from .agnix.toml when present.
#[tokio::test]
async fn test_initialize_loads_config_from_file() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create a .agnix.toml config file
    let config_path = temp_dir.path().join(".agnix.toml");
    std::fs::write(
        &config_path,
        r#"
severity = "Warning"
target = "ClaudeCode"
exclude = []

[rules]
skills = false
"#,
    )
    .unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let init_params = InitializeParams {
        root_uri: Some(root_uri),
        ..Default::default()
    };

    let result = service.inner().initialize(init_params).await;
    assert!(result.is_ok());

    // The config should have been loaded (we can't directly access it,
    // but the test verifies initialize handles .agnix.toml without error)
}

/// Test that initialize handles invalid .agnix.toml gracefully.
#[tokio::test]
async fn test_initialize_handles_invalid_config() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create an invalid .agnix.toml config file
    let config_path = temp_dir.path().join(".agnix.toml");
    std::fs::write(&config_path, "this is not valid toml [[[").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let init_params = InitializeParams {
        root_uri: Some(root_uri),
        ..Default::default()
    };

    // Should still succeed (logs warning, uses default config)
    let result = service.inner().initialize(init_params).await;
    assert!(result.is_ok());
}

/// Test that files within workspace are validated normally.
#[tokio::test]
async fn test_file_within_workspace_validated() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create workspace with a skill file
    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
    )
    .unwrap();

    // Initialize with workspace root
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    let init_params = InitializeParams {
        root_uri: Some(root_uri),
        ..Default::default()
    };
    service.inner().initialize(init_params).await.unwrap();

    // File within workspace should be validated
    let uri = Url::from_file_path(&skill_path).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Should complete without error (file is within workspace)
}

/// Test that files outside workspace are rejected.
/// This tests the workspace boundary validation security feature.
#[tokio::test]
async fn test_file_outside_workspace_rejected() {
    let (service, _socket) = LspService::new(Backend::new);

    // Create two separate directories
    let workspace_dir = tempfile::tempdir().unwrap();
    let outside_dir = tempfile::tempdir().unwrap();

    // Create a file outside the workspace
    let outside_file = outside_dir.path().join("SKILL.md");
    std::fs::write(
        &outside_file,
        r#"---
name: outside-skill
version: 1.0.0
model: sonnet
---

# Outside Skill
"#,
    )
    .unwrap();

    // Initialize with workspace root
    let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
    let init_params = InitializeParams {
        root_uri: Some(root_uri),
        ..Default::default()
    };
    service.inner().initialize(init_params).await.unwrap();

    // Try to validate file outside workspace
    let uri = Url::from_file_path(&outside_file).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Should complete without error (logs warning and returns early)
    // The file is rejected but no panic occurs
}

/// Test validation without workspace root (backwards compatibility).
/// When no workspace root is set, all files should be accepted.
#[tokio::test]
async fn test_validation_without_workspace_root() {
    let (service, _socket) = LspService::new(Backend::new);

    // Initialize without root_uri
    let init_params = InitializeParams::default();
    service.inner().initialize(init_params).await.unwrap();

    // Create a file anywhere
    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
    )
    .unwrap();

    // Should validate normally (no workspace boundary check)
    let uri = Url::from_file_path(&skill_path).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: String::new(),
            },
        })
        .await;

    // Should complete without error
}

/// Test that cached config is used (performance optimization).
/// We verify this indirectly by running multiple validations.
#[tokio::test]
async fn test_cached_config_used_for_multiple_validations() {
    let (service, _socket) = LspService::new(Backend::new);

    // Initialize
    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Create multiple skill files
    let temp_dir = tempfile::tempdir().unwrap();
    for i in 0..3 {
        let skill_path = temp_dir.path().join(format!("skill{}/SKILL.md", i));
        std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        std::fs::write(
            &skill_path,
            format!(
                r#"---
name: test-skill-{}
version: 1.0.0
model: sonnet
---

# Test Skill {}
"#,
                i, i
            ),
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(),
                },
            })
            .await;
    }

    // All validations should complete (config is reused internally)
}

/// Regression test: validates multiple files using the cached registry.
/// Verifies the Arc<ValidatorRegistry> is thread-safe across spawn_blocking tasks.
#[tokio::test]
async fn test_cached_registry_used_for_multiple_validations() {
    let (service, _socket) = LspService::new(Backend::new);

    // Initialize
    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let temp_dir = tempfile::tempdir().unwrap();

    // Skill file
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
    )
    .unwrap();

    // CLAUDE.md file
    let claude_path = temp_dir.path().join("CLAUDE.md");
    std::fs::write(
        &claude_path,
        r#"# Project Memory

This is a test project.
"#,
    )
    .unwrap();

    for path in [&skill_path, &claude_path] {
        let uri = Url::from_file_path(path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: String::new(),
                },
            })
            .await;
    }
}

// ===== Cache Invalidation Tests =====

/// Test that document cache is cleared when document is closed.
#[tokio::test]
async fn test_document_cache_cleared_on_close() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: test\ndescription: Test\n---\n# Test",
    )
    .unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Open document
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: test\ndescription: Test\n---\n# Test".to_string(),
            },
        })
        .await;

    // Verify document is cached (hover should work)
    let hover_before = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;
    assert!(hover_before.is_ok());
    assert!(hover_before.unwrap().is_some());

    // Close document
    service
        .inner()
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    // Verify document cache is cleared (hover should return None)
    let hover_after = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;
    assert!(hover_after.is_ok());
    assert!(hover_after.unwrap().is_none());
}

/// Test that document cache is updated on change.
#[tokio::test]
async fn test_document_cache_updated_on_change() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Initial").unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Open with initial content
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Initial".to_string(),
            },
        })
        .await;

    // Change to content with frontmatter
    service
        .inner()
        .did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "---\nname: updated\ndescription: Updated\n---\n# Updated".to_string(),
            }],
        })
        .await;

    // Verify cache has new content (hover should work on frontmatter)
    let hover = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;
    assert!(hover.is_ok());
    assert!(hover.unwrap().is_some());
}

/// Regression: cached document reads should share the same allocation.
#[tokio::test]
async fn test_get_document_content_returns_shared_arc() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Shared").unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Shared".to_string(),
            },
        })
        .await;

    let first = service
        .inner()
        .get_document_content(&uri)
        .await
        .expect("cached content should exist");
    let second = service
        .inner()
        .get_document_content(&uri)
        .await
        .expect("cached content should exist");

    assert!(Arc::ptr_eq(&first, &second));
}

/// Test that multiple documents have independent caches.
#[tokio::test]
async fn test_multiple_documents_independent_caches() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create two skill files
    let skill1_path = temp_dir.path().join("skill1").join("SKILL.md");
    let skill2_path = temp_dir.path().join("skill2").join("SKILL.md");
    std::fs::create_dir_all(skill1_path.parent().unwrap()).unwrap();
    std::fs::create_dir_all(skill2_path.parent().unwrap()).unwrap();

    std::fs::write(
        &skill1_path,
        "---\nname: skill-one\ndescription: First\n---\n# One",
    )
    .unwrap();
    std::fs::write(
        &skill2_path,
        "---\nname: skill-two\ndescription: Second\n---\n# Two",
    )
    .unwrap();

    let uri1 = Url::from_file_path(&skill1_path).unwrap();
    let uri2 = Url::from_file_path(&skill2_path).unwrap();

    // Open both documents
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri1.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: skill-one\ndescription: First\n---\n# One".to_string(),
            },
        })
        .await;

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri2.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: skill-two\ndescription: Second\n---\n# Two".to_string(),
            },
        })
        .await;

    // Close first document
    service
        .inner()
        .did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri1.clone() },
        })
        .await;

    // First document should be cleared
    let hover1 = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri1 },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;
    assert!(hover1.is_ok());
    assert!(hover1.unwrap().is_none());

    // Second document should still be cached
    let hover2 = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: uri2 },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;
    assert!(hover2.is_ok());
    assert!(hover2.unwrap().is_some());
}

// ===== Configuration Change Tests =====

/// Test that did_change_configuration handles valid settings.
#[tokio::test]
async fn test_did_change_configuration_valid_settings() {
    let (service, _socket) = LspService::new(Backend::new);

    // Initialize first
    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Send valid configuration
    let settings = serde_json::json!({
        "severity": "Error",
        "target": "ClaudeCode",
        "rules": {
            "skills": false,
            "hooks": true
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
    // The config is internally updated but we can't directly access it
}

/// Test that did_change_configuration handles partial settings.
#[tokio::test]
async fn test_did_change_configuration_partial_settings() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Send only severity (partial config)
    let settings = serde_json::json!({
        "severity": "Info"
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test that did_change_configuration handles invalid JSON gracefully.
#[tokio::test]
async fn test_did_change_configuration_invalid_json() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Send invalid JSON type (string instead of object)
    let settings = serde_json::json!("not an object");

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error (logs warning and returns early)
}

/// Test bounded helper used by did_change_configuration.
#[test]
fn test_config_revalidation_concurrency_bounds() {
    let expected_cap = std::thread::available_parallelism()
        .map(|count| count.get())
        .unwrap_or(4)
        .clamp(1, MAX_CONFIG_REVALIDATION_CONCURRENCY);

    assert_eq!(config_revalidation_concurrency(0), 0);
    assert_eq!(config_revalidation_concurrency(1), 1);
    assert_eq!(
        config_revalidation_concurrency(MAX_CONFIG_REVALIDATION_CONCURRENCY * 4),
        expected_cap
    );
}

/// Test bounded helper handles empty inputs with no task errors.
#[tokio::test]
async fn test_for_each_bounded_empty_input() {
    let errors = for_each_bounded(Vec::<usize>::new(), 3, |_| async {}).await;
    assert!(errors.is_empty());
}

/// Test bounded helper reports join errors when inner tasks panic.
#[tokio::test]
async fn test_for_each_bounded_collects_join_errors() {
    let errors = for_each_bounded(vec![0usize, 1, 2], 2, |idx| async move {
        if idx == 1 {
            panic!("intentional panic for join error coverage");
        }
    })
    .await;

    assert_eq!(errors.len(), 1);
    assert!(errors[0].is_panic());
}

/// Test generation guard for config-change batch publishing.
#[tokio::test]
async fn test_should_publish_diagnostics_guard() {
    let (service, _socket) = LspService::new(Backend::new);
    let backend = service.inner();

    let temp_dir = tempfile::tempdir().unwrap();
    let path = temp_dir.path().join("SKILL.md");
    std::fs::write(&path, "# test").unwrap();
    let uri = Url::from_file_path(&path).unwrap();

    let snapshot = Arc::new("# test".to_string());
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::clone(&snapshot));
    backend.config_generation.store(7, Ordering::SeqCst);

    assert!(
        backend
            .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
            .await
    );
    assert!(
        !backend
            .should_publish_diagnostics(&uri, Some(6), Some(&snapshot))
            .await
    );

    // New content (new Arc) means stale validation result should not publish.
    backend
        .documents
        .write()
        .await
        .insert(uri.clone(), Arc::new("# updated".to_string()));
    assert!(
        !backend
            .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
            .await
    );

    backend.documents.write().await.remove(&uri);
    assert!(
        !backend
            .should_publish_diagnostics(&uri, Some(7), Some(&snapshot))
            .await
    );

    assert!(backend.should_publish_diagnostics(&uri, None, None).await);
}

/// Test bounded helper used by did_change_configuration.
#[tokio::test]
async fn test_did_change_configuration_concurrency_bound_helper() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::Barrier;

    let max_concurrency = 3usize;
    let in_flight = Arc::new(AtomicUsize::new(0));
    let peak_in_flight = Arc::new(AtomicUsize::new(0));
    let completed = Arc::new(AtomicUsize::new(0));
    let ready = Arc::new(Barrier::new(max_concurrency + 1));
    let release = Arc::new(Barrier::new(max_concurrency + 1));
    let total_items = 12usize;

    let run = tokio::spawn(for_each_bounded(0..total_items, max_concurrency, {
        let in_flight = Arc::clone(&in_flight);
        let peak_in_flight = Arc::clone(&peak_in_flight);
        let completed = Arc::clone(&completed);
        let ready = Arc::clone(&ready);
        let release = Arc::clone(&release);
        move |idx| {
            let in_flight = Arc::clone(&in_flight);
            let peak_in_flight = Arc::clone(&peak_in_flight);
            let completed = Arc::clone(&completed);
            let ready = Arc::clone(&ready);
            let release = Arc::clone(&release);

            async move {
                let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                peak_in_flight.fetch_max(current, Ordering::SeqCst);

                if idx < max_concurrency {
                    ready.wait().await;
                    release.wait().await;
                } else {
                    tokio::task::yield_now().await;
                }

                in_flight.fetch_sub(1, Ordering::SeqCst);
                completed.fetch_add(1, Ordering::SeqCst);
            }
        }
    }));

    // Wait for the first wave of tasks to all be in-flight at once.
    tokio::time::timeout(Duration::from_secs(2), ready.wait())
        .await
        .expect("timed out waiting for first wave");
    assert_eq!(peak_in_flight.load(Ordering::SeqCst), max_concurrency);
    tokio::time::timeout(Duration::from_secs(2), release.wait())
        .await
        .expect("timed out releasing first wave");

    let join_errors = tokio::time::timeout(Duration::from_secs(2), run)
        .await
        .expect("timed out waiting for bounded worker completion")
        .unwrap();

    assert!(join_errors.is_empty());
    assert_eq!(completed.load(Ordering::SeqCst), total_items);
}

/// Test that did_change_configuration triggers revalidation.
#[tokio::test]
async fn test_did_change_configuration_triggers_revalidation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        r#"---
name: test-skill
version: 1.0.0
model: sonnet
---

# Test Skill
"#,
    )
    .unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: std::fs::read_to_string(&skill_path).unwrap(),
            },
        })
        .await;

    // Now change configuration - should trigger revalidation
    let settings = serde_json::json!({
        "severity": "Error",
        "rules": {
            "skills": false
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error - open document was revalidated
}

/// Test that config changes revalidate all currently open documents.
#[tokio::test]
async fn test_did_change_configuration_triggers_revalidation_for_multiple_documents() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let document_count = 6usize;

    for i in 0..document_count {
        let skill_path = temp_dir.path().join(format!("skill-{i}/SKILL.md"));
        std::fs::create_dir_all(skill_path.parent().unwrap()).unwrap();
        std::fs::write(
            &skill_path,
            format!(
                r#"---
name: test-skill-{i}
version: 1.0.0
model: sonnet
---

# Test Skill {i}
"#
            ),
        )
        .unwrap();

        let uri = Url::from_file_path(&skill_path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&skill_path).unwrap(),
                },
            })
            .await;
    }

    let settings = serde_json::json!({
        "severity": "Error",
        "rules": {
            "skills": false
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    let open_documents = service.inner().documents.read().await.len();
    assert_eq!(open_documents, document_count);
}

/// Test that empty settings object doesn't crash.
#[tokio::test]
async fn test_did_change_configuration_empty_settings() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Send empty object
    let settings = serde_json::json!({});

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test configuration with all tool versions set.
#[tokio::test]
async fn test_did_change_configuration_with_versions() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let settings = serde_json::json!({
        "versions": {
            "claude_code": "1.0.0",
            "codex": "0.1.0",
            "cursor": "0.45.0",
            "copilot": "1.2.0"
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test configuration with spec revisions.
#[tokio::test]
async fn test_did_change_configuration_with_specs() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let settings = serde_json::json!({
        "specs": {
            "mcp_protocol": "2025-11-25",
            "agent_skills_spec": "1.0",
            "agents_md_spec": "1.0"
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test configuration with tools array.
#[tokio::test]
async fn test_did_change_configuration_with_tools_array() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let settings = serde_json::json!({
        "tools": ["claude-code", "cursor", "github-copilot"]
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test configuration with disabled rules.
#[tokio::test]
async fn test_did_change_configuration_with_disabled_rules() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let settings = serde_json::json!({
        "rules": {
            "disabled_rules": ["AS-001", "PE-003", "MCP-008"]
        }
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    // Should complete without error
}

/// Test that did_change_configuration handles locale setting.
#[tokio::test]
async fn test_did_change_configuration_with_locale() {
    let (service, _socket) = {
        let _guard = crate::locale::LOCALE_MUTEX.lock().unwrap();
        LspService::new(Backend::new)
    };

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let settings = serde_json::json!({
        "severity": "Warning",
        "locale": "es"
    });

    service
        .inner()
        .did_change_configuration(DidChangeConfigurationParams { settings })
        .await;

    {
        let _guard = crate::locale::LOCALE_MUTEX.lock().unwrap();
        // Verify locale was actually changed
        assert_eq!(&*rust_i18n::locale(), "es");
    }

    // Reset locale for other tests
    rust_i18n::set_locale("en");
}

// ===== normalize_path() Unit Tests =====

/// Test that '..' components are resolved by removing the preceding normal component.
#[test]
fn test_normalize_path_resolves_parent() {
    let result = normalize_path(Path::new("/a/b/../c"));
    assert_eq!(result, PathBuf::from("/a/c"));
}

/// Test that '.' components are removed entirely.
#[test]
fn test_normalize_path_removes_curdir() {
    let result = normalize_path(Path::new("/a/./b/./c"));
    assert_eq!(result, PathBuf::from("/a/b/c"));
}

/// Test that multiple '..' components are resolved correctly.
#[test]
fn test_normalize_path_multiple_parent() {
    let result = normalize_path(Path::new("/a/b/../../c"));
    assert_eq!(result, PathBuf::from("/c"));
}

/// Test that a path without special components is returned unchanged.
#[test]
fn test_normalize_path_already_clean() {
    let result = normalize_path(Path::new("/a/b/c"));
    assert_eq!(result, PathBuf::from("/a/b/c"));
}

/// Test that '..' cannot traverse above root.
#[test]
fn test_normalize_path_cannot_escape_root() {
    let result = normalize_path(Path::new("/../a"));
    assert_eq!(result, PathBuf::from("/a"));
}

/// Test that root alone is preserved.
#[test]
fn test_normalize_path_root_only() {
    let result = normalize_path(Path::new("/"));
    assert_eq!(result, PathBuf::from("/"));
}

/// Test excessive '..' beyond root is clamped.
#[test]
fn test_normalize_path_excessive_parent_traversal() {
    let result = normalize_path(Path::new("/a/../../../b"));
    assert_eq!(result, PathBuf::from("/b"));
}

/// Test mixed '.' and '..' components together.
#[test]
fn test_normalize_path_mixed_special_components() {
    let result = normalize_path(Path::new("/a/./b/../c/./d"));
    assert_eq!(result, PathBuf::from("/a/c/d"));
}

// ===== Path Traversal Regression Tests =====

/// Regression: a URI with '..' that escapes the workspace must be rejected
/// even when the file does not exist on disk (so canonicalize() fails).
#[tokio::test]
async fn test_path_traversal_outside_workspace_rejected() {
    let (service, _socket) = LspService::new(Backend::new);

    let workspace_dir = tempfile::tempdir().unwrap();
    let outside_dir = tempfile::tempdir().unwrap();

    // Extract the outside directory name for the traversal path
    let outside_name = outside_dir
        .path()
        .file_name()
        .expect("should have a file name")
        .to_str()
        .expect("should be valid UTF-8");

    // Initialize with workspace root
    let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Construct a path that uses '..' to escape the workspace.
    // The file does not exist, so canonicalize() will fail and
    // the code must fall back to normalize_path().
    let traversal_path = workspace_dir
        .path()
        .join("..")
        .join("..")
        .join(outside_name)
        .join("SKILL.md");
    let uri = Url::from_file_path(&traversal_path).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: evil\n---\n# Evil".to_string(),
            },
        })
        .await;

    // Should complete without panic -- the file is outside the workspace
    // so it is silently rejected (warning logged, no diagnostics published).
}

/// Regression: a URI with '..' that resolves *inside* the workspace must
/// still be accepted for validation.
#[tokio::test]
async fn test_path_traversal_inside_workspace_accepted() {
    let (service, _socket) = LspService::new(Backend::new);

    let workspace_dir = tempfile::tempdir().unwrap();

    // Create subdir and a SKILL.md at the workspace root
    let subdir = workspace_dir.path().join("subdir");
    std::fs::create_dir(&subdir).unwrap();
    let skill_path = workspace_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: test-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Test Skill\n",
    )
    .unwrap();

    // Initialize with workspace root
    let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // URI with '..' that resolves back into the workspace

    // URI with '..' that resolves back into the workspace
    let traversal_path = workspace_dir
        .path()
        .join("subdir")
        .join("..")
        .join("SKILL.md");
    let uri = Url::from_file_path(&traversal_path).unwrap();
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: std::fs::read_to_string(&skill_path).unwrap(),
            },
        })
        .await;

    // Should complete without error -- file resolves inside workspace
}

/// Regression: a non-existent file within the workspace boundary
/// (without any '..' components) must not be rejected.
#[tokio::test]
async fn test_nonexistent_file_in_workspace_accepted() {
    let (service, _socket) = LspService::new(Backend::new);

    let workspace_dir = tempfile::tempdir().unwrap();

    // Initialize with workspace root
    let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Non-existent file inside workspace (no '..' components)
    let nonexistent = workspace_dir.path().join("SKILL.md");
    let uri = Url::from_file_path(&nonexistent).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: ghost\n---\n# Ghost".to_string(),
            },
        })
        .await;

    // Should pass boundary check -- path is inside workspace
}

/// Regression: a URI with '.' components (current-dir markers) must be
/// accepted when the file is inside the workspace.
#[tokio::test]
async fn test_dot_components_in_path_accepted() {
    let (service, _socket) = LspService::new(Backend::new);

    let workspace_dir = tempfile::tempdir().unwrap();
    let skill_path = workspace_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: test-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Test Skill\n",
    )
    .unwrap();

    // Initialize with workspace root
    let root_uri = Url::from_file_path(workspace_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // URI with '.' components
    let dot_path = format!("{}/./SKILL.md", workspace_dir.path().display());
    let uri = Url::parse(&format!("file://{}", dot_path)).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: std::fs::read_to_string(&skill_path).unwrap(),
            },
        })
        .await;

    // Should pass boundary check -- '.' resolves to the same directory
}

// ===== Project-Level Validation Tests =====

/// Test that validate_project_rules_and_publish returns early without panic
/// when no workspace root is set.
#[tokio::test]
async fn test_validate_project_rules_no_workspace() {
    let (service, _socket) = LspService::new(Backend::new);

    // Initialize without workspace root
    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    // Should return early without error (no workspace root)
    service.inner().validate_project_rules_and_publish().await;

    // Verify no project diagnostics were stored
    let proj_diags = service.inner().project_level_diagnostics.read().await;
    assert!(
        proj_diags.is_empty(),
        "No project diagnostics should be stored without workspace root"
    );
}

/// Test that project-level diagnostics are cached after running validation.
#[tokio::test]
async fn test_project_diagnostics_cached() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files to trigger AGM-006
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // initialize() now spawns project validation in the background.
    // Wait for it to complete before asserting.
    for _ in 0..80 {
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        if !proj_diags.is_empty() {
            break;
        }
        drop(proj_diags);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Verify project diagnostics are stored
    let proj_diags = service.inner().project_level_diagnostics.read().await;
    assert!(
        !proj_diags.is_empty(),
        "Project diagnostics should be cached for AGM-006"
    );

    // Verify URIs are tracked for cleanup
    let proj_uris = service.inner().project_diagnostics_uris.read().await;
    assert!(
        !proj_uris.is_empty(),
        "Project diagnostic URIs should be tracked"
    );
}

/// Test that stale project diagnostics are cleared on re-run.
#[tokio::test]
async fn test_project_diagnostics_cleared_on_rerun() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files to trigger AGM-006
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // initialize() now spawns project validation in the background.
    // Wait for it to complete before continuing.
    for _ in 0..80 {
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        if !proj_diags.is_empty() {
            break;
        }
        drop(proj_diags);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    let count_before = service.inner().project_diagnostics_uris.read().await.len();
    assert!(
        count_before > 0,
        "Should have project diagnostics before cleanup"
    );

    // Remove the nested AGENTS.md to resolve the issue
    std::fs::remove_file(sub.join("AGENTS.md")).unwrap();

    // Second run: AGM-006 should no longer fire
    service.inner().validate_project_rules_and_publish().await;

    let proj_diags = service.inner().project_level_diagnostics.read().await;
    let agm006_count: usize = proj_diags
        .values()
        .flat_map(|diags| diags.iter())
        .filter(|d| {
            d.code
                .as_ref()
                .map(|c| matches!(c, NumberOrString::String(s) if s == "AGM-006"))
                .unwrap_or(false)
        })
        .count();
    assert_eq!(agm006_count, 0, "AGM-006 should be cleared after fix");
}

/// Test stale generation guard returns early without mutating cached project diagnostics.
#[tokio::test]
async fn test_project_validation_stale_generation_returns_early() {
    let (service, _socket) = LspService::new(Backend::new);
    let backend = service.inner().clone();

    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files so project validation has work to do.
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Pre-populate cache so we can verify stale run does not overwrite it.
    let sentinel_path = temp_dir.path().join("sentinel.md");
    let sentinel_uri = Url::from_file_path(&sentinel_path).unwrap();
    let sentinel_diag = Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String("SENTINEL".to_string())),
        code_description: None,
        source: Some("agnix".to_string()),
        message: "sentinel".to_string(),
        related_information: None,
        tags: None,
        data: None,
    };
    {
        let mut proj_diags = service.inner().project_level_diagnostics.write().await;
        proj_diags.insert(sentinel_uri.clone(), vec![sentinel_diag]);
    }
    {
        let mut proj_uris = service.inner().project_diagnostics_uris.write().await;
        proj_uris.insert(sentinel_uri.clone());
    }

    // Continuously bump generation to force stale detection in the running validation.
    let bump_backend = service.inner().clone();
    let bump = tokio::spawn(async move {
        for _ in 0..200 {
            bump_backend
                .project_validation_generation
                .store(9_999, Ordering::SeqCst);
            tokio::task::yield_now().await;
        }
    });

    backend.validate_project_rules_and_publish().await;
    bump.abort();

    let proj_diags = service.inner().project_level_diagnostics.read().await;
    assert!(
        proj_diags.contains_key(&sentinel_uri),
        "stale generation run should return before overwriting cached diagnostics"
    );

    let proj_uris = service.inner().project_diagnostics_uris.read().await;
    assert!(
        proj_uris.contains(&sentinel_uri),
        "stale generation run should return before mutating cached URI set"
    );
}

/// Test is_project_level_trigger for various file names.
#[test]
fn test_is_project_level_trigger() {
    // Instruction files should trigger
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/CLAUDE.md"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/AGENTS.md"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.clinerules"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.cursorrules"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.github/copilot-instructions.md"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.github/instructions/test.instructions.md"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.cursor/rules/test.mdc"
    )));
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/GEMINI.md"
    )));

    // .agnix.toml should trigger
    assert!(Backend::is_project_level_trigger(Path::new(
        "/project/.agnix.toml"
    )));

    // Non-instruction files should not trigger
    assert!(!Backend::is_project_level_trigger(Path::new(
        "/project/SKILL.md"
    )));
    assert!(!Backend::is_project_level_trigger(Path::new(
        "/project/README.md"
    )));
    assert!(!Backend::is_project_level_trigger(Path::new(
        "/project/settings.json"
    )));
    assert!(!Backend::is_project_level_trigger(Path::new(
        "/project/plugin.json"
    )));
}

/// Test that initialize advertises executeCommand capability.
#[tokio::test]
async fn test_initialize_advertises_execute_command() {
    let (service, _socket) = LspService::new(Backend::new);

    let result = service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    match result.capabilities.execute_command_provider {
        Some(ref opts) => {
            assert!(
                opts.commands
                    .contains(&"agnix.validateProjectRules".to_string()),
                "Expected agnix.validateProjectRules in execute commands, got: {:?}",
                opts.commands
            );
        }
        None => panic!("Expected execute command capability"),
    }
}

/// Test that execute_command handles the validateProjectRules command.
#[tokio::test]
async fn test_execute_command_validate_project_rules() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Execute the command
    let result = service
        .inner()
        .execute_command(ExecuteCommandParams {
            command: "agnix.validateProjectRules".to_string(),
            arguments: vec![],
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test that execute_command handles unknown commands gracefully.
#[tokio::test]
async fn test_execute_command_unknown() {
    let (service, _socket) = LspService::new(Backend::new);

    service
        .inner()
        .initialize(InitializeParams::default())
        .await
        .unwrap();

    let result = service
        .inner()
        .execute_command(ExecuteCommandParams {
            command: "unknown.command".to_string(),
            arguments: vec![],
            work_done_progress_params: WorkDoneProgressParams::default(),
        })
        .await;

    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

/// Test that project-level diagnostics are merged with per-file diagnostics
/// when validate_from_content_and_publish is called.
///
/// Pre-populates the project_level_diagnostics cache with a diagnostic for
/// a file URI, then opens the file so per-file validation runs and the merge
/// path in validate_from_content_and_publish is exercised.
#[tokio::test]
async fn test_project_and_file_diagnostics_merged() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    // Initialize with workspace root
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Create a CLAUDE.md that will produce per-file diagnostics (e.g. XML-001)
    let claude_path = temp_dir.path().join("CLAUDE.md");
    std::fs::write(&claude_path, "<unclosed>\n# Project\n").unwrap();
    let uri = Url::from_file_path(&claude_path).unwrap();

    // Wait for the background project validation spawned by initialize()
    // to complete before injecting fake diagnostics, to avoid a race where
    // the background run overwrites our manually inserted data.
    for _ in 0..80 {
        let generation = service
            .inner()
            .project_validation_generation
            .load(std::sync::atomic::Ordering::SeqCst);
        if generation >= 1 {
            // Give the async task a moment to finish writing results
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    // Pre-populate project_level_diagnostics with a fake AGM-006 diagnostic
    // for this URI, simulating what validate_project_rules_and_publish would store.
    {
        let fake_project_diag = Diagnostic {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: 0,
                    character: 0,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("AGM-006".to_string())),
            code_description: None,
            source: Some("agnix".to_string()),
            message: "Nested AGENTS.md detected".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };
        let mut proj_diags = service.inner().project_level_diagnostics.write().await;
        proj_diags.insert(uri.clone(), vec![fake_project_diag]);
    }

    // Verify the project diagnostics are in the cache
    {
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        assert!(
            proj_diags.contains_key(&uri),
            "Project diagnostics should be pre-populated for the URI"
        );
    }

    // Open the file -- this triggers validate_from_content_and_publish which
    // should merge per-file diagnostics (e.g. XML-001) with the cached
    // project-level diagnostics (AGM-006).
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: std::fs::read_to_string(&claude_path).unwrap(),
            },
        })
        .await;

    // The merge code path in validate_from_content_and_publish (lines 309-315)
    // was exercised: it reads project_level_diagnostics and extends the
    // per-file diagnostics with any matching project-level entries.
    // Verify the project cache is still intact after the merge.
    {
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        let diags = proj_diags
            .get(&uri)
            .expect("Project diagnostics should still be cached");
        assert!(
            diags
                .iter()
                .any(|d| d.code == Some(NumberOrString::String("AGM-006".to_string()))),
            "Cached project diagnostic should be preserved after merge"
        );
    }
}

// ===== for_each_bounded additional tests =====

#[tokio::test]
async fn test_for_each_bounded_concurrency_limit_one() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let max_concurrent = Arc::new(AtomicUsize::new(0));
    let current = Arc::new(AtomicUsize::new(0));

    let items: Vec<usize> = (0..5).collect();

    let max_c = Arc::clone(&max_concurrent);
    let cur = Arc::clone(&current);

    let errors = for_each_bounded(items, 1, move |_item| {
        let max_c = Arc::clone(&max_c);
        let cur = Arc::clone(&cur);
        async move {
            let c = cur.fetch_add(1, Ordering::SeqCst) + 1;
            // Update max observed concurrency
            max_c.fetch_max(c, Ordering::SeqCst);
            // Yield to give other tasks a chance to run
            tokio::task::yield_now().await;
            cur.fetch_sub(1, Ordering::SeqCst);
        }
    })
    .await;

    assert!(errors.is_empty());
    assert_eq!(
        max_concurrent.load(Ordering::SeqCst),
        1,
        "With concurrency limit 1, at most 1 task should run concurrently"
    );
}

#[tokio::test]
async fn test_for_each_bounded_zero_concurrency_defaults_to_one() {
    // Passing 0 as max_concurrency should be clamped to 1 (not hang or panic)
    let items = vec![1, 2, 3];
    let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count_clone = Arc::clone(&count);

    let errors = for_each_bounded(items, 0, move |_| {
        let count = Arc::clone(&count_clone);
        async move {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        }
    })
    .await;

    assert!(errors.is_empty());
    assert_eq!(
        count.load(std::sync::atomic::Ordering::SeqCst),
        3,
        "All items should be processed even with concurrency 0"
    );
}

/// Test that GenericMarkdown files are not validated by the LSP.
///
/// A `.md` file that doesn't match any specific agent pattern gets classified
/// as GenericMarkdown. The LSP should skip validation for these to avoid
/// false positives on developer docs, project specs, etc.
#[tokio::test]
async fn test_generic_markdown_not_validated() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Create a generic markdown file (not a known agent config pattern).
    // "notes.md" at the project root is classified as GenericMarkdown.
    let notes_path = temp_dir.path().join("notes.md");
    let content = "<unclosed>\n# Some developer notes\n";
    std::fs::write(&notes_path, content).unwrap();

    let uri = Url::from_file_path(&notes_path).unwrap();

    // Open the file - the LSP should skip validation for GenericMarkdown
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // did_open always caches the document content before calling
    // validate_from_content_and_publish. The GenericMarkdown early return
    // skips validation but does not prevent caching.
    let docs = service.inner().documents.read().await;
    assert!(
        docs.contains_key(&uri),
        "Document should be cached (did_open always caches)"
    );
}

/// Test that hover() returns None for GenericMarkdown files.
#[tokio::test]
async fn test_hover_returns_none_for_generic_markdown() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Create a generic markdown file and open it
    let notes_path = temp_dir.path().join("notes.md");
    let content = "---\nname: test\n---\n# Notes\n";
    std::fs::write(&notes_path, content).unwrap();
    let uri = Url::from_file_path(&notes_path).unwrap();

    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    // Hover on a GenericMarkdown file should return None
    let hover_result = service
        .inner()
        .hover(HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position: Position {
                    line: 1,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
        })
        .await
        .unwrap();

    assert!(
        hover_result.is_none(),
        "Hover should return None for GenericMarkdown files"
    );
}

/// Test that specific agent config files ARE validated (not skipped).
///
/// Ensures the GenericMarkdown skip doesn't accidentally filter out
/// real agent configuration files.
#[tokio::test]
async fn test_agent_config_files_still_validated() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // CLAUDE.md is FileType::ClaudeMd - should be validated (not generic)
    let claude_path = temp_dir.path().join("CLAUDE.md");
    let content = "# Project\n\nSome instructions.\n";
    std::fs::write(&claude_path, content).unwrap();

    let uri = Url::from_file_path(&claude_path).unwrap();

    // Verify the file type is NOT generic
    let config = service.inner().config.load();
    let file_type = agnix_core::resolve_file_type(&claude_path, &config);
    assert!(
        !file_type.is_generic(),
        "CLAUDE.md should NOT be classified as generic (got {:?})",
        file_type
    );
    assert_eq!(file_type, agnix_core::FileType::ClaudeMd);
    drop(config);

    // Open should proceed through full validation path
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;
}

/// Test that disabled_validators from .agnix.toml config are respected
/// when validating via the LSP content path.
///
/// This verifies that the LSP uses `validate_content()` (which checks
/// disabled_validators) rather than a manual validator loop.
#[tokio::test]
async fn test_disabled_validators_respected_in_content_validation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create config that disables the XmlValidator
    let config_path = temp_dir.path().join(".agnix.toml");
    std::fs::write(
        &config_path,
        r#"
[rules]
disabled_validators = ["XmlValidator"]
"#,
    )
    .unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Verify the config was loaded with disabled validators
    let config = service.inner().config.load();
    assert!(
        config
            .rules()
            .disabled_validators
            .iter()
            .any(|v| v == "XmlValidator"),
        "XmlValidator should be in disabled_validators list"
    );
    drop(config);

    // Create a CLAUDE.md with content that would trigger XmlValidator
    let claude_path = temp_dir.path().join("CLAUDE.md");
    let content = "<unclosed>\n# Project\n";
    std::fs::write(&claude_path, content).unwrap();

    let uri = Url::from_file_path(&claude_path).unwrap();

    // Open the file - exercises validate_from_content_and_publish
    // with validate_content() that respects disabled_validators.
    // This should complete without error (the disabled validator is skipped).
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;
}

/// Test that project-level validation starts during initialize().
///
/// Previously, project validation only ran in initialized() (after the
/// client sends the initialized notification). Now it starts in
/// initialize() so diagnostics are available sooner.
#[tokio::test]
async fn test_project_validation_starts_in_initialize() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create two AGENTS.md files to trigger AGM-006
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    // Only call initialize (NOT initialized) - project validation should
    // still start because we moved spawn_project_validation() there.
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Wait for async project validation to complete
    let mut found = false;
    for _ in 0..80 {
        let proj_diags = service.inner().project_level_diagnostics.read().await;
        if !proj_diags.is_empty() {
            found = true;
            break;
        }
        drop(proj_diags);
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    assert!(
        found,
        "Project-level validation should run during initialize(), \
         producing AGM-006 diagnostics for duplicate AGENTS.md files"
    );
}

// ===== Concurrent Revalidation Stress Tests =====

/// Stress test: 20 concurrent document open/close cycles must not panic
/// or leave stale entries in the document cache.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_concurrent_document_open_close() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let doc_count = 20usize;

    // Create 20 subdirectories, each with a valid SKILL.md
    for i in 0..doc_count {
        let dir = temp_dir.path().join(format!("skill-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: stress-skill-{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Stress Skill {i}\n"
            ),
        )
        .unwrap();
    }

    let backend = service.inner().clone();

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut handles = Vec::new();
        for i in 0..doc_count {
            let backend = backend.clone();
            let path = temp_dir.path().join(format!("skill-{i}")).join("SKILL.md");
            // Read content before spawning: avoids running 20 concurrent blocking
            // std::fs reads inside spawned tasks (one per task in a hot loop).
            let content = std::fs::read_to_string(&path).unwrap();
            let uri = Url::from_file_path(&path).unwrap();
            handles.push(tokio::spawn(async move {
                backend
                    .did_open(DidOpenTextDocumentParams {
                        text_document: TextDocumentItem {
                            uri: uri.clone(),
                            language_id: "markdown".to_string(),
                            version: 1,
                            text: content,
                        },
                    })
                    .await;
                // did_close removes the document from the cache synchronously
                // before spawning any background I/O, so the post-join emptiness
                // assertion below is safe without a drain step.
                backend
                    .did_close(DidCloseTextDocumentParams {
                        text_document: TextDocumentIdentifier { uri },
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }
    })
    .await;

    assert!(result.is_ok(), "concurrent open/close timed out");

    // After all close operations, the document cache should be empty
    let docs = service.inner().documents.read().await;
    assert!(
        docs.is_empty(),
        "documents cache should be empty after all close operations, found {} entries",
        docs.len()
    );
}

/// Stress test: concurrent config_generation increments and should_publish_diagnostics
/// checks. Exercises the stale-batch generation guard under concurrent load by
/// directly driving the AtomicU64 counter while concurrently querying the
/// staleness predicate.
///
/// Note: calling did_change_configuration once works fine (see
/// test_did_change_configuration_triggers_revalidation_for_multiple_documents),
/// but calling it N times in a tight loop would fill the bounded channel
/// (capacity 1) because each call unconditionally sends a log_message via
/// send_notification_unchecked and the test socket is not consumed. This test
/// drives the same AtomicU64 counter directly to exercise N concurrent probes
/// without going through the notification path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_rapid_config_changes_drop_stale_batches() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Insert a SKILL.md into the document cache directly (no did_open) so
    // should_publish_diagnostics has a live URI to check.
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: stress-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Stress Test\n",
    )
    .unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();
    {
        let mut docs = service.inner().documents.write().await;
        docs.insert(
            uri.clone(),
            Arc::new(std::fs::read_to_string(&skill_path).unwrap()),
        );
    }

    let change_count = 50u64;
    let backend_a = service.inner().clone();
    let backend_b = service.inner().clone();
    let uri_b = uri.clone();

    // Task A: rapidly increments config_generation (simulating rapid config changes).
    let task_a = tokio::spawn(async move {
        for _ in 0..change_count {
            backend_a.config_generation.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
        }
    });

    // Task B: concurrently queries should_publish_diagnostics with progressively
    // stale generation values. As Task A bumps the counter, more of these should
    // return false (stale detected).
    let task_b = tokio::spawn(async move {
        let mut stale_count = 0u32;
        // Each iteration probes a different generation value (0, 1, 2...).
        // Once Task A has advanced the counter past probe_gen, the check returns
        // false (stale). When probe_gen matches the current counter it returns true.
        for probe_gen in 0..change_count {
            if !backend_b
                .should_publish_diagnostics(&uri_b, Some(probe_gen), None)
                .await
            {
                stale_count += 1;
            }
            tokio::task::yield_now().await;
        }
        stale_count
    });

    let result = tokio::time::timeout(std::time::Duration::from_secs(10), async move {
        task_a.await.unwrap();
        task_b.await.unwrap()
    })
    .await;

    assert!(
        result.is_ok(),
        "concurrent config_generation stress test timed out"
    );
    // Discard the concurrent stale-count: it is scheduler-dependent and covered
    // deterministically by the post-completion loop below.
    let _ = result.unwrap();

    let final_gen = service.inner().config_generation.load(Ordering::SeqCst);
    assert_eq!(
        final_gen, change_count,
        "config_generation should be {} after {} increments, got {}",
        change_count, change_count, final_gen
    );

    // Deterministic post-completion check: with the counter now at change_count,
    // every probe value in [0, change_count) MUST be stale. The range includes
    // change_count - 1 (one-behind the final value) to catch the boundary case.
    let backend = service.inner().clone();
    for probe in 0..change_count {
        assert!(
            !backend
                .should_publish_diagnostics(&uri, Some(probe), None)
                .await,
            "probe_gen {} should be stale when config_generation is {}",
            probe,
            final_gen
        );
    }

    assert_eq!(
        service.inner().documents.read().await.len(),
        1,
        "document should still be in cache after concurrent stress"
    );
}

/// Stress test: 30 concurrent did_change calls on the same document must
/// not corrupt the cache or panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_concurrent_changes_same_document() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: concurrent-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Concurrent\n",
    )
    .unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Open the document first
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: std::fs::read_to_string(&skill_path).unwrap(),
            },
        })
        .await;

    let backend = service.inner().clone();
    let change_count = 30usize;

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut handles = Vec::new();
        for i in 0..change_count {
            let backend = backend.clone();
            let uri = uri.clone();
            handles.push(tokio::spawn(async move {
                backend
                    .did_change(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri,
                            version: (i + 2) as i32,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: format!(
                                "---\nname: v{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Version {i}\n"
                            ),
                        }],
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }
    })
    .await;

    assert!(result.is_ok(), "concurrent changes timed out");

    let docs = service.inner().documents.read().await;
    assert_eq!(
        docs.len(),
        1,
        "exactly 1 entry should be in cache for the document, found {}",
        docs.len()
    );
    assert!(
        docs.contains_key(&uri),
        "the URI should still be present in the cache"
    );
}

/// Stress test: concurrent config change and generation bump must not
/// corrupt atomic state or panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_config_change_during_active_validation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Open 5 SKILL.md documents
    for i in 0..5 {
        let dir = temp_dir.path().join(format!("skill-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        std::fs::write(
            &path,
            format!(
                "---\nname: active-skill-{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Active Skill {i}\n"
            ),
        )
        .unwrap();

        let uri = Url::from_file_path(&path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&path).unwrap(),
                },
            })
            .await;
    }

    let backend_a = service.inner().clone();
    let backend_b = service.inner().clone();

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Task A: fire a config change (which increments config_generation and revalidates)
        let task_a = tokio::spawn(async move {
            backend_a
                .did_change_configuration(DidChangeConfigurationParams {
                    settings: serde_json::json!({ "severity": "Warning" }),
                })
                .await;
        });

        // Task B: concurrently bump config_generation to a high value
        let task_b = tokio::spawn(async move {
            backend_b
                .config_generation
                .fetch_add(9_999, Ordering::SeqCst);
        });

        task_a.await.expect("config change task should not panic");
        task_b.await.expect("generation bump task should not panic");
    })
    .await;

    assert!(result.is_ok(), "concurrent config change timed out");

    // Task A does fetch_add(1) (config_generation 0→1) and Task B does
    // fetch_add(9_999). Both tasks always run to completion before the assertion,
    // so the total is always 0 + 1 + 9_999 = 10_000 regardless of ordering.
    let generation = service.inner().config_generation.load(Ordering::SeqCst);
    assert_eq!(
        generation, 10_000,
        "config_generation should be 10000 (1 from config change + 9999 from bump), got {}",
        generation
    );

    let open_docs = service.inner().documents.read().await.len();
    assert_eq!(
        open_docs, 5,
        "all 5 documents should still be in cache, found {}",
        open_docs
    );
}

/// Stress test: concurrent project validation and per-file validation
/// must not interfere with each other.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_concurrent_project_and_file_validation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();

    // Create 2 AGENTS.md files to trigger AGM-006
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root AGENTS").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub AGENTS").unwrap();

    // Create 5 SKILL.md files
    for i in 0..5 {
        let dir = temp_dir.path().join(format!("skill-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!(
                "---\nname: project-skill-{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Project Skill {i}\n"
            ),
        )
        .unwrap();
    }

    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Open all 7 files: 2 AGENTS.md + 5 SKILL.md
    for path in [temp_dir.path().join("AGENTS.md"), sub.join("AGENTS.md")] {
        let uri = Url::from_file_path(&path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&path).unwrap(),
                },
            })
            .await;
    }
    for i in 0..5 {
        let path = temp_dir.path().join(format!("skill-{i}")).join("SKILL.md");
        let uri = Url::from_file_path(&path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&path).unwrap(),
                },
            })
            .await;
    }

    // Wait for the background project validation spawned by initialize() to
    // complete BEFORE starting the concurrent workload. This ensures the
    // explicit validate_project_rules_and_publish() call below is never
    // stale-dropped, making the post-run assertion deterministic.
    let sync_result = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            {
                let proj_diags = service.inner().project_level_diagnostics.read().await;
                if !proj_diags.is_empty() {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(
        sync_result.is_ok(),
        "initialize() background project validation did not complete within 10s"
    );

    let backend_project = service.inner().clone();

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut handles = Vec::new();

        // 1 task: project-level validation
        handles.push(tokio::spawn(async move {
            backend_project.validate_project_rules_and_publish().await;
        }));

        // 5 tasks: concurrent did_change on SKILL files
        for i in 0..5 {
            let backend = service.inner().clone();
            let path = temp_dir
                .path()
                .join(format!("skill-{i}"))
                .join("SKILL.md");
            let uri = Url::from_file_path(&path).unwrap();
            handles.push(tokio::spawn(async move {
                backend
                    .did_change(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri,
                            version: 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: format!(
                                "---\nname: updated-skill-{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Updated {i}\n"
                            ),
                        }],
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "concurrent project and file validation timed out"
    );

    // Because we waited for initialize()'s background validation to complete
    // before the concurrent block, the explicit validate_project_rules_and_publish
    // call above will not be stale-dropped. Assert diagnostics directly.
    let proj_diags = service.inner().project_level_diagnostics.read().await;
    assert!(
        !proj_diags.is_empty(),
        "project_level_diagnostics should be non-empty (AGM-006 from duplicate AGENTS.md)"
    );
    drop(proj_diags);

    // All 7 documents should still be in cache
    let open_docs = service.inner().documents.read().await.len();
    assert_eq!(
        open_docs, 7,
        "all 7 documents should still be in cache, found {}",
        open_docs
    );
}

/// Stress test: revalidation of many open documents after a single config
/// change must complete without panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_high_document_count_revalidation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let doc_count = 20usize;

    // Open 20 SKILL.md documents
    for i in 0..doc_count {
        let dir = temp_dir.path().join(format!("skill-{i}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("SKILL.md");
        std::fs::write(
            &path,
            format!(
                "---\nname: high-count-skill-{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# High Count {i}\n"
            ),
        )
        .unwrap();

        let uri = Url::from_file_path(&path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&path).unwrap(),
                },
            })
            .await;
    }

    // Single config change triggers revalidation of all open documents.
    // Wrapped in a timeout to catch deadlocks in for_each_bounded under load.
    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        service
            .inner()
            .did_change_configuration(DidChangeConfigurationParams {
                settings: serde_json::json!({ "severity": "Error" }),
            })
            .await;
    })
    .await;
    assert!(
        result.is_ok(),
        "high document count revalidation timed out after 30s"
    );

    let generation = service.inner().config_generation.load(Ordering::SeqCst);
    assert_eq!(
        generation, 1,
        "config_generation should be 1 after single config change, got {}",
        generation
    );

    let open_docs = service.inner().documents.read().await.len();
    assert_eq!(
        open_docs, doc_count,
        "all {} documents should still be in cache, found {}",
        doc_count, open_docs
    );
}

/// Stress test: concurrent hover requests during active validation must
/// not panic or deadlock.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_concurrent_hover_during_validation() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();
    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    let skill_path = temp_dir.path().join("SKILL.md");
    let content = "---\nname: hover-skill\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Hover Skill\n";
    std::fs::write(&skill_path, content).unwrap();

    let uri = Url::from_file_path(&skill_path).unwrap();

    // Open the document with frontmatter content
    service
        .inner()
        .did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: content.to_string(),
            },
        })
        .await;

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut handles = Vec::new();

        // 10 tasks: concurrent did_change
        for i in 0..10 {
            let backend = service.inner().clone();
            let uri = uri.clone();
            handles.push(tokio::spawn(async move {
                backend
                    .did_change(DidChangeTextDocumentParams {
                        text_document: VersionedTextDocumentIdentifier {
                            uri,
                            version: i + 2,
                        },
                        content_changes: vec![TextDocumentContentChangeEvent {
                            range: None,
                            range_length: None,
                            text: format!(
                                "---\nname: hover-v{i}\nversion: 1.0.0\nmodel: sonnet\n---\n\n# Hover V{i}\n"
                            ),
                        }],
                    })
                    .await;
            }));
        }

        // 10 tasks: concurrent hover at (1, 0) - the "name" key in frontmatter
        for _ in 0..10 {
            let backend = service.inner().clone();
            let uri = uri.clone();
            handles.push(tokio::spawn(async move {
                let _ = backend
                    .hover(HoverParams {
                        text_document_position_params: TextDocumentPositionParams {
                            text_document: TextDocumentIdentifier { uri },
                            position: Position {
                                line: 1,
                                character: 0,
                            },
                        },
                        work_done_progress_params: WorkDoneProgressParams::default(),
                    })
                    .await;
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }
    })
    .await;

    assert!(
        result.is_ok(),
        "concurrent hover during validation timed out"
    );

    let docs = service.inner().documents.read().await;
    assert!(
        docs.contains_key(&uri),
        "document should still be in cache after concurrent hover and changes"
    );
}

/// Stress test: 10 concurrent project validation runs must not corrupt
/// the generation counter or panic.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_rapid_project_validation_generation_guard() {
    let (service, _socket) = LspService::new(Backend::new);

    let temp_dir = tempfile::tempdir().unwrap();

    // Create 2 AGENTS.md files to trigger AGM-006
    std::fs::write(temp_dir.path().join("AGENTS.md"), "# Root AGENTS").unwrap();
    let sub = temp_dir.path().join("sub");
    std::fs::create_dir(&sub).unwrap();
    std::fs::write(sub.join("AGENTS.md"), "# Sub AGENTS").unwrap();

    let root_uri = Url::from_file_path(temp_dir.path()).unwrap();
    service
        .inner()
        .initialize(InitializeParams {
            root_uri: Some(root_uri),
            ..Default::default()
        })
        .await
        .unwrap();

    // Wait for the initial background project validation to complete before
    // spawning concurrent runs. Assert the wait succeeded so a stalled
    // background task causes an explicit failure rather than a silent race.
    let init_sync = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            {
                let proj_diags = service.inner().project_level_diagnostics.read().await;
                if !proj_diags.is_empty() {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await;
    assert!(
        init_sync.is_ok(),
        "initialize() background project validation did not complete within 10s"
    );

    // Open both AGENTS.md files
    for path in [temp_dir.path().join("AGENTS.md"), sub.join("AGENTS.md")] {
        let uri = Url::from_file_path(&path).unwrap();
        service
            .inner()
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "markdown".to_string(),
                    version: 1,
                    text: std::fs::read_to_string(&path).unwrap(),
                },
            })
            .await;
    }

    let validation_count = 10usize;

    let result = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        let mut handles = Vec::new();

        for _ in 0..validation_count {
            let backend = service.inner().clone();
            handles.push(tokio::spawn(async move {
                backend.validate_project_rules_and_publish().await;
            }));
        }

        for handle in handles {
            handle.await.expect("task should not panic");
        }
    })
    .await;

    assert!(result.is_ok(), "rapid project validation timed out");

    // Each call to validate_project_rules_and_publish does fetch_add(1),
    // plus the initial background run from initialize(). The generation
    // should be at least validation_count (10) but could be higher due to
    // the initial background run.
    let generation = service
        .inner()
        .project_validation_generation
        .load(Ordering::SeqCst);
    assert!(
        generation >= validation_count as u64,
        "project_validation_generation should be >= {}, got {}",
        validation_count,
        generation
    );
}

// ===== Document Version Tracking Tests =====

/// Test that document version is tracked when a document is opened.
#[tokio::test]
async fn test_document_version_tracked_on_open() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test").unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Test".to_string(),
            },
        })
        .await;

    let version = backend.get_document_version(&uri).await;
    assert_eq!(version, Some(1));
}

/// Test that document version is updated when a document changes.
#[tokio::test]
async fn test_document_version_updated_on_change() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test").unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Test".to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(1));

    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "# Updated".to_string(),
            }],
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(2));
}

/// Test that document version is cleared when a document is closed.
#[tokio::test]
async fn test_document_version_cleared_on_close() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test").unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Test".to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(1));

    backend
        .handle_did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, None);
}

/// Test that get_document_version returns None for a URI that was never opened.
#[tokio::test]
async fn test_document_version_returns_none_for_unknown_uri() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let never_opened = temp_dir.path().join("never-opened.md");
    let uri = Url::from_file_path(&never_opened).unwrap();
    assert_eq!(backend.get_document_version(&uri).await, None);
}

/// Test that the version is updated even when content_changes is empty.
///
/// Per LSP spec, VersionedTextDocumentIdentifier.version is the authoritative
/// post-change version regardless of content. The version must always be stored
/// so that published diagnostics carry the correct version tag.
#[tokio::test]
async fn test_document_version_updated_even_on_empty_content_changes() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(&skill_path, "# Test").unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    // Open with version 1
    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "# Test".to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(1));

    // Send did_change with version 2 but an empty content_changes vec
    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![],
        })
        .await;

    // Version should be 2 because the version from VersionedTextDocumentIdentifier
    // is always authoritative per LSP spec.
    assert_eq!(backend.get_document_version(&uri).await, Some(2));
}

/// Test that multiple documents track independent versions.
#[tokio::test]
async fn test_multiple_documents_track_independent_versions() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();

    let path_a = temp_dir.path().join("a").join("SKILL.md");
    let path_b = temp_dir.path().join("b").join("SKILL.md");
    std::fs::create_dir_all(path_a.parent().unwrap()).unwrap();
    std::fs::create_dir_all(path_b.parent().unwrap()).unwrap();
    std::fs::write(&path_a, "# A").unwrap();
    std::fs::write(&path_b, "# B").unwrap();

    let uri_a = Url::from_file_path(&path_a).unwrap();
    let uri_b = Url::from_file_path(&path_b).unwrap();

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri_a.clone(),
                language_id: "markdown".to_string(),
                version: 5,
                text: "# A".to_string(),
            },
        })
        .await;

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri_b.clone(),
                language_id: "markdown".to_string(),
                version: 10,
                text: "# B".to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri_a).await, Some(5));
    assert_eq!(backend.get_document_version(&uri_b).await, Some(10));

    // Update only document A
    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri_a.clone(),
                version: 6,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "# A updated".to_string(),
            }],
        })
        .await;

    assert_eq!(backend.get_document_version(&uri_a).await, Some(6));
    assert_eq!(backend.get_document_version(&uri_b).await, Some(10));
}

/// Integration-style test: open, change, and close a document and verify
/// the version state tracks correctly through the full lifecycle.
#[tokio::test]
async fn test_document_version_lifecycle_through_events() {
    let backend = Backend::new_test();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: lifecycle\nversion: 1.0.0\nmodel: sonnet\n---\n# Lifecycle Test\n",
    )
    .unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    // Phase 1: Not opened yet - no version tracked
    assert_eq!(backend.get_document_version(&uri).await, None);

    // Phase 2: Open with version 1
    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text:
                    "---\nname: lifecycle\nversion: 1.0.0\nmodel: sonnet\n---\n# Lifecycle Test\n"
                        .to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(1));

    // Phase 3: Change to version 2
    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "---\nname: lifecycle\nversion: 2.0.0\nmodel: sonnet\n---\n# Lifecycle Test v2\n".to_string(),
            }],
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(2));

    // Phase 4: Another change to version 5 (versions may skip)
    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 5,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "---\nname: lifecycle\nversion: 3.0.0\nmodel: sonnet\n---\n# Lifecycle Test v3\n".to_string(),
            }],
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(5));

    // Phase 5: Close the document
    backend
        .handle_did_close(DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, None);

    // Phase 6: Re-open with a new version - simulates client re-opening
    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: lifecycle\nversion: 1.0.0\nmodel: sonnet\n---\n# Lifecycle Reopened\n".to_string(),
            },
        })
        .await;

    assert_eq!(backend.get_document_version(&uri).await, Some(1));
}

/// Oversized didOpen payloads must be rejected without caching any state.
/// Prevents memory exhaustion from malicious or pathologically large documents.
#[tokio::test]
async fn test_oversized_did_open_is_rejected() {
    use agnix_core::MAX_LSP_DOCUMENT_BYTES;

    let (service, _socket) = LspService::new(Backend::new);
    let backend = service.inner();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("huge.md");
    let uri = Url::from_file_path(&skill_path).unwrap();

    // One byte past the limit is enough - we don't need to actually write the
    // file, since the size check is purely on the in-memory string.
    let oversized = "x".repeat(MAX_LSP_DOCUMENT_BYTES + 1);

    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: oversized,
            },
        })
        .await;

    // Nothing should be cached for the rejected document.
    assert!(backend.get_document_content(&uri).await.is_none());
    assert_eq!(backend.get_document_version(&uri).await, None);
}

/// Oversized didChange payloads must be rejected and clear any prior state.
#[tokio::test]
async fn test_oversized_did_change_is_rejected() {
    use agnix_core::MAX_LSP_DOCUMENT_BYTES;

    let (service, _socket) = LspService::new(Backend::new);
    let backend = service.inner();

    let temp_dir = tempfile::tempdir().unwrap();
    let skill_path = temp_dir.path().join("grows.md");
    std::fs::write(&skill_path, "---\nname: grows\n---\n").unwrap();
    let uri = Url::from_file_path(&skill_path).unwrap();

    // First open a small document (within the cap) so we have cached state.
    backend
        .handle_did_open(DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "markdown".to_string(),
                version: 1,
                text: "---\nname: grows\n---\n".to_string(),
            },
        })
        .await;
    assert_eq!(backend.get_document_version(&uri).await, Some(1));

    // Then push an oversized change; cached state should be dropped.
    let oversized = "y".repeat(MAX_LSP_DOCUMENT_BYTES + 1);
    backend
        .handle_did_change(DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: oversized,
            }],
        })
        .await;

    assert!(backend.get_document_content(&uri).await.is_none());
    assert_eq!(backend.get_document_version(&uri).await, None);
}
