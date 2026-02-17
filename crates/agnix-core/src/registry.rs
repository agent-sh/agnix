//! Validator registry and factory functions.

use std::collections::{HashMap, HashSet};

use crate::file_types::FileType;
use crate::rules::Validator;

/// Factory function type that creates validator instances.
pub type ValidatorFactory = fn() -> Box<dyn Validator>;

/// A provider of validator factories.
///
/// Implement this trait to supply validators from an external source (e.g., a
/// plugin or a secondary rule set). The built-in validators are packaged as
/// a `BuiltinProvider` (internal to the crate).
///
/// # Example
///
/// ```
/// use agnix_core::{FileType, ValidatorFactory, ValidatorProvider, ValidatorRegistry};
///
/// struct MyProvider;
///
/// impl ValidatorProvider for MyProvider {
///     fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
///         // Return custom validators here
///         vec![]
///     }
/// }
///
/// let registry = ValidatorRegistry::builder()
///     .with_defaults()
///     .with_provider(&MyProvider)
///     .build();
/// ```
pub trait ValidatorProvider: Send + Sync {
    /// Human-readable name for this provider.
    ///
    /// Defaults to the unqualified struct name (e.g., `"BuiltinProvider"`).
    fn name(&self) -> &str {
        let full = std::any::type_name::<Self>();
        full.rsplit("::").next().unwrap_or(full)
    }

    /// Return the validator factories supplied by this provider.
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)>;
}

/// The built-in validator provider shipping with agnix-core.
///
/// Contains all built-in validators across all supported file types. Used
/// internally by [`ValidatorRegistry::with_defaults`] and
/// [`ValidatorRegistryBuilder::with_defaults`].
pub(crate) struct BuiltinProvider;

impl ValidatorProvider for BuiltinProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        DEFAULTS.to_vec()
    }
}

/// Registry that maps [`FileType`] values to cached validator instances.
///
/// This is the extension point for the validation engine. A
/// `ValidatorRegistry` owns pre-constructed [`Validator`] instances for each
/// supported [`FileType`], eliminating per-file instantiation overhead.
///
/// Most callers should use [`ValidatorRegistry::with_defaults`] to obtain a
/// registry pre-populated with all built-in validators. For advanced use cases
/// (custom providers, disabling validators), use [`ValidatorRegistry::builder`].
pub struct ValidatorRegistry {
    /// Cached validator instances, keyed by file type. Each factory is called
    /// exactly once at registration time; validators_for() returns a reference
    /// to this pre-built slice.
    validators: HashMap<FileType, Vec<Box<dyn Validator>>>,
    disabled_validators: HashSet<String>,
}

