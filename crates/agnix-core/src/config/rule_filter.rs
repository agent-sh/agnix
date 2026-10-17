use super::*;
use crate::pipeline::{FILES_MATCH_OPTIONS, normalize_rel_path};
use std::ops::Deref;

/// Rule filtering logic encapsulated for clarity.
///
/// This trait and its implementation extract the rule enablement logic
/// from LintConfig, making it easier to test and maintain.
trait RuleFilter {
    /// Check if a specific rule is enabled based on config.
    fn is_rule_enabled(&self, rule_id: &str) -> bool;
}

/// Default implementation of rule filtering logic.
///
/// Determines whether a rule is enabled based on:
/// 1. Explicit disabled_rules list
/// 2. Target tool or tools array filtering
/// 3. Category enablement flags
struct DefaultRuleFilter<'a> {
    rules: &'a RuleConfig,
    target: TargetTool,
    tools: &'a [String],
}

impl<'a> DefaultRuleFilter<'a> {
    fn new(rules: &'a RuleConfig, target: TargetTool, tools: &'a [String]) -> Self {
        Self {
            rules,
            target,
            tools,
        }
    }

    /// Check if a rule applies to the current target tool(s)
    fn is_rule_for_target(&self, rule_id: &str) -> bool {
        // If tools array is specified, use it for filtering
        if !self.tools.is_empty() {
            return Self::is_rule_for_tool(rule_id, |tool| {
                // Case-insensitive, and accept backward-compat aliases
                // (e.g., "copilot" for "github-copilot")
                self.tools
                    .iter()
                    .any(|t| t.eq_ignore_ascii_case(tool) || Self::is_tool_alias(t, tool))
            });
        }

        // A specific legacy target filters exactly like `tools = ["<tool>"]`:
        // `target = "cursor"` used to disable only CC-* rules, so it still ran
        // Codex, Copilot and Kiro rules. Generic selects every rule.
        match self.target.tool_name() {
            Some(name) => Self::is_rule_for_tool(rule_id, |tool| tool == name),
            None => true,
        }
    }

    /// Check if a rule applies to the selected tools: a tool-specific rule (see
    /// TOOL_RULE_PREFIXES) applies only when `selected` accepts its tool.
    fn is_rule_for_tool(rule_id: &str, selected: impl Fn(&str) -> bool) -> bool {
        for (prefix, tool) in agnix_rules::TOOL_RULE_PREFIXES {
            if rule_id.starts_with(prefix) {
                return selected(tool);
            }
        }

        // CC-* rules are Claude Code's by naming convention, also in a family the
        // prefix table leaves out because one rule names no tool (CC-MEM-007).
        if rule_id.starts_with("CC-") {
            return selected("claude-code");
        }

        // Generic rules (AS-*, XML-*, REF-*, XP-*, AGM-*, MCP-*, PE-*) apply to all tools
        true
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        // Backward compatibility: accept short names as aliases
        match canonical_tool {
            "github-copilot" => user_tool.eq_ignore_ascii_case("copilot"),
            _ => false,
        }
    }

    /// Check if a rule's category is enabled
    fn is_category_enabled(&self, rule_id: &str) -> bool {
        match rule_id {
            s if [
                "AS-", "CC-SK-", "CR-SK-", "CL-SK-", "CP-SK-", "CX-SK-", "OC-SK-", "WS-SK-",
                "KR-SK-", "AMP-SK-", "RC-SK-",
            ]
            .iter()
            .any(|p| s.starts_with(p)) =>
            {
                self.rules.skills
            }
            s if s.starts_with("AMP-") => self.rules.amp_checks,
            s if s.starts_with("CC-HK-") => self.rules.hooks,
            s if s.starts_with("CC-AG-") => self.rules.agents,
            s if s.starts_with("CC-MEM-") => self.rules.memory,
            s if s.starts_with("CC-OS-") => self.rules.output_styles,
            s if s.starts_with("CC-PL-") => self.rules.plugins,
            s if s.starts_with("XML-") => self.rules.xml,
            s if s.starts_with("MCP-") => self.rules.mcp,
            s if s.starts_with("REF-") || s.starts_with("imports::") => self.rules.imports,
            s if s.starts_with("XP-") => self.rules.cross_platform,
            s if s.starts_with("AGM-") => self.rules.agents_md,
            s if s.starts_with("COP-") => self.rules.copilot,
            s if s.starts_with("CUR-") => self.rules.cursor,
            s if s.starts_with("CLN-") => self.rules.cline,
            s if s.starts_with("OC-") => self.rules.opencode,
            s if s.starts_with("GM-") => self.rules.gemini_md,
            s if s.starts_with("CDX-") => self.rules.codex,
            s if s.starts_with("WS-") => self.rules.windsurf,
            s if s.starts_with("KIRO-") || s.starts_with("KR-") => self.is_kiro_category_enabled(s),
            s if s.starts_with("PE-") => self.rules.prompt_engineering,
            // Unknown rules are enabled by default
            _ => true,
        }
    }

