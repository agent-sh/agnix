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

    /// Return validator factories with optional static names.
    ///
    /// This is a **performance optimization hook**, not a rename mechanism.
    /// When a name is `Some(name)`, the registry can skip calling `factory()`
    /// entirely for disabled validators, avoiding the heap allocation that
    /// would otherwise be needed just to read the validator's name.
    ///
    /// # Name invariant
    ///
    /// Each `Some(name)` **must** equal the value returned by `factory().name()`.
    /// Violating this silently breaks the disabled-validator mechanism:
    /// `register_named()` checks the static name against the disabled set, so a
    /// mismatch causes the wrong validator to be excluded or allows a disabled
    /// validator to slip through undetected. In debug builds, a
    /// `#[cfg(debug_assertions)]` check inside `register_named()` catches this
    /// early with zero overhead in release builds.
    ///
    /// # Default implementation
    ///
    /// The default implementation delegates to
    /// [`validators()`](ValidatorProvider::validators) and maps each entry
    /// into a `(FileType, None, factory)` tuple, incurring an extra allocation
    /// compared to a direct override. Providers that know their validator names
    /// at compile time should override this method and return `Some(name)` for
    /// each entry to avoid the overhead.
    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        self.validators()
            .into_iter()
            .map(|(ft, f)| (ft, None, f))
            .collect()
    }
}

/// The built-in validator provider shipping with agnix-core.
///
/// Contains all built-in validators across all supported file types. Used
/// internally by [`ValidatorRegistry::with_defaults`] and
/// [`ValidatorRegistryBuilder::with_defaults`].
pub(crate) struct BuiltinProvider;

impl ValidatorProvider for BuiltinProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        let providers: &[&dyn ValidatorProvider] = &[
            &SkillProvider,
            &ClaudeProvider,
            &CopilotProvider,
            &CursorProvider,
            &GeminiProvider,
            &RooProvider,
            &WindsurfProvider,
            &MiscProvider,
        ];
        let result: Vec<_> = providers
            .iter()
            .flat_map(|p| p.named_validators())
            .collect();
        debug_assert_eq!(
            result.len(),
            EXPECTED_BUILTIN_COUNT,
            "BuiltinProvider produced {} entries but expected {}",
            result.len(),
            EXPECTED_BUILTIN_COUNT
        );
        result
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
    /// validator name). For built-in validators registered via
    /// [`with_defaults()`](ValidatorRegistry::with_defaults), this factory
    /// call is avoided automatically using static names.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) {
        let instance = factory();
        if self.disabled_validators.contains(instance.name() as &str) {
            return;
        }
        self.validators.entry(file_type).or_default().push(instance);
    }

    /// Register a validator factory whose name is already known.
    ///
    /// If `name` appears in the disabled set, the factory is never called,
    /// avoiding the allocation entirely. This is the fast path used by
    /// `register_defaults()` for built-in validators.
    ///
    /// In debug builds, a `#[cfg(debug_assertions)]` block verifies that `name`
    /// matches `factory().name()`. The check is compiled out entirely in release
    /// builds, so calling `instance.name()` - a vtable dispatch on
    /// `Box<dyn Validator>` - incurs zero overhead in production. A mismatch
    /// means the static name passed to
    /// [`named_validators()`](ValidatorProvider::named_validators) is wrong,
    /// which silently breaks the disabled-validator mechanism.
    fn register_named(&mut self, file_type: FileType, name: &str, factory: ValidatorFactory) {
        if self.disabled_validators.contains(name) {
            return;
        }
        let instance = factory();
        #[cfg(debug_assertions)]
        {
            let runtime_name = instance.name();
            assert_eq!(
                name, runtime_name,
                "ValidatorProvider name/factory mismatch: static name \"{name}\" \
                 does not match factory().name() \"{runtime_name}\". The static name \
                 passed to named_validators() must equal the value returned by \
                 Validator::name().",
            );
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
        for (file_type, name, factory) in BuiltinProvider.named_validators() {
            match name {
                Some(n) => self.register_named(file_type, n, factory),
                None => self.register(file_type, factory),
            }
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
    entries: Vec<(FileType, Option<&'static str>, ValidatorFactory)>,
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
        self.entries.extend(provider.named_validators());
        self
    }

    /// Register a single validator factory for a file type.
    pub fn register(&mut self, file_type: FileType, factory: ValidatorFactory) -> &mut Self {
        self.entries.push((file_type, None, factory));
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
    /// [`std::mem::take`]), but the entries list is preserved so all
    /// non-disabled factories are re-called. Reuse a builder by calling
    /// configuration methods again before a subsequent `build()`.
    ///
    /// For entries added via `with_defaults()` or any provider that overrides
    /// `named_validators()`, disabled validators skip the factory call
    /// entirely. Entries added via `register()` always call the factory once
    /// to obtain the name.
    pub fn build(&mut self) -> ValidatorRegistry {
        let mut registry = ValidatorRegistry {
            validators: HashMap::new(),
            disabled_validators: std::mem::take(&mut self.disabled_validators),
        };
        for &(file_type, name, factory) in &self.entries {
            match name {
                Some(n) => registry.register_named(file_type, n, factory),
                None => registry.register(file_type, factory),
            }
        }
        registry
    }
}

// ============================================================================
// Built-in defaults
// ============================================================================

/// Expected number of validator registrations across all built-in providers.
///
/// Used by `BuiltinProvider` (via `debug_assert_eq!`) and tests to catch
/// accidental additions or removals without updating all providers.
const EXPECTED_BUILTIN_COUNT: usize = 72;

// -- Category providers -----------------------------------------------------
//
// Each struct groups validators for a related family of file types and
// implements `ValidatorProvider` with `named_validators()` returning
// `Some(name)` for the fast-path optimization (skip factory call for
// disabled validators).

/// Skill file validators.
struct SkillProvider;

impl ValidatorProvider for SkillProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (FileType::Skill, Some("SkillValidator"), skill_validator),
            (
                FileType::Skill,
                Some("PerClientSkillValidator"),
                per_client_skill_validator,
            ),
            (FileType::Skill, Some("XmlValidator"), xml_validator),
            (FileType::Skill, Some("ImportsValidator"), imports_validator),
        ]
    }
}

