//! Kiro IDE 0.x hook file schema helpers.
//!
//! Covers `.kiro/hooks/*.kiro.hook` JSON payloads. Kiro CLI never reads these
//! files; Kiro IDE 1.0 lists them as legacy hooks and offers to upgrade them
//! to the `.kiro/hooks/*.json` v1 format. Real files use
//! `{ name, description, version, enabled, when: { type, patterns?,
//! toolTypes? }, then: { type: askAgent | runCommand, prompt | command } }`.
//! The flat `event` / `patterns` / `runCommand` / `askAgent` spellings are
//! still read for compatibility with older agnix fixtures.

use crate::schemas::kiro_hook_v3::Seg;
use serde_json::Value;

/// `when.type` values documented for IDE 0.x (IDE 0.x reference trigger
/// table, plus `agentSpawn` from the IDE 1.0 migration table).
pub const DOCUMENTED_KIRO_IDE_HOOK_EVENTS: &[&str] = &[
    "fileEdited",
    "fileCreated",
    "fileDeleted",
    "promptSubmit",
    "agentStop",
    "preToolUse",
    "postToolUse",
    "preTaskExecution",
    "postTaskExecution",
    "userTriggered",
    "sessionStart",
    "agentSpawn",
];

/// Spellings agnix accepted before the docs were re-checked. They are not
/// documented, but whether IDE 0.x accepted them is unverified (no IDE code
/// was available), so they are tolerated rather than reported.
pub const TOLERATED_KIRO_IDE_HOOK_EVENTS: &[&str] = &["fileCreate", "fileDelete", "manual"];

pub fn is_known_ide_event(event: &str) -> bool {
    DOCUMENTED_KIRO_IDE_HOOK_EVENTS.contains(&event)
        || TOLERATED_KIRO_IDE_HOOK_EVENTS.contains(&event)
}

/// A field with the wrong JSON type: where it is and what was expected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldTypeError {
    pub path: Vec<Seg<'static>>,
    pub expected: &'static str,
}

/// A string value and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Located<'a> {
    pub value: &'a str,
    pub path: Vec<Seg<'static>>,
}

/// A list of strings (non-string entries are reported as type errors).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StringList<'a> {
    pub values: Vec<&'a str>,
    /// The list exists but is not an array, so its contents are unknown.
    pub malformed: bool,
    pub path: Vec<Seg<'static>>,
}

/// Lenient view of a legacy IDE hook. A field of the wrong type is recorded
/// in `type_errors` and treated as present but unreadable, so checks that
/// depend on it stay silent instead of reporting it as missing.
#[derive(Debug, Clone, Default)]
pub struct KiroIdeHook<'a> {
    pub event: Option<Located<'a>>,
    pub event_malformed: bool,
    pub patterns: Option<StringList<'a>>,
    pub tool_types: Option<StringList<'a>>,
    pub run_command: Option<Located<'a>>,
    pub ask_agent: Option<Located<'a>>,
    pub action_malformed: bool,
    pub enabled: Option<bool>,
    pub type_errors: Vec<FieldTypeError>,
    /// `timeout` / `then.timeout` values below zero (the IDE documents the
    /// field in seconds, with 0 allowed).
    pub negative_timeouts: Vec<Vec<Seg<'static>>>,
}

