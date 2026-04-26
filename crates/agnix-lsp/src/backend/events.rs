use agnix_core::{MAX_LSP_DOCUMENT_BYTES, normalize_line_endings};

use super::*;

impl Backend {
    /// Reject a document that exceeds [`MAX_LSP_DOCUMENT_BYTES`]: log a warning,
    /// drop any cached state for the URI, and publish a single diagnostic at
    /// the file head so the user sees *why* validation was skipped rather
    /// than just "no diagnostics".
    async fn reject_oversized_document(&self, uri: Url, size: usize, version: Option<i32>) {
        self.client
            .log_message(
                MessageType::WARNING,
                format!(
                    "agnix-lsp: skipping validation of {} ({} bytes > {} byte limit)",
                    uri, size, MAX_LSP_DOCUMENT_BYTES
                ),
            )
            .await;
        {
            let mut docs = self.documents.write().await;
            let mut versions = self.document_versions.write().await;
            docs.remove(&uri);
            versions.remove(&uri);
        }
        let diag = Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("agnix-lsp".to_string()),
            message: format!(
                "agnix-lsp skipped this document: {} bytes exceeds the {} byte limit.",
                size, MAX_LSP_DOCUMENT_BYTES
            ),
            ..Default::default()
        };
        self.client
            .publish_diagnostics(uri, vec![diag], version)
            .await;
    }

    pub(crate) async fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let version = params.text_document.version;
        let uri = params.text_document.uri;
        let raw = params.text_document.text;
        if raw.len() > MAX_LSP_DOCUMENT_BYTES {
            self.reject_oversized_document(uri, raw.len(), Some(version))
                .await;
            return;
        }
        // Normalize CRLF so the cached content matches the LF-relative byte offsets
        // produced by validate_content and used by code actions for fix ranges.
        // Match on the Cow to reuse the original String for LF-only documents.
        let text = match normalize_line_endings(&raw) {
            std::borrow::Cow::Borrowed(_) => raw,
            std::borrow::Cow::Owned(normalized) => normalized,
        };
        // Acquire both locks atomically to update content and version together.
        // Readers that need both values must capture them in a single operation
        // (see validate_from_content_and_publish).
        {
            let mut docs = self.documents.write().await;
            let mut versions = self.document_versions.write().await;
            docs.insert(uri.clone(), Arc::new(text));
            versions.insert(uri.clone(), version);
            // Both guards dropped here in reverse acquisition order (versions then docs)
        }
        self.validate_from_content_and_publish(uri, None).await;
    }

    pub(crate) async fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let version = params.text_document.version;
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            let raw = change.text;
            if raw.len() > MAX_LSP_DOCUMENT_BYTES {
                self.reject_oversized_document(uri, raw.len(), Some(version))
                    .await;
                return;
            }
            // Normalize CRLF so the cached content matches the LF-relative byte offsets
            // produced by validate_content and used by code actions for fix ranges.
            // Match on the Cow to reuse the original String for LF-only documents.
            let text = match normalize_line_endings(&raw) {
                std::borrow::Cow::Borrowed(_) => raw,
                std::borrow::Cow::Owned(normalized) => normalized,
            };
            // Acquire both locks atomically to update content and version together.
            // Readers that need both values must capture them in a single operation
            // (see validate_from_content_and_publish).
            {
                let mut docs = self.documents.write().await;
                let mut versions = self.document_versions.write().await;
                docs.insert(uri.clone(), Arc::new(text));
                versions.insert(uri.clone(), version);
                // Both guards dropped here in reverse acquisition order (versions then docs)
            }
            self.validate_from_content_and_publish(uri, None).await;
        } else {
            // Even when content_changes is empty, the version from
            // VersionedTextDocumentIdentifier is authoritative per LSP spec.
            self.document_versions.write().await.insert(uri, version);
        }
    }

    pub(crate) async fn handle_did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.validate_from_content_and_publish(uri.clone(), None)
            .await;

        // Re-run project-level validation when a relevant file is saved
        if let Ok(path) = uri.to_file_path() {
            if Self::is_project_level_trigger(&path) {
                self.spawn_project_validation();
            }
        }
    }

    pub(crate) async fn handle_did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        {
            let mut docs = self.documents.write().await;
            docs.remove(&uri);
        }
        self.document_versions.write().await.remove(&uri);
        // Clearing diagnostics for a closed document - version is intentionally None
        // since the document is no longer tracked.
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}