    /// Kiro category toggles, routed by the rule's `category` in rules.json
    /// so new Kiro rule IDs land under the right toggle whatever their prefix:
    ///
    /// - `rules.kiro_agents`: agent profile rules, category `kiro-agents`
    ///   (KR-AG-*), plus KR-HK-005 and KR-HK-006, which check the hooks
    ///   embedded in an agent profile.
    /// - `rules.skills`: category `kiro-skills` (KR-SK-*, matched earlier).
    /// - `rules.kiro_steering`: every other Kiro rule: steering (KIRO-*),
    ///   hook files (KR-HK-*), MCP (KR-MCP-*), powers (KR-PW-*), CLI settings
    ///   (KR-SET-*), permissions (KR-PM-*) and file placement (KIRO-026).
    ///
    /// A Kiro-prefixed ID missing from rules.json falls back to its prefix.
    fn is_kiro_category_enabled(&self, rule_id: &str) -> bool {
        if matches!(rule_id, "KR-HK-005" | "KR-HK-006") {
            return self.rules.kiro_agents;
        }
        match agnix_rules::get_rule_metadata(rule_id).map(|(category, _, _)| category) {
            Some("kiro-agents") => self.rules.kiro_agents,
            Some("kiro-skills") => self.rules.skills,
            Some(_) => self.rules.kiro_steering,
            None if rule_id.starts_with("KR-AG-") => self.rules.kiro_agents,
            None if rule_id.starts_with("KR-SK-") => self.rules.skills,
            None => self.rules.kiro_steering,
        }
    }
}

impl RuleFilter for DefaultRuleFilter<'_> {
    fn is_rule_enabled(&self, rule_id: &str) -> bool {
        // Check if explicitly disabled
        if self.rules.disabled_rules.iter().any(|r| r == rule_id) {
            return false;
        }

        // Check if rule applies to target
        if !self.is_rule_for_target(rule_id) {
            return false;
        }

        // Check if category is enabled
        self.is_category_enabled(rule_id)
    }
}

impl LintConfig {
    // =========================================================================
    // Rule Filtering (delegates to DefaultRuleFilter)
    // =========================================================================

    /// Check if a specific rule is enabled based on config
    ///
    /// A rule is enabled if:
    /// 1. It's not in the disabled_rules list
    /// 2. It's applicable to the current target tool
    /// 3. Its category is enabled
    ///
    /// This delegates to `DefaultRuleFilter` which encapsulates the filtering logic.
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        let filter = DefaultRuleFilter::new(&self.data.rules, self.data.target, &self.data.tools);
        filter.is_rule_enabled(rule_id)
    }

    /// Check if a user-provided tool name is a backward-compatible alias
    /// for the canonical tool name from rules.json.
    ///
    /// Currently only "github-copilot" has an alias ("copilot"). This exists for
    /// backward compatibility: early versions of agnix used the shorter "copilot"
    /// name in configs, and we need to continue supporting that for existing users.
    /// The canonical names in rules.json use the full "github-copilot" to match
    /// the official tool name from GitHub's documentation.
    ///
    /// Note: This function does NOT treat canonical names as aliases of themselves.
    /// For example, "github-copilot" is NOT an alias for "github-copilot" - that's
    /// handled by the direct eq_ignore_ascii_case comparison in is_rule_for_tools().
    pub fn is_tool_alias(user_tool: &str, canonical_tool: &str) -> bool {
        DefaultRuleFilter::is_tool_alias(user_tool, canonical_tool)
    }

    /// Build a per-file view of this config, applying `[[overrides]]`
    /// whose `paths` globs match `path`.
    ///
    /// The returned [`PerFileLintConfig`] dereferences to `&LintConfig`,
    /// so all `LintConfig` accessors (e.g., `rules()`, `target()`,
    /// `fs()`, `root_dir()`) are available unchanged. Only
    /// [`PerFileLintConfig::is_rule_enabled`] differs from the global
    /// behavior: it returns `false` for rule IDs disabled by any matching
    /// override, falling through to [`LintConfig::is_rule_enabled`]
    /// otherwise.
    ///
    /// **Symlinked paths**: when `root_dir` has been canonicalized by the
    /// caller (e.g., the CLI canonicalizes the project root) but `path`
    /// retains a symlink prefix (e.g., `~/.claude/CLAUDE.md` where
    /// `~/.claude` is a dotfile-manager symlink), the direct
    /// `strip_prefix` would fail. `for_path` detects this and retries
    /// once via the configured [`FileSystem::canonicalize`] so overrides
    /// still match. Non-symlinked paths pay no syscall cost.
    pub fn for_path(&self, path: &Path) -> PerFileLintConfig<'_> {
        // Empty fast-path: skip matching entirely when there are no overrides.
        if self.runtime.compiled_overrides.is_empty() {
            return PerFileLintConfig {
                config: self,
                extra_disabled: Vec::new(),
            };
        }

        let rel_path = if let Some(root) = self.runtime.root_dir.as_ref() {
            let direct = normalize_rel_path(path, root);
            // `strip_prefix` failure leaves `direct` absolute. Common cause:
            // the caller canonicalized `root_dir` (e.g. the CLI canonicalizes
            // the project root via `std::fs::canonicalize`) but `path` still
            // carries a symlink prefix — a typical pattern with dotfile
            // managers (`~/.claude` symlinked into a `mackup` / `stow` /
            // `chezmoi` store). Retry once with `path` canonicalized so the
            // override actually matches in that case.
            if std::path::Path::new(&direct).is_absolute() {
                match self.runtime.fs.canonicalize(path) {
                    Ok(canonical) => normalize_rel_path(&canonical, root),
                    Err(_) => direct,
                }
            } else {
                direct
            }
        } else {
            // No root_dir: match against filename only, like resolve_with_compiled.
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string()
        };

        let mut extra_disabled: Vec<&str> = Vec::new();
        for (compiled, src) in self
            .runtime
            .compiled_overrides
            .iter()
            .zip(self.data.overrides.iter())
        {
            let matched = compiled
                .patterns
                .iter()
                .any(|p| p.matches_with(&rel_path, FILES_MATCH_OPTIONS));
            if matched {
                extra_disabled.extend(src.disabled_rules.iter().map(String::as_str));
            }
        }

        PerFileLintConfig {
            config: self,
            extra_disabled,
        }
    }
}

