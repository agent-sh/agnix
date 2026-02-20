//! LSP backend implementation for agnix.
//!
//! Implements the Language Server Protocol using tower-lsp, providing
//! real-time validation of agent configuration files.

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use arc_swap::ArcSwap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};

use crate::code_actions::fixes_to_code_actions_with_diagnostic;
use crate::completion_provider::completion_items_for_document;
use crate::diagnostic_mapper::{deserialize_fixes, to_lsp_diagnostic, to_lsp_diagnostics};
use crate::hover_provider::hover_at_position;

pub(crate) mod events;
pub(crate) mod helpers;
pub(crate) mod revalidation;

use helpers::{create_error_diagnostic, normalize_path};
#[cfg(test)]
use revalidation::{
    MAX_CONFIG_REVALIDATION_CONCURRENCY, config_revalidation_concurrency, for_each_bounded,
};
/// LSP backend that handles validation requests.
///
/// The backend maintains a connection to the LSP client and validates
/// files on open, change, and save events. It also provides code actions
/// for quick fixes and hover documentation for configuration fields.
///
/// # Performance Notes
///
/// Both `LintConfig` and `ValidatorRegistry` are cached and reused across
/// validations to avoid repeated allocations.
#[derive(Clone)]
pub struct Backend {
    pub(crate) client: Client,
    /// Cached lint configuration reused across validations.
    /// Wrapped in ArcSwap for lock-free reads; initially loaded from .agnix.toml during initialize()
    /// and atomically updated on configuration changes (e.g., VS Code settings merges).
    pub(crate) config: Arc<ArcSwap<agnix_core::LintConfig>>,
    /// Workspace root path for boundary validation (security).
    /// Set during initialize() from the client's root_uri.
    pub(crate) workspace_root: Arc<RwLock<Option<PathBuf>>>,
    /// Canonicalized workspace root cached at initialize() to avoid blocking I/O on hot paths.
    pub(crate) workspace_root_canonical: Arc<RwLock<Option<PathBuf>>>,
    pub(crate) documents: Arc<RwLock<HashMap<Url, Arc<String>>>>,
    /// Monotonic generation incremented on each config change.
    /// Used to drop stale diagnostics from older revalidation batches.
    pub(crate) config_generation: Arc<AtomicU64>,
    /// Monotonic generation incremented on each project validation.
    /// Used to drop stale project-level diagnostics from slower validation runs.
    pub(crate) project_validation_generation: Arc<AtomicU64>,
    /// Cached validator registry reused across validations.
    /// Immutable after construction; Arc enables sharing across spawn_blocking tasks.
    pub(crate) registry: Arc<agnix_core::ValidatorRegistry>,
    /// Cached project-level diagnostics per URI (from validate_project_rules).
    /// Stored separately so they can be merged with per-file diagnostics at publish time.
    pub(crate) project_level_diagnostics: Arc<RwLock<HashMap<Url, Vec<Diagnostic>>>>,
    /// Tracks which URIs received project-level diagnostics so stale ones can be cleared.
    pub(crate) project_diagnostics_uris: Arc<RwLock<HashSet<Url>>>,
}