impl ValidatorRegistry {
    /// Create an empty registry with no registered validators.
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            disabled_validators: HashSet::new(),
        }
    }

    /// Create a registry pre-populated with built-in validators.
    pub fn with_defaults() -> Self {
        let mut registry = Self::new();
        registry.register_defaults();
        registry
    }

    /// Create a [`ValidatorRegistryBuilder`] for ergonomic construction.
    ///
    /// # Example
    ///
    /// ```
    /// use agnix_core::ValidatorRegistry;
    ///
    /// let registry = ValidatorRegistry::builder()
    ///     .with_defaults()
    ///     .without_validator("XmlValidator")
    ///     .build();
    /// ```
    pub fn builder() -> ValidatorRegistryBuilder {
        ValidatorRegistryBuilder::new()
    }

    /// Register a validator factory for a given file type.
    ///
    /// The factory is called exactly once at registration time. If the
    /// validator's name appears in the disabled set, the instance is
    /// immediately dropped (the factory is still called once to obtain the
    /// validator name).
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) {
        let instance = factory();
        if self.disabled_validators.contains(instance.name() as &str) {
            return;
        }
        self.validators.entry(file_type).or_default().push(instance);
    }

    /// Return the total number of cached validator instances across all file types.
    pub fn total_validator_count(&self) -> usize {
        self.validators.values().map(|v| v.len()).sum()
    }

    /// Return the total number of registered validator instances across all file types.
    #[deprecated(
        since = "0.12.2",
        note = "renamed to total_validator_count() - validators are now cached, not re-instantiated"
    )]
    pub fn total_factory_count(&self) -> usize {
        self.total_validator_count()
    }

    /// Return a reference to the cached validator instances for the given file type.
    ///
    /// Returns an empty slice if no validators are registered for `file_type`.
    /// Instances whose [`name()`](Validator::name) appeared in the
    /// `disabled_validators` set were already excluded at registration time.
    pub fn validators_for(&self, file_type: FileType) -> &[Box<dyn Validator>] {
        match self.validators.get(&file_type) {
            Some(v) => v,
            None => &[],
        }
    }

    /// Disable a validator by name at runtime.
    ///
    /// The name must match the value returned by [`Validator::name()`]
    /// (e.g., `"XmlValidator"`). Matching cached instances are removed from all
    /// file types. This is an O(n) scan over all cached validators, which is
    /// acceptable since this method is only called at startup.
    pub fn disable_validator(&mut self, name: &'static str) {
        if self.disabled_validators.insert(name.to_string()) {
            self.remove_disabled_from_cache(name);
        }
    }

    /// Disable a validator by name from a runtime string.
    ///
    /// Prefer [`disable_validator`](ValidatorRegistry::disable_validator) for
    /// string literals.
    pub fn disable_validator_owned(&mut self, name: &str) {
        if self.disabled_validators.insert(name.to_string()) {
            self.remove_disabled_from_cache(name);
        }
    }

    /// Return the number of validator names currently disabled.
    pub fn disabled_validator_count(&self) -> usize {
        self.disabled_validators.len()
    }

    /// Remove cached instances whose name matches the given disabled name.
    fn remove_disabled_from_cache(&mut self, name: &str) {
        for instances in self.validators.values_mut() {
            instances.retain(|v| v.name() != name);
        }
    }

    fn register_defaults(&mut self) {
        for &(file_type, factory) in DEFAULTS {
            self.register(file_type, factory);
        }
    }
}

impl Default for ValidatorRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Builder for constructing a [`ValidatorRegistry`] with fine-grained control.
///
/// Supports adding built-in validators, custom [`ValidatorProvider`]
/// implementations, individual factories, and disabling validators by name.
///
/// # Example
///
/// ```
/// use agnix_core::ValidatorRegistry;
///
/// let registry = ValidatorRegistry::builder()
///     .with_defaults()
///     .without_validator("PromptValidator")
///     .without_validator("XmlValidator")
///     .build();
///
/// // The built registry excludes PromptValidator and XmlValidator
/// assert!(registry.disabled_validator_count() > 0);
/// ```
pub struct ValidatorRegistryBuilder {
    entries: Vec<(FileType, ValidatorFactory)>,
    disabled_validators: HashSet<String>,
}

