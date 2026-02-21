//! Diagnostic types and error reporting for lint results

#[cfg(feature = "filesystem")]
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

pub type LintResult<T> = Result<T, LintError>;

/// An automatic fix for a diagnostic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    /// Byte offset start (inclusive)
    pub start_byte: usize,
    /// Byte offset end (exclusive)
    pub end_byte: usize,
    /// Text to insert/replace with
    pub replacement: String,
    /// Human-readable description of what this fix does
    pub description: String,
    /// Legacy safety flag retained for backwards compatibility.
    /// New code should prefer `confidence`, `is_safe()`, and `confidence_tier()`.
    pub safe: bool,
    /// Confidence score (0.0 to 1.0).
    ///
    /// - HIGH: >= 0.95
    /// - MEDIUM: >= 0.75 and < 0.95
    /// - LOW: < 0.75
    ///
    /// When this is `None` (legacy serialized payloads), confidence is inferred
    /// from `safe` for compatibility.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    /// Optional group key. Fixes in the same group are treated as alternatives.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    /// Optional dependency key (group or description) required before applying this fix.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub depends_on: Option<String>,
}

pub const FIX_CONFIDENCE_HIGH_THRESHOLD: f32 = 0.95;
pub const FIX_CONFIDENCE_MEDIUM_THRESHOLD: f32 = 0.75;
const LEGACY_UNSAFE_CONFIDENCE: f32 = 0.80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FixConfidenceTier {
    High,
    Medium,
    Low,
}

impl Fix {
    /// Create a replacement fix
    pub fn replace(
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        debug_assert!(
            start <= end,
            "Fix::replace: start_byte ({start}) must be <= end_byte ({end})"
        );
        let confidence = if safe { 1.0 } else { LEGACY_UNSAFE_CONFIDENCE };
        Self {
            start_byte: start,
            end_byte: end,
            replacement: replacement.into(),
            description: description.into(),
            safe,
            confidence: Some(confidence),
            group: None,
            depends_on: None,
        }
    }

    /// Create a replacement fix with explicit confidence.
    pub fn replace_with_confidence(
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        debug_assert!(
            start <= end,
            "Fix::replace_with_confidence: start_byte ({start}) must be <= end_byte ({end})"
        );
        Self {
            start_byte: start,
            end_byte: end,
            replacement: replacement.into(),
            description: description.into(),
            safe: confidence >= FIX_CONFIDENCE_HIGH_THRESHOLD,
            confidence: Some(clamp_confidence(confidence)),
            group: None,
            depends_on: None,
        }
    }

    /// Create an insertion fix (start == end)
    pub fn insert(
        position: usize,
        text: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        let confidence = if safe { 1.0 } else { LEGACY_UNSAFE_CONFIDENCE };
        Self {
            start_byte: position,
            end_byte: position,
            replacement: text.into(),
            description: description.into(),
            safe,
            confidence: Some(confidence),
            group: None,
            depends_on: None,
        }
    }

    /// Create an insertion fix with explicit confidence.
    pub fn insert_with_confidence(
        position: usize,
        text: impl Into<String>,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self {
            start_byte: position,
            end_byte: position,
            replacement: text.into(),
            description: description.into(),
            safe: confidence >= FIX_CONFIDENCE_HIGH_THRESHOLD,
            confidence: Some(clamp_confidence(confidence)),
            group: None,
            depends_on: None,
        }
    }

    /// Create a deletion fix (replacement is empty)
    pub fn delete(start: usize, end: usize, description: impl Into<String>, safe: bool) -> Self {
        debug_assert!(
            start <= end,
            "Fix::delete: start_byte ({start}) must be <= end_byte ({end})"
        );
        let confidence = if safe { 1.0 } else { LEGACY_UNSAFE_CONFIDENCE };
        Self {
            start_byte: start,
            end_byte: end,
            replacement: String::new(),
            description: description.into(),
            safe,
            confidence: Some(confidence),
            group: None,
            depends_on: None,
        }
    }

    /// Create a deletion fix with explicit confidence.
    pub fn delete_with_confidence(
        start: usize,
        end: usize,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        debug_assert!(
            start <= end,
            "Fix::delete_with_confidence: start_byte ({start}) must be <= end_byte ({end})"
        );
        Self {
            start_byte: start,
            end_byte: end,
            replacement: String::new(),
            description: description.into(),
            safe: confidence >= FIX_CONFIDENCE_HIGH_THRESHOLD,
            confidence: Some(clamp_confidence(confidence)),
            group: None,
            depends_on: None,
        }
    }

    /// Internal helper for debug-only validation of byte ranges and UTF-8 char boundaries.
    ///
    /// Used by `_checked` constructors to keep assertions and error messages consistent.
    /// No-op in release builds since it only contains `debug_assert!` calls.
    fn debug_assert_valid_range(content: &str, start: usize, end: usize, context: &'static str) {
        debug_assert!(
            start <= end,
            "{context}: start_byte ({start}) must be <= end_byte ({end})"
        );
        debug_assert!(
            start <= content.len(),
            "{context}: start_byte ({start}) is out of bounds (len={})",
            content.len()
        );
        debug_assert!(
            content.is_char_boundary(start),
            "{context}: start_byte ({start}) is not on a UTF-8 char boundary"
        );
        debug_assert!(
            end <= content.len(),
            "{context}: end_byte ({end}) is out of bounds (len={})",
            content.len()
        );
        debug_assert!(
            content.is_char_boundary(end),
            "{context}: end_byte ({end}) is not on a UTF-8 char boundary"
        );
    }

    /// Internal helper for debug-only validation of a single byte position and UTF-8 char boundary.
    ///
    /// Used by insert `_checked` constructors. No-op in release builds.
    fn debug_assert_valid_position(content: &str, position: usize, context: &'static str) {
        debug_assert!(
            position <= content.len(),
            "{context}: position ({position}) is out of bounds (len={})",
            content.len()
        );
        debug_assert!(
            content.is_char_boundary(position),
            "{context}: position ({position}) is not on a UTF-8 char boundary"
        );
    }

    /// Create a replacement fix, asserting UTF-8 char boundary alignment in debug builds.
    ///
    /// Validates that both `start` and `end` land on UTF-8 char boundaries in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::replace`] when `content` is not available.
    pub fn replace_checked(
        content: &str,
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        Self::debug_assert_valid_range(content, start, end, "Fix::replace_checked");
        Self::replace(start, end, replacement, description, safe)
    }