/// Claude family validators: ClaudeMd, Agent, ClaudeRule.
struct ClaudeProvider;

impl ValidatorProvider for ClaudeProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (
                FileType::ClaudeMd,
                Some("ClaudeMdValidator"),
                claude_md_validator,
            ),
            (
                FileType::ClaudeMd,
                Some("CrossPlatformValidator"),
                cross_platform_validator,
            ),
            (
                FileType::ClaudeMd,
                Some("AgentsMdValidator"),
                agents_md_validator,
            ),
            (FileType::ClaudeMd, Some("AmpValidator"), amp_validator),
            (FileType::ClaudeMd, Some("XmlValidator"), xml_validator),
            (
                FileType::ClaudeMd,
                Some("ImportsValidator"),
                imports_validator,
            ),
            (
                FileType::ClaudeMd,
                Some("PromptValidator"),
                prompt_validator,
            ),
            // CodexValidator on ClaudeMd catches AGENTS.override.md files (CDX-003).
            // The validator early-returns for all other ClaudeMd filenames.
            (FileType::ClaudeMd, Some("CodexValidator"), codex_validator),
            (FileType::Agent, Some("AgentValidator"), agent_validator),
            (FileType::Agent, Some("XmlValidator"), xml_validator),
            (
                FileType::ClaudeRule,
                Some("ClaudeRulesValidator"),
                claude_rules_validator,
            ),
        ]
    }
}

/// Copilot family validators.
struct CopilotProvider;

impl ValidatorProvider for CopilotProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (
                FileType::Copilot,
                Some("CopilotValidator"),
                copilot_validator,
            ),
            (FileType::Copilot, Some("XmlValidator"), xml_validator),
            (
                FileType::CopilotScoped,
                Some("CopilotValidator"),
                copilot_validator,
            ),
            (FileType::CopilotScoped, Some("XmlValidator"), xml_validator),
            (
                FileType::CopilotAgent,
                Some("CopilotValidator"),
                copilot_validator,
            ),
            (FileType::CopilotAgent, Some("XmlValidator"), xml_validator),
            (
                FileType::CopilotPrompt,
                Some("CopilotValidator"),
                copilot_validator,
            ),
            (FileType::CopilotPrompt, Some("XmlValidator"), xml_validator),
            (
                FileType::CopilotHooks,
                Some("CopilotValidator"),
                copilot_validator,
            ),
        ]
    }
}

