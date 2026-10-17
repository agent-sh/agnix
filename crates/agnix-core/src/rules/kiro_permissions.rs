//! Kiro capability permission policies (KR-PM-001 to KR-PM-012).
//!
//! Validates permission policy files read by the V3 engine:
//! `~/.kiro/settings/permissions.yaml` (or `.json`, user scope),
//! `~/.kiro/workspace-roots/<hash>/permissions.yaml` (or `.json`, workspace
//! scope) and an administrator `managed-settings.json` (deny and ask only).
//! A repository-local `.kiro/settings/permissions.yaml` is validated as a
//! template, and KR-PM-009 says that Kiro does not read it.
//!
//! The engine's parser is strict: a syntax error, a missing `rules` array, a
//! rule that is not an object or lacks a string `capability`, an invalid
//! `effect`, a non-string-array `match`/`exclude` or an unknown rule key is
//! FATAL. A fatal error drops every rule and the session fails closed: every
//! tool call is denied until the file is fixed. An unknown capability, the
//! unsupported effect `evaluate` and an unknown preset only skip that entry.
//! Sources: spec section 6.3 (kiro.dev/docs/permissions, V3 policy parser).
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_permissions::{
        ADMIN_PERMISSION_EFFECTS, UNSUPPORTED_PERMISSION_EFFECT, VALID_PERMISSION_CAPABILITIES,
        VALID_PERMISSION_EFFECTS, VALID_PERMISSION_POLICIES, VALID_PERMISSION_RULE_FIELDS,
        parse_kiro_permissions,
    },
};
use rust_i18n::t;
use serde_yaml::Value;
use std::path::{Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "KR-PM-001",
    "KR-PM-002",
    "KR-PM-003",
    "KR-PM-004",
    "KR-PM-005",
    "KR-PM-006",
    "KR-PM-007",
    "KR-PM-008",
    "KR-PM-009",
    "KR-PM-010",
    "KR-PM-011",
    "KR-PM-012",
];

pub struct KiroPermissionsValidator;

/// Which policy scope a file belongs to; it decides the allowed effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scope {
    /// User or workspace permissions: deny, ask, allow.
    User,
    /// Administrator managed-settings.json: deny and ask only.
    Administration,
}

impl Scope {
    fn of(path: &Path) -> Self {
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.eq_ignore_ascii_case("managed-settings.json"))
        {
            Self::Administration
        } else {
            Self::User
        }
    }

    fn effects(self) -> &'static [&'static str] {
        match self {
            Self::User => VALID_PERMISSION_EFFECTS,
            Self::Administration => ADMIN_PERMISSION_EFFECTS,
        }
    }
}

impl Validator for KiroPermissionsValidator {
    fn metadata(&self) -> ValidatorMetadata {
        ValidatorMetadata {
            name: self.name(),
            rule_ids: RULE_IDS,
        }
    }

    fn validate_per_file(
        &self,
        path: &Path,
        content: &str,
        config: &PerFileLintConfig<'_>,
    ) -> Vec<Diagnostic> {
        let mut cx = Cx {
            path,
            locator: Locator::new(content),
            config,
            diagnostics: Vec::new(),
        };
        let scope = Scope::of(path);
        check_location(&mut cx, path, config);

        // The engine parses .yaml/.yml as YAML and anything else as JSON.
        let is_yaml = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("yaml") || e.eq_ignore_ascii_case("yml"));
        let parsed = if is_yaml {
            parse_kiro_permissions(content).map_err(|error| {
                let (line, column) = error.location().map_or((1, 0), |l| (l.line(), l.column()));
                (error.to_string(), line, column)
            })
        } else {
            serde_json::from_str::<serde_json::Value>(content)
                .map_err(|e| (e.to_string(), e.line().max(1), e.column()))
                .and_then(|json| serde_yaml::to_value(json).map_err(|e| (e.to_string(), 1, 0)))
        };
        let policy = match parsed {
            Ok(policy) => policy,
            Err((error, line, column)) => {
                if cx.enabled("KR-PM-001") {
                    cx.push_at(
                        DiagnosticLevel::Error,
                        "KR-PM-001",
                        line,
                        column,
                        t!("rules.kr_pm_001.parse_error", error = error.as_str()).to_string(),
                        t!("rules.kr_pm_001.suggestion").to_string(),
                    );
                }
                return cx.diagnostics;
            }
        };