    /// Create a replacement fix with explicit confidence, asserting UTF-8 char boundary
    /// alignment in debug builds.
    ///
    /// Validates that both `start` and `end` land on UTF-8 char boundaries in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::replace_with_confidence`] when
    /// `content` is not available.
    pub fn replace_with_confidence_checked(
        content: &str,
        start: usize,
        end: usize,
        replacement: impl Into<String>,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self::debug_assert_valid_range(content, start, end, "Fix::replace_with_confidence_checked");
        Self::replace_with_confidence(start, end, replacement, description, confidence)
    }

    /// Create an insertion fix, asserting UTF-8 char boundary alignment in debug builds.
    ///
    /// Validates that `position` lands on a UTF-8 char boundary in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::insert`] when `content` is not available.
    pub fn insert_checked(
        content: &str,
        position: usize,
        text: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        Self::debug_assert_valid_position(content, position, "Fix::insert_checked");
        Self::insert(position, text, description, safe)
    }

    /// Create an insertion fix with explicit confidence, asserting UTF-8 char boundary
    /// alignment in debug builds.
    ///
    /// Validates that `position` lands on a UTF-8 char boundary in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::insert_with_confidence`] when
    /// `content` is not available.
    pub fn insert_with_confidence_checked(
        content: &str,
        position: usize,
        text: impl Into<String>,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self::debug_assert_valid_position(content, position, "Fix::insert_with_confidence_checked");
        Self::insert_with_confidence(position, text, description, confidence)
    }

    /// Create a deletion fix, asserting UTF-8 char boundary alignment in debug builds.
    ///
    /// Validates that both `start` and `end` land on UTF-8 char boundaries in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::delete`] when `content` is not available.
    pub fn delete_checked(
        content: &str,
        start: usize,
        end: usize,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        Self::debug_assert_valid_range(content, start, end, "Fix::delete_checked");
        Self::delete(start, end, description, safe)
    }

    /// Create a deletion fix with explicit confidence, asserting UTF-8 char boundary
    /// alignment in debug builds.
    ///
    /// Validates that both `start` and `end` land on UTF-8 char boundaries in `content`.
    /// These checks are no-ops in release builds; the function otherwise behaves identically to its unchecked counterpart. Use [`Self::delete_with_confidence`] when
    /// `content` is not available.
    pub fn delete_with_confidence_checked(
        content: &str,
        start: usize,
        end: usize,
        description: impl Into<String>,
        confidence: f32,
    ) -> Self {
        Self::debug_assert_valid_range(content, start, end, "Fix::delete_with_confidence_checked");
        Self::delete_with_confidence(start, end, description, confidence)
    }

    /// Override confidence for this fix and sync legacy `safe`.
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        let clamped = clamp_confidence(confidence);
        self.confidence = Some(clamped);
        self.safe = clamped >= FIX_CONFIDENCE_HIGH_THRESHOLD;
        self
    }

    /// Mark this fix as part of an alternatives group.
    pub fn with_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    /// Set a dependency key (group or description) that must be applied first.
    pub fn with_dependency(mut self, depends_on: impl Into<String>) -> Self {
        self.depends_on = Some(depends_on.into());
        self
    }

    /// Resolve confidence score with legacy fallback.
    pub fn confidence_score(&self) -> f32 {
        self.confidence.unwrap_or({
            if self.safe {
                1.0
            } else {
                LEGACY_UNSAFE_CONFIDENCE
            }
        })
    }

    /// Derived safety check based on confidence threshold.
    pub fn is_safe(&self) -> bool {
        self.confidence_score() >= FIX_CONFIDENCE_HIGH_THRESHOLD
    }

    /// Confidence tier used for certainty filtering.
    pub fn confidence_tier(&self) -> FixConfidenceTier {
        let confidence = self.confidence_score();
        if confidence >= FIX_CONFIDENCE_HIGH_THRESHOLD {
            FixConfidenceTier::High
        } else if confidence >= FIX_CONFIDENCE_MEDIUM_THRESHOLD {
            FixConfidenceTier::Medium
        } else {
            FixConfidenceTier::Low
        }
    }

    /// Check if this is an insertion (start == end)
    pub fn is_insertion(&self) -> bool {
        self.start_byte == self.end_byte && !self.replacement.is_empty()
    }

    /// Check if this is a deletion (empty replacement)
    pub fn is_deletion(&self) -> bool {
        self.replacement.is_empty() && self.start_byte < self.end_byte
    }
}

impl PartialEq for Fix {
    fn eq(&self, other: &Self) -> bool {
        self.start_byte == other.start_byte
            && self.end_byte == other.end_byte
            && self.replacement == other.replacement
            && self.description == other.description
            && self.safe == other.safe
            && confidence_option_eq(self.confidence, other.confidence)
            && self.group == other.group
            && self.depends_on == other.depends_on
    }
}

impl Eq for Fix {}

fn clamp_confidence(confidence: f32) -> f32 {
    confidence.clamp(0.0, 1.0)
}

fn confidence_option_eq(a: Option<f32>, b: Option<f32>) -> bool {
    match (a, b) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        (None, None) => true,
        _ => false,
    }
}

/// Structured metadata about the rule that triggered a diagnostic.
///
/// Populated automatically from `agnix-rules` build-time data when using
/// the `Diagnostic::error()`, `warning()`, or `info()` constructors, or
/// manually via `Diagnostic::with_metadata()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleMetadata {
    /// Rule category (e.g., "agent-skills", "claude-code-hooks").
    pub category: String,
    /// Rule severity from the rules catalog (e.g., "HIGH", "MEDIUM", "LOW").
    pub severity: String,
    /// Tool this rule specifically applies to (e.g., "claude-code", "cursor").
    /// `None` for generic rules that apply to all tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub applies_to_tool: Option<String>,
}

/// A diagnostic message from the linter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub level: DiagnosticLevel,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub rule: String,
    pub suggestion: Option<String>,
    /// Automatic fixes for this diagnostic
    #[serde(default)]
    pub fixes: Vec<Fix>,
    /// Assumption note for version-aware validation
    ///
    /// When tool/spec versions are not pinned, validators may use default
    /// assumptions. This field documents those assumptions to help users
    /// understand what behavior is expected and how to get version-specific
    /// validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assumption: Option<String>,
    /// Structured metadata about the rule (category, severity, tool).
    ///
    /// Auto-populated from `agnix-rules` at construction time when using the
    /// `error()`, `warning()`, or `info()` constructors. Can also be set
    /// manually via `with_metadata()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<RuleMetadata>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DiagnosticLevel {
    Error,
    Warning,
    Info,
}