impl Backend {
    /// Create a new backend instance with the given client connection.
    pub fn new(client: Client) -> Self {
        Self {
            client,
            config: Arc::new(ArcSwap::from_pointee(agnix_core::LintConfig::default())),
            workspace_root: Arc::new(RwLock::new(None)),
            workspace_root_canonical: Arc::new(RwLock::new(None)),
            documents: Arc::new(RwLock::new(HashMap::new())),
            config_generation: Arc::new(AtomicU64::new(0)),
            project_validation_generation: Arc::new(AtomicU64::new(0)),
            registry: Arc::new(agnix_core::ValidatorRegistry::with_defaults()),
            project_level_diagnostics: Arc::new(RwLock::new(HashMap::new())),
            project_diagnostics_uris: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Spawn project-level validation in a background task.
    ///
    /// Logs a warning if the spawned task panics, preventing silent failures.
    pub(crate) fn spawn_project_validation(&self) {
        let backend = self.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let result = tokio::spawn(async move {
                backend.validate_project_rules_and_publish().await;
            })
            .await;
            if let Err(e) = result {
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("Project-level validation task panicked: {}", e),
                    )
                    .await;
            }
        });
    }

    /// Run validation on a file in a blocking task.
    ///
    /// agnix-core validation is CPU-bound and synchronous, so we run it
    /// in a blocking task to avoid blocking the async runtime.
    ///
    /// Both `LintConfig` and `ValidatorRegistry` are cloned from cached
    /// instances to avoid repeated allocations on each validation.
    pub(crate) async fn validate_file(&self, path: PathBuf) -> Vec<Diagnostic> {
        let config = self.config.load_full();
        let registry = Arc::clone(&self.registry);
        let result = tokio::task::spawn_blocking(move || {
            agnix_core::validate_file_with_registry(&path, &config, &registry)
        })
        .await;

        match result {
            Ok(Ok(outcome)) => to_lsp_diagnostics(outcome.into_diagnostics()),
            Ok(Err(e)) => vec![create_error_diagnostic(
                "agnix::validation-error",
                format!("Validation error: {}", e),
            )],
            Err(e) => vec![create_error_diagnostic(
                "agnix::internal-error",
                format!("Internal error: {}", e),
            )],
        }
    }

    /// Validate from cached content and publish diagnostics.
    ///
    /// Used for did_change events where we have the content in memory.
    /// This avoids reading from disk and provides real-time feedback.
    pub(crate) async fn validate_from_content_and_publish(
        &self,
        uri: Url,
        expected_config_generation: Option<u64>,
    ) {
        let file_path = match uri.to_file_path() {
            Ok(p) => p,
            Err(()) => {
                self.client
                    .log_message(MessageType::WARNING, format!("Invalid file URI: {}", uri))
                    .await;
                return;
            }
        };

        // Security: Validate file is within workspace boundaries
        if let Some(ref workspace_root) = *self.workspace_root.read().await {
            let (canonical_path, canonical_root) = match file_path.canonicalize() {
                Ok(path) => {
                    let root = self
                        .workspace_root_canonical
                        .read()
                        .await
                        .clone()
                        .unwrap_or_else(|| normalize_path(workspace_root));
                    (path, root)
                }
                Err(_) => (normalize_path(&file_path), normalize_path(workspace_root)),
            };

            if !canonical_path.starts_with(&canonical_root) {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("File outside workspace boundary: {}", uri),
                    )
                    .await;
                return;
            }
        }

        // Skip generic markdown files in LSP to avoid false positives on
        // developer docs, project specs, etc. Only validate files that are
        // specifically identified as agent configuration files.
        {
            let config = self.config.load();
            let file_type = agnix_core::resolve_file_type(&file_path, &config);
            if file_type.is_generic() {
                // Publish empty diagnostics to clear any stale results
                self.client.publish_diagnostics(uri, vec![], None).await;
                return;
            }
        }

        // Get content from cache
        let (content, expected_content) = {
            let docs = self.documents.read().await;
            match docs.get(&uri) {
                Some(cached) => {
                    let snapshot = Arc::clone(cached);
                    (Arc::clone(&snapshot), Some(snapshot))
                }
                None => {
                    // Fall back to file-based validation
                    drop(docs);
                    let diagnostics = self.validate_file(file_path).await;
                    if !self
                        .should_publish_diagnostics(&uri, expected_config_generation, None)
                        .await
                    {
                        return;
                    }
                    self.client
                        .publish_diagnostics(uri, diagnostics, None)
                        .await;
                    return;
                }
            }
        };

        let config = self.config.load_full();
        let registry = Arc::clone(&self.registry);
        let result = tokio::task::spawn_blocking(move || {
            Ok::<_, agnix_core::LintError>(agnix_core::validate_content(
                &file_path,
                content.as_str(),
                &config,
                &registry,
            ))
        })
        .await;

        let mut diagnostics = match result {
            Ok(Ok(diagnostics)) => to_lsp_diagnostics(diagnostics),
            Ok(Err(e)) => vec![create_error_diagnostic(
                "agnix::validation-error",
                format!("Validation error: {}", e),
            )],
            Err(e) => vec![create_error_diagnostic(
                "agnix::internal-error",
                format!("Internal error: {}", e),
            )],
        };

        // Merge cached project-level diagnostics for this URI (AGM-006, XP-004/005/006, VER-001)
        {
            let proj_diags = self.project_level_diagnostics.read().await;
            if let Some(project_diags) = proj_diags.get(&uri) {
                diagnostics.extend(project_diags.iter().cloned());
            }
        }

        if !self
            .should_publish_diagnostics(&uri, expected_config_generation, expected_content.as_ref())
            .await
        {
            return;
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[cfg(test)]
impl Backend {
    /// Creates a Backend with a disconnected client suitable for unit tests.
    /// The client has no transport - diagnostics published to it are silently dropped.
    pub(crate) fn new_test() -> Self {
        let (service, _socket) = tower_lsp::LspService::new(Backend::new);
        service.inner().clone()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // Capture workspace root for path boundary validation
        if let Some(root_uri) = params.root_uri {
            if let Ok(root_path) = root_uri.to_file_path() {
                *self.workspace_root.write().await = Some(root_path.clone());
                *self.workspace_root_canonical.write().await = Some(
                    root_path
                        .canonicalize()
                        .unwrap_or_else(|_| normalize_path(&root_path)),
                );

                // Try to load config from .agnix.toml in workspace root
                let config_path = root_path.join(".agnix.toml");
                if config_path.exists() {
                    match agnix_core::LintConfig::load(&config_path) {
                        Ok(loaded_config) => {
                            // Apply config-specified locale if present
                            if let Some(config_locale) = loaded_config.locale() {
                                crate::locale::init_from_config(config_locale);
                            }
                            let mut config_with_root = loaded_config;
                            config_with_root.set_root_dir(root_path.clone());
                            self.config.store(Arc::new(config_with_root));
                        }
                        Err(e) => {
                            // Log error but continue with default config
                            self.client
                                .log_message(
                                    MessageType::WARNING,
                                    format!("Failed to load .agnix.toml: {}", e),
                                )
                                .await;
                        }
                    }
                }
            }
        }

        // Start project-level validation early so diagnostics are available
        // as soon as files are opened. Runs asynchronously in the background.
        self.spawn_project_validation();

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                code_action_provider: Some(CodeActionProviderCapability::Options(
                    CodeActionOptions {
                        code_action_kinds: Some(vec![CodeActionKind::QUICKFIX]),
                        ..Default::default()
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![":".to_string(), "\"".to_string()]),
                    ..Default::default()
                }),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["agnix.validateProjectRules".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "agnix-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "agnix-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.handle_did_open(params).await;
    }
    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        self.handle_did_change(params).await;
    }
    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.handle_did_save(params).await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.handle_did_close(params).await;
    }
    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = &params.text_document.uri;

        // Get document content for byte-to-position conversion
        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        let mut actions = Vec::new();

        // Extract fixes from diagnostics that overlap with the request range
        for diag in &params.context.diagnostics {
            // Check if this diagnostic overlaps with the requested range
            let diag_range = &diag.range;
            let req_range = &params.range;

            let overlaps = diag_range.start.line <= req_range.end.line
                && diag_range.end.line >= req_range.start.line;

            if !overlaps {
                continue;
            }

            // Deserialize fixes from diagnostic.data
            let fixes = deserialize_fixes(diag.data.as_ref());
            if !fixes.is_empty() {
                actions.extend(fixes_to_code_actions_with_diagnostic(
                    uri,
                    &fixes,
                    content.as_str(),
                    diag,
                ));
            }
        }

        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                actions
                    .into_iter()
                    .map(CodeActionOrCommand::CodeAction)
                    .collect(),
            ))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        // Get document content
        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        let config = self.config.load();
        let file_type = uri
            .to_file_path()
            .ok()
            .map(|path| agnix_core::resolve_file_type(&path, &config))
            .unwrap_or(agnix_core::FileType::Unknown);
        if matches!(file_type, agnix_core::FileType::Unknown) || file_type.is_generic() {
            return Ok(None);
        }

        // Get hover info for the position
        Ok(hover_at_position(file_type, content.as_str(), position))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = &params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let path = match uri.to_file_path() {
            Ok(path) => path,
            Err(_) => return Ok(None),
        };

        let content = match self.get_document_content(uri).await {
            Some(c) => c,
            None => return Ok(None),
        };

        let config = self.config.load();
        let items = completion_items_for_document(&path, content.as_str(), position, &config);
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        self.handle_did_change_configuration(params).await;
    }
    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        match params.command.as_str() {
            "agnix.validateProjectRules" => {
                self.client
                    .log_message(
                        MessageType::INFO,
                        "Running project-level validation (via executeCommand)",
                    )
                    .await;
                self.validate_project_rules_and_publish().await;
                Ok(None)
            }
            _ => {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        format!("Unknown command: {}", params.command),
                    )
                    .await;
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests;