impl<'a> KiroIdeHook<'a> {
    pub fn from_value(root: &'a Value) -> Self {
        let mut hook = Self::default();
        let Some(root) = root.as_object() else {
            return hook;
        };
        let when = object_field(root, "when", &[], &mut hook.type_errors);
        let then = object_field(root, "then", &[], &mut hook.type_errors);

        let when_path = [Seg::Key("when")];
        let mut event_error = false;
        hook.event = when
            .and_then(|when| {
                string_field(
                    when,
                    "type",
                    &when_path,
                    &mut hook.type_errors,
                    &mut event_error,
                )
            })
            .or_else(|| string_field(root, "event", &[], &mut hook.type_errors, &mut event_error));
        hook.event_malformed = event_error || root.get("when").is_some_and(|w| !w.is_object());

        let when_patterns =
            when.and_then(|when| list_field(when, "patterns", &when_path, &mut hook.type_errors));
        let when_tool_types =
            when.and_then(|when| list_field(when, "toolTypes", &when_path, &mut hook.type_errors));
        let top_patterns = list_field(root, "patterns", &[], &mut hook.type_errors);
        let top_tool_types = list_field(root, "toolTypes", &[], &mut hook.type_errors);
        hook.tool_types = when_tool_types
            .clone()
            .or_else(|| when_patterns.clone())
            .or(top_tool_types);
        hook.patterns = when_patterns.or(top_patterns);

        let then_path = [Seg::Key("then")];
        let mut action_error = root.get("then").is_some_and(|t| !t.is_object());
        hook.run_command = string_field(
            root,
            "runCommand",
            &[],
            &mut hook.type_errors,
            &mut action_error,
        )
        .or_else(|| {
            let then = then?;
            string_field(
                then,
                "runCommand",
                &then_path,
                &mut hook.type_errors,
                &mut action_error,
            )
            .or_else(|| {
                string_field(
                    then,
                    "command",
                    &then_path,
                    &mut hook.type_errors,
                    &mut action_error,
                )
            })
        });
        hook.ask_agent = string_field(
            root,
            "askAgent",
            &[],
            &mut hook.type_errors,
            &mut action_error,
        )
        .or_else(|| {
            let then = then?;
            string_field(
                then,
                "askAgent",
                &then_path,
                &mut hook.type_errors,
                &mut action_error,
            )
            .or_else(|| {
                string_field(
                    then,
                    "prompt",
                    &then_path,
                    &mut hook.type_errors,
                    &mut action_error,
                )
            })
        });
        hook.action_malformed = action_error;
        hook.enabled = root.get("enabled").and_then(Value::as_bool);
        // IDE files carry `then.timeout` in seconds (the IDE 0.x reference
        // lists `timeout` among the 1.0 fields, example 10); no upper bound.
        for (object, parent) in [(Some(root), &[][..]), (then, &then_path[..])] {
            let Some(value) = object.and_then(|object| object.get("timeout")) else {
                continue;
            };
            let path = child_path(parent, "timeout");
            match value.as_f64() {
                Some(seconds) if seconds < 0.0 => hook.negative_timeouts.push(path),
                Some(_) => {}
                None => hook.type_errors.push(FieldTypeError {
                    path,
                    expected: "a number of seconds",
                }),
            }
        }
        hook.type_errors.dedup();
        hook
    }

    pub fn effective_event(&self) -> Option<&'a str> {
        self.event.as_ref().map(|event| event.value)
    }

    pub fn effective_run_command(&self) -> Option<&'a str> {
        self.run_command.as_ref().map(|command| command.value)
    }

    pub fn effective_ask_agent(&self) -> Option<&'a str> {
        self.ask_agent.as_ref().map(|prompt| prompt.value)
    }

    /// True if either supported action is configured with non-blank text.
    pub fn has_action(&self) -> bool {
        self.effective_run_command()
            .is_some_and(|command| !command.trim().is_empty())
            || self
                .effective_ask_agent()
                .is_some_and(|prompt| !prompt.trim().is_empty())
    }
}

fn child_path(parent: &[Seg<'static>], key: &'static str) -> Vec<Seg<'static>> {
    let mut path = parent.to_vec();
    path.push(Seg::Key(key));
    path
}

fn object_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &'static str,
    parent: &[Seg<'static>],
    errors: &mut Vec<FieldTypeError>,
) -> Option<&'a serde_json::Map<String, Value>> {
    match object.get(key)? {
        Value::Object(map) => Some(map),
        _ => {
            errors.push(FieldTypeError {
                path: child_path(parent, key),
                expected: "an object",
            });
            None
        }
    }
}

fn string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &'static str,
    parent: &[Seg<'static>],
    errors: &mut Vec<FieldTypeError>,
    malformed: &mut bool,
) -> Option<Located<'a>> {
    match object.get(key)? {
        Value::String(value) => Some(Located {
            value,
            path: child_path(parent, key),
        }),
        _ => {
            errors.push(FieldTypeError {
                path: child_path(parent, key),
                expected: "a string",
            });
            *malformed = true;
            None
        }
    }
}

fn list_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &'static str,
    parent: &[Seg<'static>],
    errors: &mut Vec<FieldTypeError>,
) -> Option<StringList<'a>> {
    let path = child_path(parent, key);
    match object.get(key)? {
        Value::Array(items) => {
            let mut list = StringList {
                path: path.clone(),
                ..StringList::default()
            };
            for (index, item) in items.iter().enumerate() {
                match item.as_str() {
                    Some(value) => list.values.push(value),
                    None => {
                        let mut item_path = path.clone();
                        item_path.push(Seg::Index(index));
                        errors.push(FieldTypeError {
                            path: item_path,
                            expected: "a string",
                        });
                        list.malformed = true;
                    }
                }
            }
            Some(list)
        }
        _ => {
            errors.push(FieldTypeError {
                path: path.clone(),
                expected: "an array of strings",
            });
            Some(StringList {
                values: Vec::new(),
                malformed: true,
                path,
            })
        }
    }
}