/// Build a `RuleMetadata` from the compile-time rules catalog.
fn lookup_rule_metadata(rule_id: &str) -> Option<RuleMetadata> {
    agnix_rules::get_rule_metadata(rule_id).map(|(category, severity, tool)| RuleMetadata {
        category: category.to_string(),
        severity: severity.to_string(),
        applies_to_tool: (!tool.is_empty()).then_some(tool.to_string()),
    })
}

impl Diagnostic {
    pub fn error(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
        Self {
            level: DiagnosticLevel::Error,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
            metadata,
        }
    }

    pub fn warning(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
        Self {
            level: DiagnosticLevel::Warning,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
            metadata,
        }
    }

    pub fn info(
        file: PathBuf,
        line: usize,
        column: usize,
        rule: &str,
        message: impl Into<String>,
    ) -> Self {
        let metadata = lookup_rule_metadata(rule);
        Self {
            level: DiagnosticLevel::Info,
            message: message.into(),
            file,
            line,
            column,
            rule: rule.to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
            metadata,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Add an assumption note for version-aware validation
    ///
    /// Used when tool/spec versions are not pinned to document what
    /// default behavior the validator is assuming.
    pub fn with_assumption(mut self, assumption: impl Into<String>) -> Self {
        self.assumption = Some(assumption.into());
        self
    }

    /// Add an automatic fix to this diagnostic
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }

    /// Add multiple automatic fixes to this diagnostic
    pub fn with_fixes(mut self, fixes: impl IntoIterator<Item = Fix>) -> Self {
        self.fixes.extend(fixes);
        self
    }

    /// Set structured rule metadata on this diagnostic
    pub fn with_metadata(mut self, metadata: RuleMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Check if this diagnostic has any fixes available
    pub fn has_fixes(&self) -> bool {
        !self.fixes.is_empty()
    }

    /// Check if this diagnostic has any safe fixes available
    pub fn has_safe_fixes(&self) -> bool {
        self.fixes.iter().any(Fix::is_safe)
    }
}

/// File operation errors
#[derive(Error, Debug)]
pub enum FileError {
    #[error("Failed to read file: {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Failed to write file: {path}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Refusing to read symlink: {path}")]
    Symlink { path: PathBuf },

    #[error("File too large: {path} ({size} bytes, limit {limit} bytes)")]
    TooBig {
        path: PathBuf,
        size: u64,
        limit: u64,
    },

    #[error("Not a regular file: {path}")]
    NotRegular { path: PathBuf },
}

/// Validation errors
#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Too many files to validate: {count} files found, limit is {limit}")]
    TooManyFiles { count: usize, limit: usize },

    #[error("Validation root not found: {path}")]
    RootNotFound { path: PathBuf },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Invalid exclude pattern: {pattern} ({message})")]
    InvalidExcludePattern { pattern: String, message: String },

    #[error("Failed to parse configuration")]
    ParseError(#[from] anyhow::Error),
}

/// Core error type hierarchy
#[derive(Error, Debug)]
pub enum CoreError {
    #[error(transparent)]
    File(#[from] FileError),

    #[error(transparent)]
    Validation(#[from] ValidationError),

    #[error(transparent)]
    Config(#[from] ConfigError),
}

impl CoreError {
    /// Extract file-level errors from this error.
    ///
    /// Returns a vector containing the FileError if this is a File variant,
    /// or an empty vector for other error types.
    pub fn source_diagnostics(&self) -> Vec<&FileError> {
        match self {
            CoreError::File(e) => vec![e],
            _ => vec![],
        }
    }

    /// Get the path associated with this error, if any.
    pub fn path(&self) -> Option<&PathBuf> {
        match self {
            CoreError::File(FileError::Read { path, .. })
            | CoreError::File(FileError::Write { path, .. })
            | CoreError::File(FileError::Symlink { path })
            | CoreError::File(FileError::TooBig { path, .. })
            | CoreError::File(FileError::NotRegular { path }) => Some(path),
            CoreError::Validation(ValidationError::RootNotFound { path }) => Some(path),
            _ => None,
        }
    }
}

/// `LintError` is the canonical public name for [`CoreError`].
///
/// Both names are re-exported at the crate root. Internal code constructs
/// variants via `CoreError`; public API surfaces and function signatures
/// use `LintError` and [`LintResult`].
pub type LintError = CoreError;

/// Outcome of validating a single file.
///
/// Returned by [`crate::validate_file`] and [`crate::validate_file_with_registry`].
/// The `Ok` path of `LintResult<ValidationOutcome>` covers all per-file
/// situations - successful validation, I/O errors, and skipped files - while
/// the `Err` path is reserved for config-level errors (e.g. invalid exclude
/// patterns, too many files).
///
/// Use [`into_diagnostics()`](ValidationOutcome::into_diagnostics) for a
/// quick migration path from the previous `Vec<Diagnostic>` return type.
#[derive(Debug)]
#[non_exhaustive]
pub enum ValidationOutcome {
    /// Validation ran successfully. The contained diagnostics may be empty
    /// (no issues found) or non-empty (issues found).
    Success(Vec<Diagnostic>),

    /// The file could not be read due to an I/O or filesystem error.
    #[cfg(feature = "filesystem")]
    IoError(FileError),

    /// The file type is unknown and no validation was performed.
    Skipped,
}

impl ValidationOutcome {
    /// Returns `true` if validation ran (regardless of whether diagnostics were found).
    pub fn is_success(&self) -> bool {
        matches!(self, ValidationOutcome::Success(_))
    }

    /// Returns `true` if the file could not be read.
    #[cfg(feature = "filesystem")]
    pub fn is_io_error(&self) -> bool {
        matches!(self, ValidationOutcome::IoError(_))
    }

    /// Returns `true` if the file was skipped (unknown file type).
    pub fn is_skipped(&self) -> bool {
        matches!(self, ValidationOutcome::Skipped)
    }

    /// Borrow the diagnostics from a successful validation.
    ///
    /// Returns an empty slice for `IoError` and `Skipped` variants.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        match self {
            ValidationOutcome::Success(diags) => diags,
            #[cfg(feature = "filesystem")]
            ValidationOutcome::IoError(_) => &[],
            ValidationOutcome::Skipped => &[],
        }
    }

    /// Consume the outcome and return the diagnostics.
    ///
    /// - `Success`: returns the contained diagnostics.
    /// - `IoError`: returns a single `file::read` diagnostic describing the error.
    /// - `Skipped`: returns an empty `Vec`.
    pub fn into_diagnostics(self) -> Vec<Diagnostic> {
        match self {
            ValidationOutcome::Success(diags) => diags,
            #[cfg(feature = "filesystem")]
            ValidationOutcome::IoError(file_error) => {
                let error_msg = file_error.to_string();
                let path = match file_error {
                    FileError::Read { path, .. }
                    | FileError::Write { path, .. }
                    | FileError::Symlink { path }
                    | FileError::TooBig { path, .. }
                    | FileError::NotRegular { path } => path,
                };
                vec![
                    Diagnostic::error(
                        path,
                        0,
                        0,
                        "file::read",
                        t!("rules.file_read_error", error = error_msg),
                    )
                    .with_suggestion(t!("rules.file_read_error_suggestion")),
                ]
            }
            ValidationOutcome::Skipped => vec![],
        }
    }

    /// If this is an `IoError`, return a reference to the underlying [`FileError`].
    #[cfg(feature = "filesystem")]
    pub fn io_error(&self) -> Option<&FileError> {
        match self {
            ValidationOutcome::IoError(e) => Some(e),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Auto-populate metadata tests =====

    #[test]
    fn test_error_auto_populates_metadata_for_known_rule() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(
            diag.metadata.is_some(),
            "Metadata should be auto-populated for known rule AS-001"
        );
        let meta = diag.metadata.unwrap();
        assert_eq!(meta.category, "agent-skills");
        assert_eq!(meta.severity, "HIGH");
        assert!(
            meta.applies_to_tool.is_none(),
            "AS-001 is generic, should have no tool"
        );
    }

    #[test]
    fn test_warning_auto_populates_metadata() {
        let diag = Diagnostic::warning(PathBuf::from("test.md"), 1, 1, "CC-HK-001", "Test");
        assert!(diag.metadata.is_some());
        let meta = diag.metadata.unwrap();
        assert_eq!(meta.applies_to_tool, Some("claude-code".to_string()));
    }

    #[test]
    fn test_info_auto_populates_metadata() {
        let diag = Diagnostic::info(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(diag.metadata.is_some());
    }

    #[test]
    fn test_unknown_rule_has_no_metadata() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "UNKNOWN-999", "Test");
        assert!(
            diag.metadata.is_none(),
            "Unknown rules should not have metadata"
        );
    }

    #[test]
    fn test_lookup_rule_metadata_empty_string() {
        let meta = lookup_rule_metadata("");
        assert!(meta.is_none(), "Empty string should return None");
    }

    #[test]
    fn test_lookup_rule_metadata_special_characters() {
        let meta = lookup_rule_metadata("@#$%^&*()");
        assert!(
            meta.is_none(),
            "Rule ID with special characters should return None"
        );
    }

    // ===== Builder method tests =====

    #[test]
    fn test_with_metadata_builder() {
        let meta = RuleMetadata {
            category: "custom".to_string(),
            severity: "LOW".to_string(),
            applies_to_tool: Some("my-tool".to_string()),
        };
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "UNKNOWN-999", "Test")
            .with_metadata(meta.clone());
        assert_eq!(diag.metadata, Some(meta));
    }

    #[test]
    fn test_with_metadata_overrides_auto_populated() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 1, 1, "AS-001", "Test");
        assert!(diag.metadata.is_some());

        let custom_meta = RuleMetadata {
            category: "custom".to_string(),
            severity: "LOW".to_string(),
            applies_to_tool: None,
        };
        let diag = diag.with_metadata(custom_meta.clone());
        assert_eq!(diag.metadata, Some(custom_meta));
    }