impl ValidatorRegistryBuilder {
    /// Create a new empty builder.
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            disabled_validators: HashSet::new(),
        }
    }

    /// Add all built-in validators (equivalent to [`ValidatorRegistry::with_defaults`]).
    ///
    /// This method is additive: calling it multiple times will register
    /// duplicate factories. For most use cases, call it once.
    pub fn with_defaults(&mut self) -> &mut Self {
        self.with_provider(&BuiltinProvider)
    }

    /// Add all validators from a [`ValidatorProvider`].
    pub fn with_provider(&mut self, provider: &dyn ValidatorProvider) -> &mut Self {
        self.entries.extend(provider.validators());
        self
    }

    /// Register a single validator factory for a file type.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) -> &mut Self {
        self.entries.push((file_type, factory));
        self
    }

    /// Mark a validator name as disabled (excluded from the built registry).
    ///
    /// The name must match the value returned by [`Validator::name()`]
    /// (e.g., `"XmlValidator"`).
    pub fn without_validator(&mut self, name: &'static str) -> &mut Self {
        self.disabled_validators.insert(name.to_string());
        self
    }

    /// Mark a validator name as disabled from a runtime string.
    ///
    /// Prefer [`without_validator`](ValidatorRegistryBuilder::without_validator)
    /// for string literals.
    pub fn without_validator_owned(&mut self, name: &str) -> &mut Self {
        self.disabled_validators.insert(name.to_string());
        self
    }

    /// Produce a [`ValidatorRegistry`] from this builder.
    ///
    /// Note: Calling `build()` a second time produces a registry with no
    /// disabled validators (the disabled set is consumed via
    /// [`std::mem::take`]), but all registered factories are re-called (the
    /// entries list is preserved). Each `build()` call invokes all registered
    /// factories. Reuse a builder by calling configuration methods again
    /// before a subsequent `build()`.
    pub fn build(&mut self) -> ValidatorRegistry {
        let mut registry = ValidatorRegistry {
            validators: HashMap::new(),
            disabled_validators: std::mem::take(&mut self.disabled_validators),
        };
        for &(file_type, factory) in &self.entries {
            registry.register(file_type, factory);
        }
        registry
    }
}

// ============================================================================
// Built-in defaults
// ============================================================================

const DEFAULTS: &[(FileType, ValidatorFactory)] = &[
    (FileType::Skill, skill_validator),
    (FileType::Skill, per_client_skill_validator),
    (FileType::Skill, xml_validator),
    (FileType::Skill, imports_validator),
    (FileType::AmpCheck, amp_validator),
    (FileType::ClaudeMd, claude_md_validator),
    (FileType::ClaudeMd, cross_platform_validator),
    (FileType::ClaudeMd, agents_md_validator),
    (FileType::ClaudeMd, amp_validator),
    (FileType::ClaudeMd, xml_validator),
    (FileType::ClaudeMd, imports_validator),
    (FileType::ClaudeMd, prompt_validator),
    (FileType::Agent, agent_validator),
    (FileType::Agent, xml_validator),
    (FileType::Hooks, hooks_validator),
    (FileType::Plugin, plugin_validator),
    (FileType::Mcp, mcp_validator),
    (FileType::Copilot, copilot_validator),
    (FileType::Copilot, xml_validator),
    (FileType::CopilotScoped, copilot_validator),
    (FileType::CopilotScoped, xml_validator),
    (FileType::CopilotAgent, copilot_validator),
    (FileType::CopilotAgent, xml_validator),
    (FileType::CopilotPrompt, copilot_validator),
    (FileType::CopilotPrompt, xml_validator),
    (FileType::CopilotHooks, copilot_validator),
    (FileType::ClaudeRule, claude_rules_validator),
    (FileType::CursorRule, cursor_validator),
    (FileType::CursorRule, prompt_validator),
    (FileType::CursorRule, claude_md_validator),
    (FileType::CursorHooks, cursor_validator),
    (FileType::CursorAgent, cursor_validator),
    (FileType::CursorEnvironment, cursor_validator),
    (FileType::CursorRulesLegacy, cursor_validator),
    (FileType::CursorRulesLegacy, prompt_validator),
    (FileType::CursorRulesLegacy, claude_md_validator),
    (FileType::ClineRules, cline_validator),
    (FileType::ClineRulesFolder, cline_validator),
    (FileType::OpenCodeConfig, opencode_validator),
    (FileType::GeminiMd, gemini_md_validator),
    (FileType::GeminiMd, prompt_validator),
    (FileType::GeminiMd, xml_validator),
    (FileType::GeminiMd, imports_validator),
    (FileType::GeminiMd, cross_platform_validator),
    (FileType::GeminiSettings, gemini_settings_validator),
    (FileType::AmpSettings, amp_validator),
    (FileType::GeminiExtension, gemini_extension_validator),
    (FileType::GeminiIgnore, gemini_ignore_validator),
    (FileType::CodexConfig, codex_validator),
    // CodexValidator on ClaudeMd catches AGENTS.override.md files (CDX-003).
    // The validator early-returns for all other ClaudeMd filenames.
    (FileType::ClaudeMd, codex_validator),
    (FileType::RooRules, roo_validator),
    (FileType::RooModes, roo_validator),
    (FileType::RooIgnore, roo_validator),
    (FileType::RooModeRules, roo_validator),
    (FileType::RooMcp, roo_validator),
    (FileType::WindsurfRule, windsurf_validator),
    (FileType::WindsurfWorkflow, windsurf_validator),
    (FileType::WindsurfRulesLegacy, windsurf_validator),
    (FileType::KiroSteering, kiro_steering_validator),
    (FileType::GenericMarkdown, cross_platform_validator),
    (FileType::GenericMarkdown, xml_validator),
    (FileType::GenericMarkdown, imports_validator),
];