        let Some(rules) = policy
            .as_mapping()
            .and_then(|m| m.get("rules"))
            .and_then(Value::as_sequence)
        else {
            if cx.enabled("KR-PM-001") {
                let (line, column) = cx.locator.top_key("rules").unwrap_or((1, 0));
                cx.push_at(
                    DiagnosticLevel::Error,
                    "KR-PM-001",
                    line,
                    column,
                    t!("rules.kr_pm_001.shape").to_string(),
                    t!("rules.kr_pm_001.suggestion").to_string(),
                );
            }
            return cx.diagnostics;
        };

        if let Some(policies) = policy.as_mapping().and_then(|m| m.get("policies")) {
            check_policies(&mut cx, policies, scope);
        }

        for (index, rule) in rules.iter().enumerate() {
            check_rule(&mut cx, index, rule, scope);
        }
        check_sandbox_network_order(&mut cx, rules);
        cx.diagnostics
    }
}

struct Cx<'a> {
    path: &'a Path,
    locator: Locator<'a>,
    config: &'a PerFileLintConfig<'a>,
    diagnostics: Vec<Diagnostic>,
}

impl Cx<'_> {
    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    fn push_at(
        &mut self,
        level: DiagnosticLevel,
        rule: &str,
        line: usize,
        column: usize,
        message: String,
        suggestion: String,
    ) {
        let path = self.path.to_path_buf();
        let diagnostic = match level {
            DiagnosticLevel::Error => Diagnostic::error(path, line, column, rule, message),
            DiagnosticLevel::Warning => Diagnostic::warning(path, line, column, rule, message),
            DiagnosticLevel::Info => Diagnostic::info(path, line, column, rule, message),
        };
        self.diagnostics
            .push(diagnostic.with_suggestion(suggestion));
    }

    /// Push a diagnostic located at rule `index` (and `key` inside it).
    fn push_rule(
        &mut self,
        level: DiagnosticLevel,
        rule_id: &str,
        index: usize,
        key: Option<&str>,
        message: String,
        suggestion: String,
    ) {
        let (line, column) = self.locator.rule(index, key);
        self.push_at(level, rule_id, line, column, message, suggestion);
    }
}