/// True when a `.kiro.hook` payload is shaped like a v1 document or a v1
/// hook definition rather than an IDE 0.x hook.
pub fn looks_like_v1_hook(root: &Value) -> bool {
    let Some(root) = root.as_object() else {
        return false;
    };
    let legacy = ["when", "then", "event", "runCommand", "askAgent"]
        .iter()
        .any(|key| root.contains_key(*key));
    !legacy
        && (root.get("hooks").is_some_and(Value::is_array)
            || root.get("trigger").is_some_and(Value::is_string)
            || root.get("action").is_some_and(Value::is_object))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hook(content: &str) -> (Value, Vec<FieldTypeError>) {
        let value: Value = serde_json::from_str(content).unwrap();
        let errors = KiroIdeHook::from_value(&value).type_errors;
        (value, errors)
    }

    #[test]
    fn reads_real_ide_format() {
        let value: Value = serde_json::from_str(
            r#"{"enabled":true,"name":"n","version":"1",
                "when":{"type":"fileEdited","patterns":["src/**/*.ts"]},
                "then":{"type":"askAgent","prompt":"Review"}}"#,
        )
        .unwrap();
        let hook = KiroIdeHook::from_value(&value);
        assert_eq!(hook.effective_event(), Some("fileEdited"));
        assert_eq!(hook.patterns.as_ref().unwrap().values, ["src/**/*.ts"]);
        assert_eq!(hook.effective_ask_agent(), Some("Review"));
        assert!(hook.type_errors.is_empty());
        assert_eq!(hook.enabled, Some(true));
    }

    #[test]
    fn reads_flat_compatibility_format() {
        let value: Value = serde_json::from_str(
            r#"{"event":"fileEdited","patterns":["**/*.md"],"runCommand":"echo changed"}"#,
        )
        .unwrap();
        let hook = KiroIdeHook::from_value(&value);
        assert_eq!(hook.effective_event(), Some("fileEdited"));
        assert_eq!(hook.effective_run_command(), Some("echo changed"));
        assert!(hook.has_action());
    }

    #[test]
    fn type_errors_are_located_and_mark_fields_malformed() {
        let (value, errors) =
            hook(r#"{"when":{"type":1,"patterns":"*.ts"},"then":{"command":["x"]}}"#);
        let hook = KiroIdeHook::from_value(&value);
        assert!(hook.event_malformed);
        assert!(hook.patterns.as_ref().unwrap().malformed);
        assert!(hook.action_malformed);
        assert_eq!(
            errors.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
            [
                vec![Seg::Key("when"), Seg::Key("type")],
                vec![Seg::Key("when"), Seg::Key("patterns")],
                vec![Seg::Key("then"), Seg::Key("command")],
            ]
        );
    }

    #[test]
    fn timeouts_are_seconds_with_zero_allowed() {
        let value: Value = serde_json::from_str(
            r#"{"timeout":"10","then":{"type":"runCommand","command":"x","timeout":-1}}"#,
        )
        .unwrap();
        let hook = KiroIdeHook::from_value(&value);
        assert_eq!(hook.type_errors.len(), 1);
        assert_eq!(hook.type_errors[0].path, [Seg::Key("timeout")]);
        assert_eq!(
            hook.negative_timeouts,
            [vec![Seg::Key("then"), Seg::Key("timeout")]]
        );
        for ok in [
            r#"{"then":{"timeout":0}}"#,
            r#"{"then":{"timeout":30}}"#,
            r#"{"then":{"timeout":300000}}"#,
        ] {
            let value: Value = serde_json::from_str(ok).unwrap();
            let hook = KiroIdeHook::from_value(&value);
            assert!(
                hook.type_errors.is_empty() && hook.negative_timeouts.is_empty(),
                "{ok}"
            );
        }
    }

    #[test]
    fn blank_actions_are_not_treated_as_present() {
        let value: Value = serde_json::from_str(
            r#"{"event":"promptSubmit","runCommand":"   ","then":{"askAgent":"  "}}"#,
        )
        .unwrap();
        assert!(!KiroIdeHook::from_value(&value).has_action());
    }

    #[test]
    fn v1_shapes_are_recognized() {
        for (content, expected) in [
            (r#"{"version":"v1","hooks":[]}"#, true),
            (
                r#"{"name":"a","trigger":"Stop","action":{"type":"command","command":"x"}}"#,
                true,
            ),
            (r#"{"when":{"type":"fileEdited"},"hooks":[]}"#, false),
            (r#"{"event":"promptSubmit"}"#, false),
            ("[]", false),
        ] {
            let value: Value = serde_json::from_str(content).unwrap();
            assert_eq!(looks_like_v1_hook(&value), expected, "{content}");
        }
    }
}
