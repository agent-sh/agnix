//! Kiro V3 standalone hook file validation.
//!
//! Validates `.kiro/hooks/*.json` (and `~/.kiro/hooks/*.json`) against the
//! Kiro CLI V3 engine, which Kiro IDE 1.0 shares. The engine reads each file
//! with `JSON.parse` and validates the whole document with one schema, so a
//! single malformed hook makes it skip the whole file; only an unrecognized
//! trigger name skips just that hook. Unknown keys are stripped silently.
//!
//! - KR-HK-010: hardcoded secrets in `action.command` or
//!   `confirm.confirmCommand` (also legacy `.kiro.hook` run commands)
//! - KR-HK-011: `version` missing or not `"v1"` (file skipped)
//! - KR-HK-012: invalid JSON, wrapper shape, `name` / `trigger` / `action`
//!   presence and types, unknown trigger names
//! - KR-HK-013: `action.type` and its `command` / `prompt` payload
//! - KR-HK-014: trigger alias with a canonical name (info, safe fix); also the
//!   CLI-spelling hint for legacy `.kiro.hook` files
//! - KR-HK-015: legacy IDE `when`/`then` hook saved as `.json`
//! - KR-HK-017: `description`, `matcher`, `enabled`, `timeout` values
//! - KR-HK-018: `confirm` block shape and placement
//! - KR-HK-019: matcher that is not a valid regex (hook never fires)
//! - KR-HK-020: matcher on a trigger that ignores it
//! - KR-HK-021: tool matcher written with CLI 2.x or Claude Code tool names
//! - KR-HK-022: misplaced or misspelled hook fields that Kiro drops
//! - KR-HK-023: command placeholders and variables Kiro does not provide
//! - KR-HK-024: hooks that never run (SessionEnd agent actions, Manual)
//! - KR-HK-025: standalone hook files under a pinned Kiro CLI 2.x
//!
//! Sources: <https://kiro.dev/docs/hooks/>, <https://kiro.dev/docs/hooks/types/>,
//! <https://kiro.dev/docs/cli/v3/hooks-migration/>, and the V3 engine bundled
//! with kiro-cli 2.32.1 (hook file loader and zod schema).

use super::kiro_engine::{KiroEngine, pinned_engine};
use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, DiagnosticLevel, Fix},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_hook_v3::{
        JsonSpans, KIRO_V3_HOOK_ACTION_KEYS, KIRO_V3_HOOK_KEYS, MatcherSubject, Seg,
        VALID_KIRO_V3_HOOK_DOC_VERSIONS, compile_js_regex, foreign_tool_names, is_tool_selector,
        matcher_subject, matches_everything, resolve_trigger, tool_matcher_matches_builtin,
        trigger_case_insensitive_match,
    },
};
use rust_i18n::t;
use serde_json::{Map, Value};
use std::{cell::OnceCell, path::Path};

const RULE_IDS: &[&str] = &[
    "KR-HK-010",
    "KR-HK-011",
    "KR-HK-012",
    "KR-HK-013",
    "KR-HK-014",
    "KR-HK-015",
    "KR-HK-017",
    "KR-HK-018",
    "KR-HK-019",
    "KR-HK-020",
    "KR-HK-021",
    "KR-HK-022",
    "KR-HK-023",
    "KR-HK-024",
    "KR-HK-025",
    "KR-HK-027",
    "KR-HK-028",
    "KR-HK-029",
    "KR-HK-030",
];

pub struct KiroV3HookValidator;

impl Validator for KiroV3HookValidator {
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
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("hook"))
        {
            return super::kiro_hook::validate_legacy_migration(path, content, config);
        }
        let mut ctx = Ctx {
            path,
            content,
            config,
            spans: OnceCell::new(),
            diagnostics: Vec::new(),
        };
        if config.is_rule_enabled("KR-HK-025") && pinned_engine(config) == Some(KiroEngine::V2) {
            let version = config.get_kiro_version().unwrap_or_default().to_string();
            ctx.push(
                Finding::new(
                    "KR-HK-025",
                    DiagnosticLevel::Info,
                    At::Value(&[]),
                    t!("rules.kr_hk_025.message", version = version.as_str()),
                )
                .suggest(t!("rules.kr_hk_025.suggestion")),
            );
        }
        if config.is_rule_enabled("KR-HK-030")
            && let Some(name) = path.file_name().and_then(|name| name.to_str())
            && !is_kebab_case_hook_file_name(name)
        {
            ctx.push(
                Finding::new(
                    "KR-HK-030",
                    DiagnosticLevel::Info,
                    At::Value(&[]),
                    t!("rules.kr_hk_030.message", name = name),
                )
                .suggest(t!("rules.kr_hk_030.suggestion")),
            );
        }
        // Hook files go through strict JSON.parse; duplicate keys are legal
        // there and the last one wins, which serde_json::Value also does.
        match serde_json::from_str::<Value>(content) {
            Ok(root) => ctx.check_document(&root),
            Err(error) => {
                if config.is_rule_enabled("KR-HK-012") {
                    ctx.diagnostics.push(
                        Diagnostic::error(
                            path.to_path_buf(),
                            error.line().max(1),
                            error.column(),
                            "KR-HK-012",
                            t!("rules.kr_hk_012.parse_error", error = error.to_string()),
                        )
                        .with_suggestion(t!("rules.kr_hk_012.parse_suggestion")),
                    );
                }
            }
        }
        ctx.diagnostics
    }
}

/// Where a diagnostic points: a value, or the key of an object member. A
/// missing path falls back to its nearest existing ancestor.
#[derive(Clone, Copy)]
enum At<'p> {
    Value(&'p [Seg<'p>]),
    Key(&'p [Seg<'p>]),
}

struct FixSpec<'p> {
    path: &'p [Seg<'p>],
    replacement: String,
    description: String,
    safe: bool,
}

struct Finding<'p> {
    rule: &'static str,
    level: DiagnosticLevel,
    at: At<'p>,
    message: String,
    suggestion: Option<String>,
    fix: Option<FixSpec<'p>>,
}

impl<'p> Finding<'p> {
    fn new(
        rule: &'static str,
        level: DiagnosticLevel,
        at: At<'p>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            rule,
            level,
            at,
            message: message.into(),
            suggestion: None,
            fix: None,
        }
    }

    fn error(rule: &'static str, at: At<'p>, message: impl Into<String>) -> Self {
        Self::new(rule, DiagnosticLevel::Error, at, message)
    }

    fn warning(rule: &'static str, at: At<'p>, message: impl Into<String>) -> Self {
        Self::new(rule, DiagnosticLevel::Warning, at, message)
    }

    fn suggest(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Replace the exact value at `path` (only when its location is not
    /// ambiguous because of duplicate keys).
    fn fix(
        mut self,
        path: &'p [Seg<'p>],
        replacement: impl Into<String>,
        description: impl Into<String>,
        safe: bool,
    ) -> Self {
        self.fix = Some(FixSpec {
            path,
            replacement: replacement.into(),
            description: description.into(),
            safe,
        });
        self
    }
}

struct Ctx<'a> {
    path: &'a Path,
    content: &'a str,
    config: &'a PerFileLintConfig<'a>,
    /// Built on the first diagnostic only, so valid files never index spans.
    spans: OnceCell<JsonSpans>,
    diagnostics: Vec<Diagnostic>,
}