/// Cursor family validators.
struct CursorProvider;

impl ValidatorProvider for CursorProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (
                FileType::CursorRule,
                Some("CursorValidator"),
                cursor_validator,
            ),
            (
                FileType::CursorRule,
                Some("PromptValidator"),
                prompt_validator,
            ),
            (
                FileType::CursorRule,
                Some("ClaudeMdValidator"),
                claude_md_validator,
            ),
            (
                FileType::CursorHooks,
                Some("CursorValidator"),
                cursor_validator,
            ),
            (
                FileType::CursorAgent,
                Some("CursorValidator"),
                cursor_validator,
            ),
            (
                FileType::CursorEnvironment,
                Some("CursorValidator"),
                cursor_validator,
            ),
            (
                FileType::CursorRulesLegacy,
                Some("CursorValidator"),
                cursor_validator,
            ),
            (
                FileType::CursorRulesLegacy,
                Some("PromptValidator"),
                prompt_validator,
            ),
            (
                FileType::CursorRulesLegacy,
                Some("ClaudeMdValidator"),
                claude_md_validator,
            ),
        ]
    }
}

/// Gemini family validators.
struct GeminiProvider;

impl ValidatorProvider for GeminiProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (
                FileType::GeminiMd,
                Some("GeminiMdValidator"),
                gemini_md_validator,
            ),
            (
                FileType::GeminiMd,
                Some("PromptValidator"),
                prompt_validator,
            ),
            (FileType::GeminiMd, Some("XmlValidator"), xml_validator),
            (
                FileType::GeminiMd,
                Some("ImportsValidator"),
                imports_validator,
            ),
            (
                FileType::GeminiMd,
                Some("CrossPlatformValidator"),
                cross_platform_validator,
            ),
            (
                FileType::GeminiSettings,
                Some("GeminiSettingsValidator"),
                gemini_settings_validator,
            ),
            (
                FileType::GeminiExtension,
                Some("GeminiExtensionValidator"),
                gemini_extension_validator,
            ),
            (
                FileType::GeminiIgnore,
                Some("GeminiIgnoreValidator"),
                gemini_ignore_validator,
            ),
        ]
    }
}

/// Roo Code validators.
struct RooProvider;

impl ValidatorProvider for RooProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (FileType::RooRules, Some("RooCodeValidator"), roo_validator),
            (FileType::RooModes, Some("RooCodeValidator"), roo_validator),
            (FileType::RooIgnore, Some("RooCodeValidator"), roo_validator),
            (
                FileType::RooModeRules,
                Some("RooCodeValidator"),
                roo_validator,
            ),
            (FileType::RooMcp, Some("RooCodeValidator"), roo_validator),
        ]
    }
}

/// Windsurf validators.
struct WindsurfProvider;

impl ValidatorProvider for WindsurfProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (
                FileType::WindsurfRule,
                Some("WindsurfValidator"),
                windsurf_validator,
            ),
            (
                FileType::WindsurfWorkflow,
                Some("WindsurfValidator"),
                windsurf_validator,
            ),
            (
                FileType::WindsurfRulesLegacy,
                Some("WindsurfValidator"),
                windsurf_validator,
            ),
        ]
    }
}

/// Miscellaneous validators that do not belong to a larger family.
struct MiscProvider;