// ============================================================================
// Factory functions
// ============================================================================

fn skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::skill::SkillValidator)
}

fn per_client_skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::per_client_skill::PerClientSkillValidator)
}

fn amp_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::amp::AmpValidator)
}

fn claude_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_md::ClaudeMdValidator)
}

fn agents_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agents_md::AgentsMdValidator)
}

fn agent_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::agent::AgentValidator)
}

fn hooks_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::hooks::HooksValidator)
}

fn plugin_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::plugin::PluginValidator)
}

fn mcp_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::mcp::McpValidator)
}

fn xml_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::xml::XmlValidator)
}

fn imports_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::imports::ImportsValidator)
}

fn cross_platform_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cross_platform::CrossPlatformValidator)
}

fn prompt_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::prompt::PromptValidator)
}

fn copilot_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::copilot::CopilotValidator)
}

fn claude_rules_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::claude_rules::ClaudeRulesValidator)
}

fn cursor_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cursor::CursorValidator)
}

fn cline_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cline::ClineValidator)
}

fn opencode_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::opencode::OpenCodeValidator)
}

fn gemini_md_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_md::GeminiMdValidator)
}

fn gemini_settings_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_settings::GeminiSettingsValidator)
}

fn gemini_extension_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_extension::GeminiExtensionValidator)
}

fn gemini_ignore_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::gemini_ignore::GeminiIgnoreValidator)
}

fn codex_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::codex::CodexValidator)
}

fn roo_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::roo::RooCodeValidator)
}

fn windsurf_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::windsurf::WindsurfValidator)
}

