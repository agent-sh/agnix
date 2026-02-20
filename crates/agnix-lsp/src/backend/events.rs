use agnix_core::normalize_line_endings;

use super::*;

impl Backend {
    pub(crate) async fn handle_did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        // Normalize CRLF so the cached content matches the LF-relative byte offsets
        // produced by validate_content and used by code actions for fix ranges.
        // Match on the Cow to reuse the original String for LF-only documents.
        let raw = params.text_document.text;
        let text = match normalize_line_endings(&raw) {
            std::borrow::Cow::Borrowed(_) => raw,
            std::borrow::Cow::Owned(normalized) => normalized,
        };
        {
            let mut docs = self.documents.write().await;
            docs.insert(uri.clone(), Arc::new(text));
        }
        self.validate_from_content_and_publish(uri, None).await;
    }

    pub(crate) async fn handle_did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().next() {
            // Normalize CRLF so the cached content matches the LF-relative byte offsets
            // produced by validate_content and used by code actions for fix ranges.
            // Match on the Cow to reuse the original String for LF-only documents.
            let raw = change.text;
            let text = match normalize_line_endings(&raw) {
                std::borrow::Cow::Borrowed(_) => raw,
                std::borrow::Cow::Owned(normalized) => normalized,
            };
            {
                let mut docs = self.documents.write().await;
                docs.insert(uri.clone(), Arc::new(text));
            }
            self.validate_from_content_and_publish(uri, None).await;
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
        {
            let mut docs = self.documents.write().await;
            docs.remove(&params.text_document.uri);
        }
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }
}