impl ValidatorProvider for MiscProvider {
    fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
        self.named_validators()
            .into_iter()
            .map(|(ft, _, f)| (ft, f))
            .collect()
    }

    fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
        vec![
            (FileType::AmpCheck, Some("AmpValidator"), amp_validator),
            (FileType::Hooks, Some("HooksValidator"), hooks_validator),
            (FileType::Plugin, Some("PluginValidator"), plugin_validator),
            (FileType::Mcp, Some("McpValidator"), mcp_validator),
            (
                FileType::ClineRules,
                Some("ClineValidator"),
                cline_validator,
            ),
            (
                FileType::ClineRulesFolder,
                Some("ClineValidator"),
                cline_validator,
            ),
            (
                FileType::ClineRulesFolder,
                Some("ClineSkillValidator"),
                cline_skill_validator,
            ),
            (
                FileType::OpenCodeConfig,
                Some("OpenCodeValidator"),
                opencode_validator,
            ),
            (FileType::AmpSettings, Some("AmpValidator"), amp_validator),
            (
                FileType::CodexConfig,
                Some("CodexConfigValidator"),
                codex_config_validator,
            ),
            (
                FileType::KiroSteering,
                Some("KiroSteeringValidator"),
                kiro_steering_validator,
            ),
            (
                FileType::KiroPower,
                Some("KiroPowerValidator"),
                kiro_power_validator,
            ),
            (
                FileType::KiroPower,
                Some("ImportsValidator"),
                imports_validator,
            ),
            (
                FileType::KiroPower,
                Some("CrossPlatformValidator"),
                cross_platform_validator,
            ),
            (FileType::KiroPower, Some("XmlValidator"), xml_validator),
            (
                FileType::KiroAgent,
                Some("KiroAgentValidator"),
                kiro_agent_validator,
            ),
            (
                FileType::KiroAgent,
                Some("ImportsValidator"),
                imports_validator,
            ),
            (
                FileType::KiroHook,
                Some("KiroHookValidator"),
                kiro_hook_validator,
            ),
            (
                FileType::KiroHook,
                Some("ImportsValidator"),
                imports_validator,
            ),
            (
                FileType::KiroMcp,
                Some("KiroMcpValidator"),
                kiro_mcp_validator,
            ),
            (
                FileType::GenericMarkdown,
                Some("CrossPlatformValidator"),
                cross_platform_validator,
            ),
            (
                FileType::GenericMarkdown,
                Some("XmlValidator"),
                xml_validator,
            ),
            (
                FileType::GenericMarkdown,
                Some("ImportsValidator"),
                imports_validator,
            ),
        ]
    }
}

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

fn cline_skill_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::cline::ClineSkillValidator)
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

fn codex_config_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::codex::CodexConfigValidator)
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

fn kiro_agent_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::kiro_agent::KiroAgentValidator)
}

fn kiro_power_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::kiro_power::KiroPowerValidator)
}

fn kiro_hook_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::kiro_hook::KiroHookValidator)
}