fn kiro_steering_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::kiro_steering::KiroSteeringValidator)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    // ---- BuiltinProvider tests ----

    #[test]
    fn builtin_provider_returns_expected_count() {
        let provider = BuiltinProvider;
        let entries = provider.validators();
        assert_eq!(
            entries.len(),
            DEFAULTS.len(),
            "BuiltinProvider should return the same number of entries as DEFAULTS"
        );
    }

    #[test]
    fn builtin_provider_name() {
        let provider = BuiltinProvider;
        assert_eq!(provider.name(), "BuiltinProvider");
    }

    // ---- Builder tests ----

    #[test]
    fn builder_with_defaults_matches_with_defaults() {
        let via_builder = ValidatorRegistry::builder().with_defaults().build();
        let via_direct = ValidatorRegistry::with_defaults();

        assert_eq!(
            via_builder.total_validator_count(),
            via_direct.total_validator_count(),
            "Builder with_defaults should produce the same validator count as with_defaults()"
        );
    }

    #[test]
    fn builder_empty_produces_empty_registry() {
        let registry = ValidatorRegistry::builder().build();
        assert_eq!(registry.total_validator_count(), 0);
    }

    #[test]
    fn builder_register_adds_single_factory() {
        let registry = ValidatorRegistry::builder()
            .register(FileType::Skill, skill_validator)
            .build();

        assert_eq!(registry.total_validator_count(), 1);
        let validators = registry.validators_for(FileType::Skill);
        assert_eq!(validators.len(), 1);
        assert_eq!(validators[0].name(), "SkillValidator");
    }

    #[test]
    fn builder_without_validator_disables() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("XmlValidator")
            .build();

        // XmlValidator should be excluded from Skill validators
        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(
            !names.contains(&"XmlValidator"),
            "XmlValidator should be disabled, got: {:?}",
            names
        );

        // But SkillValidator should still be present
        assert!(
            names.contains(&"SkillValidator"),
            "SkillValidator should still be present, got: {:?}",
            names
        );
    }

    // ---- Custom provider tests ----

    struct TestProvider;
    impl ValidatorProvider for TestProvider {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Skill, skill_validator)]
        }
    }

    #[test]
    fn custom_provider_adds_validators() {
        let registry = ValidatorRegistry::builder()
            .with_provider(&TestProvider)
            .build();

        assert_eq!(registry.total_validator_count(), 1);
        let validators = registry.validators_for(FileType::Skill);
        assert_eq!(validators.len(), 1);
    }

    #[test]
    fn custom_provider_name() {
        let provider = TestProvider;
        assert_eq!(provider.name(), "TestProvider");
    }

    // ---- disable_validator() direct mutation tests ----

    #[test]
    fn disable_validator_filters_from_results() {
        let mut registry = ValidatorRegistry::with_defaults();
        assert_eq!(registry.disabled_validator_count(), 0);

        registry.disable_validator("XmlValidator");
        assert_eq!(registry.disabled_validator_count(), 1);

        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(!names.contains(&"XmlValidator"));
    }

    // ---- Per-test counting validators (separate statics to avoid races) ----

    // Used by register_skips_disabled_validators
    static SKIP_COUNTING_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

    struct SkipCountingValidator;

    impl Validator for SkipCountingValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "SkipCountingValidator"
        }
    }

    fn skip_counting_validator_factory() -> Box<dyn Validator> {
        SKIP_COUNTING_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        Box::new(SkipCountingValidator)
    }

    // Used by register_calls_factory_exactly_once
    static ONCE_COUNTING_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

    struct OnceCountingValidator;

    impl Validator for OnceCountingValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "OnceCountingValidator"
        }
    }

    fn once_counting_validator_factory() -> Box<dyn Validator> {
        ONCE_COUNTING_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        Box::new(OnceCountingValidator)
    }

    // Used by register_calls_factory_exactly_once_via_builder
    static BUILDER_COUNTING_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

    struct BuilderCountingValidator;

    impl Validator for BuilderCountingValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "BuilderCountingValidator"
        }
    }

    fn builder_counting_validator_factory() -> Box<dyn Validator> {
        BUILDER_COUNTING_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        Box::new(BuilderCountingValidator)
    }

    #[test]
    fn register_skips_disabled_validators() {
        SKIP_COUNTING_CONSTRUCTED.store(0, Ordering::SeqCst);

        let registry = ValidatorRegistry::builder()
            .register(FileType::Skill, skip_counting_validator_factory)
            .without_validator("SkipCountingValidator")
            .build();

        // Factory is called once during build() (via the internal registry.register()
        // call) to obtain the instance name, but the instance is discarded
        // because the name is in the disabled set.
        assert_eq!(SKIP_COUNTING_CONSTRUCTED.load(Ordering::SeqCst), 1);

        // No cached instances remain for disabled validators.
        let validators = registry.validators_for(FileType::Skill);
        assert!(validators.is_empty());

        // validators_for() no longer calls factories - counter stays at 1.
        assert_eq!(SKIP_COUNTING_CONSTRUCTED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn disable_nonexistent_validator_is_harmless() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator("NonExistentValidator");
        assert_eq!(registry.disabled_validator_count(), 1);

        // Should still work normally
        let count_before = ValidatorRegistry::with_defaults().total_validator_count();
        assert_eq!(registry.total_validator_count(), count_before);
    }

    // ---- validators_for filtering ----

    #[test]
    fn validators_for_returns_all_when_none_disabled() {
        let registry = ValidatorRegistry::with_defaults();
        let skill_validators = registry.validators_for(FileType::Skill);
        // Skill has: SkillValidator, PerClientSkillValidator, XmlValidator, ImportsValidator
        assert_eq!(skill_validators.len(), 4);
    }

    #[test]
    fn validators_for_unknown_file_type_returns_empty() {
        let registry = ValidatorRegistry::with_defaults();
        let validators = registry.validators_for(FileType::Unknown);
        assert!(validators.is_empty());
    }

    // ---- Multiple disabled validators ----

    #[test]
    fn builder_multiple_without_validators() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("XmlValidator")
            .without_validator("PromptValidator")
            .build();

        assert_eq!(registry.disabled_validator_count(), 2);

        let skill_names: Vec<&str> = registry
            .validators_for(FileType::Skill)
            .iter()
            .map(|v| v.name())
            .collect();
        assert!(!skill_names.contains(&"XmlValidator"));

        let claude_names: Vec<&str> = registry
            .validators_for(FileType::ClaudeMd)
            .iter()
            .map(|v| v.name())
            .collect();
        assert!(!claude_names.contains(&"PromptValidator"));
        assert!(!claude_names.contains(&"XmlValidator"));
    }

    #[test]
    fn disable_all_validators_for_file_type() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator("SkillValidator")
            .without_validator("PerClientSkillValidator")
            .without_validator("XmlValidator")
            .without_validator("ImportsValidator")
            .build();

        assert!(
            registry.validators_for(FileType::Skill).is_empty(),
            "All Skill validators disabled, should return empty"
        );
    }

    #[test]
    fn disable_same_validator_twice_is_idempotent() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator("XmlValidator");
        registry.disable_validator("XmlValidator");
        assert_eq!(registry.disabled_validator_count(), 1);
    }

    #[test]
    fn disable_validator_owned_filters_from_results() {
        let mut registry = ValidatorRegistry::with_defaults();
        let name = String::from("XmlValidator");
        registry.disable_validator_owned(&name);
        assert_eq!(registry.disabled_validator_count(), 1);

        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(!names.contains(&"XmlValidator"));
    }

    #[test]
    fn disable_validator_owned_twice_is_idempotent() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator_owned("XmlValidator");
        registry.disable_validator_owned("XmlValidator");
        assert_eq!(registry.disabled_validator_count(), 1);
    }

    #[test]
    fn mixed_static_and_owned_disable() {
        let mut registry = ValidatorRegistry::with_defaults();
        registry.disable_validator("XmlValidator");
        registry.disable_validator_owned("PromptValidator");
        assert_eq!(registry.disabled_validator_count(), 2);

        let claude_validators = registry.validators_for(FileType::ClaudeMd);
        let names: Vec<&str> = claude_validators.iter().map(|v| v.name()).collect();
        assert!(!names.contains(&"XmlValidator"));
        assert!(!names.contains(&"PromptValidator"));
    }

    #[test]
    fn builder_without_validator_owned_disables() {
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .without_validator_owned("XmlValidator")
            .build();

        let skill_validators = registry.validators_for(FileType::Skill);
        let names: Vec<&str> = skill_validators.iter().map(|v| v.name()).collect();
        assert!(!names.contains(&"XmlValidator"));
    }

    // ---- Multiple providers ----

    struct ProviderA;
    impl ValidatorProvider for ProviderA {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Skill, skill_validator)]
        }
    }

    struct ProviderB;
    impl ValidatorProvider for ProviderB {
        fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
            vec![(FileType::Agent, agent_validator)]
        }
    }

    #[test]
    fn builder_multiple_providers() {
        let registry = ValidatorRegistry::builder()
            .with_provider(&ProviderA)
            .with_provider(&ProviderB)
            .build();

        assert!(!registry.validators_for(FileType::Skill).is_empty());
        assert!(!registry.validators_for(FileType::Agent).is_empty());
        assert_eq!(registry.total_validator_count(), 2);
    }

    // ---- Backward compatibility ----

    #[test]
    fn with_defaults_returns_expected_factories() {
        let registry = ValidatorRegistry::with_defaults();
        assert_eq!(
            registry.total_validator_count(),
            DEFAULTS.len(),
            "with_defaults() should register exactly as many validators as DEFAULTS"
        );
    }

    #[test]
    fn default_trait_matches_with_defaults() {
        let via_default = ValidatorRegistry::default();
        let via_explicit = ValidatorRegistry::with_defaults();
        assert_eq!(
            via_default.total_validator_count(),
            via_explicit.total_validator_count()
        );
    }

    // ---- Coverage: every validatable FileType has validators ----

    #[test]
    fn every_validatable_file_type_has_at_least_one_validator() {
        let validatable_types: [FileType; 37] = [
            FileType::Skill,
            FileType::ClaudeMd,
            FileType::Agent,
            FileType::AmpCheck,
            FileType::Hooks,
            FileType::Plugin,
            FileType::Mcp,
            FileType::Copilot,
            FileType::CopilotScoped,
            FileType::CopilotAgent,
            FileType::CopilotPrompt,
            FileType::CopilotHooks,
            FileType::ClaudeRule,
            FileType::CursorRule,
            FileType::CursorHooks,
            FileType::CursorAgent,
            FileType::CursorEnvironment,
            FileType::CursorRulesLegacy,
            FileType::ClineRules,
            FileType::ClineRulesFolder,
            FileType::OpenCodeConfig,
            FileType::GeminiMd,
            FileType::GeminiSettings,
            FileType::AmpSettings,
            FileType::GeminiExtension,
            FileType::GeminiIgnore,
            FileType::CodexConfig,
            FileType::RooRules,
            FileType::RooModes,
            FileType::RooIgnore,
            FileType::RooModeRules,
            FileType::RooMcp,
            FileType::WindsurfRule,
            FileType::WindsurfWorkflow,
            FileType::WindsurfRulesLegacy,
            FileType::KiroSteering,
            FileType::GenericMarkdown,
        ];

        // Exhaustive match with no wildcard arm - a new variant will cause a
        // compile error, forcing the developer to update this test.
        for ft in &validatable_types {
            match *ft {
                FileType::Skill
                | FileType::ClaudeMd
                | FileType::Agent
                | FileType::AmpCheck
                | FileType::Hooks
                | FileType::Plugin
                | FileType::Mcp
                | FileType::Copilot
                | FileType::CopilotScoped
                | FileType::CopilotAgent
                | FileType::CopilotPrompt
                | FileType::CopilotHooks
                | FileType::ClaudeRule
                | FileType::CursorRule
                | FileType::CursorHooks
                | FileType::CursorAgent
                | FileType::CursorEnvironment
                | FileType::CursorRulesLegacy
                | FileType::ClineRules
                | FileType::ClineRulesFolder
                | FileType::OpenCodeConfig
                | FileType::GeminiMd
                | FileType::GeminiSettings
                | FileType::AmpSettings
                | FileType::GeminiExtension
                | FileType::GeminiIgnore
                | FileType::CodexConfig
                | FileType::RooRules
                | FileType::RooModes
                | FileType::RooIgnore
                | FileType::RooModeRules
                | FileType::RooMcp
                | FileType::WindsurfRule
                | FileType::WindsurfWorkflow
                | FileType::WindsurfRulesLegacy
                | FileType::KiroSteering
                | FileType::GenericMarkdown => (),
                FileType::Unknown => {
                    panic!("Unknown must not appear in validatable_types")
                }
            }
        }

        let registry = ValidatorRegistry::with_defaults();

        for ft in &validatable_types {
            let validators = registry.validators_for(*ft);
            assert!(
                !validators.is_empty(),
                "{ft:?} has no validators registered in the default registry"
            );
        }
    }

    // ---- Caching correctness tests ----

    #[test]
    fn validators_for_returns_same_slice_on_repeated_calls() {
        let registry = ValidatorRegistry::with_defaults();
        let first = registry.validators_for(FileType::Skill);
        let second = registry.validators_for(FileType::Skill);

        // Both calls must return the same underlying slice (same pointer and length).
        assert_eq!(first.len(), second.len());
        assert!(
            std::ptr::eq(first.as_ptr(), second.as_ptr()),
            "validators_for() must return the same cached slice on repeated calls"
        );
    }

    #[test]
    fn register_calls_factory_exactly_once() {
        ONCE_COUNTING_CONSTRUCTED.store(0, Ordering::SeqCst);

        let mut registry = ValidatorRegistry::new();
        registry.register(FileType::Skill, once_counting_validator_factory);

        // Factory called exactly once during register().
        assert_eq!(ONCE_COUNTING_CONSTRUCTED.load(Ordering::SeqCst), 1);

        // validators_for() should NOT call the factory again.
        let _validators = registry.validators_for(FileType::Skill);
        assert_eq!(
            ONCE_COUNTING_CONSTRUCTED.load(Ordering::SeqCst),
            1,
            "validators_for() must not re-instantiate cached validators"
        );

        // Even repeated calls should not increment the counter.
        let _validators = registry.validators_for(FileType::Skill);
        assert_eq!(ONCE_COUNTING_CONSTRUCTED.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn register_calls_factory_exactly_once_via_builder() {
        BUILDER_COUNTING_CONSTRUCTED.store(0, Ordering::SeqCst);

        let registry = ValidatorRegistry::builder()
            .register(FileType::Skill, builder_counting_validator_factory)
            .build();

        // Factory called exactly once during build().
        assert_eq!(BUILDER_COUNTING_CONSTRUCTED.load(Ordering::SeqCst), 1);

        // validators_for() should NOT call the factory again.
        let _validators = registry.validators_for(FileType::Skill);
        assert_eq!(
            BUILDER_COUNTING_CONSTRUCTED.load(Ordering::SeqCst),
            1,
            "validators_for() must not re-instantiate cached validators via builder path"
        );
    }

    #[test]
    fn disable_after_construction_removes_from_cache() {
        let mut registry = ValidatorRegistry::with_defaults();
        let total_before = registry.total_validator_count();

        // Verify XmlValidator is present before disabling.
        let before = registry.validators_for(FileType::Skill);
        assert!(
            before.iter().any(|v| v.name() == "XmlValidator"),
            "XmlValidator should be present before disabling"
        );

        registry.disable_validator("XmlValidator");

        // After disabling, XmlValidator must be absent from the cached slice.
        let after = registry.validators_for(FileType::Skill);
        assert!(
            !after.iter().any(|v| v.name() == "XmlValidator"),
            "XmlValidator should be removed after disable_validator()"
        );

        // Also absent from other file types that had XmlValidator.
        let claude_after = registry.validators_for(FileType::ClaudeMd);
        assert!(
            !claude_after.iter().any(|v| v.name() == "XmlValidator"),
            "XmlValidator should be removed from all file types"
        );

        // XmlValidator appears in 9 file types in DEFAULTS. Count via function
        // pointer comparison (no allocations) and verify the total decreases
        // by exactly that amount.
        let xml_occurrences_in_defaults = DEFAULTS
            .iter()
            .filter(|(_, factory)| *factory as usize == xml_validator as usize)
            .count();
        assert_eq!(
            xml_occurrences_in_defaults, 9,
            "Expected XmlValidator in 9 DEFAULTS entries"
        );
        let total_after = registry.total_validator_count();
        assert_eq!(
            total_before - total_after,
            xml_occurrences_in_defaults,
            "Disabling XmlValidator should remove exactly {} instances, \
             but removed {}",
            xml_occurrences_in_defaults,
            total_before - total_after
        );
    }

    #[test]
    fn registry_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ValidatorRegistry>();
    }

    #[test]
    #[allow(deprecated)]
    fn deprecated_total_factory_count_matches_total_validator_count() {
        let registry = ValidatorRegistry::with_defaults();
        assert_eq!(
            registry.total_factory_count(),
            registry.total_validator_count()
        );
    }
}