fn check_rule(cx: &mut Cx<'_>, index: usize, rule: &Value, scope: Scope) {
    let Some(map) = rule.as_mapping() else {
        if cx.enabled("KR-PM-001") {
            cx.push_rule(
                DiagnosticLevel::Error,
                "KR-PM-001",
                index,
                None,
                t!("rules.kr_pm_001.entry", index = index).to_string(),
                t!("rules.kr_pm_001.suggestion").to_string(),
            );
        }
        return;
    };

    // Capability is checked first: an unknown capability skips the rule before
    // its other fields are read. Problems in such a rule are reported as
    // warnings: they become fatal once the capability is fixed.
    let mut latent = false;
    match map.get("capability").and_then(Value::as_str) {
        None => {
            if cx.enabled("KR-PM-002") {
                cx.push_rule(
                    DiagnosticLevel::Error,
                    "KR-PM-002",
                    index,
                    Some("capability"),
                    t!("rules.kr_pm_002.missing", index = index).to_string(),
                    t!("rules.kr_pm_002.suggestion").to_string(),
                );
            }
        }
        Some(capability) if !VALID_PERMISSION_CAPABILITIES.contains(&capability) => {
            latent = true;
            if cx.enabled("KR-PM-002") {
                cx.push_rule(
                    DiagnosticLevel::Error,
                    "KR-PM-002",
                    index,
                    Some("capability"),
                    t!(
                        "rules.kr_pm_002.unknown",
                        index = index,
                        capability = capability
                    )
                    .to_string(),
                    t!("rules.kr_pm_002.suggestion").to_string(),
                );
            }
        }
        Some(_) => {}
    }
    let (fatal_level, consequence) = if latent {
        (
            DiagnosticLevel::Warning,
            t!("rules.kr_pm_common.latent").to_string(),
        )
    } else {
        (
            DiagnosticLevel::Error,
            t!("rules.kr_pm_common.fatal").to_string(),
        )
    };
    let consequence = consequence.as_str();

    let mut skipped = latent;
    if cx.enabled("KR-PM-003") {
        match map.get("effect").and_then(Value::as_str) {
            Some(UNSUPPORTED_PERMISSION_EFFECT) => {
                skipped = true;
                if !latent {
                    cx.push_rule(
                        DiagnosticLevel::Warning,
                        "KR-PM-003",
                        index,
                        Some("effect"),
                        t!("rules.kr_pm_003.evaluate", index = index).to_string(),
                        t!("rules.kr_pm_003.suggestion").to_string(),
                    );
                }
            }
            Some(effect) if VALID_PERMISSION_EFFECTS.contains(&effect) => {
                if !scope.effects().contains(&effect) {
                    cx.push_rule(
                        fatal_level,
                        "KR-PM-003",
                        index,
                        Some("effect"),
                        t!(
                            "rules.kr_pm_003.scope",
                            index = index,
                            effect = effect,
                            consequence = consequence
                        )
                        .to_string(),
                        t!("rules.kr_pm_003.admin_suggestion").to_string(),
                    );
                }
            }
            _ => cx.push_rule(
                fatal_level,
                "KR-PM-003",
                index,
                Some("effect"),
                t!(
                    "rules.kr_pm_003.message",
                    index = index,
                    consequence = consequence
                )
                .to_string(),
                t!("rules.kr_pm_003.suggestion").to_string(),
            ),
        }
    }

    let mut patterns_ok = true;
    for field in ["match", "exclude"] {
        if let Some(value) = map.get(field)
            && !is_string_sequence(value)
        {
            patterns_ok = false;
            if cx.enabled("KR-PM-004") {
                cx.push_rule(
                    fatal_level,
                    "KR-PM-004",
                    index,
                    Some(field),
                    t!(
                        "rules.kr_pm_004.message",
                        index = index,
                        field = field,
                        consequence = consequence
                    )
                    .to_string(),
                    t!("rules.kr_pm_004.suggestion").to_string(),
                );
            }
        }
    }

    let unknown: Vec<String> = map
        .keys()
        .filter_map(|k| match k.as_str() {
            Some(key) if VALID_PERMISSION_RULE_FIELDS.contains(&key) => None,
            Some(key) => Some(key.to_string()),
            None => Some(format!("{k:?}")),
        })
        .collect();
    if let Some(first) = unknown.first()
        && cx.enabled("KR-PM-005")
    {
        let fields = unknown.join(", ");
        cx.push_rule(
            fatal_level,
            "KR-PM-005",
            index,
            Some(first),
            t!(
                "rules.kr_pm_005.message",
                index = index,
                fields = fields.as_str(),
                consequence = consequence
            )
            .to_string(),
            t!("rules.kr_pm_005.suggestion").to_string(),
        );
    }

    if skipped || !patterns_ok {
        return;
    }
    let capability = map
        .get("capability")
        .and_then(Value::as_str)
        .unwrap_or_default();
    check_patterns(cx, index, capability, map);
}

fn is_string_sequence(value: &Value) -> bool {
    value
        .as_sequence()
        .is_some_and(|items| items.iter().all(|item| item.as_str().is_some()))
}