    // ===== Serde roundtrip tests =====

    #[test]
    fn test_rule_metadata_serde_roundtrip() {
        let meta = RuleMetadata {
            category: "agent-skills".to_string(),
            severity: "HIGH".to_string(),
            applies_to_tool: Some("claude-code".to_string()),
        };
        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: RuleMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta, deserialized);
    }

    #[test]
    fn test_rule_metadata_serde_none_tool_omitted() {
        let meta = RuleMetadata {
            category: "agent-skills".to_string(),
            severity: "HIGH".to_string(),
            applies_to_tool: None,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(
            !json.contains("applies_to_tool"),
            "None tool should be omitted via skip_serializing_if"
        );
    }

    #[test]
    fn test_diagnostic_serde_roundtrip_with_metadata() {
        let diag = Diagnostic::error(PathBuf::from("test.md"), 10, 5, "AS-001", "Test error");
        let json = serde_json::to_string(&diag).unwrap();
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.metadata, diag.metadata);
        assert_eq!(deserialized.rule, "AS-001");
    }

    #[test]
    fn test_diagnostic_serde_roundtrip_without_metadata() {
        let diag = Diagnostic {
            level: DiagnosticLevel::Error,
            message: "Test".to_string(),
            file: PathBuf::from("test.md"),
            line: 1,
            column: 1,
            rule: "UNKNOWN".to_string(),
            suggestion: None,
            fixes: Vec::new(),
            assumption: None,
            metadata: None,
        };
        let json = serde_json::to_string(&diag).unwrap();
        assert!(
            !json.contains("metadata"),
            "None metadata should be omitted"
        );
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();
        assert!(deserialized.metadata.is_none());
    }

    #[test]
    fn test_diagnostic_deserialize_without_metadata_field() {
        // Simulate old JSON that doesn't have the metadata field at all
        let json = r#"{
            "level": "Error",
            "message": "Test",
            "file": "test.md",
            "line": 1,
            "column": 1,
            "rule": "AS-001",
            "fixes": []
        }"#;
        let diag: Diagnostic = serde_json::from_str(json).unwrap();
        assert!(
            diag.metadata.is_none(),
            "Missing metadata field should deserialize as None"
        );
    }

    #[test]
    fn test_diagnostic_manual_metadata_serde_roundtrip() {
        let manual_metadata = RuleMetadata {
            category: "custom-category".to_string(),
            severity: "MEDIUM".to_string(),
            applies_to_tool: Some("custom-tool".to_string()),
        };

        let diag = Diagnostic::error(
            PathBuf::from("test.md"),
            5,
            10,
            "CUSTOM-001",
            "Custom error",
        )
        .with_metadata(manual_metadata.clone());

        // Serialize to JSON
        let json = serde_json::to_string(&diag).unwrap();

        // Deserialize back
        let deserialized: Diagnostic = serde_json::from_str(&json).unwrap();

        // Verify metadata is preserved
        assert_eq!(deserialized.metadata, Some(manual_metadata));
        assert_eq!(deserialized.rule, "CUSTOM-001");
        assert_eq!(deserialized.message, "Custom error");
    }

    // ===== Fix::is_insertion() tests =====

    #[test]
    fn test_fix_is_insertion_true_when_start_equals_end() {
        let fix = Fix::insert(10, "inserted text", "insert something", true);
        assert!(fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_false_when_replacement_empty() {
        // start == end but replacement is empty -> not an insertion
        let fix = Fix {
            start_byte: 5,
            end_byte: 5,
            replacement: String::new(),
            description: "no-op".to_string(),
            safe: true,
            confidence: Some(1.0),
            group: None,
            depends_on: None,
        };
        assert!(!fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_false_when_range_differs() {
        let fix = Fix::replace(0, 10, "replacement", "replace", true);
        assert!(!fix.is_insertion());
    }

    #[test]
    fn test_fix_is_insertion_at_zero() {
        let fix = Fix::insert(0, "prepend", "prepend text", true);
        assert!(fix.is_insertion());
    }

    // ===== Fix::is_deletion() tests =====

    #[test]
    fn test_fix_is_deletion_true_when_replacement_empty() {
        let fix = Fix::delete(5, 15, "remove text", true);
        assert!(fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_false_when_replacement_nonempty() {
        let fix = Fix::replace(5, 15, "new text", "replace", true);
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_false_when_start_equals_end() {
        // Empty range with empty replacement -> not a deletion
        let fix = Fix {
            start_byte: 5,
            end_byte: 5,
            replacement: String::new(),
            description: "no-op".to_string(),
            safe: true,
            confidence: Some(1.0),
            group: None,
            depends_on: None,
        };
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_is_deletion_single_byte() {
        let fix = Fix::delete(10, 11, "delete one byte", false);
        assert!(fix.is_deletion());
    }

    // ===== Fix constructors =====

    #[test]
    fn test_fix_replace_fields() {
        let fix = Fix::replace(2, 8, "new", "replace old", false);
        assert_eq!(fix.start_byte, 2);
        assert_eq!(fix.end_byte, 8);
        assert_eq!(fix.replacement, "new");
        assert_eq!(fix.description, "replace old");
        assert!(!fix.safe);
        assert_eq!(fix.confidence_tier(), FixConfidenceTier::Medium);
        assert!(!fix.is_insertion());
        assert!(!fix.is_deletion());
    }

    #[test]
    fn test_fix_insert_fields() {
        let fix = Fix::insert(42, "text", "insert", true);
        assert_eq!(fix.start_byte, 42);
        assert_eq!(fix.end_byte, 42);
        assert_eq!(fix.replacement, "text");
        assert!(fix.safe);
        assert_eq!(fix.confidence_tier(), FixConfidenceTier::High);
    }

    #[test]
    fn test_fix_delete_fields() {
        let fix = Fix::delete(0, 100, "remove block", true);
        assert_eq!(fix.start_byte, 0);
        assert_eq!(fix.end_byte, 100);
        assert!(fix.replacement.is_empty());
        assert!(fix.safe);
        assert_eq!(fix.confidence_tier(), FixConfidenceTier::High);
    }

    #[test]
    fn test_fix_explicit_confidence_fields() {
        let fix = Fix::replace_with_confidence(0, 4, "NAME", "normalize", 0.42)
            .with_group("name-normalization")
            .with_dependency("fix-prefix");

        assert_eq!(fix.confidence_score(), 0.42);
        assert_eq!(fix.confidence_tier(), FixConfidenceTier::Low);
        assert!(!fix.is_safe());
        assert_eq!(fix.group.as_deref(), Some("name-normalization"));
        assert_eq!(fix.depends_on.as_deref(), Some("fix-prefix"));
    }

    #[test]
    fn test_fix_with_confidence_updates_safe_compat_flag() {
        let fix = Fix::replace(0, 4, "NAME", "normalize", true).with_confidence(0.80);
        assert!(!fix.safe);
        assert_eq!(fix.confidence_tier(), FixConfidenceTier::Medium);
    }

    // ===== Diagnostic builder methods =====

    #[test]
    fn test_diagnostic_with_suggestion() {
        let diag = Diagnostic::warning(PathBuf::from("test.md"), 1, 0, "AS-001", "test message")
            .with_suggestion("try this instead");

        assert_eq!(diag.suggestion, Some("try this instead".to_string()));
        assert_eq!(diag.level, DiagnosticLevel::Warning);
        assert_eq!(diag.message, "test message");
    }

    #[test]
    fn test_diagnostic_with_fix() {
        let fix = Fix::insert(0, "added", "add prefix", true);
        let diag = Diagnostic::error(PathBuf::from("a.md"), 5, 3, "CC-AG-001", "missing prefix")
            .with_fix(fix);

        assert!(diag.has_fixes());
        assert!(diag.has_safe_fixes());
        assert_eq!(diag.fixes.len(), 1);
        assert_eq!(diag.fixes[0].replacement, "added");
    }

    #[test]
    fn test_diagnostic_with_fixes_multiple() {
        let fixes = vec![
            Fix::insert(0, "a", "fix a", true),
            Fix::delete(10, 20, "fix b", false),
        ];
        let diag =
            Diagnostic::info(PathBuf::from("b.md"), 1, 0, "XML-001", "xml issue").with_fixes(fixes);

        assert_eq!(diag.fixes.len(), 2);
        assert!(diag.has_fixes());
        // One safe, one unsafe
        assert!(diag.has_safe_fixes());
    }

    #[test]
    fn test_diagnostic_with_assumption() {
        let diag = Diagnostic::warning(PathBuf::from("c.md"), 2, 0, "CC-HK-001", "hook issue")
            .with_assumption("Assuming Claude Code >= 1.0.0");

        assert_eq!(
            diag.assumption,
            Some("Assuming Claude Code >= 1.0.0".to_string())
        );
    }

    #[test]
    fn test_diagnostic_builder_chaining() {
        let diag = Diagnostic::error(PathBuf::from("d.md"), 10, 5, "MCP-001", "mcp error")
            .with_suggestion("fix it")
            .with_fix(Fix::replace(0, 5, "fixed", "auto fix", true))
            .with_assumption("Assuming MCP protocol 2025-11-25");

        assert_eq!(diag.suggestion, Some("fix it".to_string()));
        assert_eq!(diag.fixes.len(), 1);
        assert!(diag.assumption.is_some());
        assert_eq!(diag.level, DiagnosticLevel::Error);
        assert_eq!(diag.rule, "MCP-001");
    }

    #[test]
    fn test_diagnostic_no_fixes_by_default() {
        let diag = Diagnostic::warning(PathBuf::from("e.md"), 1, 0, "AS-005", "something wrong");

        assert!(!diag.has_fixes());
        assert!(!diag.has_safe_fixes());
        assert!(diag.fixes.is_empty());
        assert!(diag.suggestion.is_none());
        assert!(diag.assumption.is_none());
    }

    #[test]
    fn test_diagnostic_has_safe_fixes_false_when_all_unsafe() {
        let fixes = vec![
            Fix::delete(0, 5, "remove a", false),
            Fix::delete(10, 15, "remove b", false),
        ];
        let diag = Diagnostic::error(PathBuf::from("f.md"), 1, 0, "CC-AG-002", "agent error")
            .with_fixes(fixes);

        assert!(diag.has_fixes());
        assert!(!diag.has_safe_fixes());
    }

    // ===== Diagnostic level constructors =====

    #[test]
    fn test_diagnostic_error_level() {
        let diag = Diagnostic::error(PathBuf::from("x.md"), 1, 0, "R-001", "err");
        assert_eq!(diag.level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_diagnostic_warning_level() {
        let diag = Diagnostic::warning(PathBuf::from("x.md"), 1, 0, "R-002", "warn");
        assert_eq!(diag.level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_diagnostic_info_level() {
        let diag = Diagnostic::info(PathBuf::from("x.md"), 1, 0, "R-003", "info");
        assert_eq!(diag.level, DiagnosticLevel::Info);
    }

    // ===== Serialization roundtrip =====

    #[test]
    fn test_diagnostic_serialization_roundtrip() {
        let original = Diagnostic::error(
            PathBuf::from("project/CLAUDE.md"),
            42,
            7,
            "CC-AG-003",
            "Agent configuration issue",
        )
        .with_suggestion("Add the required field")
        .with_fix(Fix::insert(100, "new_field: true\n", "add field", true))
        .with_fix(Fix::delete(200, 250, "remove deprecated", false))
        .with_assumption("Assuming Claude Code >= 1.0.0");

        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Diagnostic =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.level, original.level);
        assert_eq!(deserialized.message, original.message);
        assert_eq!(deserialized.file, original.file);
        assert_eq!(deserialized.line, original.line);
        assert_eq!(deserialized.column, original.column);
        assert_eq!(deserialized.rule, original.rule);
        assert_eq!(deserialized.suggestion, original.suggestion);
        assert_eq!(deserialized.assumption, original.assumption);
        assert_eq!(deserialized.fixes.len(), 2);
        assert_eq!(deserialized.fixes[0].replacement, "new_field: true\n");
        assert!(deserialized.fixes[0].safe);
        assert!(deserialized.fixes[1].replacement.is_empty());
        assert!(!deserialized.fixes[1].safe);
    }

    #[test]
    fn test_fix_serialization_roundtrip() {
        let original = Fix::replace(10, 20, "replaced", "test fix", true);
        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Fix =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.start_byte, original.start_byte);
        assert_eq!(deserialized.end_byte, original.end_byte);
        assert_eq!(deserialized.replacement, original.replacement);
        assert_eq!(deserialized.description, original.description);
        assert_eq!(deserialized.safe, original.safe);
    }

    #[test]
    fn test_diagnostic_without_optional_fields_roundtrip() {
        let original =
            Diagnostic::info(PathBuf::from("simple.md"), 1, 0, "AS-001", "simple message");

        let json = serde_json::to_string(&original).expect("serialization should succeed");
        let deserialized: Diagnostic =
            serde_json::from_str(&json).expect("deserialization should succeed");

        assert_eq!(deserialized.suggestion, None);
        assert_eq!(deserialized.assumption, None);
        assert!(deserialized.fixes.is_empty());
    }

    // ===== DiagnosticLevel ordering =====

    #[test]
    fn test_diagnostic_level_ordering() {
        assert!(DiagnosticLevel::Error < DiagnosticLevel::Warning);
        assert!(DiagnosticLevel::Warning < DiagnosticLevel::Info);
        assert!(DiagnosticLevel::Error < DiagnosticLevel::Info);
    }

    // Fix debug_assert! reversed-range tests

    #[cfg(debug_assertions)]
    mod fix_debug_assert_tests {
        use super::*;
        use std::panic;

        #[test]
        fn test_fix_replace_reversed_range_panics() {
            assert!(panic::catch_unwind(|| Fix::replace(10, 5, "x", "bad", true)).is_err());
        }

        #[test]
        fn test_fix_replace_with_confidence_reversed_range_panics() {
            assert!(
                panic::catch_unwind(|| Fix::replace_with_confidence(10, 5, "x", "bad", 0.9))
                    .is_err()
            );
        }

        #[test]
        fn test_fix_delete_reversed_range_panics() {
            assert!(panic::catch_unwind(|| Fix::delete(20, 10, "bad", true)).is_err());
        }

        #[test]
        fn test_fix_delete_with_confidence_reversed_range_panics() {
            assert!(
                panic::catch_unwind(|| Fix::delete_with_confidence(20, 10, "bad", 0.9)).is_err()
            );
        }

        #[test]
        fn test_fix_replace_equal_start_end_ok() {
            // start == end is a valid zero-width replacement
            let fix = Fix::replace(5, 5, "x", "ok", true);
            assert_eq!(fix.start_byte, 5);
            assert_eq!(fix.end_byte, 5);
        }
    }

    // Fix _checked constructor tests

    mod fix_checked_tests {
        use super::*;

        // "hel\u{00e9}lo" = 7 bytes: h(0) e(1) l(2) e-acute(3,4) l(5) o(6)
        // Byte 4 is mid-codepoint (inside the 2-byte e-acute)
        const CONTENT_2BYTE: &str = "hel\u{00e9}lo";

        #[test]
        fn test_fix_replace_checked_valid_boundaries() {
            let fix = Fix::replace_checked(CONTENT_2BYTE, 0, 5, "x", "ok", true);
            assert_eq!(fix.start_byte, 0);
            assert_eq!(fix.end_byte, 5);
        }

        #[test]
        fn test_fix_insert_checked_valid_boundary() {
            // byte 3 is start of e-acute, which is a valid char boundary
            let fix = Fix::insert_checked(CONTENT_2BYTE, 3, "x", "ok", true);
            assert_eq!(fix.start_byte, 3);
        }

        #[test]
        fn test_fix_checked_at_content_end() {
            let fix = Fix::insert_checked(CONTENT_2BYTE, CONTENT_2BYTE.len(), "x", "ok", true);
            assert_eq!(fix.start_byte, CONTENT_2BYTE.len());
        }

        #[test]
        fn test_fix_replace_with_confidence_checked_valid() {
            let fix = Fix::replace_with_confidence_checked(CONTENT_2BYTE, 0, 3, "x", "ok", 0.9);
            assert_eq!(fix.start_byte, 0);
            assert_eq!(fix.end_byte, 3);
            assert!((fix.confidence_score() - 0.9).abs() < 1e-6);
        }

        #[test]
        fn test_fix_delete_checked_valid() {
            let fix = Fix::delete_checked(CONTENT_2BYTE, 0, 3, "ok", true);
            assert_eq!(fix.start_byte, 0);
            assert_eq!(fix.end_byte, 3);
        }

        #[test]
        fn test_fix_insert_with_confidence_checked_valid() {
            // byte 3 is start of e-acute, a valid char boundary
            let fix = Fix::insert_with_confidence_checked(CONTENT_2BYTE, 3, "x", "ok", 0.9);
            assert_eq!(fix.start_byte, 3);
            assert_eq!(fix.end_byte, 3);
            assert!((fix.confidence_score() - 0.9).abs() < 1e-6);
        }

        #[test]
        fn test_fix_delete_with_confidence_checked_valid() {
            let fix = Fix::delete_with_confidence_checked(CONTENT_2BYTE, 0, 3, "ok", 0.9);
            assert_eq!(fix.start_byte, 0);
            assert_eq!(fix.end_byte, 3);
            assert!((fix.confidence_score() - 0.9).abs() < 1e-6);
        }

        // 4-byte emoji: "a\u{1f600}b" = 6 bytes: a(0) grinning-face(1,2,3,4) b(5)
        // Valid range: 1..5 covers whole emoji. Invalid: 1..3 (mid-emoji).
        const CONTENT_4BYTE: &str = "a\u{1f600}b";

        #[test]
        fn test_fix_replace_checked_four_byte_valid() {
            // 1..5 covers the full emoji - valid char boundaries
            let fix = Fix::replace_checked(CONTENT_4BYTE, 1, 5, "x", "ok", true);
            assert_eq!(fix.start_byte, 1);
            assert_eq!(fix.end_byte, 5);
        }

        #[test]
        fn test_fix_replace_checked_zero_width_at_valid_boundary() {
            // start == end at a valid char boundary is permitted (zero-width replacement)
            let fix = Fix::replace_checked(CONTENT_2BYTE, 3, 3, "x", "ok", true);
            assert_eq!(fix.start_byte, 3);
            assert_eq!(fix.end_byte, 3);
        }

        #[test]
        fn test_fix_replace_checked_ascii_content() {
            // ASCII content: all byte positions are valid char boundaries
            let content = "hello";
            let fix = Fix::replace_checked(content, 1, 3, "i", "ok", true);
            assert_eq!(fix.start_byte, 1);
            assert_eq!(fix.end_byte, 3);
        }

        // These tests exercise debug_assert! paths and only compile when debug assertions are enabled.
        #[cfg(debug_assertions)]
        mod fix_checked_panic_tests {
            use super::*;
            use std::panic;

            #[test]
            fn test_fix_replace_checked_mid_codepoint_start_panics() {
                // byte 4 is inside the 2-byte e-acute (bytes 3-4)
                assert!(
                    panic::catch_unwind(|| {
                        Fix::replace_checked(CONTENT_2BYTE, 4, 5, "x", "bad", true)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_checked_mid_codepoint_end_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::replace_checked(CONTENT_2BYTE, 0, 4, "x", "bad", true)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_insert_checked_mid_codepoint_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::insert_checked(CONTENT_2BYTE, 4, "x", "bad", true)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_checked_mid_codepoint_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::delete_checked(CONTENT_2BYTE, 3, 4, "bad", true)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_with_confidence_checked_mid_codepoint_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::replace_with_confidence_checked(CONTENT_2BYTE, 4, 5, "x", "bad", 0.9)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_insert_with_confidence_checked_mid_codepoint_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::insert_with_confidence_checked(CONTENT_2BYTE, 4, "x", "bad", 0.9)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_with_confidence_checked_mid_codepoint_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::delete_with_confidence_checked(CONTENT_2BYTE, 3, 4, "bad", 0.9)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_checked_mid_codepoint_start_panics() {
                // start=4 is the continuation byte of the 2-byte e-acute; only end was covered before
                assert!(
                    panic::catch_unwind(|| Fix::delete_checked(CONTENT_2BYTE, 4, 5, "bad", true))
                        .is_err()
                );
            }

            #[test]
            fn test_fix_delete_with_confidence_checked_mid_codepoint_start_panics() {
                // start=4 is the continuation byte of the 2-byte e-acute
                assert!(
                    panic::catch_unwind(|| Fix::delete_with_confidence_checked(
                        CONTENT_2BYTE,
                        4,
                        5,
                        "bad",
                        0.9
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_with_confidence_checked_mid_codepoint_end_panics() {
                // end byte 4 is inside the 2-byte e-acute (bytes 3-4)
                assert!(
                    panic::catch_unwind(|| Fix::replace_with_confidence_checked(
                        CONTENT_2BYTE,
                        0,
                        4,
                        "x",
                        "bad",
                        0.9
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_insert_with_confidence_checked_out_of_bounds_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::insert_with_confidence_checked(
                        CONTENT_2BYTE,
                        CONTENT_2BYTE.len() + 1,
                        "x",
                        "bad",
                        0.9
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_checked_reversed_range_panics() {
                // The _checked variants also contain their own start <= end assertion
                assert!(
                    panic::catch_unwind(|| Fix::replace_checked(
                        CONTENT_2BYTE,
                        5,
                        3,
                        "x",
                        "bad",
                        true
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_checked_reversed_range_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::delete_checked(CONTENT_2BYTE, 5, 3, "bad", true))
                        .is_err()
                );
            }

            #[test]
            fn test_fix_replace_with_confidence_checked_reversed_range_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::replace_with_confidence_checked(
                        CONTENT_2BYTE,
                        5,
                        3,
                        "x",
                        "bad",
                        0.9
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_with_confidence_checked_reversed_range_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::delete_with_confidence_checked(
                        CONTENT_2BYTE,
                        5,
                        3,
                        "bad",
                        0.9
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_checked_out_of_bounds_panics() {
                assert!(
                    panic::catch_unwind(|| {
                        Fix::insert_checked(
                            CONTENT_2BYTE,
                            CONTENT_2BYTE.len() + 1,
                            "x",
                            "bad",
                            true,
                        )
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_checked_four_byte_mid_emoji_panics() {
                // end byte 3 is mid-codepoint (third byte of the 4-byte emoji); start byte 1 is a valid char boundary
                assert!(
                    panic::catch_unwind(|| {
                        Fix::replace_checked(CONTENT_4BYTE, 1, 3, "x", "bad", true)
                    })
                    .is_err()
                );
            }

            #[test]
            fn test_fix_replace_checked_end_out_of_bounds_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::replace_checked(
                        CONTENT_2BYTE,
                        0,
                        CONTENT_2BYTE.len() + 1,
                        "x",
                        "bad",
                        true
                    ))
                    .is_err()
                );
            }

            #[test]
            fn test_fix_delete_checked_end_out_of_bounds_panics() {
                assert!(
                    panic::catch_unwind(|| Fix::delete_checked(
                        CONTENT_2BYTE,
                        0,
                        CONTENT_2BYTE.len() + 1,
                        "bad",
                        true
                    ))
                    .is_err()
                );
            }
        }
    }

    // ===== ValidationOutcome tests =====

    #[test]
    fn test_validation_outcome_success_empty() {
        let outcome = ValidationOutcome::Success(vec![]);
        assert!(outcome.is_success());
        assert!(!outcome.is_skipped());
        assert!(outcome.diagnostics().is_empty());
        assert!(outcome.into_diagnostics().is_empty());
    }

    #[test]
    fn test_validation_outcome_success_with_diagnostics() {
        let diag = Diagnostic::warning(PathBuf::from("test.md"), 1, 0, "AS-001", "test");
        let outcome = ValidationOutcome::Success(vec![diag]);
        assert!(outcome.is_success());
        assert_eq!(outcome.diagnostics().len(), 1);
        assert_eq!(outcome.diagnostics()[0].rule, "AS-001");
        let diags = outcome.into_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "AS-001");
    }

    #[test]
    fn test_validation_outcome_skipped() {
        let outcome = ValidationOutcome::Skipped;
        assert!(outcome.is_skipped());
        assert!(!outcome.is_success());
        assert!(outcome.diagnostics().is_empty());
        assert!(outcome.into_diagnostics().is_empty());
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_validation_outcome_io_error() {
        let file_error = FileError::Read {
            path: PathBuf::from("/tmp/missing.md"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let outcome = ValidationOutcome::IoError(file_error);
        assert!(outcome.is_io_error());
        assert!(!outcome.is_success());
        assert!(!outcome.is_skipped());
        // diagnostics() returns empty slice for IoError
        assert!(outcome.diagnostics().is_empty());
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_validation_outcome_io_error_ref() {
        let file_error = FileError::Read {
            path: PathBuf::from("/tmp/missing.md"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let outcome = ValidationOutcome::IoError(file_error);
        let err = outcome.io_error().expect("should be Some for IoError");
        match err {
            FileError::Read { path, .. } => {
                assert_eq!(path, &PathBuf::from("/tmp/missing.md"));
            }
            _ => panic!("expected FileError::Read"),
        }
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_validation_outcome_io_error_into_diagnostics() {
        let file_error = FileError::Read {
            path: PathBuf::from("/tmp/missing.md"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "not found"),
        };
        let outcome = ValidationOutcome::IoError(file_error);
        let diags = outcome.into_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "file::read");
        assert_eq!(diags[0].file, PathBuf::from("/tmp/missing.md"));
        assert_eq!(diags[0].level, DiagnosticLevel::Error);
        // Message uses i18n ("rules.file_read_error"); verify it is non-empty and
        // the suggestion is also populated.
        assert!(!diags[0].message.is_empty());
        assert!(diags[0].suggestion.is_some());
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_validation_outcome_io_error_symlink() {
        let file_error = FileError::Symlink {
            path: PathBuf::from("/tmp/link.md"),
        };
        let outcome = ValidationOutcome::IoError(file_error);
        let diags = outcome.into_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "file::read");
        assert_eq!(diags[0].level, DiagnosticLevel::Error);
        assert!(diags[0].suggestion.is_some());
    }

    #[cfg(feature = "filesystem")]
    #[test]
    fn test_validation_outcome_io_error_too_big() {
        let file_error = FileError::TooBig {
            path: PathBuf::from("/tmp/huge.md"),
            size: 5_000_000,
            limit: 1_048_576,
        };
        let outcome = ValidationOutcome::IoError(file_error);
        let diags = outcome.into_diagnostics();
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].rule, "file::read");
        assert_eq!(diags[0].level, DiagnosticLevel::Error);
        assert!(diags[0].suggestion.is_some());
    }

    #[test]
    fn test_validation_outcome_success_io_error_ref_is_none() {
        let outcome = ValidationOutcome::Success(vec![]);
        #[cfg(feature = "filesystem")]
        assert!(outcome.io_error().is_none());
        let _ = outcome;
    }

    #[test]
    fn test_validation_outcome_skipped_io_error_ref_is_none() {
        let outcome = ValidationOutcome::Skipped;
        #[cfg(feature = "filesystem")]
        assert!(outcome.io_error().is_none());
        let _ = outcome;
    }

    // ===== CoreError::path() tests =====

    #[test]
    fn test_core_error_path_root_not_found() {
        let path = PathBuf::from("/some/nonexistent/path");
        let err = CoreError::Validation(ValidationError::RootNotFound { path: path.clone() });
        assert_eq!(err.path(), Some(&path));
    }
}
