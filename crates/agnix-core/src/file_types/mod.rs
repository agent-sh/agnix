//! File type detection for validator dispatch.
//!
//! This module provides:
//!
//! - [`FileType`] -- enum of all recognised configuration file types
//! - [`detect_file_type`] -- built-in path-based detection function
//! - [`FileTypeDetector`] -- trait for custom detection strategies
//! - [`FileTypeDetectorChain`] -- chain-of-responsibility dispatcher
//!
//! ## Extending detection
//!
//! Implement [`FileTypeDetector`] and register it via
//! [`FileTypeDetectorChain::with_builtin().prepend(your_detector)`](FileTypeDetectorChain::prepend)
//! to override detection for specific paths without modifying agnix-core.
//!
//! **Stability: unstable** -- interface may change on minor releases.

mod detection;
mod detector;
mod types;

// Primary re-exports (backward-compatible with the old single-file module)
pub use detection::detect_file_type;
pub use types::FileType;

// New public API
pub use detection::{DOCUMENTATION_DIRECTORIES, EXCLUDED_FILENAMES, EXCLUDED_PARENT_DIRECTORIES};
pub(crate) use detection::{
    KIRO_WORKFLOW_SUFFIXES, KiroNotLoadedReason, is_claude_managed_settings_path,
    is_claude_system_managed_settings_path, is_inside_kiro_dir, is_kiro_power_skill,
    kiro_not_loaded_reason, path_contains_consecutive_components,
};
// Sibling-file refinement applied by the pipeline after path detection.
pub(crate) use detection::refine_file_type;
// The extension root of a Gemini CLI extension policy file.
pub(crate) use detection::gemini_policy_extension_root;
// Amp skill roots, for the Amp skill validator.
pub(crate) use detection::{AmpSkillLocation, amp_skill_location};
// Path facts for the Kiro agent and permissions validators.
#[allow(unused_imports)]
pub(crate) use detection::{KiroAgentLoaders, kiro_agent_loaders};
pub use detector::{BuiltinDetector, FileTypeDetector, FileTypeDetectorChain};