/// KR-PM-007 and KR-PM-008: patterns whose meaning differs from what they
/// appear to say.
fn check_patterns(cx: &mut Cx<'_>, index: usize, capability: &str, map: &serde_yaml::Mapping) {
    // KR-PM-008: `match: []` is the same as omitting match: every resource.
    if let Some(items) = map.get("match").and_then(Value::as_sequence)
        && items.is_empty()
        && cx.enabled("KR-PM-008")
    {
        let effect = map.get("effect").and_then(Value::as_str).unwrap_or("allow");
        cx.push_rule(
            DiagnosticLevel::Warning,
            "KR-PM-008",
            index,
            Some("match"),
            t!(
                "rules.kr_pm_008.message",
                index = index,
                effect = effect,
                capability = capability
            )
            .to_string(),
            t!("rules.kr_pm_008.suggestion").to_string(),
        );
    }
    let effect = map
        .get("effect")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let match_items = map.get("match").and_then(Value::as_sequence);
    let exclude_items = map.get("exclude").and_then(Value::as_sequence);
    let strings = |items: Option<&Vec<Value>>| -> Vec<String> {
        items
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    };
    let (matches, excludes) = (strings(match_items), strings(exclude_items));

    // KR-PM-010: an allow rule for a write-capable capability with no match
    // restriction and no exclude. (`match: []` is KR-PM-008.)
    if effect == "allow"
        && WRITE_CAPABLE.contains(&capability)
        && match_items.is_none_or(|items| !items.is_empty())
        && matches.iter().all(|p| p == "*" || p == "**")
        && excludes.is_empty()
        && cx.enabled("KR-PM-010")
    {
        cx.push_rule(
            DiagnosticLevel::Warning,
            "KR-PM-010",
            index,
            Some("capability"),
            t!(
                "rules.kr_pm_010.message",
                index = index,
                capability = capability
            )
            .to_string(),
            t!("rules.kr_pm_010.suggestion").to_string(),
        );
    }

    for (field, patterns) in [("match", &matches), ("exclude", &excludes)] {
        for pattern in patterns {
            if cx.enabled("KR-PM-007")
                && let Some(reason) = ineffective_pattern(capability, effect, field, pattern)
            {
                cx.push_rule(
                    DiagnosticLevel::Warning,
                    "KR-PM-007",
                    index,
                    Some(field),
                    t!(
                        "rules.kr_pm_007.message",
                        index = index,
                        field = field,
                        reason = reason.as_str()
                    )
                    .to_string(),
                    t!("rules.kr_pm_007.suggestion").to_string(),
                );
            }
            if cx.enabled("KR-PM-011")
                && let Some(syntax) = unsupported_syntax(capability, pattern)
            {
                cx.push_rule(
                    DiagnosticLevel::Warning,
                    "KR-PM-011",
                    index,
                    Some(field),
                    t!(
                        "rules.kr_pm_011.message",
                        index = index,
                        pattern = pattern.as_str(),
                        capability = capability,
                        syntax = syntax
                    )
                    .to_string(),
                    t!("rules.kr_pm_011.suggestion").to_string(),
                );
            }
        }
    }
}

/// Capabilities that can change files or run code (meta-capabilities
/// included).
const WRITE_CAPABLE: &[&str] = &["shell", "fs_write", "filesystem", "all", "builtin", "mcp"];

/// Capabilities whose patterns are compared with Cedar `like`, where only `*`
/// is a wildcard.
const LIKE_CAPABILITIES: &[&str] = &["shell", "web_fetch", "web_search", "mcp"];

/// Paths whose writes Kiro always asks about or denies, whatever allow rule
/// exists (Kiro scope).
const PROTECTED_WRITE_PATHS: &[&str] = &[
    ".git",
    ".vscode",
    ".kiroignore",
    ".kiro/agents",
    ".kiro/hooks",
    ".kiro/workflows",
    ".kiro/powers",
    ".kiro/settings",
    ".kiro/workspace-roots",
];