fn path<'v>(base: &[Seg<'v>], rest: &[Seg<'v>]) -> Vec<Seg<'v>> {
    let mut path = base.to_vec();
    path.extend_from_slice(rest);
    path
}

/// JSON type of a value, with an article, for messages.
fn found(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

impl Ctx<'_> {
    fn spans(&self) -> &JsonSpans {
        self.spans.get_or_init(|| JsonSpans::new(self.content))
    }

    fn enabled(&self, rule: &str) -> bool {
        self.config.is_rule_enabled(rule)
    }

    fn push(&mut self, finding: Finding<'_>) {
        if !self.enabled(finding.rule) {
            return;
        }
        let spans = self.spans();
        let (line, column) = match finding.at {
            At::Value(path) => spans
                .nearest(path)
                .map_or((1, 1), |loc| (loc.line, loc.column)),
            At::Key(path) => match spans.get(path) {
                Some(loc) => (loc.key_line, loc.key_column),
                None => spans
                    .nearest(path)
                    .map_or((1, 1), |loc| (loc.line, loc.column)),
            },
        };
        let fix = finding.fix.and_then(|spec| {
            let loc = spans.unique(spec.path)?;
            Some(Fix::replace(
                loc.start,
                loc.end,
                spec.replacement,
                spec.description,
                spec.safe,
            ))
        });
        let file = self.path.to_path_buf();
        let mut diagnostic = match finding.level {
            DiagnosticLevel::Error => {
                Diagnostic::error(file, line, column, finding.rule, finding.message)
            }
            DiagnosticLevel::Warning => {
                Diagnostic::warning(file, line, column, finding.rule, finding.message)
            }
            DiagnosticLevel::Info => {
                Diagnostic::info(file, line, column, finding.rule, finding.message)
            }
        };
        if let Some(suggestion) = finding.suggestion {
            diagnostic = diagnostic.with_suggestion(suggestion);
        }
        if let Some(fix) = fix {
            diagnostic = diagnostic.with_fix(fix);
        }
        self.diagnostics.push(diagnostic);
    }

    fn check_document(&mut self, root: &Value) {
        let Some(doc) = root.as_object() else {
            self.push(
                Finding::error(
                    "KR-HK-012",
                    At::Value(&[]),
                    t!("rules.kr_hk_012.root_type", found = found(root)),
                )
                .suggest(t!("rules.kr_hk_012.suggestion")),
            );
            return;
        };
        let hooks = doc.get("hooks");

        // A legacy IDE 0.x hook saved as .json: one diagnostic for the cause.
        if hooks.is_none() && (doc.contains_key("when") || doc.contains_key("then")) {
            let key = if doc.contains_key("when") {
                "when"
            } else {
                "then"
            };
            self.push(
                Finding::error(
                    "KR-HK-015",
                    At::Key(&[Seg::Key(key)]),
                    t!("rules.kr_hk_015.document"),
                )
                .suggest(t!("rules.kr_hk_015.suggestion")),
            );
            return;
        }

        self.check_version(doc, hooks.is_none());

        let Some(hooks) = hooks else {
            // A single hook object (or a bare action) without the wrapper.
            let bare = ["trigger", "action", "matcher", "command", "prompt"]
                .iter()
                .any(|key| doc.contains_key(*key));
            let finding = if bare {
                Finding::error("KR-HK-012", At::Value(&[]), t!("rules.kr_hk_012.bare_hook"))
                    .suggest(t!("rules.kr_hk_012.bare_hook_suggestion"))
            } else {
                Finding::error(
                    "KR-HK-012",
                    At::Value(&[]),
                    t!("rules.kr_hk_012.missing_hooks"),
                )
                .suggest(t!("rules.kr_hk_012.suggestion"))
            };
            self.push(finding);
            return;
        };
        const HOOKS: &[Seg<'static>] = &[Seg::Key("hooks")];
        let entries = match hooks {
            Value::Array(entries) if entries.is_empty() => {
                self.push(
                    Finding::error(
                        "KR-HK-012",
                        At::Value(HOOKS),
                        t!("rules.kr_hk_012.empty_hooks"),
                    )
                    .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return;
            }
            Value::Array(entries) => entries,
            other => {
                self.push(
                    Finding::error(
                        "KR-HK-012",
                        At::Value(HOOKS),
                        t!("rules.kr_hk_012.hooks_type", found = found(other)),
                    )
                    .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return;
            }
        };
        for (index, entry) in entries.iter().enumerate() {
            self.check_entry(index, entry);
        }
    }

    /// KR-HK-011: the wrapper schema requires the literal string "v1"; any
    /// other value makes Kiro skip the whole file.
    fn check_version(&mut self, doc: &Map<String, Value>, hooks_missing: bool) {
        const VERSION: &[Seg<'static>] = &[Seg::Key("version")];
        match doc.get("version") {
            // Without a hooks array either, KR-HK-012 already covers the file.
            None if hooks_missing => {}
            None => self.push(
                Finding::error(
                    "KR-HK-011",
                    At::Value(&[]),
                    t!("rules.kr_hk_011.missing_version"),
                )
                .suggest(t!("rules.kr_hk_011.suggestion")),
            ),
            Some(Value::String(version))
                if VALID_KIRO_V3_HOOK_DOC_VERSIONS.contains(&version.as_str()) => {}
            Some(Value::String(version)) => {
                let mut finding = Finding::error(
                    "KR-HK-011",
                    At::Value(VERSION),
                    t!(
                        "rules.kr_hk_011.invalid_version",
                        version = version.as_str()
                    ),
                )
                .suggest(t!("rules.kr_hk_011.suggestion"));
                // Unsafe: this activates hooks Kiro currently skips. Other
                // values (future versions, "1.0.0" legacy documents) are not
                // guessed.
                if matches!(version.as_str(), "1" | "1.0") {
                    finding = finding.fix(VERSION, "\"v1\"", t!("rules.kr_hk_011.fix"), false);
                }
                self.push(finding);
            }
            Some(other) => self.push(
                Finding::error(
                    "KR-HK-011",
                    At::Value(VERSION),
                    t!("rules.kr_hk_011.version_type", found = found(other)),
                )
                .suggest(t!("rules.kr_hk_011.suggestion")),
            ),
        }
    }

    fn check_entry(&mut self, index: usize, entry: &Value) {
        let base = [Seg::Key("hooks"), Seg::Index(index)];
        let Some(hook) = entry.as_object() else {
            self.push(
                Finding::error(
                    "KR-HK-012",
                    At::Value(&base),
                    t!(
                        "rules.kr_hk_012.entry_type",
                        index = index,
                        found = found(entry)
                    ),
                )
                .suggest(t!("rules.kr_hk_012.suggestion")),
            );
            return;
        };
        if !hook.contains_key("trigger")
            && !hook.contains_key("action")
            && (hook.contains_key("when") || hook.contains_key("then"))
        {
            let key = if hook.contains_key("when") {
                "when"
            } else {
                "then"
            };
            self.push(
                Finding::error(
                    "KR-HK-015",
                    At::Key(&path(&base, &[Seg::Key(key)])),
                    t!("rules.kr_hk_015.entry", index = index),
                )
                .suggest(t!("rules.kr_hk_015.suggestion")),
            );
            return;
        }

        self.check_name(index, &base, hook);
        let trigger = self.check_trigger(index, &base, hook);
        let action = self.check_action(index, &base, hook);
        self.check_optional_fields(index, &base, hook);
        if let (Some(trigger), Some(Value::String(matcher))) = (trigger, hook.get("matcher")) {
            self.check_matcher(index, &base, trigger, matcher);
        }
        if let Some(trigger) = trigger {
            match hook.get("matcher") {
                None => self.check_scope(index, &base, trigger, None),
                Some(Value::String(matcher)) => {
                    self.check_scope(index, &base, trigger, Some(matcher))
                }
                Some(_) => {} // KR-HK-017
            }
        }
        if let Some(confirm) = hook.get("confirm") {
            self.check_confirm(index, &base, confirm, trigger, action);
        }
        self.check_misplaced_fields(index, &base, hook);
        self.check_never_runs(index, &base, trigger, action);
    }

    fn check_name(&mut self, index: usize, base: &[Seg<'_>], hook: &Map<String, Value>) {
        let name_path = path(base, &[Seg::Key("name")]);
        let message = match hook.get("name") {
            None => Some((
                At::Value(base),
                t!("rules.kr_hk_012.missing_name", index = index),
            )),
            // zod min(1): whitespace-only names are accepted.
            Some(Value::String(name)) if name.is_empty() => Some((
                At::Value(&name_path),
                t!("rules.kr_hk_012.empty_name", index = index),
            )),
            Some(Value::String(_)) => None,
            Some(other) => Some((
                At::Value(&name_path),
                t!(
                    "rules.kr_hk_012.name_type",
                    index = index,
                    found = found(other)
                ),
            )),
        };
        if let Some((at, message)) = message {
            self.push(
                Finding::error("KR-HK-012", at, message).suggest(t!("rules.kr_hk_012.suggestion")),
            );
        }
    }

    /// Returns the canonical trigger when Kiro recognizes it.
    fn check_trigger(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        hook: &Map<String, Value>,
    ) -> Option<&'static str> {
        let trigger_path = path(base, &[Seg::Key("trigger")]);
        let trigger = match hook.get("trigger") {
            None => {
                self.push(
                    Finding::error(
                        "KR-HK-012",
                        At::Value(base),
                        t!("rules.kr_hk_012.missing_trigger", index = index),
                    )
                    .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return None;
            }
            Some(Value::String(trigger)) if trigger.is_empty() => {
                self.push(
                    Finding::error(
                        "KR-HK-012",
                        At::Value(&trigger_path),
                        t!("rules.kr_hk_012.empty_trigger", index = index),
                    )
                    .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return None;
            }
            Some(Value::String(trigger)) => trigger,
            Some(other) => {
                self.push(
                    Finding::error(
                        "KR-HK-012",
                        At::Value(&trigger_path),
                        t!(
                            "rules.kr_hk_012.trigger_type",
                            index = index,
                            found = found(other)
                        ),
                    )
                    .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return None;
            }
        };
        match resolve_trigger(trigger) {
            Some(canonical) if canonical == trigger => Some(canonical),
            Some(canonical) => {
                // Safe: the engine maps the alias to this canonical name
                // before doing anything else (stdin carries the canonical
                // name too), so the rewrite cannot change behavior.
                self.push(
                    Finding::new(
                        "KR-HK-014",
                        DiagnosticLevel::Info,
                        At::Value(&trigger_path),
                        t!(
                            "rules.kr_hk_014.alias",
                            trigger = trigger,
                            canonical = canonical
                        ),
                    )
                    .suggest(t!("rules.kr_hk_014.alias_suggestion"))
                    .fix(
                        &trigger_path,
                        format!("\"{canonical}\""),
                        t!("rules.kr_hk_014.fix", canonical = canonical),
                        true,
                    ),
                );
                Some(canonical)
            }
            None => {
                let mut finding = Finding::error(
                    "KR-HK-012",
                    At::Value(&trigger_path),
                    t!(
                        "rules.kr_hk_012.unknown_trigger",
                        index = index,
                        trigger = trigger
                    ),
                )
                .suggest(t!("rules.kr_hk_012.unknown_trigger_suggestion"));
                // Unsafe: the hook is skipped today and would start running.
                if let Some(canonical) = trigger_case_insensitive_match(trigger) {
                    finding = finding.fix(
                        &trigger_path,
                        format!("\"{canonical}\""),
                        t!("rules.kr_hk_012.unknown_trigger_fix", canonical = canonical),
                        false,
                    );
                }
                self.push(finding);
                None
            }
        }
    }

    /// Returns the action type ("command" or "agent") when it is valid.
    fn check_action(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        hook: &Map<String, Value>,
    ) -> Option<&'static str> {
        let action_path = path(base, &[Seg::Key("action")]);
        let action = match hook.get("action") {
            None => {
                let flat = ["command", "prompt"]
                    .into_iter()
                    .find(|key| hook.contains_key(*key));
                let message = match flat {
                    Some(key) => t!(
                        "rules.kr_hk_012.missing_action_flat",
                        index = index,
                        key = key
                    ),
                    None => t!("rules.kr_hk_012.missing_action", index = index),
                };
                self.push(
                    Finding::error("KR-HK-012", At::Value(base), message)
                        .suggest(t!("rules.kr_hk_012.suggestion")),
                );
                return None;
            }
            Some(Value::Object(action)) => action,
            Some(other) => {
                self.push(
                    Finding::error(
                        "KR-HK-013",
                        At::Value(&action_path),
                        t!(
                            "rules.kr_hk_013.action_type",
                            index = index,
                            found = found(other)
                        ),
                    )
                    .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
        };
        let type_path = path(&action_path, &[Seg::Key("type")]);
        let kind: &'static str = match action.get("type") {
            None => {
                self.push(
                    Finding::error(
                        "KR-HK-013",
                        At::Value(&action_path),
                        t!("rules.kr_hk_013.missing_type", index = index),
                    )
                    .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
            Some(Value::String(kind)) if kind == "command" => "command",
            Some(Value::String(kind)) if kind == "agent" => "agent",
            Some(Value::String(kind)) => {
                let mut finding = Finding::error(
                    "KR-HK-013",
                    At::Value(&type_path),
                    t!(
                        "rules.kr_hk_013.invalid_type",
                        index = index,
                        kind = kind.as_str()
                    ),
                )
                .suggest(t!("rules.kr_hk_013.suggestion"));
                // IDE 0.x `then.type` spellings and case slips. Unsafe: the
                // file is skipped today and would start running.
                let replacement = match kind.as_str() {
                    k if k.eq_ignore_ascii_case("command") || k == "runCommand" => Some("command"),
                    k if k.eq_ignore_ascii_case("agent") || k == "askAgent" => Some("agent"),
                    _ => None,
                };
                if let Some(replacement) = replacement
                    && action.contains_key(if replacement == "command" {
                        "command"
                    } else {
                        "prompt"
                    })
                {
                    finding = finding.fix(
                        &type_path,
                        format!("\"{replacement}\""),
                        t!("rules.kr_hk_013.invalid_type_fix", kind = replacement),
                        false,
                    );
                }
                self.push(finding);
                return None;
            }
            Some(other) => {
                self.push(
                    Finding::error(
                        "KR-HK-013",
                        At::Value(&type_path),
                        t!(
                            "rules.kr_hk_013.type_type",
                            index = index,
                            found = found(other)
                        ),
                    )
                    .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
        };
        let field = if kind == "command" {
            "command"
        } else {
            "prompt"
        };
        let field_path = path(&action_path, &[Seg::Key(field)]);
        match action.get(field) {
            None => {
                let key = if kind == "command" {
                    "rules.kr_hk_013.command_required"
                } else {
                    "rules.kr_hk_013.prompt_required"
                };
                self.push(
                    Finding::error("KR-HK-013", At::Value(&action_path), t!(key, index = index))
                        .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
            Some(Value::String(text)) if text.is_empty() => {
                self.push(
                    Finding::error(
                        "KR-HK-013",
                        At::Value(&field_path),
                        t!(
                            "rules.kr_hk_013.payload_empty",
                            index = index,
                            field = field
                        ),
                    )
                    .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
            Some(Value::String(text)) => {
                if text.trim().is_empty() {
                    // Passes the schema (min length 1) but does nothing.
                    self.push(
                        Finding::warning(
                            "KR-HK-013",
                            At::Value(&field_path),
                            t!(
                                "rules.kr_hk_013.payload_blank",
                                index = index,
                                field = field
                            ),
                        )
                        .suggest(t!("rules.kr_hk_013.suggestion")),
                    );
                } else if kind == "command" {
                    self.check_command_text(index, &field_path, "action.command", text, false);
                }
            }
            Some(other) => {
                self.push(
                    Finding::error(
                        "KR-HK-013",
                        At::Value(&field_path),
                        t!(
                            "rules.kr_hk_013.payload_type",
                            index = index,
                            field = field,
                            found = found(other)
                        ),
                    )
                    .suggest(t!("rules.kr_hk_013.suggestion")),
                );
                return None;
            }
        }
        Some(kind)
    }

    /// KR-HK-017: optional fields with the wrong type or range fail the
    /// schema, so Kiro skips the whole file. There is no upper bound on
    /// `timeout`: the engine passes `timeout * 1000` ms to the process runner.
    fn check_optional_fields(&mut self, index: usize, base: &[Seg<'_>], hook: &Map<String, Value>) {
        for field in ["description", "matcher"] {
            if let Some(value) = hook.get(field).filter(|value| !value.is_string()) {
                let field_path = path(base, &[Seg::Key(field)]);
                self.push(
                    Finding::error(
                        "KR-HK-017",
                        At::Value(&field_path),
                        t!(
                            "rules.kr_hk_017.field_type",
                            index = index,
                            field = field,
                            found = found(value),
                            expected = "a string"
                        ),
                    )
                    .suggest(t!("rules.kr_hk_017.suggestion")),
                );
            }
        }
        if let Some(value) = hook.get("enabled").filter(|value| !value.is_boolean()) {
            let field_path = path(base, &[Seg::Key("enabled")]);
            let mut finding = Finding::error(
                "KR-HK-017",
                At::Value(&field_path),
                t!(
                    "rules.kr_hk_017.field_type",
                    index = index,
                    field = "enabled",
                    found = found(value),
                    expected = "true or false"
                ),
            )
            .suggest(t!("rules.kr_hk_017.suggestion"));
            // Unsafe like every quoted-boolean fix: the file is skipped today.
            if let Some(boolean) = value.as_str().filter(|s| matches!(*s, "true" | "false")) {
                finding = finding.fix(
                    &field_path,
                    boolean,
                    t!("rules.kr_hk_017.fix", value = boolean),
                    false,
                );
            }
            self.push(finding);
        }
        if let Some(value) = hook.get("timeout") {
            let field_path = path(base, &[Seg::Key("timeout")]);
            let problem = match value {
                Value::Number(number) => match number.as_f64() {
                    Some(seconds) if seconds.fract() != 0.0 => Some(t!(
                        "rules.kr_hk_017.timeout_fraction",
                        index = index,
                        value = number.to_string()
                    )),
                    Some(seconds) if seconds < 0.0 => Some(t!(
                        "rules.kr_hk_017.timeout_negative",
                        index = index,
                        value = number.to_string()
                    )),
                    _ => None,
                },
                other => Some(t!(
                    "rules.kr_hk_017.field_type",
                    index = index,
                    field = "timeout",
                    found = found(other),
                    expected = "a whole number of seconds"
                )),
            };
            if let Some(message) = problem {
                let mut finding = Finding::error("KR-HK-017", At::Value(&field_path), message)
                    .suggest(t!("rules.kr_hk_017.suggestion"));
                if let Some(digits) = value.as_str().filter(|s| is_plain_integer(s)) {
                    finding = finding.fix(
                        &field_path,
                        digits,
                        t!("rules.kr_hk_017.fix", value = digits),
                        false,
                    );
                }
                self.push(finding);
            }
        }
    }

    fn check_matcher(&mut self, index: usize, base: &[Seg<'_>], trigger: &str, matcher: &str) {
        if matcher.is_empty() {
            return; // empty = match everything
        }
        let matcher_path = path(base, &[Seg::Key("matcher")]);
        let subject = matcher_subject(trigger);
        // A tool matcher without regex metacharacters that fails to compile
        // is kept as a glob selector (for example "*" or "?ead*").
        let glob_selector = subject == MatcherSubject::Tool
            && is_tool_selector(matcher)
            && matcher.contains(['*', '?']);
        if let Err(error) = compile_js_regex(matcher)
            && !glob_selector
        {
            self.push(
                Finding::error(
                    "KR-HK-019",
                    At::Value(&matcher_path),
                    t!(
                        "rules.kr_hk_019.message",
                        index = index,
                        matcher = matcher,
                        error = error.to_string()
                    ),
                )
                .suggest(t!("rules.kr_hk_019.suggestion")),
            );
            return;
        }
        match subject {
            MatcherSubject::None => self.push(
                Finding::warning(
                    "KR-HK-020",
                    At::Value(&matcher_path),
                    t!("rules.kr_hk_020.message", index = index, trigger = trigger),
                )
                .suggest(t!("rules.kr_hk_020.suggestion")),
            ),
            MatcherSubject::Tool => {
                let names = if !matcher.contains('@')
                    && !matcher.contains("mcp_")
                    && tool_matcher_matches_builtin(matcher) == Some(false)
                {
                    foreign_tool_names(matcher)
                } else {
                    Vec::new()
                };
                if !names.is_empty() {
                    let names = names
                        .iter()
                        .map(|(name, origin, replacement)| {
                            t!(
                                "rules.kr_hk_021.name",
                                name = *name,
                                origin = *origin,
                                replacement = *replacement
                            )
                            .to_string()
                        })
                        .collect::<Vec<_>>()
                        .join("; ");
                    self.push(
                        Finding::warning(
                            "KR-HK-021",
                            At::Value(&matcher_path),
                            t!(
                                "rules.kr_hk_021.message",
                                index = index,
                                matcher = matcher,
                                names = names
                            ),
                        )
                        .suggest(t!("rules.kr_hk_021.suggestion")),
                    );
                    return;
                }
                // KR-HK-029: a regex-form tool matcher is an unanchored search
                // over tool ids, so `fs_write|str_replace` also matches any id
                // that merely contains those names.
                if !is_tool_selector(matcher)
                    && matcher.contains(['|', '('])
                    && !(matcher.starts_with('^') && matcher.ends_with('$'))
                {
                    self.push(
                        Finding::new(
                            "KR-HK-029",
                            DiagnosticLevel::Info,
                            At::Value(&matcher_path),
                            t!("rules.kr_hk_029.message", index = index, matcher = matcher),
                        )
                        .suggest(t!("rules.kr_hk_029.suggestion")),
                    );
                }
            }
            _ => {}
        }
    }

    /// KR-HK-027 / KR-HK-028: tool and file hooks that run on every event.
    /// The engine itself warns "Hook has no matcher and will run on every
    /// tool call"; the hooks best-practices page asks for precise matchers
    /// and specific file patterns.
    fn check_scope(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        trigger: &str,
        matcher: Option<&str>,
    ) {
        let subject = matcher_subject(trigger);
        if subject == MatcherSubject::None || !matches_everything(matcher, subject) {
            return;
        }
        let at_path = match matcher {
            Some(_) => path(base, &[Seg::Key("matcher")]),
            None => path(base, &[Seg::Key("trigger")]),
        };
        let (rule, message, suggestion) = if subject == MatcherSubject::Tool {
            (
                "KR-HK-027",
                t!("rules.kr_hk_027.message", index = index, trigger = trigger),
                t!("rules.kr_hk_027.suggestion"),
            )
        } else {
            (
                "KR-HK-028",
                t!("rules.kr_hk_028.message", index = index, trigger = trigger),
                t!("rules.kr_hk_028.suggestion"),
            )
        };
        self.push(
            Finding::new(rule, DiagnosticLevel::Info, At::Value(&at_path), message)
                .suggest(suggestion),
        );
    }

    /// KR-HK-018: `confirm` is validated with the hook, so a malformed block
    /// skips the whole file; it only acts on Stop hooks with command actions.
    fn check_confirm(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        confirm: &Value,
        trigger: Option<&str>,
        action: Option<&str>,
    ) {
        let confirm_path = path(base, &[Seg::Key("confirm")]);
        let Some(block) = confirm.as_object() else {
            self.push(
                Finding::error(
                    "KR-HK-018",
                    At::Value(&confirm_path),
                    t!(
                        "rules.kr_hk_018.confirm_type",
                        index = index,
                        found = found(confirm)
                    ),
                )
                .suggest(t!("rules.kr_hk_018.suggestion")),
            );
            return;
        };
        let mut problems = Vec::new();
        let block_path: Vec<Seg<'static>> =
            vec![Seg::Key("hooks"), Seg::Index(index), Seg::Key("confirm")];
        problems.extend(confirm_string_problem(
            index,
            block,
            &block_path,
            "question",
            "question",
            true,
        ));
        if block.contains_key("confirmCommand") {
            problems.extend(confirm_string_problem(
                index,
                block,
                &block_path,
                "confirmCommand",
                "confirmCommand",
                true,
            ));
        }
        let options_path = path(&block_path, &[Seg::Key("options")]);
        match block.get("options") {
            None => problems.push((
                block_path.clone(),
                t!(
                    "rules.kr_hk_018.missing_field",
                    index = index,
                    field = "options"
                )
                .to_string(),
            )),
            Some(Value::Array(options)) if options.is_empty() => problems.push((
                options_path,
                t!("rules.kr_hk_018.empty_options", index = index).to_string(),
            )),
            Some(Value::Array(options)) => {
                for (option_index, option) in options.iter().enumerate() {
                    let option_path = path(&options_path, &[Seg::Index(option_index)]);
                    let prefix = format!("options[{option_index}]");
                    let Some(option) = option.as_object() else {
                        problems.push((
                            option_path,
                            t!(
                                "rules.kr_hk_018.field_type",
                                index = index,
                                field = prefix,
                                found = found(option),
                                expected = "an object"
                            )
                            .to_string(),
                        ));
                        continue;
                    };
                    for field in ["id", "label"] {
                        problems.extend(confirm_string_problem(
                            index,
                            option,
                            &option_path,
                            field,
                            &format!("{prefix}.{field}"),
                            true,
                        ));
                    }
                    if option.contains_key("continueReason") {
                        problems.extend(confirm_string_problem(
                            index,
                            option,
                            &option_path,
                            "continueReason",
                            &format!("{prefix}.continueReason"),
                            false,
                        ));
                    }
                    let display = format!("{prefix}.run");
                    match option.get("run") {
                        Some(Value::Bool(_)) => {}
                        None => problems.push((
                            option_path.clone(),
                            t!(
                                "rules.kr_hk_018.missing_field",
                                index = index,
                                field = display
                            )
                            .to_string(),
                        )),
                        Some(other) => problems.push((
                            path(&option_path, &[Seg::Key("run")]),
                            t!(
                                "rules.kr_hk_018.field_type",
                                index = index,
                                field = display,
                                found = found(other),
                                expected = "true or false"
                            )
                            .to_string(),
                        )),
                    }
                }
            }
            Some(other) => problems.push((
                options_path,
                t!(
                    "rules.kr_hk_018.field_type",
                    index = index,
                    field = "options",
                    found = found(other),
                    expected = "an array"
                )
                .to_string(),
            )),
        }
        for (problem_path, message) in &problems {
            self.push(
                Finding::error("KR-HK-018", At::Value(problem_path), message.clone())
                    .suggest(t!("rules.kr_hk_018.suggestion")),
            );
        }
        if problems.is_empty()
            && let Some(trigger) = trigger
            && (trigger != "Stop" || action == Some("agent"))
        {
            self.push(
                Finding::warning(
                    "KR-HK-018",
                    At::Key(&confirm_path),
                    t!("rules.kr_hk_018.no_effect", index = index),
                )
                .suggest(t!("rules.kr_hk_018.no_effect_suggestion")),
            );
        }
        if let Some(Value::String(command)) = block.get("confirmCommand")
            && !command.is_empty()
        {
            let command_path = path(&confirm_path, &[Seg::Key("confirmCommand")]);
            self.check_command_text(
                index,
                &command_path,
                "confirm.confirmCommand",
                command,
                true,
            );
        }
    }

    /// KR-HK-022: keys Kiro strips silently, reported only where the intent
    /// is clear and dropping the key changes what the hook does.
    fn check_misplaced_fields(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        hook: &Map<String, Value>,
    ) {
        let has_action = hook.get("action").is_some_and(Value::is_object);
        // A misplaced copy is harmless when the real field is also set.
        let has_timeout = hook.contains_key("timeout");
        let has_matcher = hook.contains_key("matcher");
        for key in hook.keys() {
            if KIRO_V3_HOOK_KEYS.contains(&key.as_str()) {
                continue;
            }
            let message = if let Some(expected) = KIRO_V3_HOOK_KEYS
                .iter()
                .find(|known| known.eq_ignore_ascii_case(key))
            {
                t!(
                    "rules.kr_hk_022.misspelled",
                    index = index,
                    key = key.as_str(),
                    expected = *expected
                )
            } else {
                match key.as_str() {
                    "timeout_ms" | "timeoutMs" if !has_timeout => {
                        t!(
                            "rules.kr_hk_022.v2_timeout",
                            index = index,
                            key = key.as_str()
                        )
                    }
                    "patterns" | "toolTypes" if !has_matcher => {
                        t!(
                            "rules.kr_hk_022.legacy_filter",
                            index = index,
                            key = key.as_str()
                        )
                    }
                    "command" | "prompt" | "type" if has_action => {
                        t!(
                            "rules.kr_hk_022.belongs_in_action",
                            index = index,
                            key = key.as_str()
                        )
                    }
                    _ => continue,
                }
            };
            let key_path = path(base, &[Seg::Key(key)]);
            self.push(
                Finding::warning("KR-HK-022", At::Key(&key_path), message)
                    .suggest(t!("rules.kr_hk_022.suggestion")),
            );
        }
        let Some(Value::Object(action)) = hook.get("action") else {
            return;
        };
        let action_path = path(base, &[Seg::Key("action")]);
        for key in action.keys() {
            if KIRO_V3_HOOK_ACTION_KEYS.contains(&key.as_str()) {
                continue;
            }
            let message = if key == "timeout" && !has_timeout {
                t!("rules.kr_hk_022.action_timeout", index = index)
            } else if let Some(expected) = KIRO_V3_HOOK_ACTION_KEYS
                .iter()
                .find(|known| known.eq_ignore_ascii_case(key))
            {
                t!(
                    "rules.kr_hk_022.misspelled",
                    index = index,
                    key = format!("action.{key}"),
                    expected = format!("action.{expected}")
                )
            } else {
                continue;
            };
            let key_path = path(&action_path, &[Seg::Key(key)]);
            self.push(
                Finding::warning("KR-HK-022", At::Key(&key_path), message)
                    .suggest(t!("rules.kr_hk_022.suggestion")),
            );
        }
    }

    /// KR-HK-010: literal credentials in the command.
    /// KR-HK-023: the engine substitutes only `${WORKSPACE_ROOT}` in
    /// `action.command` (and only `$HOME` in `confirmCommand`); event data
    /// arrives as JSON on stdin and the environment only gains
    /// KIRO_SESSION_ID. Everything else reaches the shell unchanged.
    fn check_command_text(
        &mut self,
        index: usize,
        command_path: &[Seg<'_>],
        field: &str,
        command: &str,
        confirm_command: bool,
    ) {
        if super::kiro_hook::command_has_secret(command) {
            self.push(
                Finding::error(
                    "KR-HK-010",
                    At::Value(command_path),
                    t!("rules.kr_hk_010.message"),
                )
                .suggest(t!("rules.kr_hk_010.suggestion")),
            );
        }
        let mut messages = Vec::new();
        for placeholder in template_placeholders(command) {
            messages.push(t!(
                "rules.kr_hk_023.template",
                index = index,
                field = field,
                placeholder = placeholder
            ));
        }
        if bare_variable_references(command, "WORKSPACE_ROOT") {
            messages.push(t!(
                "rules.kr_hk_023.workspace_root",
                index = index,
                field = field
            ));
        }
        if confirm_command && command.contains("${WORKSPACE_ROOT}") {
            messages.push(t!("rules.kr_hk_023.confirm_workspace_root", index = index));
        }
        for variable in UNSET_EVENT_VARIABLES {
            if references_variable(command, variable) {
                messages.push(t!(
                    "rules.kr_hk_023.env",
                    index = index,
                    field = field,
                    variable = *variable
                ));
            }
        }
        for message in messages {
            self.push(
                Finding::warning("KR-HK-023", At::Value(command_path), message)
                    .suggest(t!("rules.kr_hk_023.suggestion")),
            );
        }
    }

    /// KR-HK-024: hook definitions that load but never run.
    fn check_never_runs(
        &mut self,
        index: usize,
        base: &[Seg<'_>],
        trigger: Option<&str>,
        action: Option<&str>,
    ) {
        let trigger_path = path(base, &[Seg::Key("trigger")]);
        match (trigger, action) {
            (Some("SessionEnd"), Some("agent")) => self.push(
                Finding::warning(
                    "KR-HK-024",
                    At::Value(&trigger_path),
                    t!("rules.kr_hk_024.session_end_agent", index = index),
                )
                .suggest(t!("rules.kr_hk_024.session_end_suggestion")),
            ),
            (Some("Manual"), _) => self.push(
                Finding::new(
                    "KR-HK-024",
                    DiagnosticLevel::Info,
                    At::Value(&trigger_path),
                    t!("rules.kr_hk_024.manual", index = index),
                )
                .suggest(t!("rules.kr_hk_024.manual_suggestion")),
            ),
            _ => {}
        }
    }
}

/// A `confirm` string field that is missing (when required), not a string,
/// or empty when `non_empty` (zod `min(1)`).
fn confirm_string_problem(
    index: usize,
    object: &Map<String, Value>,
    object_path: &[Seg<'static>],
    field: &'static str,
    display: &str,
    non_empty: bool,
) -> Option<(Vec<Seg<'static>>, String)> {
    let field_path = path(object_path, &[Seg::Key(field)]);
    match object.get(field) {
        None => Some((
            object_path.to_vec(),
            t!(
                "rules.kr_hk_018.missing_field",
                index = index,
                field = display
            )
            .to_string(),
        )),
        Some(Value::String(text)) if non_empty && text.is_empty() => Some((
            field_path,
            t!(
                "rules.kr_hk_018.empty_field",
                index = index,
                field = display
            )
            .to_string(),
        )),
        Some(Value::String(_)) => None,
        Some(other) => Some((
            field_path,
            t!(
                "rules.kr_hk_018.field_type",
                index = index,
                field = display,
                found = found(other),
                expected = "a string"
            )
            .to_string(),
        )),
    }
}

/// Environment variables other tools set for hooks; Kiro V3 passes the same
/// data only as JSON on stdin. USER_PROMPT is set by Kiro CLI 2.x.
const UNSET_EVENT_VARIABLES: &[&str] = &[
    "USER_PROMPT",
    "FILE_PATH",
    "TOOL_NAME",
    "TOOL_INPUT",
    "TOOL_RESPONSE",
];

/// Template names documented for IDE hooks or used by other tools for the
/// event's file or prompt; Kiro V3 substitutes none of them.
const TEMPLATE_NAMES: &[&str] = &[
    "filepath",
    "file_path",
    "file",
    "filename",
    "file_name",
    "prompt",
    "userprompt",
    "user_prompt",
    "toolname",
    "tool_name",
    "toolinput",
    "tool_input",
];

/// `{{name}}` placeholders (with optional inner spaces) whose name is one of
/// [`TEMPLATE_NAMES`], in order of appearance, without duplicates.
fn template_placeholders(command: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = command;
    while let Some(open) = rest.find("{{") {
        let after = &rest[open + 2..];
        let Some(close) = after.find("}}") else {
            break;
        };
        let name = after[..close].trim();
        if TEMPLATE_NAMES
            .iter()
            .any(|known| known.eq_ignore_ascii_case(name))
        {
            let placeholder = format!("{{{{{}}}}}", &after[..close]);
            if !found.contains(&placeholder) {
                found.push(placeholder);
            }
        }
        rest = &after[close + 2..];
    }
    found
}

fn is_identifier_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

/// `$NAME` written without braces (the shell expands it; Kiro does not).
fn bare_variable_references(command: &str, name: &str) -> bool {
    let needle = format!("${name}");
    command.match_indices(&needle).any(|(start, _)| {
        !command[start + needle.len()..]
            .chars()
            .next()
            .is_some_and(is_identifier_char)
    })
}

/// `$NAME` or `${NAME}`, not part of a longer identifier.
fn references_variable(command: &str, name: &str) -> bool {
    bare_variable_references(command, name) || command.contains(&format!("${{{name}}}"))
}

/// The hooks docs ask for "descriptive kebab-case names (e.g.,
/// `lint-on-save.json`)". Dot-separated parts such as `lint.hook.json` are
/// accepted; uppercase letters, underscores, spaces and other characters are
/// not.
fn is_kebab_case_hook_file_name(name: &str) -> bool {
    let stem = name.strip_suffix(".json").unwrap_or(name);
    !stem.is_empty()
        && stem.split('.').all(|part| {
            !part.is_empty()
                && part.split('-').all(|word| {
                    !word.is_empty()
                        && word
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                })
        })
}

/// A JSON integer token Kiro would accept as a timeout (no sign, no leading
/// zeros, short enough to stay an exact integer).
fn is_plain_integer(text: &str) -> bool {
    !text.is_empty()
        && text.len() <= 15
        && text.bytes().all(|b| b.is_ascii_digit())
        && (text == "0" || !text.starts_with('0'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;

    fn validate(content: &str) -> Vec<Diagnostic> {
        KiroV3HookValidator.validate(
            Path::new(".kiro/hooks/lint-on-save.json"),
            content,
            &LintConfig::default(),
        )
    }

    fn rules(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.rule.as_str()).collect()
    }

    fn hook_doc(hook: serde_json::Value) -> String {
        serde_json::to_string_pretty(&serde_json::json!({"version": "v1", "hooks": [hook]}))
            .unwrap()
    }

    fn command_hook(extra: serde_json::Value) -> String {
        let mut hook = serde_json::json!({
            "name": "a",
            "trigger": "Stop",
            "action": {"type": "command", "command": "echo"}
        });
        for (key, value) in extra.as_object().unwrap() {
            hook[key] = value.clone();
        }
        hook_doc(hook)
    }

    #[test]
    fn alias_spans_are_indexed_once_for_8000_hooks() {
        use crate::schemas::kiro_hook_v3::SPAN_INDEX_BUILDS;

        // Exercise both a long single line and many lines: neither source
        // lookup nor diagnostic location may repeatedly scan the prefix.
        for separator in [",", ",\n"] {
            let hooks = (0..8000).map(|i| format!(
                r#"{{"name":"é🙂-{i}","trigger":"agentSpawn","action":{{"type":"command","command":"echo ok"}}}}"#
            )).collect::<Vec<_>>().join(separator);
            let content = format!(r#"{{"version":"1","hooks":[{hooks}]}}"#);
            let before = SPAN_INDEX_BUILDS.with(|builds| builds.get());
            let diagnostics = validate(&content);
            assert_eq!(SPAN_INDEX_BUILDS.with(|builds| builds.get()) - before, 1);
            assert_eq!(diagnostics.len(), 8001);
            assert_eq!(diagnostics[0].rule, "KR-HK-011");
            for diagnostic in &diagnostics[1..] {
                assert_eq!(diagnostic.rule, "KR-HK-014");
                assert_eq!(diagnostic.fixes.len(), 1);
                let fix = &diagnostic.fixes[0];
                assert!(fix.is_safe());
                assert_eq!(&content[fix.start_byte..fix.end_byte], "\"agentSpawn\"");
                assert_eq!(fix.replacement, "\"SessionStart\"");
            }
            let last = diagnostics.last().unwrap();
            assert_eq!(
                (last.line, last.column),
                crate::rules::line_col_at_offset(&content, last.fixes[0].start_byte)
            );

            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec!["KR-HK-011".into(), "KR-HK-014".into()];
            let before = SPAN_INDEX_BUILDS.with(|builds| builds.get());
            assert!(
                KiroV3HookValidator
                    .validate(Path::new(".kiro/hooks/large.json"), &content, &config)
                    .is_empty()
            );
            assert_eq!(SPAN_INDEX_BUILDS.with(|builds| builds.get()), before);
        }
    }

    #[test]
    fn test_valid_standalone_hook_doc_produces_no_diagnostics() {
        let content = r#"{
  "version": "v1",
  "hooks": [
    {
      "name": "lint-on-save",
      "trigger": "PostFileSave",
      "matcher": "\\.(ts|tsx)$",
      "action": { "type": "command", "command": "npx eslint --fix" },
      "timeout": 30,
      "enabled": true
    },
    { "name": "ctx", "trigger": "UserPromptSubmit", "action": { "type": "agent", "prompt": "Add context" } },
    { "name": "guard", "trigger": "PreToolUse", "matcher": "write", "action": { "type": "command", "command": "./guard.sh ${WORKSPACE_ROOT}" }, "timeout": 0 },
    { "name": "mcp", "trigger": "PostToolUse", "matcher": "@github", "action": { "type": "command", "command": "jq -r .tool_name" }, "timeout": 7200 },
    {
      "name": "submit", "trigger": "Stop", "description": "Ask first",
      "action": { "type": "command", "command": "./submit.sh" },
      "confirm": {
        "question": "Submit?", "confirmCommand": "$HOME/bin/confirm.sh",
        "options": [ { "id": "yes", "label": "Yes", "run": true }, { "id": "no", "label": "No", "run": false, "continueReason": "skipped" } ]
      }
    }
  ]
}"#;
        let diagnostics = validate(content);
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
    }

    #[test]
    fn test_kr_hk_011_missing_invalid_and_typed_versions_are_errors() {
        // The engine skips the whole file for any version but "v1".
        for (content, line) in [
            (
                r#"{ "hooks": [ { "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "echo" } } ] }"#,
                1,
            ),
            (
                "{\n \"version\": \"v2\",\n \"hooks\": [ { \"name\": \"a\", \"trigger\": \"Stop\", \"action\": { \"type\": \"command\", \"command\": \"echo\" } } ] }",
                2,
            ),
            (
                "{\n \"version\": 1,\n \"hooks\": [ { \"name\": \"a\", \"trigger\": \"Stop\", \"action\": { \"type\": \"command\", \"command\": \"echo\" } } ] }",
                2,
            ),
        ] {
            let diagnostics = validate(content);
            assert_eq!(rules(&diagnostics), ["KR-HK-011"], "{content}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
            assert_eq!(diagnostics[0].line, line);
        }
    }

    #[test]
    fn type_errors_are_reported_where_they_are_and_checking_continues() {
        // Previously any type error became one "not valid JSON" KR-HK-012
        // and hid every other check.
        let content = "{\n  \"version\": 1,\n  \"hooks\": [\n    { \"name\": \"a\", \"trigger\": \"Stop\", \"enabled\": \"true\",\n      \"action\": { \"type\": \"command\", \"command\": \"echo\" } },\n    { \"name\": 2, \"trigger\": \"onSave\", \"action\": { \"type\": \"http\" } }\n  ]\n}";
        let diagnostics = validate(content);
        let summary: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.rule.as_str(), d.line, d.column))
            .collect();
        assert_eq!(
            summary,
            [
                ("KR-HK-011", 2, 14),
                ("KR-HK-017", 4, 50),
                ("KR-HK-012", 6, 15),
                ("KR-HK-012", 6, 29),
                ("KR-HK-013", 6, 59),
            ],
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|d| !d.message.contains("not valid JSON"))
        );
    }

    #[test]
    fn test_kr_hk_012_shapes() {
        for (content, needle) in [
            (r#"[]"#, "must be a JSON object"),
            (r#"{ "version": "v1" }"#, "no \"hooks\" array"),
            (r#"{ "version": "v1", "hooks": [] }"#, "empty"),
            (r#"{ "version": "v1", "hooks": {} }"#, "not an array"),
            (r#"{ "version": "v1", "hooks": ["x"] }"#, "not an object"),
            (
                r#"{ "name": "a", "trigger": "Stop", "action": { "type": "command", "command": "x" } }"#,
                "wrapper",
            ),
        ] {
            let diagnostics = validate(content);
            assert_eq!(
                rules(&diagnostics),
                ["KR-HK-012"],
                "{content}: {diagnostics:?}"
            );
            assert!(
                diagnostics[0].message.contains(needle),
                "{content}: {}",
                diagnostics[0].message
            );
        }
    }

    #[test]
    fn test_kr_hk_012_required_fields() {
        let diagnostics = validate(r#"{ "version": "v1", "hooks": [ { "trigger": "" } ] }"#);
        assert_eq!(rules(&diagnostics), ["KR-HK-012", "KR-HK-012", "KR-HK-012"]);
        // Whitespace passes zod's min(1); only "" is empty.
        assert!(validate(&hook_doc(serde_json::json!({"name": " ", "trigger": "Stop", "action": {"type": "command", "command": "echo"}}))).is_empty());
        let flat = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "Stop", "command": "echo"}),
        ));
        assert_eq!(rules(&flat), ["KR-HK-012"]);
        assert!(
            flat[0].message.contains("must be inside action"),
            "{}",
            flat[0].message
        );
    }

    #[test]
    fn test_kr_hk_012_unknown_trigger_skips_only_the_hook() {
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "OnFileChange", "action": {"type": "command", "command": "echo"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-012"]);
        assert!(diagnostics[0].message.contains("OnFileChange"));
        assert!(diagnostics[0].message.contains("skips this hook"));
        assert!(diagnostics[0].fixes.is_empty());
        // Untrimmed names are unknown too: the lookup is exact.
        assert_eq!(
            rules(&validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": " Stop", "action": {"type": "command", "command": "echo"}})
            ))),
            ["KR-HK-012"]
        );
    }

    #[test]
    fn pascal_case_agent_spawn_is_rejected_with_unsafe_fix() {
        let content = hook_doc(
            serde_json::json!({"name": "a", "trigger": "AgentSpawn", "action": {"type": "command", "command": "git status"}}),
        );
        let diagnostics = validate(&content);
        assert_eq!(rules(&diagnostics), ["KR-HK-012"]);
        let fix = &diagnostics[0].fixes[0];
        assert!(!fix.is_safe());
        assert_eq!(&content[fix.start_byte..fix.end_byte], "\"AgentSpawn\"");
        assert_eq!(fix.replacement, "\"SessionStart\"");
    }

    #[test]
    fn every_engine_alias_is_accepted_with_a_safe_canonical_fix() {
        for (alias, canonical) in crate::schemas::kiro_hook_v3::KIRO_V3_HOOK_TRIGGER_ALIASES {
            let content = hook_doc(
                serde_json::json!({"name": "a", "trigger": alias, "action": {"type": "command", "command": "echo"}}),
            );
            let diagnostics: Vec<_> = validate(&content)
                .into_iter()
                // Hooks without a matcher on tool and file triggers also get the
                // KR-HK-027 / KR-HK-028 scope notes, and Manual gets KR-HK-024.
                .filter(|d| !matches!(d.rule.as_str(), "KR-HK-024" | "KR-HK-027" | "KR-HK-028"))
                .collect();
            assert_eq!(rules(&diagnostics), ["KR-HK-014"], "{alias}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
            assert!(diagnostics[0].fixes[0].is_safe());
            assert_eq!(
                diagnostics[0].fixes[0].replacement,
                format!("\"{canonical}\"")
            );
        }
    }

    #[test]
    fn test_kr_hk_013_actions() {
        for (action, level) in [
            (
                serde_json::json!({"type": "http", "url": "https://x"}),
                DiagnosticLevel::Error,
            ),
            (
                serde_json::json!({"type": "command"}),
                DiagnosticLevel::Error,
            ),
            (
                serde_json::json!({"type": "command", "command": ""}),
                DiagnosticLevel::Error,
            ),
            (
                serde_json::json!({"type": "command", "command": 3}),
                DiagnosticLevel::Error,
            ),
            (
                serde_json::json!({"type": "agent", "prompt": "  "}),
                DiagnosticLevel::Warning,
            ),
            (
                serde_json::json!({"command": "echo"}),
                DiagnosticLevel::Error,
            ),
            (serde_json::json!("echo"), DiagnosticLevel::Error),
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "Stop", "action": action}),
            ));
            assert_eq!(rules(&diagnostics), ["KR-HK-013"], "{action}");
            assert_eq!(diagnostics[0].level, level, "{action}");
        }
    }

    #[test]
    fn legacy_action_type_gets_unsafe_fix_only_with_matching_payload() {
        let content = hook_doc(
            serde_json::json!({"name": "a", "trigger": "Stop", "action": {"type": "runCommand", "command": "echo"}}),
        );
        let diagnostics = validate(&content);
        assert_eq!(diagnostics[0].fixes[0].replacement, "\"command\"");
        assert!(!diagnostics[0].fixes[0].is_safe());
        let content = hook_doc(
            serde_json::json!({"name": "a", "trigger": "Stop", "action": {"type": "askAgent", "command": "echo"}}),
        );
        assert!(validate(&content)[0].fixes.is_empty());
    }

    #[test]
    fn test_kr_hk_017_timeout_has_no_ceiling_and_bad_values_fail_the_file() {
        for timeout in [
            serde_json::json!(0),
            serde_json::json!(60),
            serde_json::json!(7200),
            serde_json::json!(30.0),
        ] {
            assert!(
                validate(&command_hook(serde_json::json!({"timeout": timeout}))).is_empty(),
                "{timeout}"
            );
        }
        for timeout in [
            serde_json::json!(-1),
            serde_json::json!(1.5),
            serde_json::json!("30"),
            serde_json::json!(null),
        ] {
            let diagnostics = validate(&command_hook(serde_json::json!({"timeout": timeout})));
            assert_eq!(rules(&diagnostics), ["KR-HK-017"], "{timeout}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
            assert_eq!(
                diagnostics[0].fixes.len(),
                usize::from(timeout == "30"),
                "{timeout}"
            );
        }
    }

    #[test]
    fn test_kr_hk_017_field_types() {
        for field in [
            serde_json::json!({"enabled": "false"}),
            serde_json::json!({"enabled": 1}),
            serde_json::json!({"matcher": ["a"]}),
            serde_json::json!({"description": 3}),
        ] {
            assert_eq!(
                rules(&validate(&command_hook(field.clone()))),
                ["KR-HK-017"],
                "{field}"
            );
        }
        let content = command_hook(serde_json::json!({"enabled": "false"}));
        let fix = &validate(&content)[0].fixes[0];
        assert_eq!((fix.replacement.as_str(), fix.is_safe()), ("false", false));
    }

    #[test]
    fn test_kr_hk_018_confirm() {
        let valid = serde_json::json!({"question": "Go?", "options": [{"id": "y", "label": "Yes", "run": true}]});
        assert!(validate(&command_hook(serde_json::json!({"confirm": valid}))).is_empty());
        for confirm in [
            serde_json::json!("yes"),
            serde_json::json!({"options": [{"id": "y", "label": "Yes", "run": true}]}),
            serde_json::json!({"question": "Go?", "options": []}),
            serde_json::json!({"question": "Go?", "options": [{"id": "y", "label": "Yes", "run": "yes"}]}),
            serde_json::json!({"question": "Go?", "confirmCommand": "", "options": [{"id": "y", "label": "Yes", "run": true}]}),
        ] {
            let diagnostics = validate(&command_hook(serde_json::json!({"confirm": confirm})));
            assert_eq!(rules(&diagnostics), ["KR-HK-018"], "{confirm}");
            assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
        }
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": "shell", "action": {"type": "command", "command": "echo"}, "confirm": valid}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-018"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
    }

    #[test]
    fn test_kr_hk_019_invalid_regex_matchers() {
        for (trigger, matcher) in [
            ("PostFileSave", "*.ts"),
            ("PostFileSave", "src/**"),
            ("PreToolUse", "*|read"),
            ("Stop", "("),
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": trigger, "matcher": matcher, "action": {"type": "command", "command": "echo"}}),
            ));
            assert_eq!(rules(&diagnostics), ["KR-HK-019"], "{matcher}");
        }
        // Glob selectors on tool triggers are fine even though they are not
        // regexes; "*" selects every tool, which is a KR-HK-027 note.
        for matcher in ["*", "?ead*", "fs_*", "@mcp", "@github/*"] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": matcher, "action": {"type": "command", "command": "echo"}}),
            ));
            let expected: &[&str] = if matcher == "*" { &["KR-HK-027"] } else { &[] };
            assert_eq!(rules(&diagnostics), expected, "{matcher}");
        }
    }

    #[test]
    fn test_kr_hk_020_and_021_matcher_semantics() {
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "UserPromptSubmit", "matcher": "deploy", "action": {"type": "agent", "prompt": "x"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-020"]);
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": "^Bash$|^Write$", "action": {"type": "command", "command": "echo"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-021"]);
        assert!(diagnostics[0].message.contains("Claude Code"));
        for matcher in [
            "fs_write|Bash",
            "^(execute_bash|bash)$",
            "mcp_github_Bash",
            "@builtin",
            "grep",
            "^my_tool$",
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": matcher, "action": {"type": "command", "command": "echo"}}),
            ));
            // fs_write matches, so no KR-HK-021; the alternation is unanchored (KR-HK-029).
            let expected: &[&str] = if matcher == "fs_write|Bash" {
                &["KR-HK-029"]
            } else {
                &[]
            };
            assert_eq!(rules(&diagnostics), expected, "{matcher}");
        }
    }

    #[test]
    fn test_kr_hk_027_028_hooks_without_a_specific_matcher() {
        for (trigger, matcher, expected) in [
            ("PreToolUse", None, Some("KR-HK-027")),
            ("PostToolUse", Some(""), Some("KR-HK-027")),
            ("preToolUse", Some("*"), Some("KR-HK-027")),
            ("PreToolUse", Some(".*"), Some("KR-HK-027")),
            ("PreToolUse", Some("shell"), None),
            ("PostFileSave", None, Some("KR-HK-028")),
            ("PostFileCreate", Some("^.*$"), Some("KR-HK-028")),
            ("fileDeleted", Some(".+"), Some("KR-HK-028")),
            ("PostFileSave", Some("\\.ts$"), None),
            ("Stop", None, None),
        ] {
            let mut hook = serde_json::json!({"name": "a", "trigger": trigger, "action": {"type": "command", "command": "echo"}});
            if let Some(matcher) = matcher {
                hook["matcher"] = matcher.into();
            }
            let content = hook_doc(hook);
            let diagnostics: Vec<_> = validate(&content)
                .into_iter()
                .filter(|d| d.rule != "KR-HK-014")
                .collect();
            assert_eq!(
                rules(&diagnostics),
                expected.into_iter().collect::<Vec<_>>(),
                "{trigger} {matcher:?}"
            );
            if let Some(d) = diagnostics.first() {
                assert_eq!(d.level, DiagnosticLevel::Info);
                // Points at the matcher, or at the trigger when there is none.
                let key = if matcher.is_some() {
                    "\"matcher\": "
                } else {
                    "\"trigger\": "
                };
                let offset = content.find(key).unwrap() + key.len();
                assert_eq!(
                    (d.line, d.column),
                    crate::rules::line_col_at_offset(&content, offset),
                    "{trigger} {matcher:?}"
                );
            }
        }
    }

    #[test]
    fn test_kr_hk_029_unanchored_regex_tool_matchers() {
        for (matcher, expected) in [
            ("fs_write|str_replace", true),
            ("([Ss]hell|[Bb]ash)", true),
            ("^fs_write|str_replace", true),
            ("^(fs_write|str_replace)$", false),
            ("^fs_write$|^str_replace$", false),
            ("shell", false),          // selector form
            ("^execute_bash$", false), // no alternation or group
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": matcher, "action": {"type": "command", "command": "echo"}}),
            ));
            let expected: &[&str] = if expected { &["KR-HK-029"] } else { &[] };
            assert_eq!(rules(&diagnostics), expected, "{matcher}");
        }
        // File matchers are paths, not tool ids: not reported.
        assert!(validate(&hook_doc(serde_json::json!({"name": "a", "trigger": "PostFileSave", "matcher": "\\.(ts|tsx)$", "action": {"type": "command", "command": "echo"}}))).is_empty());
    }

    #[test]
    fn test_kr_hk_021_cli_2x_aws_alias() {
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "PreToolUse", "matcher": "aws", "action": {"type": "command", "command": "echo"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-021"]);
    }

    #[test]
    fn test_kr_hk_030_hook_file_names() {
        let content = command_hook(serde_json::json!({}));
        for (name, kebab) in [
            ("lint-on-save.json", true),
            ("guard-writes.json", true),
            ("v2-hooks.json", true),
            ("lint.hook.json", true),
            ("LintOnSave.json", false),
            ("lint_on_save.json", false),
            ("lint on save.json", false),
            ("-lint.json", false),
        ] {
            let diagnostics = KiroV3HookValidator.validate(
                &Path::new(".kiro/hooks").join(name),
                &content,
                &LintConfig::default(),
            );
            let expected: &[&str] = if kebab { &[] } else { &["KR-HK-030"] };
            assert_eq!(rules(&diagnostics), expected, "{name}");
        }
    }

    #[test]
    fn test_kr_hk_028_legacy_patterns_matching_every_file() {
        let path = Path::new(".kiro/hooks/all.kiro.hook");
        for (patterns, expected) in [
            (serde_json::json!(["**/*"]), true),
            (serde_json::json!(["*", "**"]), true),
            (serde_json::json!(["src/**/*.ts"]), false),
            (serde_json::json!(["**/*", "src/*.ts"]), false),
        ] {
            let content = serde_json::json!({"when": {"type": "fileEdited", "patterns": patterns}, "then": {"type": "askAgent", "prompt": "x"}}).to_string();
            let diagnostics = KiroV3HookValidator.validate(path, &content, &LintConfig::default());
            let expected: &[&str] = if expected { &["KR-HK-028"] } else { &[] };
            assert_eq!(rules(&diagnostics), expected, "{patterns}");
        }
    }

    #[test]
    fn test_kr_hk_022_misplaced_fields() {
        let content = hook_doc(serde_json::json!({
            "name": "a", "trigger": "PostFileSave", "Matcher": "\\.rs$", "timeout_ms": 5000,
            "action": {"type": "command", "command": "cargo fmt", "timeout": 30}, "note": "kept"
        }));
        let diagnostics = validate(&content);
        // "Matcher" is dropped, so the hook also runs on every file (KR-HK-028).
        assert_eq!(
            rules(&diagnostics),
            ["KR-HK-028", "KR-HK-022", "KR-HK-022", "KR-HK-022"]
        );
        assert!(
            diagnostics[1..]
                .iter()
                .all(|d| d.level == DiagnosticLevel::Warning)
        );
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("action.timeout"))
        );
        // A misplaced copy next to the real field is harmless.
        let content = hook_doc(serde_json::json!({
            "name": "a", "trigger": "PostFileSave", "matcher": "\\.rs$", "patterns": ["*.rs"],
            "timeout": 30, "timeout_ms": 30000,
            "action": {"type": "command", "command": "cargo fmt", "timeout": 30}
        }));
        assert!(validate(&content).is_empty());
    }

    #[test]
    fn test_kr_hk_023_placeholders() {
        for (command, count) in [
            ("prettier --write {{filePath}}", 1),
            ("npx stylelint \"$FILE_PATH\" --fix", 1),
            ("cd $WORKSPACE_ROOT && make", 1),
            ("echo ${USER_PROMPT}", 1),
            ("cd ${WORKSPACE_ROOT} && jq -r .file_path", 0),
            ("docker inspect --format '{{.Name}}' x", 0),
            ("echo $FILE_PATHS $WORKSPACE_ROOTS", 0),
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "PostFileSave", "matcher": "\\.rs$", "action": {"type": "command", "command": command}}),
            ));
            assert_eq!(diagnostics.len(), count, "{command}: {diagnostics:?}");
            assert!(diagnostics.iter().all(|d| d.rule == "KR-HK-023"));
        }
        let confirm = serde_json::json!({"question": "Go?", "confirmCommand": "${WORKSPACE_ROOT}/c.sh", "options": [{"id": "y", "label": "Y", "run": true}]});
        assert_eq!(
            rules(&validate(&command_hook(
                serde_json::json!({"confirm": confirm})
            ))),
            ["KR-HK-023"]
        );
    }

    #[test]
    fn test_kr_hk_010_secret_in_v1_command() {
        let content = "{\n  \"version\": \"v1\",\n  \"hooks\": [\n    {\n      \"name\": \"notify\",\n      \"trigger\": \"Stop\",\n      \"action\": {\"type\": \"command\", \"command\": \"curl -H 'Authorization: Bearer sk-live-4f9a8b7c6d5e4f3a2b1c' https://example.com\"}\n    }\n  ]\n}";
        let diagnostics = validate(content);
        assert_eq!(rules(&diagnostics), ["KR-HK-010"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Error);
        assert_eq!(diagnostics[0].line, 7);
        // The confirmation command runs in a shell too.
        let confirm = serde_json::json!({"question": "Go?", "confirmCommand": "notify --token=ghp_abcdefghijklmnopqrstuvwxyz0123456789", "options": [{"id": "y", "label": "Y", "run": true}]});
        assert_eq!(
            rules(&validate(&command_hook(
                serde_json::json!({"confirm": confirm})
            ))),
            ["KR-HK-010"]
        );
        // References resolved at run time are not secrets.
        for command in [
            "deploy --token=$DEPLOY_TOKEN",
            "curl -H \"Authorization: Bearer ${GITHUB_TOKEN}\" https://x",
        ] {
            let diagnostics = validate(&hook_doc(
                serde_json::json!({"name": "a", "trigger": "Stop", "action": {"type": "command", "command": command}}),
            ));
            assert!(diagnostics.is_empty(), "{command}: {diagnostics:?}");
        }
    }

    #[test]
    fn test_kr_hk_024_hooks_that_never_run() {
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "SessionEnd", "action": {"type": "agent", "prompt": "bye"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-024"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Warning);
        let diagnostics = validate(&hook_doc(
            serde_json::json!({"name": "a", "trigger": "Manual", "action": {"type": "command", "command": "echo"}}),
        ));
        assert_eq!(rules(&diagnostics), ["KR-HK-024"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
        assert!(validate(&hook_doc(serde_json::json!({"name": "a", "trigger": "SessionEnd", "action": {"type": "command", "command": "echo"}}))).is_empty());
    }

    #[test]
    fn test_kr_hk_015_legacy_documents_get_one_diagnostic() {
        let content = r#"{
  "name": "Lint", "version": "1.0.0",
  "when": { "type": "fileEdited", "patterns": ["**/*.ts"] },
  "then": { "type": "askAgent", "prompt": "Lint it" }
}"#;
        let diagnostics = validate(content);
        assert_eq!(rules(&diagnostics), ["KR-HK-015"]);
        assert_eq!((diagnostics[0].line, diagnostics[0].column), (3, 3));
        let entry = r#"{"version":"v1","hooks":[{"name":"a","when":{"type":"fileEdited"},"then":{"type":"command","command":"x"}}]}"#;
        assert_eq!(rules(&validate(entry)), ["KR-HK-015"]);
    }

    #[test]
    fn test_kr_hk_025_pinned_cli_2x() {
        let config = LintConfig::builder()
            .tool_versions(crate::config::ToolVersions {
                kiro: Some("2.32.1".into()),
                ..Default::default()
            })
            .build()
            .unwrap();
        let content = command_hook(serde_json::json!({}));
        let diagnostics =
            KiroV3HookValidator.validate(Path::new(".kiro/hooks/a.json"), &content, &config);
        assert_eq!(rules(&diagnostics), ["KR-HK-025"]);
        assert_eq!(diagnostics[0].level, DiagnosticLevel::Info);
    }

    #[test]
    fn test_parse_error_surfaces_as_kr_hk_012() {
        let diagnostics = validate("{ \"version\": \"v1\",\n \"hooks\": [ }");
        assert_eq!(rules(&diagnostics), ["KR-HK-012"]);
        assert_eq!(diagnostics[0].line, 2);
        assert!(diagnostics[0].message.contains("not valid JSON"));
    }

    #[test]
    fn duplicate_keys_follow_json_parse_last_wins() {
        // JSON.parse keeps the last duplicate; the first "Stop" is ignored.
        let content = r#"{"version":"v1","hooks":[{"name":"a","trigger":"Stop","trigger":"bogus","action":{"type":"command","command":"echo"}}]}"#;
        let diagnostics = validate(content);
        assert_eq!(rules(&diagnostics), ["KR-HK-012"]);
        assert!(diagnostics[0].message.contains("bogus"));
        let at = content.rfind("\"bogus\"").unwrap();
        assert_eq!(diagnostics[0].column, at + 1);
    }

    #[test]
    fn helpers() {
        assert_eq!(
            template_placeholders("a {{ filePath }} {{x}} {{filePath}} {{ filePath }}"),
            ["{{ filePath }}", "{{filePath}}"]
        );
        assert!(bare_variable_references(
            "cd $WORKSPACE_ROOT/x",
            "WORKSPACE_ROOT"
        ));
        assert!(!bare_variable_references(
            "cd ${WORKSPACE_ROOT}",
            "WORKSPACE_ROOT"
        ));
        assert!(!bare_variable_references(
            "cd $WORKSPACE_ROOTS",
            "WORKSPACE_ROOT"
        ));
        assert!(is_plain_integer("30") && is_plain_integer("0"));
        assert!(
            !is_plain_integer("030")
                && !is_plain_integer("-1")
                && !is_plain_integer("1.5")
                && !is_plain_integer("")
        );
    }
}