/// Per-file view of a [`LintConfig`] with `[[overrides]]` applied.
///
/// Validators receive `&PerFileLintConfig<'_>` instead of `&LintConfig`.
/// It [`Deref`]s to `&LintConfig`, so all existing accessor calls
/// (`config.rules()`, `config.fs()`, `config.root_dir()`, …) continue to
/// work unchanged. Only [`Self::is_rule_enabled`] is overridden, layering
/// per-file disabled rules on top of the global rule filter.
///
/// Construct via [`LintConfig::for_path`].
pub struct PerFileLintConfig<'a> {
    config: &'a LintConfig,
    /// Rule IDs disabled by matching `[[overrides]]`, borrowed from the
    /// parent config's `overrides[*].disabled_rules`.
    extra_disabled: Vec<&'a str>,
}

impl<'a> Deref for PerFileLintConfig<'a> {
    type Target = LintConfig;

    fn deref(&self) -> &LintConfig {
        self.config
    }
}

impl PerFileLintConfig<'_> {
    /// Check if a rule is enabled for this file.
    ///
    /// Returns `false` if the rule is listed in any `[[overrides]]` block
    /// whose `paths` matched this file; otherwise delegates to
    /// [`LintConfig::is_rule_enabled`].
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        if self.extra_disabled.contains(&rule_id) {
            return false;
        }
        self.config.is_rule_enabled(rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with(kiro_steering: bool, kiro_agents: bool, skills: bool) -> LintConfig {
        let mut config = LintConfig::default();
        config.rules_mut().kiro_steering = kiro_steering;
        config.rules_mut().kiro_agents = kiro_agents;
        config.rules_mut().skills = skills;
        config
    }

    /// Every Kiro rule in rules.json follows exactly one toggle, chosen by
    /// its category.
    #[test]
    fn every_kiro_rule_follows_the_toggle_of_its_category() {
        let kiro_rules: Vec<_> = agnix_rules::RULES_METADATA
            .iter()
            .filter(|(id, ..)| id.starts_with("KIRO-") || id.starts_with("KR-"))
            .collect();
        assert!(kiro_rules.len() > 60);

        for (id, category, _, _) in kiro_rules {
            let toggle = match (*id, *category) {
                ("KR-HK-005" | "KR-HK-006", _) | (_, "kiro-agents") => "kiro_agents",
                (_, "kiro-skills") => "skills",
                _ => "kiro_steering",
            };
            for (name, config) in [
                ("kiro_steering", config_with(false, true, true)),
                ("kiro_agents", config_with(true, false, true)),
                ("skills", config_with(true, true, false)),
            ] {
                assert_eq!(
                    config.is_rule_enabled(id),
                    name != toggle,
                    "{id} (category {category}) should follow rules.{toggle}, not rules.{name}"
                );
            }
        }
    }

    /// KR-HK-007..010 and KR-SET-* used to ignore both Kiro toggles.
    #[test]
    fn hook_timeout_and_settings_rules_follow_kiro_steering() {
        let config = config_with(false, true, true);
        for id in [
            "KR-HK-007",
            "KR-HK-008",
            "KR-HK-009",
            "KR-HK-010",
            "KR-SET-001",
            "KR-SET-002",
            "KR-SET-003",
        ] {
            assert!(!config.is_rule_enabled(id), "{id}");
        }
        assert!(config.is_rule_enabled("KR-AG-001"));
    }

    #[test]
    fn unknown_kiro_ids_fall_back_to_their_prefix() {
        assert!(!config_with(true, false, true).is_rule_enabled("KR-AG-999"));
        assert!(!config_with(true, true, false).is_rule_enabled("KR-SK-999"));
        assert!(!config_with(false, true, true).is_rule_enabled("KIRO-999"));
        assert!(!config_with(false, true, true).is_rule_enabled("KR-XYZ-001"));
        assert!(config_with(false, false, false).is_rule_enabled("XYZ-001"));
    }
}