/// Why a pattern cannot take effect, if it cannot.
fn ineffective_pattern(
    capability: &str,
    effect: &str,
    field: &str,
    pattern: &str,
) -> Option<String> {
    match capability {
        // MCP resources are `server/tool`; `@server` is the V2 tools selector.
        "mcp" if pattern.starts_with('@') => Some(
            t!(
                "rules.kr_pm_007.mcp_at",
                pattern = pattern,
                fixed = pattern.trim_start_matches('@')
            )
            .to_string(),
        ),
        // web_fetch rules are compared with the URL host name.
        "web_fetch" if pattern.contains("://") || pattern.contains('/') => {
            Some(t!("rules.kr_pm_007.web_fetch_url", pattern = pattern).to_string())
        }
        // These requests carry no resource beyond "*".
        "web_search" | "diagnostics" if pattern != "*" && field == "match" => Some(
            t!(
                "rules.kr_pm_007.no_resource",
                pattern = pattern,
                capability = capability
            )
            .to_string(),
        ),
        "fs_write" | "filesystem" if effect == "allow" && field == "match" => {
            let path = pattern
                .trim_start_matches("./")
                .trim_start_matches("~/")
                .trim_start_matches("**/");
            let protected = path.ends_with(".code-workspace")
                || PROTECTED_WRITE_PATHS
                    .iter()
                    .any(|p| path == *p || path.starts_with(&format!("{p}/")));
            protected.then(|| t!("rules.kr_pm_007.protected_path", pattern = pattern).to_string())
        }
        _ => None,
    }
}

/// Glob syntax shell, web and MCP patterns do not support: `**` (treated as
/// `*`), `?` and `[...]` (matched literally).
fn unsupported_syntax(capability: &str, pattern: &str) -> Option<&'static str> {
    if !LIKE_CAPABILITIES.contains(&capability) {
        return None;
    }
    if pattern.contains("**") {
        Some("**")
    } else if pattern.contains('?') {
        Some("?")
    } else if pattern.contains('[') && pattern.contains(']') {
        Some("[...]")
    } else {
        None
    }
}

/// `*`-only wildcard match over the whole string (Cedar `like`).
fn like_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.len() == 1 {
        return pattern == text;
    }
    let (first, last) = (parts[0], parts[parts.len() - 1]);
    if !text.starts_with(first) || text.len() < first.len() + last.len() || !text.ends_with(last) {
        return false;
    }
    let mut rest = &text[first.len()..text.len() - last.len()];
    for part in &parts[1..parts.len() - 1] {
        match rest.find(part) {
            Some(at) => rest = &rest[at + part.len()..],
            None => return false,
        }
    }
    true
}

/// KR-PM-012: `sandbox_network` rules are first-match in order, so a deny
/// after an allow that already matches its host never applies.
fn check_sandbox_network_order(cx: &mut Cx<'_>, rules: &[Value]) {
    if !cx.enabled("KR-PM-012") {
        return;
    }
    let host_rules: Vec<(usize, &str, Vec<String>, Vec<String>)> = rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            let map = rule.as_mapping()?;
            if map.get("capability").and_then(Value::as_str) != Some("sandbox_network") {
                return None;
            }
            let effect = map.get("effect").and_then(Value::as_str)?;
            let list = |key: &str| -> Option<Vec<String>> {
                match map.get(key) {
                    None => Some(Vec::new()),
                    Some(v) => v
                        .as_sequence()?
                        .iter()
                        .map(|p| p.as_str().map(str::to_string))
                        .collect(),
                }
            };
            Some((index, effect, list("match")?, list("exclude")?))
        })
        .collect();
    for (position, (deny_index, effect, deny_hosts, _)) in host_rules.iter().enumerate() {
        if *effect != "deny" {
            continue;
        }
        let hosts: Vec<String> = if deny_hosts.is_empty() {
            vec!["*".to_string()]
        } else {
            deny_hosts.clone()
        };
        for host in hosts {
            let shadow = host_rules[..position]
                .iter()
                .find(|(_, e, matches, excludes)| {
                    *e == "allow"
                        && (matches.is_empty()
                            || matches.iter().any(|p| p == "*" || like_match(p, &host)))
                        && !excludes.iter().any(|x| like_match(x, &host))
                });
            if let Some((allow_index, ..)) = shadow {
                cx.push_rule(
                    DiagnosticLevel::Warning,
                    "KR-PM-012",
                    *deny_index,
                    Some("match"),
                    t!(
                        "rules.kr_pm_012.message",
                        index = deny_index,
                        host = host.as_str(),
                        allow = allow_index
                    )
                    .to_string(),
                    t!("rules.kr_pm_012.suggestion").to_string(),
                );
                break;
            }
        }
    }
}