fn kiro_mcp_validator() -> Box<dyn Validator> {
    Box::new(crate::rules::kiro_mcp::KiroMcpValidator)
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
            EXPECTED_BUILTIN_COUNT,
            "BuiltinProvider should return the same number of entries as EXPECTED_BUILTIN_COUNT"
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

        // This exercises the slow (unnamed) path: builder.register() stores None
        // for the name, so build() calls registry.register() which always calls
        // the factory once to obtain the name. The instance is then discarded.
        // Contrast with named_disabled_validator_skips_factory_call which uses
        // the fast path (Some(name)) and asserts 0 factory calls.
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
            EXPECTED_BUILTIN_COUNT,
            "with_defaults() should register exactly as many validators as EXPECTED_BUILTIN_COUNT"
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
        let validatable_types: [FileType; 41] = [
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
            FileType::KiroPower,
            FileType::KiroAgent,
            FileType::KiroHook,
            FileType::KiroMcp,
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
                | FileType::KiroPower
                | FileType::KiroAgent
                | FileType::KiroHook
                | FileType::KiroMcp
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

    #[test]
    fn kiro_file_types_route_to_expected_validators() {
        let registry = ValidatorRegistry::with_defaults();

        let names_for = |file_type: FileType| -> Vec<&'static str> {
            registry
                .validators_for(file_type)
                .iter()
                .map(|validator| validator.name())
                .collect()
        };

        assert_eq!(
            names_for(FileType::KiroPower),
            vec![
                "KiroPowerValidator",
                "ImportsValidator",
                "CrossPlatformValidator",
                "XmlValidator"
            ]
        );
        assert_eq!(
            names_for(FileType::KiroAgent),
            vec!["KiroAgentValidator", "ImportsValidator"]
        );
        assert_eq!(
            names_for(FileType::KiroHook),
            vec!["KiroHookValidator", "ImportsValidator"]
        );
        assert_eq!(names_for(FileType::KiroMcp), vec!["KiroMcpValidator"]);
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

        // XmlValidator appears in 10 file types across built-in providers. Count
        // via the static names and verify the total decreases by exactly that
        // amount.
        let xml_occurrences = BuiltinProvider
            .named_validators()
            .iter()
            .filter(|(_, name, _)| *name == Some("XmlValidator"))
            .count();
        assert_eq!(
            xml_occurrences, 10,
            "Expected XmlValidator in 10 BuiltinProvider entries"
        );
        let total_after = registry.total_validator_count();
        assert_eq!(
            total_before - total_after,
            xml_occurrences,
            "Disabling XmlValidator should remove exactly {} instances, \
             but removed {}",
            xml_occurrences,
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

    // ---- Named registration tests ----

    #[test]
    fn defaults_names_match_factory_names() {
        // Every static name in BuiltinProvider must exactly match the name
        // returned by the factory-produced instance. A mismatch would silently
        // break the disabled-validator fast path.
        for (file_type, static_name, factory) in BuiltinProvider.named_validators() {
            let static_name = static_name.expect("BuiltinProvider entries must have Some(name)");
            let instance = factory();
            let runtime_name = instance.name();
            assert_eq!(
                static_name, runtime_name,
                "BuiltinProvider name mismatch for {file_type:?}: \
                 static=\"{static_name}\" vs runtime=\"{runtime_name}\""
            );
        }
    }

    // Used by named_disabled_validator_skips_factory_call
    static NAMED_SKIP_COUNTING_CONSTRUCTED: AtomicUsize = AtomicUsize::new(0);

    struct NamedSkipCountingValidator;

    impl Validator for NamedSkipCountingValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            Vec::new()
        }

        fn name(&self) -> &'static str {
            "NamedSkipCountingValidator"
        }
    }

    fn named_skip_counting_validator_factory() -> Box<dyn Validator> {
        NAMED_SKIP_COUNTING_CONSTRUCTED.fetch_add(1, Ordering::SeqCst);
        Box::new(NamedSkipCountingValidator)
    }

    #[test]
    fn named_disabled_validator_skips_factory_call() {
        // Uses a named provider so the builder stores Some("NamedSkipCountingValidator"),
        // routing through register_named() in build(). The factory must not be called.
        NAMED_SKIP_COUNTING_CONSTRUCTED.store(0, Ordering::SeqCst);

        struct NamedCountingProvider;
        impl ValidatorProvider for NamedCountingProvider {
            fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
                vec![(FileType::Skill, named_skip_counting_validator_factory)]
            }

            fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
                vec![(
                    FileType::Skill,
                    Some("NamedSkipCountingValidator"),
                    named_skip_counting_validator_factory,
                )]
            }
        }

        let registry = ValidatorRegistry::builder()
            .with_provider(&NamedCountingProvider)
            .without_validator("NamedSkipCountingValidator")
            .build();

        // Factory must NOT have been called - that is the whole point of
        // register_named: skip allocation for disabled validators.
        assert_eq!(
            NAMED_SKIP_COUNTING_CONSTRUCTED.load(Ordering::SeqCst),
            0,
            "register_named must not call factory for disabled validators"
        );

        // No cached instances for this type.
        assert!(registry.validators_for(FileType::Skill).is_empty());
    }

    #[test]
    fn builtin_provider_named_validators_returns_all_names() {
        let provider = BuiltinProvider;
        let named = provider.named_validators();

        assert_eq!(
            named.len(),
            EXPECTED_BUILTIN_COUNT,
            "named_validators() should return EXPECTED_BUILTIN_COUNT entries"
        );

        // Every entry must have Some(name).
        for (i, (ft, name, _factory)) in named.iter().enumerate() {
            assert!(
                name.is_some(),
                "Entry {i} ({ft:?}) should have Some(name), got None"
            );
        }

        // Self-consistency: validators() and named_validators() must agree on
        // count and file types.
        let unnamed = provider.validators();
        assert_eq!(
            unnamed.len(),
            named.len(),
            "validators() and named_validators() must return the same count"
        );
        for ((ft_unnamed, _), (ft_named, _, _)) in unnamed.iter().zip(named.iter()) {
            assert_eq!(
                ft_unnamed, ft_named,
                "validators() and named_validators() file types must match"
            );
        }
    }

    #[test]
    fn custom_provider_named_validators_defaults_to_none() {
        // A provider that only implements validators() should get None names
        // from the default named_validators() implementation.
        let provider = TestProvider;
        let named = provider.named_validators();

        assert_eq!(named.len(), 1);
        let (ft, name, _factory) = &named[0];
        assert_eq!(*ft, FileType::Skill);
        assert!(
            name.is_none(),
            "Default named_validators() should yield None names"
        );
    }

    // ---- Name/factory mismatch tests ----

    // Validator whose name() returns "ActualName", used to demonstrate the
    // mismatch when a provider declares the static name as "WrongName".
    struct MismatchedValidator;

    impl Validator for MismatchedValidator {
        fn validate(
            &self,
            _path: &std::path::Path,
            _content: &str,
            _config: &crate::config::LintConfig,
        ) -> Vec<crate::diagnostics::Diagnostic> {
            vec![]
        }

        fn name(&self) -> &'static str {
            "ActualName"
        }
    }

    fn mismatched_validator_factory() -> Box<dyn Validator> {
        Box::new(MismatchedValidator)
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "name/factory mismatch")]
    fn mismatched_named_validator_panics_in_debug() {
        // Provider declares the static name as "WrongName" but the factory
        // produces a validator with name() = "ActualName". The debug_assert_eq!
        // inside register_named() must catch this and panic.
        struct MismatchedProvider;
        impl ValidatorProvider for MismatchedProvider {
            fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
                // Required by the trait; not exercised by this test path.
                vec![(FileType::Skill, mismatched_validator_factory)]
            }
            fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
                vec![(
                    FileType::Skill,
                    Some("WrongName"),
                    mismatched_validator_factory,
                )]
            }
        }

        // Building the registry triggers register_named() which hits the
        // debug_assert_eq! because "WrongName" != "ActualName".
        let _registry = ValidatorRegistry::builder()
            .with_provider(&MismatchedProvider)
            .build();
    }

    // Not cfg(debug_assertions)-gated: the factory-skip path (early return
    // when the static name is in the disabled set) is taken before the
    // debug_assert_eq! can fire, so this test holds in both debug and release.
    #[test]
    fn mismatched_named_validator_silently_skips_when_disabled() {
        // Same mismatch as above, but "WrongName" is in the disabled set.
        // Because register_named() checks the static name against the disabled
        // set before calling factory(), the factory is never called, so the
        // debug_assert never fires. However, the validator with the actual
        // name "ActualName" is also NOT registered - demonstrating the
        // silent-skip failure mode that the invariant documentation warns about.
        struct MismatchedProvider;
        impl ValidatorProvider for MismatchedProvider {
            fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
                vec![(FileType::Skill, mismatched_validator_factory)]
            }
            fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
                vec![(
                    FileType::Skill,
                    Some("WrongName"),
                    mismatched_validator_factory,
                )]
            }
        }

        let registry = ValidatorRegistry::builder()
            .with_provider(&MismatchedProvider)
            .without_validator("WrongName")
            .build();

        // The factory was never called because "WrongName" matched the
        // disabled set. No validators are registered at all.
        assert!(
            registry.validators_for(FileType::Skill).is_empty(),
            "Mismatched static name caused silent skip - no validators registered"
        );
    }

    // This test only runs in release mode: in debug builds the debug_assert_eq!
    // inside register_named() would panic when the factory is called (because
    // "WrongName" is not in the disabled set), masking the slip-through behavior.
    #[cfg(not(debug_assertions))]
    #[test]
    fn mismatched_named_validator_slip_through_when_real_name_disabled() {
        // The dangerous half of the failure mode: the user tries to disable
        // "ActualName" (the real validator name), but the provider declared the
        // static name as "WrongName". register_named() checks "WrongName"
        // against the disabled set - no match - so the factory is called and
        // the validator is registered despite the disable request.
        struct MismatchedProvider;
        impl ValidatorProvider for MismatchedProvider {
            fn validators(&self) -> Vec<(FileType, ValidatorFactory)> {
                // Required by the trait; not exercised by this test path.
                vec![(FileType::Skill, mismatched_validator_factory)]
            }
            fn named_validators(&self) -> Vec<(FileType, Option<&'static str>, ValidatorFactory)> {
                vec![(
                    FileType::Skill,
                    Some("WrongName"),
                    mismatched_validator_factory,
                )]
            }
        }

        let registry = ValidatorRegistry::builder()
            .with_provider(&MismatchedProvider)
            .without_validator("ActualName") // The real name - disable attempt
            .build();

        // The validator slipped through: register_named() checked "WrongName"
        // against the disabled set (which contains "ActualName"), found no
        // match, called the factory, and registered the validator - even though
        // the user intended to disable it.
        let slipped_through = registry.validators_for(FileType::Skill);
        assert_eq!(
            slipped_through.len(),
            1,
            "Mismatched static name caused validator to slip through despite disable request"
        );
        assert_eq!(
            slipped_through[0].name(),
            "ActualName",
            "The registered validator must be the mismatched one"
        );
    }

    // ---- Category provider tests ----

    #[test]
    fn skill_provider_count() {
        assert_eq!(SkillProvider.named_validators().len(), 4);
    }

    #[test]
    fn claude_provider_count() {
        assert_eq!(ClaudeProvider.named_validators().len(), 11);
    }

    #[test]
    fn copilot_provider_count() {
        assert_eq!(CopilotProvider.named_validators().len(), 9);
    }

    #[test]
    fn cursor_provider_count() {
        assert_eq!(CursorProvider.named_validators().len(), 9);
    }

    #[test]
    fn gemini_provider_count() {
        assert_eq!(GeminiProvider.named_validators().len(), 8);
    }

    #[test]
    fn roo_provider_count() {
        assert_eq!(RooProvider.named_validators().len(), 5);
    }

    #[test]
    fn windsurf_provider_count() {
        assert_eq!(WindsurfProvider.named_validators().len(), 3);
    }

    #[test]
    fn misc_provider_count() {
        assert_eq!(MiscProvider.named_validators().len(), 23);
    }

    #[test]
    fn all_category_providers_sum_to_expected_count() {
        let total = SkillProvider.named_validators().len()
            + ClaudeProvider.named_validators().len()
            + CopilotProvider.named_validators().len()
            + CursorProvider.named_validators().len()
            + GeminiProvider.named_validators().len()
            + RooProvider.named_validators().len()
            + WindsurfProvider.named_validators().len()
            + MiscProvider.named_validators().len();
        assert_eq!(
            total, EXPECTED_BUILTIN_COUNT,
            "Sum of all category provider counts must equal EXPECTED_BUILTIN_COUNT"
        );
    }

    #[test]
    fn all_category_provider_entries_have_names() {
        let providers: &[&dyn ValidatorProvider] = &[
            &SkillProvider,
            &ClaudeProvider,
            &CopilotProvider,
            &CursorProvider,
            &GeminiProvider,
            &RooProvider,
            &WindsurfProvider,
            &MiscProvider,
        ];
        for provider in providers {
            for (i, (ft, name, _)) in provider.named_validators().iter().enumerate() {
                assert!(
                    name.is_some(),
                    "{}: entry {i} ({ft:?}) should have Some(name), got None",
                    provider.name()
                );
            }
        }
    }

    #[test]
    fn category_provider_validators_count_matches_named() {
        let providers: &[&dyn ValidatorProvider] = &[
            &SkillProvider,
            &ClaudeProvider,
            &CopilotProvider,
            &CursorProvider,
            &GeminiProvider,
            &RooProvider,
            &WindsurfProvider,
            &MiscProvider,
        ];
        for provider in providers {
            assert_eq!(
                provider.validators().len(),
                provider.named_validators().len(),
                "{}: validators() and named_validators() counts must match",
                provider.name()
            );
        }
    }

    #[test]
    fn claude_provider_includes_codex_on_claude_md() {
        // CDX-003: CodexValidator on ClaudeMd catches AGENTS.override.md files.
        let entries = ClaudeProvider.named_validators();
        let has_codex_on_claude_md = entries
            .iter()
            .any(|(ft, name, _)| *ft == FileType::ClaudeMd && *name == Some("CodexValidator"));
        assert!(
            has_codex_on_claude_md,
            "ClaudeProvider must include CodexValidator on ClaudeMd (CDX-003)"
        );
    }

    #[test]
    fn codex_validator_only_on_expected_file_types() {
        // CodexValidator must only appear on ClaudeMd (CDX-003 and CDX-AG-*).
        let entries = BuiltinProvider.named_validators();
        for (ft, name, _) in &entries {
            if *name == Some("CodexValidator") {
                assert!(
                    *ft == FileType::ClaudeMd,
                    "CodexValidator must only be registered for ClaudeMd, found {:?}",
                    ft
                );
            }
        }
        let codex_count = entries
            .iter()
            .filter(|(_, name, _)| *name == Some("CodexValidator"))
            .count();
        assert_eq!(
            codex_count, 1,
            "CodexValidator should appear exactly once (ClaudeMd)"
        );
    }

    #[test]
    fn codex_config_validator_only_on_codex_config() {
        let entries = BuiltinProvider.named_validators();
        for (ft, name, _) in &entries {
            if *name == Some("CodexConfigValidator") {
                assert_eq!(
                    *ft,
                    FileType::CodexConfig,
                    "CodexConfigValidator must only be registered for CodexConfig"
                );
            }
        }
        let count = entries
            .iter()
            .filter(|(_, name, _)| *name == Some("CodexConfigValidator"))
            .count();
        assert_eq!(
            count, 1,
            "CodexConfigValidator should appear exactly once (CodexConfig)"
        );
    }

    #[test]
    fn builder_second_build_has_no_disabled_validators() {
        // build() consumes the disabled set via mem::take. A second call
        // produces a registry where previously-disabled validators are active.
        let mut builder = ValidatorRegistry::builder();
        builder.with_defaults().without_validator("XmlValidator");

        let first = builder.build();
        let second = builder.build();

        // XmlValidator appears 9 times in the default set; first registry has them removed.
        let xml_count = BuiltinProvider
            .named_validators()
            .iter()
            .filter(|(_, name, _)| *name == Some("XmlValidator"))
            .count();
        assert_eq!(
            second.total_validator_count() - first.total_validator_count(),
            xml_count,
            "First registry should have exactly {xml_count} fewer validators (one per XmlValidator registration)"
        );
        // Second registry has the full count because the disabled set was consumed.
        assert_eq!(
            second.total_validator_count(),
            EXPECTED_BUILTIN_COUNT,
            "Second build() must produce a full registry (disabled set was consumed by first build)"
        );
    }

    #[test]
    fn builtin_provider_output_matches_sub_provider_concatenation() {
        // BuiltinProvider::named_validators() must be a pure flat_map of all
        // 8 sub-providers in declaration order with no reordering or deduplication.
        let expected: Vec<_> = [
            SkillProvider.named_validators(),
            ClaudeProvider.named_validators(),
            CopilotProvider.named_validators(),
            CursorProvider.named_validators(),
            GeminiProvider.named_validators(),
            RooProvider.named_validators(),
            WindsurfProvider.named_validators(),
            MiscProvider.named_validators(),
        ]
        .into_iter()
        .flatten()
        .collect();

        let actual = BuiltinProvider.named_validators();

        assert_eq!(
            actual.len(),
            expected.len(),
            "BuiltinProvider entry count must equal sum of sub-providers"
        );
        for (i, ((aft, aname, _), (eft, ename, _))) in
            actual.iter().zip(expected.iter()).enumerate()
        {
            assert_eq!(
                aft, eft,
                "Entry {i}: file type mismatch (actual={aft:?}, expected={eft:?})"
            );
            assert_eq!(
                aname, ename,
                "Entry {i}: name mismatch (actual={aname:?}, expected={ename:?})"
            );
        }
    }

    #[test]
    fn builder_with_defaults_called_twice_registers_all_validators_twice() {
        // with_defaults() is additive. Calling it twice duplicates every
        // built-in validator. This is the documented behaviour (see
        // ValidatorRegistryBuilder::with_defaults doc comment).
        let registry = ValidatorRegistry::builder()
            .with_defaults()
            .with_defaults()
            .build();
        assert_eq!(
            registry.total_validator_count(),
            EXPECTED_BUILTIN_COUNT * 2,
            "Calling with_defaults() twice must register all validators twice"
        );
    }
}