/// KR-PM-006: the optional `policies` preset list.
fn check_policies(cx: &mut Cx<'_>, policies: &Value, scope: Scope) {
    if !cx.enabled("KR-PM-006") {
        return;
    }
    let (line, column) = cx.locator.top_key("policies").unwrap_or((1, 0));
    let Some(items) = policies.as_sequence() else {
        cx.push_at(
            DiagnosticLevel::Error,
            "KR-PM-006",
            line,
            column,
            t!("rules.kr_pm_006.not_array").to_string(),
            t!("rules.kr_pm_006.suggestion").to_string(),
        );
        return;
    };
    for item in items {
        let (level, message) = match item.as_str() {
            None => (
                DiagnosticLevel::Error,
                t!("rules.kr_pm_006.not_array").to_string(),
            ),
            Some(id) if !VALID_PERMISSION_POLICIES.contains(&id) => (
                DiagnosticLevel::Warning,
                t!("rules.kr_pm_006.unknown", policy = id).to_string(),
            ),
            // Every preset holds allow rules, which an administrator file
            // cannot contain: Kiro skips them.
            Some(id) if scope == Scope::Administration => (
                DiagnosticLevel::Warning,
                t!("rules.kr_pm_006.admin", policy = id).to_string(),
            ),
            Some(_) => continue,
        };
        cx.push_at(
            level,
            "KR-PM-006",
            line,
            column,
            message,
            t!("rules.kr_pm_006.suggestion").to_string(),
        );
    }
}

// ---------------------------------------------------------------------------
// KR-PM-009: files Kiro does not read
// ---------------------------------------------------------------------------

fn check_location(cx: &mut Cx<'_>, path: &Path, config: &PerFileLintConfig<'_>) {
    if !cx.enabled("KR-PM-009") {
        return;
    }
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    // permissions.yaml wins when both exist in the same directory.
    if file_name.eq_ignore_ascii_case("permissions.json")
        && path
            .parent()
            .is_some_and(|dir| config.fs().exists(&dir.join("permissions.yaml")))
    {
        cx.push_at(
            DiagnosticLevel::Info,
            "KR-PM-009",
            1,
            1,
            t!("rules.kr_pm_009.shadowed").to_string(),
            t!("rules.kr_pm_009.shadowed_suggestion").to_string(),
        );
        return;
    }
    let kiro_home = std::env::var_os("KIRO_HOME").map(PathBuf::from);
    if let Some(false) =
        permissions_file_is_loaded(path, user_home().as_deref(), kiro_home.as_deref())
    {
        let message = if Scope::of(path) == Scope::Administration {
            t!("rules.kr_pm_009.admin")
        } else {
            t!("rules.kr_pm_009.message")
        };
        cx.push_at(
            DiagnosticLevel::Info,
            "KR-PM-009",
            1,
            1,
            message.to_string(),
            t!("rules.kr_pm_009.suggestion").to_string(),
        );
    }
}

#[cfg(feature = "filesystem")]
fn user_home() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(not(feature = "filesystem"))]
fn user_home() -> Option<PathBuf> {
    None
}

/// Whether Kiro reads this permissions file: `<kiro>/settings/permissions.*`
/// or `<kiro>/workspace-roots/<hash>/permissions.*` where `<kiro>` is
/// `$KIRO_HOME` (when valid) or `~/.kiro`, or a managed-settings.json at its
/// operating-system path. `None` when it cannot be told.
fn permissions_file_is_loaded(
    path: &Path,
    home: Option<&Path>,
    kiro_home: Option<&Path>,
) -> Option<bool> {
    let absolute = std::path::absolute(path).ok()?;
    if Scope::of(path) == Scope::Administration {
        let known = [
            "/Library/Application Support/Kiro/managed-settings.json",
            "/etc/kiro/managed-settings.json",
            "C:\\ProgramData\\Kiro\\managed-settings.json",
        ];
        return Some(known.iter().any(|k| absolute == Path::new(k)));
    }
    let kiro_home = kiro_home.filter(|k| {
        k.is_absolute() && k.parent().is_some() && home.is_none_or(|h| !h.starts_with(k))
    });
    let roots: Vec<PathBuf> = kiro_home
        .map(Path::to_path_buf)
        .into_iter()
        .chain(home.map(|h| h.join(".kiro")))
        .collect();
    if roots.is_empty() {
        return None;
    }
    let dir = absolute.parent()?;
    let in_root = |root: &Path| {
        dir == root.join("settings") || dir.parent() == Some(root.join("workspace-roots").as_path())
    };
    // Also accept another account's home (/home/<user>/.kiro, /Users/<user>/.kiro),
    // for example a copied or mounted home directory.
    let other_home = absolute.ancestors().find(|a| {
        a.file_name().is_some_and(|n| n == ".kiro") && is_home_like(a.parent().unwrap_or(a))
    });
    Some(roots.iter().any(|root| in_root(root)) || other_home.is_some_and(in_root))
}

/// `/home/<user>`, `/Users/<user>` or `<drive>:\Users\<user>`.
fn is_home_like(dir: &Path) -> bool {
    let parts: Vec<String> = dir
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    parts.len() == 2 && matches!(parts[0].as_str(), "home" | "Users")
}

// ---------------------------------------------------------------------------
// Locations
// ---------------------------------------------------------------------------

/// Line-based locations for block-style YAML (and JSON) policy files. Flow
/// style or unusual layouts fall back to the `rules` key, then line 1.
struct Locator<'a> {
    lines: Vec<&'a str>,
}

impl<'a> Locator<'a> {
    fn new(content: &'a str) -> Self {
        Self {
            lines: content.lines().collect(),
        }
    }

    /// 1-based (line, column) of a top-level key.
    fn top_key(&self, key: &str) -> Option<(usize, usize)> {
        let yaml = format!("{key}:");
        let json = format!("\"{key}\"");
        self.lines
            .iter()
            .position(|line| line.starts_with(&yaml) || line.trim_start().starts_with(&json))
            .map(|i| {
                (
                    i + 1,
                    1 + self.lines[i].len() - self.lines[i].trim_start().len(),
                )
            })
    }

    /// Start lines of the block sequence items under `rules:`.
    fn rule_items(&self) -> Vec<usize> {
        let Some((rules_line, _)) = self.top_key("rules") else {
            return Vec::new();
        };
        let mut items = Vec::new();
        let mut item_indent: Option<usize> = None;
        for (i, line) in self.lines.iter().enumerate().skip(rules_line) {
            let trimmed = line.trim_start();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = line.len() - trimmed.len();
            let is_item = trimmed == "-" || trimmed.starts_with("- ");
            match item_indent {
                None if is_item => {
                    item_indent = Some(indent);
                    items.push(i);
                }
                None => break,
                Some(level) if is_item && indent == level => items.push(i),
                Some(level) if indent <= level && !is_item => break,
                Some(_) => {}
            }
        }
        items
    }

    fn rule(&self, index: usize, key: Option<&str>) -> (usize, usize) {
        let items = self.rule_items();
        let Some(&start) = items.get(index) else {
            return self.top_key("rules").unwrap_or((1, 0));
        };
        let end = items.get(index + 1).copied().unwrap_or(self.lines.len());
        if let Some(key) = key {
            let needle = format!("{key}:");
            for i in start..end {
                let trimmed = self.lines[i].trim_start().trim_start_matches("- ");
                if trimmed.starts_with(&needle) {
                    let column = self.lines[i].len() - trimmed.len() + 1;
                    return (i + 1, column);
                }
            }
        }
        let line = self.lines[start];
        (start + 1, line.len() - line.trim_start().len() + 1)
    }
}

#[cfg(test)]
mod tests;
