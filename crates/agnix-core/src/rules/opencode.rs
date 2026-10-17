//! OpenCode configuration validation (`opencode.json`, `opencode.jsonc`).
//!
//! OpenCode 1 (`opencode-ai`) and OpenCode 2 (`@opencode/cli`, from
//! 2026-09-11) read the same files. Both accept JSONC: "OpenCode supports
//! JSON and JSONC. Use JSONC when you want comments or trailing commas."
//! OpenCode 2 reads OpenCode 1 configuration and adds native fields
//! (`agents`, `permissions`, `mcp.servers`, `policies`, ...). Each file is
//! checked against the lines it targets (see `opencode_version`): checks
//! that hold for one line only run when that is the only line targeted, so
//! documented OpenCode 1 configuration never gets an OpenCode 2 finding
//! unless the file uses OpenCode 2 fields or the version is pinned.

use crate::{
    config::PerFileLintConfig,
    diagnostics::{Diagnostic, Fix},
    rules::opencode_common::{
        AgentSource, BUILTIN_AGENTS, Member, Node, Out, V2_TOP_LEVEL_KEYS, Val,
        agent_needs_description, check_agent, check_command_entry, check_mixed_entry, check_model,
        check_permission_v1, check_permissions_v2, check_policies, config_has_v2_markers,
        is_credential_name, is_literal_secret, is_user_or_system_dir, opencode_global_dir,
        parse_config, rename_fix, shell_variable, string_fix,
    },
    rules::opencode_version::{OpenCodeLines, lines_for},
    rules::{Validator, ValidatorMetadata},
    schemas::kiro_json::closest_name,
    schemas::opencode::{
        KNOWN_TOP_LEVEL_KEYS, KNOWN_TUI_KEYS, MCP_PROTOCOLS, V1_OAUTH_KEYS, V2_OAUTH_KEYS,
        VALID_DIFF_STYLES, VALID_LOG_LEVELS, VALID_SHARE_MODES, VALID_UPDATE_MODES,
        config_property_spans, is_glob_pattern, validate_glob_pattern,
    },
};
use rust_i18n::t;
use std::path::{Path, PathBuf};

const RULE_IDS: &[&str] = &[
    "OC-001",
    "OC-002",
    "OC-003",
    "OC-004",
    "OC-006",
    "OC-007",
    "OC-008",
    "OC-009",
    "OC-CFG-001",
    "OC-CFG-002",
    "OC-CFG-004",
    "OC-CFG-005",
    "OC-CFG-006",
    "OC-CFG-007",
    "OC-CFG-008",
    "OC-CFG-009",
    "OC-CFG-010",
    "OC-CFG-011",
    "OC-CFG-012",
    "OC-CFG-013",
    "OC-CFG-014",
    "OC-CFG-015",
    "OC-CFG-017",
    "OC-CFG-018",
    "OC-CFG-019",
    "OC-CFG-020",
    "OC-CFG-021",
    "OC-CFG-022",
    "OC-AG-001",
    "OC-AG-002",
    "OC-AG-003",
    "OC-AG-004",
    "OC-AG-005",
    "OC-AG-006",
    "OC-AG-007",
    "OC-AG-008",
    "OC-AG-009",
    "OC-AG-011",
    "OC-DEP-001",
    "OC-DEP-002",
    "OC-DEP-003",
    "OC-DEP-005",
    "OC-DEP-006",
    "OC-DEP-007",
    "OC-DEP-009",
    "OC-LSP-001",
    "OC-LSP-002",
    "OC-TUI-001",
    "OC-TUI-002",
    "OC-TUI-003",
    "OC-PM-002",
    "OC-PM-003",
    "OC-PM-004",
    "OC-PM-005",
    "OC-PM-006",
    "OC-PM-007",
    "OC-POL-001",
    "OC-CMD-001",
    "OC-CMD-002",
];

/// More enabled MCP servers than this get OC-CFG-020. agnix heuristic:
/// OpenCode says each server adds context but gives no number.
const MANY_MCP_SERVERS: usize = 8;

/// OpenCode 1 provider members OpenCode 2 replaced.
const V1_PROVIDER_MEMBERS: &[&str] = &["api", "id", "npm", "whitelist", "blacklist", "options"];
/// Native OpenCode 2 provider members.
const V2_PROVIDER_MEMBERS: &[&str] = &[
    "canonical",
    "package",
    "compatibility",
    "settings",
    "headers",
    "body",
];
const V1_MODEL_MEMBERS: &[&str] = &[
    "id",
    "release_date",
    "tool_call",
    "modalities",
    "options",
    "status",
];
const V2_MODEL_MEMBERS: &[&str] = &[
    "modelID",
    "capabilities",
    "settings",
    "releaseDate",
    "disabled",
];

pub struct OpenCodeValidator;

impl Validator for OpenCodeValidator {
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
        let mut out = Out::new(path, config);
        let root = match parse_config(content) {
            Ok(root) => root,
            Err((error, pos)) => {
                out.error(
                    "OC-003",
                    pos,
                    t!("rules.oc_003.message", error = error.as_str()),
                    t!("rules.oc_003.suggestion"),
                );
                return out.diags;
            }
        };
        if root.obj().is_none() {
            out.error(
                "OC-003",
                root.pos,
                t!("rules.oc_003.root_type", found = root.type_name()),
                t!("rules.oc_003.suggestion"),
            );
            return out.diags;
        }
        let dir = path.parent().unwrap_or(Path::new("")).to_path_buf();
        let mut cx = Cx {
            lines: lines_for(config, config_has_v2_markers(&root)),
            project: !is_user_or_system_dir(&dir),
            dir,
            content,
            root: &root,
            out,
        };
        cx.top_level_keys();
        cx.share();
        cx.instructions();
        cx.agents();
        cx.permissions();
        cx.experimental();
        cx.models_and_updates();
        cx.default_agent();
        cx.providers();
        cx.mcp();
        cx.commands();
        cx.compaction_and_skills();
        cx.references();
        cx.v1_only_sections();
        cx.tui();
        if cx.out.on("OC-009") {
            substitutions(&mut cx.out, &root, 0);
        }
        cx.out.diags
    }
}

struct Cx<'a, 'o, 'c> {
    out: Out<'o, 'c>,
    content: &'a str,
    root: &'a Node,
    lines: OpenCodeLines,
    /// Directory of the config file.
    dir: PathBuf,
    /// False for the global and system config, whose settings apply to
    /// every project.
    project: bool,
}

impl Cx<'_, '_, '_> {
    fn member(&self, key: &str) -> Option<&Member> {
        self.root.get(key)
    }

    fn ignored_by_v2(&mut self, pos: (usize, usize), field: &str, instead: String) {
        if self.lines.v2_only() {
            self.out.warn(
                "OC-DEP-009",
                pos,
                t!(
                    "rules.oc_dep_009.message",
                    field = field,
                    instead = instead.as_str()
                ),
                t!("rules.oc_dep_009.suggestion"),
            );
        }
    }

    // ------------------------------------------------------------------
    // Top-level keys: OC-004, OC-DEP-*, OC-CFG-022
    // ------------------------------------------------------------------

    fn top_level_keys(&mut self) {
        let root = self.root;
        let members = root.obj().unwrap_or_default();
        for member in members {
            let key = member.key.as_str();
            if !KNOWN_TOP_LEVEL_KEYS.contains(&key) {
                let suggestion = closest_name(key, KNOWN_TOP_LEVEL_KEYS).map_or_else(
                    || t!("rules.oc_004.suggestion").to_string(),
                    |close| t!("rules.oc_004.did_you_mean", key = close).to_string(),
                );
                self.out.warn(
                    "OC-004",
                    member.pos,
                    t!("rules.oc_004.message", key = key),
                    suggestion,
                );
            } else if self.lines.v1_only() && V2_TOP_LEVEL_KEYS.contains(&key) {
                self.out.warn(
                    "OC-004",
                    member.pos,
                    t!("rules.oc_004.v2_key_on_v1", key = key),
                    t!("rules.oc_004.v2_key_on_v1_suggestion"),
                );
            }
        }

        // OC-DEP-001: mode map (entries are primary agents).
        if let Some(member) = self.member("mode") {
            let all_objects = member
                .value
                .obj()
                .is_some_and(|modes| modes.iter().all(|mode| mode.value.obj().is_some()));
            let fix = (!root.has("agent") && all_objects && unique_key(root, "mode"))
                .then(|| rename_fix(member, "agent", t!("rules.oc_dep_001.fix").to_string()))
                .flatten();
            if let Some(diagnostic) = self.out.warn(
                "OC-DEP-001",
                member.pos,
                t!("rules.oc_dep_001.message"),
                t!("rules.oc_dep_001.suggestion"),
            ) && let Some(fix) = fix
            {
                diagnostic.fixes.push(fix);
            }
        }
        // OC-DEP-002: top-level tools.
        if let Some(member) = self.member("tools") {
            self.out.warn(
                "OC-DEP-002",
                member.pos,
                t!("rules.oc_dep_002.message"),
                t!("rules.oc_dep_002.migration_suggestion"),
            );
        }
        // OC-DEP-003: autoshare (boolean) -> share (mode string).
        if let Some(member) = self.member("autoshare") {
            let fix = match (member.value.bool(), member.key_span, member.value.span) {
                (Some(enabled), Some((start, _)), Some((_, end)))
                    if !root.has("share") && unique_key(root, "autoshare") =>
                {
                    let mode = if enabled { "auto" } else { "manual" };
                    Some(Fix::replace(
                        start,
                        end,
                        format!("\"share\": \"{mode}\""),
                        t!("rules.oc_dep_003.fix", mode = mode),
                        false,
                    ))
                }
                _ => None,
            };
            if let Some(diagnostic) = self.out.warn(
                "OC-DEP-003",
                member.pos,
                t!("rules.oc_dep_003.message"),
                t!("rules.oc_dep_003.suggestion"),
            ) && let Some(fix) = fix
            {
                diagnostic.fixes.push(fix);
            }
        }
        // OC-DEP-007: reference -> references (same entry shapes).
        if let Some(member) = self.member("reference") {
            let fix = (!root.has("references") && unique_key(root, "reference"))
                .then(|| rename_fix(member, "references", t!("rules.oc_dep_007.fix").to_string()))
                .flatten();
            if let Some(diagnostic) = self.out.warn(
                "OC-DEP-007",
                member.pos,
                t!("rules.oc_dep_007.message"),
                t!("rules.oc_dep_007.suggestion"),
            ) && let Some(fix) = fix
            {
                diagnostic.fixes.push(fix);
            }
        }
        // OC-DEP-005: TUI keys belong in tui.json.
        if self.out.on("OC-DEP-005") {
            let cleanup = legacy_tui_cleanup(self.content);
            for key in ["theme", "keybinds", "tui"] {
                if let Some(member) = self.member(key) {
                    let fix = cleanup.as_ref().map(|replacement| {
                        Fix::replace(
                            0,
                            self.content.len(),
                            replacement.clone(),
                            t!("rules.oc_dep_005.cleanup_fix"),
                            false,
                        )
                    });
                    if let Some(diagnostic) = self.out.warn(
                        "OC-DEP-005",
                        member.pos,
                        t!("rules.oc_dep_005.message", key = key),
                        t!("rules.oc_dep_005.cleanup_suggestion", key = key),
                    ) && let Some(fix) = fix
                    {
                        diagnostic.fixes.push(fix);
                    }
                }
            }
        }
        // OC-DEP-009: top-level fields OpenCode 2 ignores.
        for (key, instead) in [
            ("logLevel", t!("rules.oc_dep_009.log_level")),
            ("server", t!("rules.oc_dep_009.server")),
            ("subagent_depth", t!("rules.oc_dep_009.subagent_depth")),
        ] {
            if let Some(member) = self.member(key) {
                let pos = member.pos;
                self.ignored_by_v2(pos, key, instead.to_string());
            }
        }
        // OC-CFG-022: global-only settings in a project config.
        if self.project
            && let Some(member) = self.member("update")
        {
            self.out.warn(
                "OC-CFG-022",
                member.pos,
                t!("rules.oc_cfg_022.message", key = "update"),
                t!("rules.oc_cfg_022.suggestion"),
            );
        }
    }

    // ------------------------------------------------------------------
    // OC-001 share, OC-002/006 instructions
    // ------------------------------------------------------------------

    fn share(&mut self) {
        let Some(node) = self.root.at("share") else {
            return;
        };
        match node.str() {
            Some(value) if VALID_SHARE_MODES.contains(&value) => {}
            Some(value) => {
                let fix =
                    crate::rules::find_closest_value(value, VALID_SHARE_MODES).and_then(|fixed| {
                        string_fix(
                            node,
                            fixed,
                            t!("rules.oc_001.fix", fixed = fixed).to_string(),
                        )
                    });
                if let Some(diagnostic) = self.out.error(
                    "OC-001",
                    node.pos,
                    t!("rules.oc_001.message", value = value),
                    t!("rules.oc_001.suggestion"),
                ) && let Some(fix) = fix
                {
                    diagnostic.fixes.push(fix);
                }
            }
            None if node.is_null() => {}
            None => {
                self.out.error(
                    "OC-001",
                    node.pos,
                    t!("rules.oc_001.type_error"),
                    t!("rules.oc_001.suggestion"),
                );
            }
        }
    }

    fn instructions(&mut self) {
        let Some(node) = self.root.at("instructions") else {
            return;
        };
        let Some(items) = node.arr() else {
            if !node.is_null() {
                self.out.error(
                    "OC-002",
                    node.pos,
                    t!("rules.oc_002.type_error"),
                    t!("rules.oc_002.suggestion"),
                );
            }
            return;
        };
        // Relative entries are searched from the workspace up to the project
        // root; a project config sits at that root (or in its `.opencode/`).
        let roots: Vec<PathBuf> = if self.project {
            let mut roots = vec![self.dir.clone()];
            if self.dir.file_name().is_some_and(|name| name == ".opencode")
                && let Some(parent) = self.dir.parent()
            {
                roots.push(parent.to_path_buf());
            }
            roots
        } else {
            Vec::new()
        };
        let fs = self.out.config.fs().clone();
        for item in items {
            let Some(entry) = item.str() else {
                self.out.error(
                    "OC-002",
                    item.pos,
                    t!("rules.oc_002.type_error"),
                    t!("rules.oc_002.suggestion"),
                );
                continue;
            };
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if entry.starts_with("http://") || entry.starts_with("https://") {
                self.out.info(
                    "OC-006",
                    item.pos,
                    t!("rules.oc_006.message", url = entry),
                    t!("rules.oc_006.suggestion"),
                );
                continue;
            }
            if is_glob_pattern(entry) {
                if !validate_glob_pattern(entry) {
                    self.out.error(
                        "OC-002",
                        item.pos,
                        t!("rules.oc_002.invalid_glob", path = entry),
                        t!("rules.oc_002.suggestion"),
                    );
                }
                continue;
            }
            // `~/` and absolute entries are documented and machine-specific;
            // they are not checked.
            if entry.starts_with('~') || Path::new(entry).is_absolute() || roots.is_empty() {
                continue;
            }
            if !roots.iter().any(|root| fs.exists(&root.join(entry))) {
                self.out.error(
                    "OC-002",
                    item.pos,
                    t!("rules.oc_002.not_found", path = entry),
                    t!("rules.oc_002.suggestion"),
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // Agents: OC-007, OC-AG-*, OC-DEP-009 (name)
    // ------------------------------------------------------------------

    fn agents(&mut self) {
        let root = self.root;
        for (key, source) in [
            ("agent", AgentSource::V1Map),
            ("agents", AgentSource::V2Map),
        ] {
            let Some(map) = root.get(key) else {
                continue;
            };
            let Some(entries) = map.value.obj() else {
                if !map.value.is_null() {
                    self.out.error(
                        "OC-007",
                        map.value.pos,
                        t!("rules.oc_007.type_error", field = key),
                        t!("rules.oc_007.suggestion"),
                    );
                }
                continue;
            };
            for entry in entries {
                let id = entry.key.as_str();
                if entry.value.obj().is_none() {
                    if !entry.value.is_null() {
                        self.out.warn(
                            "OC-007",
                            entry.pos,
                            t!("rules.oc_007.entry_type", name = id),
                            t!("rules.oc_007.suggestion"),
                        );
                    }
                    continue;
                }
                if agent_needs_description(id, &entry.value, source, self.lines) {
                    self.out.warn(
                        "OC-007",
                        entry.pos,
                        t!("rules.oc_007.message", name = id),
                        t!("rules.oc_007.suggestion"),
                    );
                }
                if let Some(name) = entry.value.get("name") {
                    let pos = name.pos;
                    self.ignored_by_v2(pos, "name", t!("rules.oc_dep_009.agent_name").to_string());
                }
                check_agent(&mut self.out, id, &entry.value, source, self.lines);
            }
        }
        if let Some(modes) = root.at("mode").and_then(Node::obj) {
            for entry in modes {
                if entry.value.obj().is_some() {
                    check_agent(
                        &mut self.out,
                        &entry.key,
                        &entry.value,
                        AgentSource::V1Map,
                        self.lines,
                    );
                }
            }
        }
    }

    fn permissions(&mut self) {
        if let Some(node) = self.root.at("permission") {
            check_permission_v1(&mut self.out, node);
        }
        if let Some(node) = self.root.at("permissions") {
            check_permissions_v2(&mut self.out, node);
        }
        if let Some(node) = self.root.at("policies") {
            check_policies(&mut self.out, node, "policies", self.lines);
        }
    }

    fn experimental(&mut self) {
        let Some(experimental) = self.root.at("experimental") else {
            return;
        };
        if let Some(policies) = experimental.at("policies") {
            check_policies(&mut self.out, policies, "experimental.policies", self.lines);
        }
        if let Some(depth) = experimental.at("subagent_depth")
            && !depth.is_null()
            && !depth.i64().is_some_and(|value| value >= 0)
        {
            self.out.warn(
                "OC-CFG-014",
                depth.pos,
                t!(
                    "rules.oc_cfg_014.message",
                    field = "experimental.subagent_depth"
                ),
                t!("rules.oc_cfg_014.suggestion"),
            );
        }
        for key in [
            "batch_tool",
            "openTelemetry",
            "primary_tools",
            "continue_loop_on_deny",
        ] {
            if let Some(member) = experimental.get(key) {
                let pos = member.pos;
                self.ignored_by_v2(
                    pos,
                    &format!("experimental.{key}"),
                    t!("rules.oc_dep_009.no_equivalent").to_string(),
                );
            }
        }
    }

    // ------------------------------------------------------------------
    // OC-CFG-001 model, OC-CFG-002 autoupdate / update
    // ------------------------------------------------------------------

    fn models_and_updates(&mut self) {
        for key in ["model", "small_model"] {
            if let Some(node) = self.root.at(key) {
                check_model(&mut self.out, node, key);
            }
        }
        if let Some(node) = self.root.at("autoupdate") {
            let valid = node.bool().is_some() || node.str() == Some("notify") || node.is_null();
            if !valid {
                self.out.error(
                    "OC-CFG-002",
                    node.pos,
                    t!("rules.oc_cfg_002.message"),
                    t!("rules.oc_cfg_002.suggestion"),
                );
            }
        }
        if let Some(node) = self.root.at("update")
            && !node
                .str()
                .is_some_and(|value| VALID_UPDATE_MODES.contains(&value))
            && !node.is_null()
        {
            self.out.error(
                "OC-CFG-002",
                node.pos,
                t!("rules.oc_cfg_002.update_message"),
                t!("rules.oc_cfg_002.update_suggestion"),
            );
        }
    }

    // ------------------------------------------------------------------
    // OC-CFG-004 default_agent
    // ------------------------------------------------------------------

    fn default_agent(&mut self) {
        let Some(node) = self.root.at("default_agent") else {
            return;
        };
        let Some(id) = node.str() else {
            if !node.is_null() {
                self.out.warn(
                    "OC-CFG-004",
                    node.pos,
                    t!("rules.oc_cfg_004.type_error"),
                    t!("rules.oc_cfg_004.suggestion"),
                );
            }
            return;
        };
        if BUILTIN_AGENTS.contains(&id) || id.contains("{env:") {
            return;
        }
        let in_config = ["agent", "agents", "mode"]
            .iter()
            .any(|key| self.root.at(key).is_some_and(|map| map.has(id)));
        if in_config || self.markdown_agent_exists(id) {
            return;
        }
        self.out.warn(
            "OC-CFG-004",
            node.pos,
            t!("rules.oc_cfg_004.message", value = id),
            t!("rules.oc_cfg_004.suggestion"),
        );
    }

    /// Whether a Markdown agent `<id>.md` exists in an agent directory this
    /// config's OpenCode installation reads.
    fn markdown_agent_exists(&self, id: &str) -> bool {
        let mut config_dirs = Vec::new();
        if self.dir.file_name().is_some_and(|name| name == ".opencode") || !self.project {
            config_dirs.push(self.dir.clone());
        } else {
            config_dirs.push(self.dir.join(".opencode"));
        }
        if let Some(global) = opencode_global_dir() {
            config_dirs.push(global);
        }
        let fs = self.out.config.fs();
        config_dirs.iter().any(|dir| {
            ["agents", "agent", "modes", "mode"]
                .iter()
                .any(|sub| fs.exists(&dir.join(sub).join(format!("{id}.md"))))
        })
    }

    // ------------------------------------------------------------------
    // Providers: OC-CFG-005, OC-CFG-015, OC-CFG-017, OC-DEP-009
    // ------------------------------------------------------------------

    fn providers(&mut self) {
        let root = self.root;
        for (key, v2) in [("provider", false), ("providers", true)] {
            let Some(providers) = root.at(key).and_then(Node::obj) else {
                continue;
            };
            for provider in providers {
                let id = provider.key.as_str();
                let entry = &provider.value;
                check_mixed_entry(
                    &mut self.out,
                    entry,
                    &t!("rules.oc_cfg_017.provider", id = id),
                    V1_PROVIDER_MEMBERS,
                    V2_PROVIDER_MEMBERS,
                );
                let settings_key = if v2 { "settings" } else { "options" };
                if let Some(settings) = entry.at(settings_key) {
                    if let Some(api_key) = settings.at("apiKey")
                        && api_key.str().is_some_and(is_literal_api_key)
                    {
                        self.out.warn(
                            "OC-CFG-005",
                            api_key.pos,
                            t!("rules.oc_cfg_005.provider", name = id),
                            t!("rules.oc_cfg_005.suggestion"),
                        );
                    }
                    shell_variables(&mut self.out, settings, 0);
                }
                if let Some(headers) = entry.at("headers") {
                    secret_headers(
                        &mut self.out,
                        headers,
                        &format!("provider '{id}'"),
                        |_, _| false,
                    );
                    shell_variables(&mut self.out, headers, 0);
                }
                if !v2 {
                    for field in ["id", "whitelist", "blacklist"] {
                        if let Some(member) = entry.get(field) {
                            let pos = member.pos;
                            self.ignored_by_v2(
                                pos,
                                &format!("provider.{id}.{field}"),
                                t!("rules.oc_dep_009.no_equivalent").to_string(),
                            );
                        }
                    }
                }
                if let Some(models) = entry.at("models").and_then(Node::obj) {
                    for model in models {
                        check_mixed_entry(
                            &mut self.out,
                            &model.value,
                            &t!("rules.oc_cfg_017.model", id = model.key.as_str()),
                            V1_MODEL_MEMBERS,
                            V2_MODEL_MEMBERS,
                        );
                        if !v2 {
                            self.ignored_model_fields(id, model);
                        }
                    }
                }
            }
        }
    }

    fn ignored_model_fields(&mut self, provider: &str, model: &Member) {
        for field in [
            "release_date",
            "attachment",
            "reasoning",
            "temperature",
            "experimental",
        ] {
            if let Some(member) = model.value.get(field) {
                let pos = member.pos;
                self.ignored_by_v2(
                    pos,
                    &format!("provider.{provider}.models.{}.{field}", model.key),
                    t!("rules.oc_dep_009.no_equivalent").to_string(),
                );
            }
        }
        if let Some(member) = model.value.get("status")
            && member.value.str() != Some("deprecated")
        {
            let pos = member.pos;
            self.ignored_by_v2(
                pos,
                &format!("provider.{provider}.models.{}.status", model.key),
                t!("rules.oc_dep_009.no_equivalent").to_string(),
            );
        }
        if let Some(member) = model.value.get("interleaved")
            && member.value.bool().is_some()
        {
            let pos = member.pos;
            self.ignored_by_v2(
                pos,
                &format!("provider.{provider}.models.{}.interleaved", model.key),
                t!("rules.oc_dep_009.no_equivalent").to_string(),
            );
        }
    }

    // ------------------------------------------------------------------
    // MCP: OC-CFG-005/006/007/011/012/015/020/021, OC-DEP-009
    // ------------------------------------------------------------------

    fn mcp(&mut self) {
        let Some(mcp) = self.root.get("mcp") else {
            return;
        };
        let Some(members) = mcp.value.obj() else {
            if !mcp.value.is_null() {
                self.out.error(
                    "OC-CFG-006",
                    mcp.value.pos,
                    t!("rules.oc_cfg_006.type_error"),
                    t!("rules.oc_cfg_006.suggestion"),
                );
            }
            return;
        };
        let mut enabled = 0usize;
        for member in members {
            let container = !self.lines.v1_only()
                && member.value.obj().is_some()
                && !member.value.has("type")
                && !member.value.has("enabled");
            match member.key.as_str() {
                "servers" if container => {
                    for server in member.value.obj().unwrap_or_default() {
                        if self.server(server, true) {
                            enabled += 1;
                        }
                    }
                }
                "timeout" if container => self.timeouts(&member.value, "mcp.timeout"),
                _ => {
                    if self.server(member, false) {
                        enabled += 1;
                    }
                }
            }
        }
        if enabled > MANY_MCP_SERVERS {
            self.out.info(
                "OC-CFG-020",
                mcp.pos,
                t!(
                    "rules.oc_cfg_020.message",
                    count = enabled,
                    limit = MANY_MCP_SERVERS
                ),
                t!("rules.oc_cfg_020.suggestion"),
            );
        }
    }

    /// Check one server entry; returns whether it connects at startup.
    fn server(&mut self, member: &Member, v2: bool) -> bool {
        let name = member.key.as_str();
        let server = &member.value;
        let Some(fields) = server.obj() else {
            self.out.error(
                "OC-CFG-006",
                member.pos,
                t!("rules.oc_cfg_006.message", server = name),
                t!("rules.oc_cfg_006.suggestion_type"),
            );
            return false;
        };
        // An OpenCode 1 entry with only `enabled` toggles a server defined
        // by remote organization config.
        if !v2 && fields.iter().all(|field| field.key == "enabled") && !fields.is_empty() {
            self.ignored_by_v2(
                member.pos,
                &format!("mcp.{name}"),
                t!("rules.oc_dep_009.enabled_only").to_string(),
            );
            return false;
        }
        let kind = server.at("type").and_then(Node::str);
        match kind {
            Some("local" | "remote") => {}
            Some(other) => {
                self.out.error(
                    "OC-CFG-006",
                    server.at("type").map_or(member.pos, |node| node.pos),
                    t!(
                        "rules.oc_cfg_006.invalid_type",
                        server = name,
                        value = other
                    ),
                    t!("rules.oc_cfg_006.suggestion"),
                );
            }
            None => {
                self.out.error(
                    "OC-CFG-006",
                    member.pos,
                    t!("rules.oc_cfg_006.missing_type", server = name),
                    t!("rules.oc_cfg_006.suggestion"),
                );
            }
        }
        if kind == Some("local") {
            match server.at("command") {
                None => {
                    self.out.error(
                        "OC-CFG-007",
                        member.pos,
                        t!("rules.oc_cfg_007.local_missing", server = name),
                        t!("rules.oc_cfg_007.suggestion_local"),
                    );
                }
                Some(command) => {
                    let valid = command.arr().is_some_and(|items| {
                        !items.is_empty()
                            && items
                                .iter()
                                .all(|item| item.str().is_some_and(|s| !s.trim().is_empty()))
                    });
                    if !valid {
                        self.out.error(
                            "OC-CFG-007",
                            command.pos,
                            t!("rules.oc_cfg_007.command_type", server = name),
                            t!("rules.oc_cfg_007.suggestion_local"),
                        );
                    }
                }
            }
            if let Some(cwd) = server.at("cwd")
                && !cwd.str().is_some_and(|value| !value.trim().is_empty())
            {
                self.out.error(
                    "OC-CFG-007",
                    cwd.pos,
                    t!("rules.oc_cfg_007.cwd_type", server = name),
                    t!("rules.oc_cfg_007.suggestion_cwd"),
                );
            }
            if let Some(environment) = server.at("environment") {
                let valid = environment
                    .obj()
                    .is_some_and(|vars| vars.iter().all(|var| var.value.str().is_some()));
                if !valid {
                    self.out.error(
                        "OC-CFG-007",
                        environment.pos,
                        t!("rules.oc_cfg_007.environment_type"),
                        t!("rules.oc_cfg_007.suggestion_environment"),
                    );
                }
                // MCP-018 owns literal secrets in MCP server environments; the
                // ones it reports are not reported again here.
                let mcp_018 = self.out.config.is_rule_enabled("MCP-018");
                for var in environment.obj().unwrap_or_default() {
                    if is_credential_name(&var.key)
                        && var.value.str().is_some_and(is_literal_secret)
                        && !(mcp_018
                            && var.value.str().is_some_and(|value| {
                                crate::rules::mcp_servers::is_literal_env_secret(&var.key, value)
                            }))
                    {
                        self.out.warn(
                            "OC-CFG-005",
                            var.value.pos,
                            t!(
                                "rules.oc_cfg_005.mcp_environment",
                                server = name,
                                name = var.key.as_str()
                            ),
                            t!("rules.oc_cfg_005.suggestion"),
                        );
                    }
                }
                shell_variables(&mut self.out, environment, 0);
            }
        } else if kind == Some("remote") {
            match server.at("url") {
                None => {
                    self.out.error(
                        "OC-CFG-007",
                        member.pos,
                        t!("rules.oc_cfg_007.remote_missing", server = name),
                        t!("rules.oc_cfg_007.suggestion_remote"),
                    );
                }
                Some(url) => {
                    let valid = url.str().is_some_and(|value| {
                        let value = value.trim();
                        value.starts_with("http://")
                            || value.starts_with("https://")
                            || value.starts_with("{env:")
                    });
                    if !valid {
                        self.out.error(
                            "OC-CFG-007",
                            url.pos,
                            t!("rules.oc_cfg_007.url_type", server = name),
                            t!("rules.oc_cfg_007.suggestion_remote"),
                        );
                    }
                    shell_variables(&mut self.out, url, 0);
                }
            }
            if let Some(headers) = server.at("headers") {
                // MCP-028 owns literal credentials in MCP server headers.
                let mcp_028 = self.out.config.is_rule_enabled("MCP-028");
                secret_headers(
                    &mut self.out,
                    headers,
                    &format!("MCP server '{name}'"),
                    |header, value| {
                        mcp_028
                            && crate::rules::mcp_servers::is_literal_header_secret(header, value)
                    },
                );
                shell_variables(&mut self.out, headers, 0);
            }
        }
        if kind != Some("local")
            && let Some(oauth) = server.get("oauth")
        {
            self.oauth(name, oauth, v2);
        }
        if server.has("env") {
            self.out.error(
                "OC-CFG-007",
                server.get("env").map_or(member.pos, |env| env.pos),
                t!("rules.oc_cfg_007.unsupported_env", server = name),
                t!("rules.oc_cfg_007.suggestion_environment"),
            );
        }
        if v2 {
            if let Some(protocol) = server.at("protocol")
                && !protocol
                    .str()
                    .is_some_and(|value| MCP_PROTOCOLS.contains(&value))
            {
                self.out.error(
                    "OC-CFG-007",
                    protocol.pos,
                    t!(
                        "rules.oc_cfg_007.protocol",
                        server = name,
                        values = MCP_PROTOCOLS.join(", ")
                    ),
                    t!("rules.oc_cfg_007.suggestion_protocol"),
                );
            }
            if let Some(enabled) = server.get("enabled") {
                self.out.warn(
                    "OC-CFG-021",
                    enabled.pos,
                    t!("rules.oc_cfg_021.enabled", server = name),
                    t!("rules.oc_cfg_021.enabled_suggestion"),
                );
            }
            match server.at("timeout") {
                Some(timeout) if timeout.is_num() => {
                    self.out.warn(
                        "OC-CFG-021",
                        timeout.pos,
                        t!("rules.oc_cfg_021.timeout", server = name),
                        t!("rules.oc_cfg_021.timeout_suggestion"),
                    );
                }
                Some(timeout) => {
                    self.timeouts(timeout, &format!("mcp.servers.{name}.timeout"));
                }
                None => {}
            }
        } else if let Some(timeout) = server.at("timeout") {
            if timeout.obj().is_none() {
                if !timeout.is_null() && !timeout.i64().is_some_and(|value| value > 0) {
                    self.out.error(
                        "OC-CFG-011",
                        timeout.pos,
                        t!("rules.oc_cfg_011.message", server = name),
                        t!("rules.oc_cfg_011.suggestion"),
                    );
                }
            } else if self.lines.v1_only() {
                self.out.error(
                    "OC-CFG-011",
                    timeout.pos,
                    t!("rules.oc_cfg_011.message", server = name),
                    t!("rules.oc_cfg_011.suggestion"),
                );
            }
        }
        let disabled = if v2 {
            server.at("disabled").and_then(Node::bool) == Some(true)
        } else {
            server.at("enabled").and_then(Node::bool) == Some(false)
        };
        !disabled
    }

    /// OpenCode 2 timeout objects (`mcp.timeout`, a server's `timeout`).
    fn timeouts(&mut self, node: &Node, field: &str) {
        let Some(members) = node.obj() else {
            if !node.is_null() {
                self.out.error(
                    "OC-CFG-011",
                    node.pos,
                    t!("rules.oc_cfg_011.v2_type", field = field),
                    t!("rules.oc_cfg_011.v2_suggestion"),
                );
            }
            return;
        };
        for member in members {
            if !["startup", "catalog", "execution"].contains(&member.key.as_str()) {
                self.out.error(
                    "OC-CFG-011",
                    member.pos,
                    t!(
                        "rules.oc_cfg_011.v2_unknown",
                        field = field,
                        key = member.key.as_str()
                    ),
                    t!("rules.oc_cfg_011.v2_suggestion"),
                );
            } else if !member.value.i64().is_some_and(|value| value > 0) {
                self.out.error(
                    "OC-CFG-011",
                    member.value.pos,
                    t!(
                        "rules.oc_cfg_011.v2_value",
                        field = field,
                        key = member.key.as_str()
                    ),
                    t!("rules.oc_cfg_011.v2_suggestion"),
                );
            }
        }
    }

    /// OC-CFG-012 (and OC-CFG-005, OC-CFG-015, OC-CFG-021) for `oauth`.
    fn oauth(&mut self, server: &str, member: &Member, v2: bool) {
        let oauth = &member.value;
        match &oauth.val {
            Val::Bool(false) | Val::Null => return,
            Val::Obj(_) => {}
            _ => {
                self.out.error(
                    "OC-CFG-012",
                    oauth.pos,
                    t!("rules.oc_cfg_012.type_error", server = server),
                    t!("rules.oc_cfg_012.suggestion"),
                );
                return;
            }
        }
        let allowed: Vec<&str> = if v2 {
            V2_OAUTH_KEYS.to_vec()
        } else if self.lines.v2_only() {
            V1_OAUTH_KEYS.iter().chain(V2_OAUTH_KEYS).copied().collect()
        } else {
            V1_OAUTH_KEYS.to_vec()
        };
        for field in oauth.obj().unwrap_or_default() {
            let key = field.key.as_str();
            if !allowed.contains(&key) {
                if v2 && V1_OAUTH_KEYS.contains(&key) {
                    let snake = to_snake_case(key);
                    self.out.warn(
                        "OC-CFG-021",
                        field.pos,
                        t!(
                            "rules.oc_cfg_021.oauth",
                            server = server,
                            key = key,
                            replacement = snake.as_str()
                        ),
                        t!("rules.oc_cfg_021.oauth_suggestion"),
                    );
                } else {
                    let suggestion = closest_name(key, &allowed).map_or_else(
                        || t!("rules.oc_cfg_012.suggestion").to_string(),
                        |close| t!("rules.oc_cfg_012.did_you_mean", key = close).to_string(),
                    );
                    self.out.error(
                        "OC-CFG-012",
                        field.pos,
                        t!("rules.oc_cfg_012.unknown", server = server, key = key),
                        suggestion,
                    );
                }
                continue;
            }
            if key == "callbackPort" || key == "callback_port" {
                if !field
                    .value
                    .i64()
                    .is_some_and(|port| (1..=65535).contains(&port))
                {
                    self.out.error(
                        "OC-CFG-012",
                        field.value.pos,
                        t!("rules.oc_cfg_012.port", server = server, key = key),
                        t!("rules.oc_cfg_012.suggestion"),
                    );
                }
            } else if field.value.str().is_none() {
                self.out.error(
                    "OC-CFG-012",
                    field.value.pos,
                    t!("rules.oc_cfg_012.string", server = server, key = key),
                    t!("rules.oc_cfg_012.suggestion"),
                );
            } else if (key == "clientSecret" || key == "client_secret")
                && field.value.str().is_some_and(is_literal_secret)
                // MCP-028 owns literal OAuth client secrets of MCP servers.
                && !(self.out.config.is_rule_enabled("MCP-028")
                    && field
                        .value
                        .str()
                        .is_some_and(crate::rules::mcp_servers::is_literal_oauth_secret))
            {
                self.out.warn(
                    "OC-CFG-005",
                    field.value.pos,
                    t!("rules.oc_cfg_005.oauth", server = server, key = key),
                    t!("rules.oc_cfg_005.suggestion"),
                );
            }
        }
        shell_variables(&mut self.out, oauth, 0);
    }

    // ------------------------------------------------------------------
    // Commands: OC-CMD-001/002, OC-CFG-001, OC-CFG-017
    // ------------------------------------------------------------------

    fn commands(&mut self) {
        for key in ["command", "commands"] {
            let Some(commands) = self.root.at(key).and_then(Node::obj) else {
                continue;
            };
            for command in commands {
                if command.value.obj().is_none() {
                    self.out.error(
                        "OC-CMD-002",
                        command.pos,
                        t!(
                            "rules.oc_cmd_002.entry_type",
                            command = command.key.as_str()
                        ),
                        t!("rules.oc_cmd_002.suggestion"),
                    );
                    continue;
                }
                check_command_entry(&mut self.out, &command.key, &command.value, false);
            }
        }
    }

    // ------------------------------------------------------------------
    // OC-CFG-009 compaction, OC-CFG-010 skills
    // ------------------------------------------------------------------

    fn compaction_and_skills(&mut self) {
        if let Some(compaction) = self.root.at("compaction") {
            let keep_tokens = compaction.at("keep").and_then(|keep| keep.at("tokens"));
            for (field, node) in [
                ("compaction.reserved", compaction.at("reserved")),
                ("compaction.buffer", compaction.at("buffer")),
                ("compaction.keep.tokens", keep_tokens),
            ] {
                if let Some(node) = node
                    && !node.is_null()
                    && !node.i64().is_some_and(|value| value >= 0)
                {
                    self.out.error(
                        "OC-CFG-009",
                        node.pos,
                        t!("rules.oc_cfg_009.message", field = field),
                        t!("rules.oc_cfg_009.suggestion"),
                    );
                }
            }
            for key in ["tail_turns", "prune"] {
                if let Some(member) = compaction.get(key) {
                    let pos = member.pos;
                    self.ignored_by_v2(
                        pos,
                        &format!("compaction.{key}"),
                        t!("rules.oc_dep_009.compaction").to_string(),
                    );
                }
            }
        }
        // The OpenCode 1 object form; the OpenCode 2 array mixes paths and
        // URLs and is not checked here.
        if let Some(urls) = self.root.at("skills").and_then(|skills| skills.at("urls")) {
            match urls.arr() {
                Some(items) => {
                    for item in items {
                        match item.str() {
                            Some(url)
                                if url.starts_with("http://") || url.starts_with("https://") => {}
                            Some(url) => {
                                self.out.error(
                                    "OC-CFG-010",
                                    item.pos,
                                    t!(
                                        "rules.oc_cfg_010.message",
                                        url = truncate_for_display(url, 200)
                                    ),
                                    t!("rules.oc_cfg_010.suggestion"),
                                );
                            }
                            None => {
                                self.out.error(
                                    "OC-CFG-010",
                                    item.pos,
                                    t!("rules.oc_cfg_010.item_type"),
                                    t!("rules.oc_cfg_010.suggestion"),
                                );
                            }
                        }
                    }
                }
                None if urls.is_null() => {}
                None => {
                    self.out.error(
                        "OC-CFG-010",
                        urls.pos,
                        t!("rules.oc_cfg_010.type_error"),
                        t!("rules.oc_cfg_010.suggestion"),
                    );
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // OC-CFG-018 / OC-CFG-019 references
    // ------------------------------------------------------------------

    fn references(&mut self) {
        for key in ["references", "reference"] {
            let Some(references) = self.root.at(key).and_then(Node::obj) else {
                continue;
            };
            for reference in references {
                let alias = reference.key.as_str();
                match &reference.value.val {
                    Val::Str(target) => {
                        let local = target.starts_with('.')
                            || target.starts_with('/')
                            || target.starts_with('~');
                        let git_like = target.contains('/') || target.contains(':');
                        if !local && !git_like && !target.trim().is_empty() {
                            self.out.info(
                                "OC-CFG-019",
                                reference.value.pos,
                                t!(
                                    "rules.oc_cfg_019.message",
                                    alias = alias,
                                    target = target.as_str()
                                ),
                                t!("rules.oc_cfg_019.suggestion", target = target.as_str()),
                            );
                        }
                        self.out.info(
                            "OC-CFG-018",
                            reference.pos,
                            t!("rules.oc_cfg_018.message", alias = alias),
                            t!("rules.oc_cfg_018.suggestion"),
                        );
                    }
                    Val::Obj(_) => {
                        let described = reference
                            .value
                            .at("description")
                            .and_then(Node::str)
                            .is_some_and(|text| !text.trim().is_empty());
                        if !described {
                            self.out.info(
                                "OC-CFG-018",
                                reference.pos,
                                t!("rules.oc_cfg_018.message", alias = alias),
                                t!("rules.oc_cfg_018.suggestion"),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Fields only OpenCode 1 reads: OC-CFG-008, OC-CFG-013, OC-CFG-014,
    // OC-LSP-001/002 (OpenCode 2 ignores logLevel, server and top-level
    // subagent_depth, and does not run language servers)
    // ------------------------------------------------------------------

    fn v1_only_sections(&mut self) {
        if !self.lines.v1 {
            return;
        }
        if let Some(node) = self.root.at("logLevel") {
            match node.str() {
                Some(level) if VALID_LOG_LEVELS.contains(&level) => {}
                Some(level) => {
                    let fixed = crate::rules::find_closest_value(level, VALID_LOG_LEVELS);
                    let fix = fixed.and_then(|fixed| {
                        string_fix(
                            node,
                            fixed,
                            t!("rules.oc_cfg_008.fix", value = fixed).to_string(),
                        )
                    });
                    if let Some(diagnostic) = self.out.error(
                        "OC-CFG-008",
                        node.pos,
                        t!(
                            "rules.oc_cfg_008.message",
                            value = level,
                            values = VALID_LOG_LEVELS.join(", ")
                        ),
                        t!(
                            "rules.oc_cfg_008.suggestion",
                            values = VALID_LOG_LEVELS.join(", ")
                        ),
                    ) && let Some(fix) = fix
                    {
                        diagnostic.fixes.push(fix);
                    }
                }
                None if node.is_null() => {}
                None => {
                    self.out.error(
                        "OC-CFG-008",
                        node.pos,
                        t!("rules.oc_cfg_008.type_error"),
                        t!(
                            "rules.oc_cfg_008.suggestion",
                            values = VALID_LOG_LEVELS.join(", ")
                        ),
                    );
                }
            }
        }
        if let Some(server) = self.root.at("server")
            && server.obj().is_some()
        {
            for (field, expected, valid) in [
                (
                    "port",
                    "a positive integer",
                    server
                        .at("port")
                        .map(|port| port.i64().is_some_and(|p| p > 0)),
                ),
                (
                    "hostname",
                    "a string",
                    server.at("hostname").map(|value| value.str().is_some()),
                ),
                (
                    "mdns",
                    "a boolean",
                    server.at("mdns").map(|value| value.bool().is_some()),
                ),
                (
                    "mdnsDomain",
                    "a string",
                    server.at("mdnsDomain").map(|value| value.str().is_some()),
                ),
                (
                    "cors",
                    "an array of origins",
                    server.at("cors").map(|value| value.arr().is_some()),
                ),
            ] {
                if valid == Some(false) {
                    let node = server.at(field).expect("field present");
                    if !node.is_null() {
                        self.out.warn(
                            "OC-CFG-013",
                            node.pos,
                            t!(
                                "rules.oc_cfg_013.message",
                                field = field,
                                expected = expected
                            ),
                            t!("rules.oc_cfg_013.suggestion"),
                        );
                    }
                }
            }
        }
        if let Some(depth) = self.root.at("subagent_depth")
            && !depth.is_null()
            && !depth.i64().is_some_and(|value| value >= 0)
        {
            self.out.warn(
                "OC-CFG-014",
                depth.pos,
                t!("rules.oc_cfg_014.message", field = "subagent_depth"),
                t!("rules.oc_cfg_014.suggestion"),
            );
        }
        if let Some(servers) = self.root.at("lsp").and_then(Node::obj) {
            for server in servers {
                let Some(_) = server.value.obj() else {
                    continue;
                };
                if server.value.has("command") && !server.value.has("extensions") {
                    self.out.warn(
                        "OC-LSP-001",
                        server.pos,
                        t!("rules.oc_lsp_001.message", server = server.key.as_str()),
                        t!("rules.oc_lsp_001.suggestion"),
                    );
                }
                if let Some(extensions) = server.value.at("extensions") {
                    let valid = extensions.arr().is_some_and(|items| {
                        !items.is_empty() && items.iter().all(|item| item.str().is_some())
                    });
                    if !valid {
                        self.out.error(
                            "OC-LSP-002",
                            extensions.pos,
                            t!("rules.oc_lsp_002.message", server = server.key.as_str()),
                            t!("rules.oc_lsp_002.suggestion"),
                        );
                    }
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // OC-TUI-001..003 (legacy `tui` object)
    // ------------------------------------------------------------------

    fn tui(&mut self) {
        let Some(tui) = self.root.at("tui") else {
            return;
        };
        for member in tui.obj().unwrap_or_default() {
            if !KNOWN_TUI_KEYS.contains(&member.key.as_str()) {
                self.out.warn(
                    "OC-TUI-001",
                    member.pos,
                    t!("rules.oc_tui_001.message", key = member.key.as_str()),
                    t!(
                        "rules.oc_tui_001.suggestion",
                        keys = KNOWN_TUI_KEYS.join(", ")
                    ),
                );
            }
        }
        if let Some(speed) = tui.at("scroll_speed")
            && !speed.is_null()
            && !speed.f64().is_some_and(|value| value >= 0.001)
        {
            self.out.error(
                "OC-TUI-002",
                speed.pos,
                t!("rules.oc_tui_002.message"),
                t!("rules.oc_tui_002.suggestion"),
            );
        }
        if let Some(style) = tui.at("diff_style") {
            match style.str() {
                Some(value) if VALID_DIFF_STYLES.contains(&value) => {}
                Some(value) => {
                    let fix = crate::rules::find_closest_value(value, VALID_DIFF_STYLES).and_then(
                        |fixed| {
                            string_fix(
                                style,
                                fixed,
                                t!("rules.oc_tui_003.fix", value = fixed).to_string(),
                            )
                        },
                    );
                    if let Some(diagnostic) = self.out.error(
                        "OC-TUI-003",
                        style.pos,
                        t!(
                            "rules.oc_tui_003.message",
                            value = value,
                            values = VALID_DIFF_STYLES.join(", ")
                        ),
                        t!(
                            "rules.oc_tui_003.suggestion",
                            values = VALID_DIFF_STYLES.join(", ")
                        ),
                    ) && let Some(fix) = fix
                    {
                        diagnostic.fixes.push(fix);
                    }
                }
                None if style.is_null() => {}
                None => {
                    self.out.error(
                        "OC-TUI-003",
                        style.pos,
                        t!("rules.oc_tui_003.type_error"),
                        t!(
                            "rules.oc_tui_003.suggestion",
                            values = VALID_DIFF_STYLES.join(", ")
                        ),
                    );
                }
            }
        }
    }
}

/// Whether `key` occurs once at the top level (duplicate keys suppress
/// fixes: the edit could target the copy OpenCode does not read).
fn unique_key(root: &Node, key: &str) -> bool {
    root.obj()
        .is_some_and(|members| members.iter().filter(|member| member.key == key).count() == 1)
}

/// `apiKey` values that are literal keys. Local providers often use a dummy
/// key (`"ollama"`), so short values and placeholders are not reported.
fn is_literal_api_key(value: &str) -> bool {
    is_literal_secret(value)
}

/// OC-CFG-005 for header maps. `reported_elsewhere` names the headers
/// another rule already reports (MCP-028 for MCP servers).
fn secret_headers(
    out: &mut Out<'_, '_>,
    headers: &Node,
    owner: &str,
    reported_elsewhere: impl Fn(&str, &str) -> bool,
) {
    for header in headers.obj().unwrap_or_default() {
        if is_credential_name(&header.key)
            && header.value.str().is_some_and(|value| {
                is_literal_secret(value) && !reported_elsewhere(&header.key, value)
            })
        {
            out.warn(
                "OC-CFG-005",
                header.value.pos,
                t!(
                    "rules.oc_cfg_005.header",
                    owner = owner,
                    header = header.key.as_str()
                ),
                t!("rules.oc_cfg_005.suggestion"),
            );
        }
    }
}

/// OC-CFG-015: `$NAME` / `${NAME}` in strings OpenCode passes literally.
fn shell_variables(out: &mut Out<'_, '_>, node: &Node, depth: usize) {
    if depth > 16 {
        return;
    }
    match &node.val {
        Val::Str(value) => {
            if let Some((written, name)) = shell_variable(value) {
                out.warn(
                    "OC-CFG-015",
                    node.pos,
                    t!(
                        "rules.oc_cfg_015.message",
                        variable = written.as_str(),
                        name = name.as_str()
                    ),
                    t!("rules.oc_cfg_015.suggestion", name = name.as_str()),
                );
            }
        }
        Val::Obj(members) => {
            for member in members {
                shell_variables(out, &member.value, depth + 1);
            }
        }
        Val::Arr(items) => {
            for item in items {
                shell_variables(out, item, depth + 1);
            }
        }
        _ => {}
    }
}

/// Maximum recursion depth for OC-009.
const MAX_SUBSTITUTION_DEPTH: usize = 64;

/// OC-009: `{word:...}` patterns other than `{env:...}` and `{file:...}`.
fn substitutions(out: &mut Out<'_, '_>, node: &Node, depth: usize) {
    if depth > MAX_SUBSTITUTION_DEPTH {
        return;
    }
    match &node.val {
        Val::Str(value) => substitution_string(out, node, value),
        Val::Obj(members) => {
            for member in members {
                substitutions(out, &member.value, depth + 1);
            }
        }
        Val::Arr(items) => {
            for item in items {
                substitutions(out, item, depth + 1);
            }
        }
        _ => {}
    }
}

fn substitution_string(out: &mut Out<'_, '_>, node: &Node, s: &str) {
    let mut start = 0;
    while let Some(open_pos) = s[start..].find('{') {
        let abs_open = start + open_pos;
        let Some(close_pos) = s[abs_open..].find('}') else {
            break;
        };
        let abs_close = abs_open + close_pos;
        let inner = &s[abs_open + 1..abs_close];
        if let Some((prefix, value_part)) = inner.split_once(':')
            && !prefix.is_empty()
            && prefix
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            let reason = if prefix != "env" && prefix != "file" {
                Some(t!("rules.oc_009.unknown_prefix", prefix = prefix).to_string())
            } else if value_part.is_empty() {
                Some(t!("rules.oc_009.empty_value", prefix = prefix).to_string())
            } else {
                None
            };
            if let Some(reason) = reason {
                let pattern = format!("{{{inner}}}");
                out.warn(
                    "OC-009",
                    node.pos,
                    t!(
                        "rules.oc_009.message",
                        pattern = pattern.as_str(),
                        reason = reason.as_str()
                    ),
                    t!("rules.oc_009.suggestion"),
                );
            }
        }
        start = abs_close + 1;
    }
}

fn to_snake_case(key: &str) -> String {
    let mut snake = String::new();
    for c in key.chars() {
        if c.is_ascii_uppercase() {
            snake.push('_');
            snake.push(c.to_ascii_lowercase());
        } else {
            snake.push(c);
        }
    }
    snake
}

/// Opt-in cleanup only: this does NOT migrate data to tui.json. Use a single
/// edit for all legacy fields so adjacent removals cannot leave a dangling comma.
/// Keep all surviving source text (including comments), rather than reserialize.
fn legacy_tui_cleanup(content: &str) -> Option<String> {
    let properties = config_property_spans(content)?;
    let legacy = |key: &str| matches!(key, "theme" | "keybinds" | "tui");
    let last_retained = properties
        .iter()
        .rposition(|property| !legacy(&property.key));
    let mut removals = Vec::new();
    for (index, property) in properties.iter().enumerate() {
        if legacy(&property.key) {
            removals.push((property.key_start, property.value_end));
        }
        if (legacy(&property.key) || Some(index) == last_retained)
            && let Some(comma) = property.comma
        {
            removals.push((comma, comma + 1));
        }
    }
    if removals.is_empty() {
        return None;
    }
    removals.sort_unstable();
    let mut fixed = content.to_string();
    for (start, end) in removals.into_iter().rev() {
        fixed.replace_range(start..end, "");
    }
    // Always check syntax after composing the cleanup, including JSONC input.
    parse_config(&fixed).ok()?;
    Some(fixed)
}

/// Truncate a string for display in diagnostic messages.
/// Appends "..." if the string exceeds `max` bytes.
fn truncate_for_display(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::diagnostics::DiagnosticLevel;

    fn validate(content: &str) -> Vec<Diagnostic> {
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, &LintConfig::default())
    }

    /// The value OpenCode reads (JSONC comments and trailing commas removed).
    fn test_value(content: &str) -> serde_json::Value {
        serde_json::from_str(&crate::schemas::kiro_json::blank_jsonc(content).0).unwrap()
    }

    fn validate_with_config(content: &str, config: &LintConfig) -> Vec<Diagnostic> {
        let validator = OpenCodeValidator;
        validator.validate(Path::new("opencode.json"), content, config)
    }

    // ===== OC-003: Parse Error =====

    #[test]
    fn test_oc_003_invalid_json() {
        let diagnostics = validate("{ invalid json }");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
        assert_eq!(oc_003[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_003_empty_content() {
        let diagnostics = validate("");
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert_eq!(oc_003.len(), 1);
    }

    #[test]
    fn test_oc_003_trailing_comma_is_valid_jsonc() {
        // OpenCode accepts trailing commas ("Use JSONC when you want comments
        // or trailing commas"); the V1 docs' own first example has them.
        let diagnostics = validate(r#"{"share": "manual",}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-003"));
        let docs_example = "{\n  \"$schema\": \"https://opencode.ai/config.json\",\n  \"model\": \"anthropic/claude-sonnet-4-5\",\n  \"autoupdate\": true,\n  \"server\": {\n    \"port\": 4096,\n  },\n}\n";
        assert!(
            validate(docs_example).is_empty(),
            "{:?}",
            validate(docs_example)
        );
    }

    #[test]
    fn test_oc_003_valid_json() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_jsonc_comments_allowed() {
        let content = r#"{
  // This is a JSONC comment
  "share": "manual"
}"#;
        let diagnostics = validate(content);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(oc_003.is_empty());
    }

    #[test]
    fn test_oc_003_blocks_further_rules() {
        // When JSON is invalid, no OC-001/OC-002 should fire
        let diagnostics = validate("{ invalid }");
        assert!(diagnostics.iter().all(|d| d.rule == "OC-003"));
    }

    // ===== OC-001: Invalid Share Mode =====

    #[test]
    fn test_oc_001_invalid_share_mode() {
        let diagnostics = validate(r#"{"share": "public"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].level, DiagnosticLevel::Error);
        assert!(oc_001[0].message.contains("public"));
    }

    #[test]
    fn test_oc_001_valid_manual() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_auto() {
        let diagnostics = validate(r#"{"share": "auto"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_valid_disabled() {
        let diagnostics = validate(r#"{"share": "disabled"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_autofix_case_insensitive() {
        // "Manual" is a case-insensitive match to "manual"
        let diagnostics = validate(r#"{"share": "Manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert!(
            oc_001[0].has_fixes(),
            "OC-001 should have auto-fix for case mismatch"
        );
        let fix = &oc_001[0].fixes[0];
        assert!(!fix.safe, "OC-001 fix should be unsafe");
        assert_eq!(fix.replacement, "manual", "Fix should suggest 'manual'");
    }

    #[test]
    fn test_oc_001_autofix_targets_top_level_share_only() {
        // Fixes now use the exact span of the top-level value, so a nested
        // "share" key with the same text no longer suppresses the fix (the
        // old text search could not tell the two apart).
        let content = r#"{"share": "Manual", "nested": {"share": "Manual"}}"#;
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        let fix = &oc_001[0].fixes[0];
        assert_eq!(fix.start_byte, 11);
        assert_eq!(&content[fix.start_byte..fix.end_byte], "Manual");
    }

    #[test]
    fn test_oc_001_no_autofix_nonsense() {
        let diagnostics = validate(r#"{"share": "public"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        // "public" has no close match - should NOT get a fix
        assert!(
            !oc_001[0].has_fixes(),
            "OC-001 should not auto-fix nonsense values"
        );
    }

    #[test]
    fn test_oc_001_autofix_targets_correct_bytes() {
        let content = r#"{"share": "Manual"}"#;
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert!(oc_001[0].has_fixes());
        let fix = &oc_001[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "Manual", "Fix should target the inner value");
    }

    #[test]
    fn test_oc_001_absent_share() {
        // No share field at all should not trigger OC-001
        let diagnostics = validate(r#"{}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_oc_001_empty_string() {
        let diagnostics = validate(r#"{"share": ""}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
    }

    #[test]
    fn test_oc_001_case_sensitive() {
        let diagnostics = validate(r#"{"share": "Manual"}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Share mode should be case-sensitive");
    }

    #[test]
    fn test_oc_001_line_number() {
        let content = "{\n  \"share\": \"invalid\"\n}";
        let diagnostics = validate(content);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1);
        assert_eq!(oc_001[0].line, 2);
    }

    // ===== OC-002: Invalid Instruction Path =====

    #[test]
    fn test_oc_002_nonexistent_path() {
        let diagnostics =
            validate(r#"{"instructions": ["nonexistent-file-that-does-not-exist.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert_eq!(oc_002[0].level, DiagnosticLevel::Error);
        assert!(oc_002[0].message.contains("nonexistent-file"));
    }

    #[test]
    fn test_oc_002_valid_glob_pattern() {
        // Valid glob patterns should pass even if no files match
        let diagnostics = validate(r#"{"instructions": ["**/*.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_invalid_glob_pattern() {
        let diagnostics = validate(r#"{"instructions": ["[unclosed"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    #[test]
    fn test_oc_002_absent_instructions() {
        // No instructions field should not trigger OC-002
        let diagnostics = validate(r#"{}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_empty_instructions_array() {
        let diagnostics = validate(r#"{"instructions": []}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    #[test]
    fn test_oc_002_multiple_invalid_paths() {
        let diagnostics = validate(r#"{"instructions": ["nonexistent1.md", "nonexistent2.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 2);
    }

    #[test]
    fn test_oc_002_mixed_valid_invalid() {
        // Glob patterns pass, nonexistent literal paths fail
        let diagnostics = validate(r#"{"instructions": ["**/*.md", "nonexistent.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
        assert!(oc_002[0].message.contains("nonexistent.md"));
    }

    #[test]
    fn test_oc_002_empty_path_skipped() {
        let diagnostics = validate(r#"{"instructions": [""]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(oc_002.is_empty());
    }

    // ===== Config Integration =====

    #[test]
    fn test_config_disabled_opencode_category() {
        let mut config = LintConfig::default();
        config.rules_mut().opencode = false;

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_rules: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule.starts_with("OC-"))
            .collect();
        assert!(oc_rules.is_empty());
    }

    #[test]
    fn test_config_disabled_specific_rule() {
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["OC-001".to_string()];

        let diagnostics = validate_with_config(r#"{"share": "invalid"}"#, &config);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert!(oc_001.is_empty());
    }

    #[test]
    fn test_all_oc_rules_can_be_disabled() {
        let rules = [
            "OC-001",
            "OC-002",
            "OC-003",
            "OC-004",
            "OC-006",
            "OC-007",
            "OC-008",
            "OC-009",
            "OC-AG-009",
            "OC-CFG-013",
            "OC-DEP-005",
            "OC-DEP-006",
            "OC-DEP-007",
        ];

        for rule in rules {
            let mut config = LintConfig::default();
            config.rules_mut().disabled_rules = vec![rule.to_string()];

            let content = match rule {
                "OC-001" => r#"{"share": "invalid"}"#,
                "OC-002" => r#"{"instructions": ["nonexistent.md"]}"#,
                "OC-003" => "{ invalid }",
                "OC-004" => r#"{"totally_unknown": true}"#,
                "OC-006" => r#"{"instructions": ["https://example.com/rules.md"]}"#,
                "OC-007" => r#"{"agent": {"test": {}}}"#,
                "OC-008" => r#"{"permission": {"read": "bogus"}}"#,
                "OC-009" => r#"{"model": "{bad:value}"}"#,
                "OC-AG-009" => r#"{"agent": {"a": {"disable": "yes"}}}"#,
                "OC-CFG-013" => r#"{"server": {"port": "8080"}}"#,
                "OC-DEP-005" => r#"{"theme": "dark"}"#,
                "OC-DEP-006" => r#"{"agent": {"a": {"maxSteps": 20}}}"#,
                "OC-DEP-007" => r#"{"reference": {"lib": "github:org/repo"}}"#,
                _ => unreachable!(),
            };

            let diagnostics = validate_with_config(content, &config);
            assert!(
                !diagnostics.iter().any(|d| d.rule == rule),
                "Rule {} should be disabled",
                rule
            );
        }
    }

    // ===== Valid Config =====

    #[test]
    fn test_valid_config_no_issues() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"]
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics.is_empty(),
            "Expected no diagnostics, got: {:?}",
            diagnostics
        );
    }

    #[test]
    fn test_empty_object_no_issues() {
        let diagnostics = validate("{}");
        assert!(diagnostics.is_empty());
    }

    // ===== Path Traversal Prevention =====

    #[test]
    fn test_oc_002_absolute_and_home_paths_are_documented() {
        // OpenCode documents "~/rules/opencode.md" and absolute entries
        // ("Used as written"); they are machine-specific, so they are not
        // checked (they were errors before).
        let diagnostics = validate(r#"{"instructions": ["/etc/passwd", "~/rules/opencode.md"]}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-002"));
    }

    #[test]
    fn test_oc_002_parent_dir_traversal_rejected() {
        let diagnostics = validate(r#"{"instructions": ["../../etc/shadow"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(oc_002.len(), 1);
    }

    // ===== Type Mismatch Handling =====

    #[test]
    fn test_type_mismatch_share_not_string() {
        // "share": true is valid JSON but wrong type; should not be OC-003
        let diagnostics = validate(r#"{"share": true}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-001 for wrong type
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Wrong type share should trigger OC-001");
        assert!(oc_001[0].message.contains("string"));
    }

    #[test]
    fn test_type_mismatch_share_number() {
        let diagnostics = validate(r#"{"share": 123}"#);
        let oc_001: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-001").collect();
        assert_eq!(oc_001.len(), 1, "Numeric share should trigger OC-001");
    }

    #[test]
    fn test_type_mismatch_instructions_not_array() {
        // "instructions": "README.md" is valid JSON but wrong type
        let diagnostics = validate(r#"{"instructions": "README.md"}"#);
        let oc_003: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-003").collect();
        assert!(
            oc_003.is_empty(),
            "Type mismatch should not be a parse error"
        );
        // Should emit OC-002 for wrong type
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert_eq!(
            oc_002.len(),
            1,
            "Non-array instructions should trigger OC-002"
        );
        assert!(oc_002[0].message.contains("array"));
    }

    #[test]
    fn test_type_mismatch_instructions_with_non_string_elements() {
        let diagnostics = validate(r#"{"instructions": [123, true]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(
            !oc_002.is_empty(),
            "Non-string array elements should trigger OC-002"
        );
    }

    // ===== OC-004: Unknown config keys =====

    #[test]
    fn test_oc_004_unknown_key() {
        let diagnostics = validate(r#"{"totally_unknown": true}"#);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 1);
        assert_eq!(oc_004[0].level, DiagnosticLevel::Warning);
        assert!(oc_004[0].message.contains("totally_unknown"));
    }

    #[test]
    fn test_oc_004_known_keys_no_warning() {
        let content = r#"{
  "share": "manual",
  "instructions": ["**/*.md"],
  "model": "claude-sonnet-4-5",
  "agent": {},
  "permission": {},
  "autoshare": "manual",
  "enterprise": {},
  "layout": "stretch",
  "logLevel": "INFO",
  "lsp": false,
  "mode": "agent",
  "skills": [],
  "snapshot": false,
  "username": "dev"
}"#;
        let diagnostics = validate(content);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert!(oc_004.is_empty(), "Known keys should not trigger OC-004");
    }

    #[test]
    fn test_oc_004_multiple_unknown_keys() {
        let content = r#"{"unknown_a": true, "unknown_b": false, "share": "manual"}"#;
        let diagnostics = validate(content);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 2);
    }

    #[test]
    fn test_oc_004_has_suggestion() {
        let diagnostics = validate(r#"{"bogus_setting": 42}"#);
        let oc_004: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-004").collect();
        assert_eq!(oc_004.len(), 1);
        assert!(
            oc_004[0].suggestion.is_some(),
            "OC-004 should have a suggestion"
        );
    }

    // ===== OC-006: Remote URL in instructions =====

    #[test]
    fn test_oc_006_https_url() {
        let diagnostics = validate(r#"{"instructions": ["https://example.com/rules.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert_eq!(oc_006.len(), 1);
        assert_eq!(oc_006[0].level, DiagnosticLevel::Info);
        assert!(oc_006[0].message.contains("https://example.com"));
    }

    #[test]
    fn test_oc_006_http_url() {
        let diagnostics = validate(r#"{"instructions": ["http://example.com/rules.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert_eq!(oc_006.len(), 1);
    }

    #[test]
    fn test_oc_006_local_path_no_warning() {
        let diagnostics = validate(r#"{"instructions": ["**/*.md"]}"#);
        let oc_006: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-006").collect();
        assert!(oc_006.is_empty());
    }

    #[test]
    fn test_oc_006_url_not_checked_as_path() {
        // URLs should trigger OC-006 but NOT OC-002 (not-found)
        let diagnostics = validate(r#"{"instructions": ["https://example.com/rules.md"]}"#);
        let oc_002: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-002").collect();
        assert!(
            oc_002.is_empty(),
            "URLs should not be checked as file paths"
        );
    }

    // ===== OC-007: Agent validation =====

    #[test]
    fn test_oc_007_missing_description() {
        let diagnostics = validate(r#"{"agent": {"my-agent": {"model": "gpt-4"}}}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1);
        assert_eq!(oc_007[0].level, DiagnosticLevel::Warning);
        assert!(oc_007[0].message.contains("my-agent"));
    }

    #[test]
    fn test_oc_007_with_description() {
        let content =
            r#"{"agent": {"my-agent": {"description": "A test agent", "model": "gpt-4"}}}"#;
        let diagnostics = validate(content);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert!(oc_007.is_empty());
    }

    #[test]
    fn test_oc_007_wrong_type() {
        let diagnostics = validate(r#"{"agent": "not an object"}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1);
        assert_eq!(oc_007[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_007_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert!(oc_007.is_empty());
    }

    #[test]
    fn test_oc_007_multiple_agents() {
        let content = r#"{"agent": {"agent-a": {}, "agent-b": {"description": "ok"}}}"#;
        let diagnostics = validate(content);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc_007.len(), 1, "Only agent-a should trigger OC-007");
        assert!(oc_007[0].message.contains("agent-a"));
    }

    #[test]
    fn test_oc_007_non_object_agent_entry() {
        // Agent entry that's a string instead of an object should trigger OC-007
        let diagnostics = validate(r#"{"agent": {"my-agent": "oops"}}"#);
        let oc_007: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(
            oc_007.len(),
            1,
            "Non-object agent entry should trigger OC-007"
        );
        assert!(oc_007[0].message.contains("my-agent"));
    }

    // ===== OC-008: Permission validation =====

    #[test]
    fn test_oc_008_valid_permissions() {
        let content = r#"{"permission": {"read": "allow", "edit": "ask", "bash": "deny"}}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_invalid_permission_value() {
        let diagnostics = validate(r#"{"permission": {"read": "yes"}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert_eq!(oc_008[0].level, DiagnosticLevel::Error);
        assert!(oc_008[0].message.contains("yes"));
        assert!(oc_008[0].message.contains("read"));
    }

    #[test]
    fn test_oc_008_has_fix() {
        // Use a case-insensitive mismatch that find_closest_value can match
        let content = r#"{"permission": "Allow"}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert!(
            oc_008[0].has_fixes(),
            "OC-008 should have auto-fix for case-mismatched permission mode"
        );
        let fix = &oc_008[0].fixes[0];
        assert!(!fix.safe, "OC-008 fix should be unsafe");
        assert_eq!(
            fix.replacement, "allow",
            "Fix should suggest 'allow' as closest match"
        );
    }

    #[test]
    fn test_oc_008_global_string_valid() {
        let diagnostics = validate(r#"{"permission": "allow"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_global_string_invalid() {
        let diagnostics = validate(r#"{"permission": "bogus"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert!(oc_008[0].message.contains("bogus"));
    }

    #[test]
    fn test_oc_008_wrong_type() {
        let diagnostics = validate(r#"{"permission": 42}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1);
        assert_eq!(oc_008[0].level, DiagnosticLevel::Error);
    }

    #[test]
    fn test_oc_008_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert!(oc_008.is_empty());
    }

    #[test]
    fn test_oc_008_nested_pattern_permissions() {
        let content = r#"{"permission": {"bash": {"*.sh": "allow", "*.py": "invalid"}}}"#;
        let diagnostics = validate(content);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(oc_008.len(), 1, "Only 'invalid' should trigger OC-008");
    }

    #[test]
    fn test_oc_008_non_string_permission_value() {
        // Permission value that's a number instead of a string should trigger OC-008
        let diagnostics = validate(r#"{"permission": {"read": 123}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(
            oc_008.len(),
            1,
            "Non-string permission value should trigger OC-008"
        );
    }

    #[test]
    fn test_oc_008_non_string_nested_permission_value() {
        // Nested permission value that's not a string should trigger OC-008
        let diagnostics = validate(r#"{"permission": {"bash": {"*.sh": true}}}"#);
        let oc_008: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-008").collect();
        assert_eq!(
            oc_008.len(),
            1,
            "Non-string nested permission should trigger OC-008"
        );
    }

    // ===== OC-009: Variable substitution validation =====

    #[test]
    fn test_oc_009_valid_env_substitution() {
        let diagnostics = validate(r#"{"model": "{env:OPENAI_MODEL}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_valid_file_substitution() {
        let diagnostics = validate(r#"{"model": "{file:model.txt}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_unknown_prefix() {
        let diagnostics = validate(r#"{"model": "{bad:value}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
        assert_eq!(oc_009[0].level, DiagnosticLevel::Warning);
        assert!(oc_009[0].message.contains("bad"));
    }

    #[test]
    fn test_oc_009_empty_env_value() {
        let diagnostics = validate(r#"{"model": "{env:}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
        assert!(oc_009[0].message.contains("empty"));
    }

    #[test]
    fn test_oc_009_empty_file_value() {
        let diagnostics = validate(r#"{"model": "{file:}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
    }

    #[test]
    fn test_oc_009_no_substitution_no_warning() {
        let diagnostics = validate(r#"{"model": "gpt-4"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty());
    }

    #[test]
    fn test_oc_009_nested_value() {
        // Substitution in a nested value should be found
        let diagnostics = validate(r#"{"tui": {"prompt": "{bogus:test}"}}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(oc_009.len(), 1);
    }

    #[test]
    fn test_oc_009_multiple_substitutions_in_one_string() {
        let diagnostics = validate(r#"{"model": "{env:MODEL} and {file:path.txt} and {bad:x}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert_eq!(
            oc_009.len(),
            1,
            "Only {{bad:x}} should flag, not {{env:MODEL}} or {{file:path.txt}}"
        );
    }

    #[test]
    fn test_oc_009_colon_in_value_part() {
        // {file:C:/path/to/file} has a colon in the value part - should still be valid
        let diagnostics = validate(r#"{"model": "{file:C:/path/to/file}"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(
            oc_009.is_empty(),
            "Colons after the first should be part of the value"
        );
    }

    #[test]
    fn test_oc_009_unmatched_opening_brace() {
        // An unmatched opening brace without closing should not crash
        let diagnostics = validate(r#"{"model": "some {env:FOO text"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(oc_009.is_empty(), "Unmatched brace should be ignored");
    }

    #[test]
    fn test_oc_009_non_substitution_braces() {
        // Plain braces like JSON-in-string should not flag
        let diagnostics = validate(r#"{"model": "value with {json} content"}"#);
        let oc_009: Vec<_> = diagnostics.iter().filter(|d| d.rule == "OC-009").collect();
        assert!(
            oc_009.is_empty(),
            "{{json}} without colon should not be flagged"
        );
    }

    // ===== Fixture Integration =====

    #[test]
    fn test_valid_opencode_fixture_no_diagnostics() {
        let fixture = include_str!("../../../../tests/fixtures/opencode/opencode.json");
        let diagnostics = validate(fixture);
        assert!(
            diagnostics.is_empty(),
            "Valid opencode fixture should produce 0 diagnostics, got: {:?}",
            diagnostics
        );
    }

    // ===== find_key_line =====

    // ===== OC-CFG-001: Invalid Model Format =====
    #[test]
    fn test_oc_cfg_001_invalid_model() {
        let diagnostics = validate(r#"{"model": "gpt-4"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-001"));
    }

    #[test]
    fn test_oc_cfg_001_valid_model() {
        let diagnostics = validate(r#"{"model": "openai/gpt-4"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-001"));
    }

    #[test]
    fn test_oc_cfg_002_invalid_autoupdate() {
        let diagnostics = validate(r#"{"autoupdate": "yes"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-002"));
    }

    #[test]
    fn test_oc_cfg_002_valid_autoupdate_notify() {
        let diagnostics = validate(r#"{"autoupdate": "notify"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-002"));
    }

    #[test]
    fn test_unknown_top_level_key_reported_once() {
        // OC-CFG-003 was retired as a duplicate of OC-004.
        let diagnostics = validate(r#"{"unknown_field": true}"#);
        assert_eq!(diagnostics.iter().filter(|d| d.rule == "OC-004").count(), 1);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-003"));
    }

    // ===== OC-CFG-004: Invalid Default Agent =====
    #[test]
    fn test_oc_cfg_004_invalid_agent() {
        let diagnostics = validate(r#"{"default_agent": "foo"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    #[test]
    fn test_oc_cfg_004_valid_agent() {
        let diagnostics = validate(r#"{"default_agent": "build"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    #[test]
    fn test_oc_cfg_004_non_string_agent() {
        let diagnostics = validate(r#"{"default_agent": 123}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-004"));
    }

    // ===== OC-CFG-005: Hardcoded API Key =====
    #[test]
    fn test_oc_cfg_005_hardcoded_key() {
        let diagnostics = validate(r#"{"provider": {"test": {"options": {"apiKey": "sk-123"}}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-005"));
    }

    #[test]
    fn test_oc_cfg_005_env_key() {
        let diagnostics =
            validate(r#"{"provider": {"test": {"options": {"apiKey": "{env:TEST}"}}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-005"));
    }

    // ===== OC-CFG-006 & OC-CFG-007: MCP Server =====
    #[test]
    fn test_oc_cfg_006_invalid_mcp_type() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "foo"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-006"));
    }

    #[test]
    fn test_oc_cfg_007_missing_command() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "local"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_006_mcp_must_be_object() {
        let diagnostics = validate(r#"{"mcp": []}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-006"));
    }

    #[test]
    fn test_oc_cfg_007_local_command_type_check() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "local", "command": "node"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_local_cwd_string_ok() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "cwd": "plugins/sub"}}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_local_cwd_empty_flags() {
        let diagnostics =
            validate(r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "cwd": ""}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_local_cwd_non_string_flags() {
        let diagnostics =
            validate(r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "cwd": 42}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_local_environment_string_map_ok() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "environment": {"NODE_ENV": "test"}}}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_local_environment_requires_string_values() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "environment": {"PORT": 3000}}}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_rejects_unsupported_env_alias() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"type": "local", "command": ["node"], "env": {"NODE_ENV": "test"}}}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_rejects_remote_env_alias() {
        let diagnostics = validate(
            r#"{"mcp": {"srv": {"type": "remote", "url": "https://example.com/mcp", "env": {"TOKEN": "test"}}}}"#,
        );
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    #[test]
    fn test_oc_cfg_007_remote_url_format_check() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"type": "remote", "url": "not-a-url"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-007"));
    }

    // ===== Agent tests =====
    #[test]
    fn test_oc_ag_001_invalid_mode() {
        let diagnostics = validate(r#"{"agent": {"a": {"mode": "foo"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-001"));
    }

    #[test]
    fn test_oc_ag_002_invalid_color() {
        // OC-AG-002 now covers malformed hex values only; an unknown name is
        // OC-AG-006 (both fired for names before).
        let diagnostics = validate(r##"{"agent": {"a": {"color": "#12"}}}"##);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
        let named = validate(r#"{"agent": {"a": {"color": "foo"}}}"#);
        assert!(!named.iter().any(|d| d.rule == "OC-AG-002"));
        assert!(named.iter().any(|d| d.rule == "OC-AG-006"));
        // Native OpenCode 2 agents accept only #RRGGBB.
        let v2 = validate(r#"{"agents": {"a": {"color": "accent"}}}"#);
        assert!(v2.iter().any(|d| d.rule == "OC-AG-002"));
    }

    #[test]
    fn test_oc_ag_002_invalid_hex_color() {
        let diagnostics = validate(r##"{"agent": {"a": {"color": "#12"}}}"##);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
    }

    #[test]
    fn test_oc_ag_002_valid_named_color() {
        // VALID_NAMED_COLORS like "primary", "secondary" must not trigger OC-AG-002
        let diagnostics = validate(r#"{"agent": {"a": {"color": "primary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
        let diagnostics = validate(r#"{"agent": {"a": {"color": "secondary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-002"));
    }

    #[test]
    fn test_oc_ag_003_invalid_temp() {
        let diagnostics = validate(r#"{"agent": {"a": {"temperature": 3.0}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-003"));
    }

    #[test]
    fn test_oc_ag_003_temperature_type_check() {
        let diagnostics = validate(r#"{"agent": {"a": {"temperature": "hot"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-003"));
    }

    #[test]
    fn test_oc_ag_004_invalid_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": -1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-004"));
    }

    #[test]
    fn test_oc_ag_004_steps_type_check() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": "many"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-004"));
    }

    #[test]
    fn test_invalid_permission_action_reported_once() {
        // OC-PM-001 was retired as a duplicate of OC-008.
        let diagnostics = validate(r#"{"permission": {"read": "yes"}}"#);
        assert_eq!(diagnostics.iter().filter(|d| d.rule == "OC-008").count(), 1);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-PM-001"));
    }

    #[test]
    fn test_oc_pm_002_invalid_perm() {
        // Permission keys are wildcard patterns over tool names (custom and
        // MCP tools), so only near misses of built-in keys are reported.
        let diagnostics = validate(r#"{"permission": {"raed": "allow"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-PM-002"));
        for key in ["foo", "*", "mymcp_*", "mymcp_search"] {
            let content = format!(r#"{{"permission": {{"{key}": "deny"}}}}"#);
            assert!(
                !validate(&content).iter().any(|d| d.rule == "OC-PM-002"),
                "{key}"
            );
        }
    }

    // ===== OC-DEP-001: Deprecated mode key =====

    #[test]
    fn test_oc_dep_001_deprecated_mode() {
        let diagnostics = validate(r#"{"mode": "agent"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-001")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("mode"));
        assert!(dep[0].message.contains("agent"));
    }

    #[test]
    fn test_oc_dep_001_autofix() {
        let content = r#"{"mode": {"build": {"mode": "primary"}}}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-001")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].has_fixes(), "OC-DEP-001 should have auto-fix");
        let fix = &dep[0].fixes[0];
        // Unsafe now: renaming a key OpenCode reads changes behavior (mode
        // entries are primary agents; agent entries default to mode all).
        assert!(!fix.safe, "OC-DEP-001 fix should be unsafe");
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "mode");
        assert_eq!(fix.replacement, "agent");
    }

    #[test]
    fn test_oc_dep_001_no_fire_when_absent() {
        let diagnostics = validate(r#"{"agent": {}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-001"));
    }

    // ===== OC-DEP-002: Deprecated tools key =====

    #[test]
    fn test_oc_dep_002_deprecated_tools() {
        let diagnostics = validate(r#"{"tools": {}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-002")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("tools"));
    }

    #[test]
    fn test_oc_dep_002_no_key_only_autofix() {
        let content = r#"{"tools": {}}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-002")
            .collect();
        assert!(
            !dep[0].has_fixes(),
            "tools booleans require value migration, not key renaming"
        );
    }

    // ===== OC-DEP-003: Deprecated autoshare key =====

    #[test]
    fn test_oc_dep_003_deprecated_autoshare() {
        let diagnostics = validate(r#"{"autoshare": "manual"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-003")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("autoshare"));
    }

    #[test]
    fn test_oc_dep_003_autofix() {
        // autoshare is a boolean and share a mode string, so a key-only
        // rename produced invalid config; the fix now maps the value and is
        // unsafe.
        let content = r#"{"autoshare": true}"#;
        let diagnostics = validate(content);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-003")
            .collect();
        let fix = &dep[0].fixes[0];
        assert!(!fix.safe);
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert_eq!(fixed, r#"{"share": "auto"}"#);
        assert!(validate(&fixed).is_empty());
        let string_value = validate(r#"{"autoshare": "manual"}"#);
        assert!(
            !string_value
                .iter()
                .find(|d| d.rule == "OC-DEP-003")
                .unwrap()
                .has_fixes()
        );
    }

    // ===== OC-CFG-008: Invalid logLevel =====

    #[test]
    fn test_oc_cfg_008_invalid_loglevel() {
        let diagnostics = validate(r#"{"logLevel": "verbose"}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_valid_loglevel() {
        // The schema enum ($defs/LogLevel) is DEBUG, INFO, WARN, ERROR and
        // case-sensitive; lowercase values were accepted before.
        let diagnostics = validate(r#"{"logLevel": "INFO"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
        let lower = validate(r#"{"logLevel": "info"}"#);
        let fix = &lower.iter().find(|d| d.rule == "OC-CFG-008").unwrap().fixes[0];
        assert_eq!(fix.replacement, "INFO");
    }

    #[test]
    fn test_oc_cfg_008_case_insensitive() {
        // "INFO" normalized to "info" should be valid
        let diagnostics = validate(r#"{"logLevel": "INFO"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_type_error() {
        let diagnostics = validate(r#"{"logLevel": 42}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-008"));
    }

    #[test]
    fn test_oc_cfg_008_autofix() {
        let content = r#"{"logLevel": "debu"}"#;
        let diagnostics = validate(content);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-008")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(
            cfg[0].has_fixes(),
            "OC-CFG-008 should have auto-fix for close match"
        );
        let fix = &cfg[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "debu");
        // Uppercase per the schema enum.
        assert_eq!(fix.replacement, "DEBUG");
    }

    // ===== OC-CFG-009: Invalid compaction.reserved =====

    #[test]
    fn test_oc_cfg_009_negative_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": -1}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_valid_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": 5}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_zero_reserved() {
        let diagnostics = validate(r#"{"compaction": {"reserved": 0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    #[test]
    fn test_oc_cfg_009_type_error() {
        let diagnostics = validate(r#"{"compaction": {"reserved": "five"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-009"));
    }

    // ===== OC-CFG-010: Invalid skills.urls =====

    #[test]
    fn test_oc_cfg_010_invalid_url() {
        let diagnostics = validate(r#"{"skills": {"urls": ["not-a-url"]}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-010"));
    }

    #[test]
    fn test_oc_cfg_010_valid_url() {
        let diagnostics = validate(r#"{"skills": {"urls": ["https://example.com"]}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-010"));
    }

    // ===== OC-CFG-011: MCP timeout =====

    #[test]
    fn test_oc_cfg_011_negative_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": -5}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_valid_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": 5000}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_type_error() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": "slow"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    #[test]
    fn test_oc_cfg_011_zero_timeout() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"timeout": 0}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-011"));
    }

    // ===== OC-CFG-012: MCP OAuth =====

    #[test]
    fn test_oc_cfg_012_missing_fields() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"oauth": {"client_id": "abc"}}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    #[test]
    fn test_oc_cfg_012_valid_oauth() {
        // The documented member sets: camelCase for OpenCode 1 servers,
        // snake_case under mcp.servers. `authorization_url` (formerly
        // required) appears on no OpenCode page.
        let v1 = validate(
            r#"{"mcp": {"srv": {"type": "remote", "url": "https://mcp.example.com/mcp", "oauth": {"clientId": "{env:ID}", "clientSecret": "{env:SECRET}", "scope": "tools:read tools:execute"}}}}"#,
        );
        assert!(v1.is_empty(), "{v1:?}");
        let v2 = validate(
            r#"{"mcp": {"servers": {"srv": {"type": "remote", "url": "https://mcp.example.com/mcp", "oauth": {"client_id": "{env:ID}", "client_secret": "{env:SECRET}", "callback_port": 19876, "redirect_uri": "http://127.0.0.1:19876/callback"}}}}}"#,
        );
        assert!(v2.is_empty(), "{v2:?}");
        let unknown = validate(
            r#"{"mcp": {"srv": {"type": "remote", "url": "https://x.example", "oauth": {"authorization_url": "https://auth.example.com"}}}}"#,
        );
        assert!(unknown.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    #[test]
    fn test_oc_cfg_012_type_error() {
        let diagnostics = validate(r#"{"mcp": {"srv": {"oauth": "invalid"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-CFG-012"));
    }

    // ===== OC-AG-005: top_p out of range =====

    #[test]
    fn test_oc_ag_005_invalid_top_p() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 1.5}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_valid_top_p() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 0.9}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_boundary() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 1.0}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_type_error() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": "high"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_boundary_zero() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": 0.0}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    #[test]
    fn test_oc_ag_005_negative() {
        let diagnostics = validate(r#"{"agent": {"a": {"top_p": -0.1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-005"));
    }

    // ===== OC-AG-006: Invalid named color =====

    #[test]
    fn test_oc_ag_006_invalid_named_color() {
        let diagnostics = validate(r#"{"agent": {"a": {"color": "purple"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_valid_named_color() {
        let diagnostics = validate(r#"{"agent": {"a": {"color": "primary"}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_valid_hex_color() {
        let diagnostics = validate(r##"{"agent": {"a": {"color": "#FF5733"}}}"##);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-006"));
    }

    #[test]
    fn test_oc_ag_006_autofix() {
        let content = r#"{"agent": {"a": {"color": "erro"}}}"#;
        let diagnostics = validate(content);
        let ag: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-AG-006")
            .collect();
        assert_eq!(ag.len(), 1);
        assert!(
            ag[0].has_fixes(),
            "OC-AG-006 should have auto-fix for close match"
        );
        let fix = &ag[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "erro");
        assert_eq!(fix.replacement, "error");
    }

    // ===== OC-AG-007: Both steps and maxSteps =====

    #[test]
    fn test_oc_ag_007_redundant_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10, "maxSteps": 20}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-007"));
    }

    #[test]
    fn test_oc_ag_007_only_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-007"));
    }

    // ===== OC-AG-008: hidden must be boolean =====

    #[test]
    fn test_oc_ag_008_invalid_hidden() {
        let diagnostics = validate(r#"{"agent": {"a": {"hidden": "yes"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-008"));
    }

    #[test]
    fn test_oc_ag_008_valid_hidden() {
        let diagnostics = validate(r#"{"agent": {"a": {"hidden": true}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-008"));
    }

    // ===== OC-LSP-001: command without extensions =====

    #[test]
    fn test_oc_lsp_001_missing_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"command": "typescript-language-server"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-001"));
    }

    #[test]
    fn test_oc_lsp_001_with_extensions() {
        let diagnostics = validate(
            r#"{"lsp": {"ts": {"command": "typescript-language-server", "extensions": [".ts"]}}}"#,
        );
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-LSP-001"));
    }

    // ===== OC-LSP-002: invalid extensions =====

    #[test]
    fn test_oc_lsp_002_empty_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": []}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_non_string_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": [1, 2]}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_not_array() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": "ts"}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    #[test]
    fn test_oc_lsp_002_null_extensions() {
        let diagnostics = validate(r#"{"lsp": {"ts": {"extensions": null}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-LSP-002"));
    }

    // ===== OC-TUI-001: Unknown TUI keys =====

    #[test]
    fn test_oc_tui_001_unknown_key() {
        let diagnostics = validate(r#"{"tui": {"unknown_opt": true}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-001"));
    }

    #[test]
    fn test_oc_tui_001_known_keys() {
        let diagnostics = validate(r#"{"tui": {"theme": "dark", "scroll_speed": 1.0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-001"));
    }

    // OpenCode v1.3.16 added `mouse: bool` to disable mouse capture.
    // Without this entry in KNOWN_TUI_KEYS, OC-TUI-001 false-positives on
    // valid v1.3.16+ configs.
    #[test]
    fn test_oc_tui_001_mouse_false_not_flagged() {
        let diagnostics = validate(r#"{"tui": {"mouse": false}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-TUI-001"),
            "tui.mouse is a valid key since OpenCode v1.3.16"
        );
    }

    #[test]
    fn test_oc_tui_001_mouse_true_not_flagged() {
        let diagnostics = validate(r#"{"tui": {"mouse": true}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-001"));
    }

    // ===== OC-TUI-002: Invalid scroll_speed =====

    #[test]
    fn test_oc_tui_002_too_small() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 0.0001}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_valid() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 1.0}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_type_error() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": "fast"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    #[test]
    fn test_oc_tui_002_boundary() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 0.001}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-002"));
    }

    // ===== OC-TUI-003: Invalid diff_style =====

    #[test]
    fn test_oc_tui_003_invalid() {
        let diagnostics = validate(r#"{"tui": {"diff_style": "unified"}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-TUI-003"));
    }

    #[test]
    fn test_oc_tui_003_valid() {
        let diagnostics = validate(r#"{"tui": {"diff_style": "auto"}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-TUI-003"));
    }

    #[test]
    fn test_oc_tui_003_autofix() {
        let content = r#"{"tui": {"diff_style": "stack"}}"#;
        let diagnostics = validate(content);
        let tui: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-TUI-003")
            .collect();
        assert_eq!(tui.len(), 1);
        assert!(
            tui[0].has_fixes(),
            "OC-TUI-003 should have auto-fix for close match"
        );
        let fix = &tui[0].fixes[0];
        let target = &content[fix.start_byte..fix.end_byte];
        assert_eq!(target, "stack");
        assert_eq!(fix.replacement, "stacked");
    }

    // ===== find_json_key_span =====

    // ===== Fix #3: OC-CFG-010 http:// URL should NOT fire =====

    #[test]
    fn test_oc_cfg_010_http_url_valid() {
        let diagnostics = validate(r#"{"skills": {"urls": ["http://example.com"]}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-CFG-010"),
            "http:// URLs should be valid for skills.urls"
        );
    }

    // ===== Fix #4: OC-CFG-012 empty oauth object =====

    #[test]
    fn test_oc_cfg_012_empty_oauth() {
        let diagnostics =
            validate(r#"{"mcp": {"srv": {"type": "remote", "url": "http://x", "oauth": {}}}}"#);
        let cfg_012: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-012")
            .collect();
        // `oauth: {}` is a documented example (dynamic client registration);
        // no OAuth field is required.
        assert!(cfg_012.is_empty(), "{cfg_012:?}");
    }

    // ===== Fix #5: OC-AG-007 only maxSteps (no steps) =====

    #[test]
    fn test_oc_ag_007_only_max_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"maxSteps": 20}}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-AG-007"),
            "Only maxSteps without steps should NOT fire OC-AG-007"
        );
    }

    // ===== Fix #6: DEP-002/DEP-003 negative tests =====

    #[test]
    fn test_oc_dep_002_no_fire_when_absent() {
        let diagnostics = validate(r#"{"permission": "allow"}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-002"),
            "OC-DEP-002 should not fire when 'tools' key is absent"
        );
    }

    #[test]
    fn test_oc_dep_003_no_fire_when_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-003"),
            "OC-DEP-003 should not fire when 'autoshare' key is absent"
        );
    }

    // ===== Fix #7: OC-TUI-003 type error =====

    #[test]
    fn test_oc_tui_003_type_error() {
        let diagnostics = validate(r#"{"tui": {"diff_style": 42}}"#);
        assert!(
            diagnostics.iter().any(|d| d.rule == "OC-TUI-003"),
            "Non-string diff_style should fire OC-TUI-003"
        );
    }

    // ===== Fix #8: find_json_key_span nested key =====

    // ===== Fix #11: truncate_for_display =====

    #[test]
    fn test_truncate_for_display_short() {
        assert_eq!(truncate_for_display("hello", 200), "hello");
    }

    #[test]
    fn test_truncate_for_display_long() {
        let long = "x".repeat(300);
        let result = truncate_for_display(&long, 200);
        assert_eq!(result.len(), 203); // 200 + "..."
        assert!(result.ends_with("..."));
    }

    // ===== OC-CFG-013: Invalid server config =====

    #[test]
    fn test_oc_cfg_013_invalid_port_type() {
        let diagnostics = validate(r#"{"server": {"port": "8080"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert_eq!(cfg[0].level, DiagnosticLevel::Warning);
        assert!(cfg[0].message.contains("port"));
    }

    #[test]
    fn test_oc_cfg_013_valid_port() {
        let diagnostics = validate(r#"{"server": {"port": 8080}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_hostname_type() {
        let diagnostics = validate(r#"{"server": {"hostname": 123}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("hostname"));
    }

    #[test]
    fn test_oc_cfg_013_valid_hostname() {
        let diagnostics = validate(r#"{"server": {"hostname": "localhost"}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_mdns_type() {
        let diagnostics = validate(r#"{"server": {"mdns": "yes"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("mdns"));
    }

    #[test]
    fn test_oc_cfg_013_valid_mdns() {
        let diagnostics = validate(r#"{"server": {"mdns": true}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_invalid_cors_type() {
        let diagnostics = validate(r#"{"server": {"cors": "all"}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert!(cfg[0].message.contains("cors"));
    }

    #[test]
    fn test_oc_cfg_013_valid_cors() {
        let diagnostics = validate(r#"{"server": {"cors": ["http://localhost:3000"]}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-013"));
    }

    #[test]
    fn test_oc_cfg_013_multiple_invalid_fields() {
        let diagnostics = validate(r#"{"server": {"port": "bad", "mdns": 1}}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-013")
            .collect();
        assert_eq!(cfg.len(), 2);
    }

    // ===== OC-CFG-014: Invalid subagent_depth value =====

    #[test]
    fn test_oc_cfg_014_valid_positive() {
        // subagent_depth: 2 → clean (no OC-CFG-014)
        let diagnostics = validate(r#"{"subagent_depth": 2}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-014"));
    }

    #[test]
    fn test_oc_cfg_014_valid_zero() {
        // subagent_depth: 0 → clean
        let diagnostics = validate(r#"{"subagent_depth": 0}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-014"));
    }

    #[test]
    fn test_oc_cfg_014_negative_integer() {
        // subagent_depth: -1 → 1 warning
        let diagnostics = validate(r#"{"subagent_depth": -1}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-014")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert_eq!(cfg[0].level, DiagnosticLevel::Warning);
        // The message no longer repeats the value; the diagnostic points at it.
        assert!(cfg[0].message.contains("non-negative"));
        assert_eq!(cfg[0].column, 20);
    }

    #[test]
    fn test_oc_cfg_014_string_type() {
        // subagent_depth: "2" → 1 warning (wrong type)
        let diagnostics = validate(r#"{"subagent_depth": "2"}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-014")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert_eq!(cfg[0].level, DiagnosticLevel::Warning);
        // Message reworded to "... must be a non-negative integer".
        assert!(cfg[0].message.contains("non-negative integer"));
    }

    #[test]
    fn test_oc_cfg_014_float_type() {
        // subagent_depth: 1.5 → 1 warning (wrong type)
        let diagnostics = validate(r#"{"subagent_depth": 1.5}"#);
        let cfg: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-014")
            .collect();
        assert_eq!(cfg.len(), 1);
        assert_eq!(cfg[0].level, DiagnosticLevel::Warning);
        // Message reworded to "... must be a non-negative integer".
        assert!(cfg[0].message.contains("non-negative integer"));
    }

    #[test]
    fn test_oc_cfg_014_null_clean() {
        // subagent_depth: null → clean
        let diagnostics = validate(r#"{"subagent_depth": null}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-014"));
    }

    #[test]
    fn test_oc_cfg_014_absent_clean() {
        // subagent_depth absent → clean
        let diagnostics = validate(r#"{"share": "manual"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-014"));
    }

    #[test]
    fn test_oc_cfg_014_rule_disabled() {
        // Rule disabled → no OC-CFG-014 even with invalid value
        let mut config = LintConfig::default();
        config.rules_mut().disabled_rules = vec!["OC-CFG-014".to_string()];
        let diagnostics = validate_with_config(r#"{"subagent_depth": -1}"#, &config);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-014"));
    }

    #[test]
    fn test_oc_cfg_014_known_key_no_oc_004() {
        // subagent_depth: 2 must NOT trigger OC-004 / OC-CFG-003 (Part 1: known-key fix)
        let diagnostics = validate(r#"{"subagent_depth": 2}"#);
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "OC-004" || d.rule == "OC-CFG-003"),
            "subagent_depth should be recognized as a known key"
        );
    }

    // ===== OC-AG-009: Invalid agent disable type =====

    #[test]
    fn test_oc_ag_009_invalid_disable_type() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": "yes"}}}"#);
        let ag: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-AG-009")
            .collect();
        assert_eq!(ag.len(), 1);
        assert_eq!(ag[0].level, DiagnosticLevel::Error);
        assert!(ag[0].message.contains("disable"));
    }

    #[test]
    fn test_oc_ag_009_valid_disable_true() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": true}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    #[test]
    fn test_oc_ag_009_valid_disable_false() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": false}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    #[test]
    fn test_oc_ag_009_disable_number() {
        let diagnostics = validate(r#"{"agent": {"a": {"disable": 1}}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-AG-009"));
    }

    // ===== OC-DEP-005: Deprecated TUI keys in opencode.json =====

    #[test]
    fn test_oc_dep_005_theme_deprecated() {
        let diagnostics = validate(r#"{"theme": "dark"}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-005")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("theme"));
    }

    #[test]
    fn test_oc_dep_005_keybinds_deprecated() {
        let diagnostics = validate(r#"{"keybinds": {}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-005")
            .collect();
        assert_eq!(dep.len(), 1);
        assert!(dep[0].message.contains("keybinds"));
    }

    #[test]
    fn test_oc_dep_005_tui_deprecated() {
        let diagnostics = validate(r#"{"tui": {"scroll_speed": 1.0}}"#);
        assert!(diagnostics.iter().any(|d| d.rule == "OC-DEP-005"));
    }

    #[test]
    fn test_oc_dep_005_no_fire_when_absent() {
        let diagnostics = validate(r#"{"share": "manual"}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-005"));
    }

    // ===== OC-DEP-006: Deprecated maxSteps =====

    #[test]
    fn test_oc_dep_006_max_steps_deprecated() {
        let diagnostics = validate(r#"{"agent": {"a": {"maxSteps": 20}}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-006")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("maxSteps"));
        assert!(dep[0].message.contains("steps"));
    }

    #[test]
    fn test_oc_dep_006_no_fire_with_steps() {
        // When both steps and maxSteps are present, OC-AG-007 fires instead
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10, "maxSteps": 20}}}"#);
        assert!(
            !diagnostics.iter().any(|d| d.rule == "OC-DEP-006"),
            "OC-DEP-006 should not fire when 'steps' is also present"
        );
    }

    #[test]
    fn test_oc_dep_006_no_fire_only_steps() {
        let diagnostics = validate(r#"{"agent": {"a": {"steps": 10}}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-006"));
    }

    // ===== OC-DEP-007: Deprecated `reference` key =====

    #[test]
    fn test_oc_dep_007_reference_deprecated() {
        let diagnostics = validate(r#"{"reference": {"lib": "github:org/repo"}}"#);
        let dep: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-DEP-007")
            .collect();
        assert_eq!(dep.len(), 1);
        assert_eq!(dep[0].level, DiagnosticLevel::Warning);
        assert!(dep[0].message.contains("reference"));
        assert!(dep[0].message.contains("references"));
    }

    #[test]
    fn test_oc_dep_007_no_fire_on_references() {
        let diagnostics = validate(r#"{"references": {"lib": "github:org/repo"}}"#);
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-007"));
    }

    // ===== Schema keys added in OpenCode 1.18.x =====

    /// These keys are all present in the published https://opencode.ai/config.json
    /// schema. Before they were added to `KNOWN_TOP_LEVEL_KEYS`, each produced a
    /// spurious pair of OC-004 + OC-CFG-003 "unknown key" warnings on a valid
    /// config.
    #[test]
    fn test_current_schema_keys_not_reported_unknown() {
        for key in ["attachment", "references", "shell", "tool_output"] {
            let content = format!(r#"{{"{}": {{}}}}"#, key);
            let diagnostics = validate(&content);
            let unknown: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "OC-004" || d.rule == "OC-CFG-003")
                .collect();
            assert!(
                unknown.is_empty(),
                "key '{}' is in the published schema but was reported unknown: {:?}",
                key,
                unknown.iter().map(|d| &d.message).collect::<Vec<_>>()
            );
        }
    }

    /// `reference` is in the schema but deprecated: it must produce the
    /// OC-DEP-007 rename warning, never an "unknown key" diagnostic.
    #[test]
    fn test_deprecated_reference_key_not_reported_unknown() {
        let diagnostics = validate(r#"{"reference": {"lib": "github:org/repo"}}"#);
        assert!(
            !diagnostics
                .iter()
                .any(|d| d.rule == "OC-004" || d.rule == "OC-CFG-003"),
            "deprecated 'reference' should warn via OC-DEP-007, not as an unknown key"
        );
        assert!(diagnostics.iter().any(|d| d.rule == "OC-DEP-007"));
    }

    #[test]
    fn test_oc_dep_001_shape_and_source_guards() {
        for content in [
            r#"{"mode":"agent"}"#,
            r#"{"mode":true}"#,
            r#"{"mode":null}"#,
            r#"{"mode":[]}"#,
            r#"{"mode":{"build":false}}"#,
            r#"{"mode":{},"agent":null}"#,
            r#"{"mode":{},"mo\u0064e":{"build":{}}}"#,
        ] {
            let diagnostics = validate(content);
            let diagnostic = diagnostics.iter().find(|d| d.rule == "OC-DEP-001").unwrap();
            assert!(!diagnostic.has_fixes(), "{content}");
        }
        for content in [
            r#"{"mode":{"build":{"mode":"primary"}},"instructions":["mémo.md"]}"#,
            "{\n// \"mode\": fake\n\"mode\": {\"build\":{\"mode\":\"primary\"}}\n}",
            r#"{"mo\u0064e":{"build":{}}}"#,
        ] {
            let diagnostics = validate(content);
            let fix = &diagnostics
                .iter()
                .find(|d| d.rule == "OC-DEP-001")
                .unwrap()
                .fixes[0];
            // Unsafe now, see test_oc_dep_001_autofix.
            assert!(!fix.safe);
            let mut fixed = content.to_string();
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            let value = test_value(&fixed);
            assert!(value.get("mode").is_none());
            assert!(value["agent"]["build"].is_object());
            assert!(value.get("agents").is_none());
            assert!(!validate(&fixed).iter().any(|d| d.rule == "OC-DEP-001"));
        }
    }

    #[test]
    fn test_oc_dep_002_never_key_renames_boolean_permissions() {
        for content in [
            r#"{"tools":{"bash":true,"write":false}}"#,
            r#"{"tools":{},"permission":{"bash":"ask"}}"#,
        ] {
            let diagnostics = validate(content);
            assert!(
                !diagnostics
                    .iter()
                    .find(|d| d.rule == "OC-DEP-002")
                    .unwrap()
                    .has_fixes()
            );
        }
    }

    #[test]
    fn test_oc_dep_005_cleanup_is_unsafe_and_roundtrips() {
        for content in [
            r#"{"theme":"dark"}"#,
            r#"{"theme":"dark","keybinds":{"x":"a"},"tui":{"diff_style":"auto"}}"#,
            r#"{"theme":"dark","model":"p/m","keybinds":{"x":"a"},"tui":{}}"#,
            r#"{"model":"p/m","theme":"dark","keybinds":{},"tui":{}}"#,
            r#"{"theme":"dark","keybinds":{},"tui":{},"model":"p/m"}"#,
            "{\r\n// keep this comment\r\n\"theme\":\"dark\",\r\n\"model\":\"p/m\",\r\n\"tui\":{}\r\n}",
            r#"{"agent":{"build":{"theme":"nested","prompt":"héllo \\\"tui\\\""}},"theme":"dark"}"#,
        ] {
            let diagnostics = validate(content);
            let fixes: Vec<_> = diagnostics
                .iter()
                .filter(|d| d.rule == "OC-DEP-005")
                .flat_map(|d| &d.fixes)
                .collect();
            assert!(!fixes.is_empty(), "{content}");
            assert!(fixes.iter().all(|fix| !fix.safe));
            let mut fixed = content.to_string();
            let fix = fixes[0];
            fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
            let mut expected = test_value(content);
            for key in ["theme", "keybinds", "tui"] {
                expected.as_object_mut().unwrap().remove(key);
            }
            assert_eq!(test_value(&fixed), expected);
            assert!(!validate(&fixed).iter().any(|d| d.rule == "OC-DEP-005"));
            if content.contains("// keep this comment") {
                assert!(fixed.contains("// keep this comment"));
            }
        }
    }

    #[test]
    fn test_oc_dep_005_cleanup_requires_opt_in_in_fix_engine() {
        use crate::fs::{FileSystem, MockFileSystem};
        use std::sync::Arc;

        let content = r#"{"theme":"dark","keybinds":{},"model":"p/m","tui":{}}"#;
        let diagnostics: Vec<_> = validate(content)
            .into_iter()
            .filter(|d| d.rule == "OC-DEP-005")
            .collect();
        let path = diagnostics[0].file.clone();
        let fs = Arc::new(MockFileSystem::new());
        fs.add_file(&path, content);
        let safe =
            crate::fixes::apply_fixes_with_fs(&diagnostics, false, true, Some(fs.clone())).unwrap();
        assert!(safe.is_empty());
        assert_eq!(fs.read_to_string(&path).unwrap(), content);
        let opted_in =
            crate::fixes::apply_fixes_with_fs(&diagnostics, false, false, Some(fs.clone()))
                .unwrap();
        assert_eq!(opted_in.len(), 1);
        let fixed = fs.read_to_string(&path).unwrap();
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&fixed).unwrap(),
            serde_json::json!({"model":"p/m"})
        );
        assert!(!validate(&fixed).iter().any(|d| d.rule == "OC-DEP-005"));
    }

    #[test]
    fn test_oc_dep_005_duplicate_keys_withhold_cleanup() {
        let content = r#"{"theme":"dark","th\u0065me":"light"}"#;
        let diagnostics = validate(content);
        let diagnostic = diagnostics.iter().find(|d| d.rule == "OC-DEP-005").unwrap();
        assert!(!diagnostic.has_fixes());
        // Comments are now blanked byte for byte, so offsets stay exact with
        // non-ASCII comments and the cleanup is offered (it was withheld).
        let content = "{/* héllo */ \"theme\": \"dark\", \"model\": \"p/m\"}";
        let diagnostics = validate(content);
        let fix = &diagnostics
            .iter()
            .find(|d| d.rule == "OC-DEP-005")
            .unwrap()
            .fixes[0];
        let mut fixed = content.to_string();
        fixed.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
        assert!(fixed.contains("/* héllo */"));
        assert_eq!(test_value(&fixed), serde_json::json!({"model": "p/m"}));
    }

    // ===== Deprecated-key rename fix must not create a duplicate key =====

    /// When only the deprecated key is present the rename is safe and offered.
    #[test]
    fn test_deprecated_key_fix_offered_when_replacement_absent() {
        for (content, rule) in [
            (r#"{"mode": {"build": {}}}"#, "OC-DEP-001"),
            (r#"{"reference": {}}"#, "OC-DEP-007"),
        ] {
            let diagnostics = validate(content);
            let dep = diagnostics
                .iter()
                .find(|d| d.rule == rule)
                .unwrap_or_else(|| panic!("{} should fire for {}", rule, content));
            assert!(
                !dep.fixes.is_empty(),
                "{} should offer a rename fix when the replacement key is absent",
                rule
            );
        }
    }

    /// When both keys are present, renaming would emit two identical keys in the
    /// same object. Warn without a fix rather than produce a duplicate.
    #[test]
    fn test_deprecated_key_fix_withheld_when_replacement_present() {
        for (content, rule) in [
            (r#"{"mode": "agent", "agent": {}}"#, "OC-DEP-001"),
            (r#"{"reference": {}, "references": {}}"#, "OC-DEP-007"),
        ] {
            let diagnostics = validate(content);
            let dep = diagnostics
                .iter()
                .find(|d| d.rule == rule)
                .unwrap_or_else(|| panic!("{} should still fire for {}", rule, content));
            assert!(
                dep.fixes.is_empty(),
                "{} must not offer a fix that would duplicate a key: {}",
                rule,
                content
            );
        }
    }

    // ===== OpenCode 1 / OpenCode 2 handling =====

    fn rules_of(diagnostics: &[Diagnostic]) -> Vec<&str> {
        diagnostics.iter().map(|d| d.rule.as_str()).collect()
    }

    fn pinned(version: &str) -> LintConfig {
        LintConfig::builder()
            .tool_versions(crate::config::ToolVersions {
                opencode: Some(version.to_string()),
                ..crate::config::ToolVersions::default()
            })
            .build()
            .unwrap()
    }

    #[test]
    fn documented_v2_config_is_clean() {
        let content = r#"{
  "$schema": "https://opencode.ai/config.json",
  "model": "anthropic/claude-sonnet-4-5",
  "permissions": [
    { "action": "shell", "resource": "*", "effect": "ask" },
    { "action": "shell", "resource": "git status *", "effect": "allow" },
    { "action": "shell", "resource": "git push *", "effect": "deny" },
  ],
  "policies": [
    { "action": "provider.use", "resource": "*", "effect": "deny" },
    { "action": "provider.use", "resource": "anthropic", "effect": "allow" },
    { "action": "tool.use", "resource": "shell:git push *", "effect": "deny" },
  ],
  "agents": {
    "reviewer": {
      "description": "Review changes without editing files",
      "mode": "subagent",
      "model": "anthropic/claude-sonnet-4-5#high",
      "system": "Focus on correctness, security, and missing tests.",
      "permissions": [{ "action": "edit", "resource": "*", "effect": "deny" }],
    },
    "writer": { "mode": "primary" },
    "build": { "permissions": [{ "action": "shell", "resource": "git push *", "effect": "ask" }] },
  },
  "default_agent": "writer",
  "mcp": {
    "timeout": { "startup": 45000, "catalog": 30000, "execution": 600000 },
    "servers": {
      "context7": {
        "type": "remote",
        "url": "https://mcp.context7.com/mcp",
        "oauth": false,
        "headers": { "CONTEXT7_API_KEY": "{env:CONTEXT7_API_KEY}" },
      },
      "company-tools": {
        "type": "remote",
        "url": "https://mcp.example.com/mcp",
        "oauth": { "client_id": "{env:MCP_CLIENT_ID}", "client_secret": "{env:MCP_CLIENT_SECRET}", "scope": "tools:read", "callback_port": 19876 },
      },
      "everything": {
        "type": "local",
        "command": ["npx", "-y", "@modelcontextprotocol/server-everything"],
        "environment": { "LOG_LEVEL": "info", "MCP_API_KEY": "{env:MCP_API_KEY}" },
        "disabled": true,
        "protocol": "auto",
      },
    },
  },
  "commands": {
    "review": { "description": "Review the current changes", "template": "Review the current diff.", "subagent": true },
  },
  "providers": {
    "openai": { "settings": { "apiKey": "{file:~/.secrets/openai-key}" } },
  },
  "references": {
    "docs": { "path": "../product-docs", "description": "Product behavior and terminology" },
    "effect": { "repository": "Effect-TS/effect", "branch": "main" },
  },
  "skills": ["./team-skills", "https://example.com/.well-known/skills/"],
  "snapshots": false,
  "compaction": { "auto": true, "keep": { "tokens": 15000 }, "buffer": 20000 },
}"#;
        let diagnostics = OpenCodeValidator.validate(
            Path::new("/home/me/project/opencode.jsonc"),
            content,
            &LintConfig::default(),
        );
        // The effect reference has no description: OC-CFG-018 (info) is the
        // only finding.
        assert_eq!(rules_of(&diagnostics), ["OC-CFG-018"], "{diagnostics:?}");
    }

    #[test]
    fn documented_v1_config_gets_no_errors() {
        let content = r#"{
  "$schema": "https://opencode.ai/config.json",
  "agent": {
    "build": { "mode": "primary", "model": "anthropic/claude-sonnet-4-20250514", "prompt": "{file:./prompts/build.txt}", "permission": { "edit": "allow", "bash": "allow" } },
    "plan": { "mode": "primary", "permission": { "edit": "deny", "bash": "deny" } },
    "code-reviewer": { "description": "Reviews code", "mode": "subagent", "permission": { "edit": "deny" }, "color": "accent" },
  },
  "permission": { "*": "ask", "bash": { "*": "ask", "git *": "allow", "rm *": "deny" }, "mymcp_*": "deny" },
  "mcp": {
    "jira": { "enabled": true },
    "sentry": { "type": "remote", "url": "https://mcp.sentry.dev/mcp", "oauth": {} },
    "my-remote-mcp": { "type": "remote", "url": "https://my-mcp-server.com", "enabled": true, "headers": { "Authorization": "Bearer MY_API_KEY" } },
    "local": { "type": "local", "command": ["npx", "-y", "my-mcp-command"], "environment": { "MY_ENV_VAR": "my_env_var_value" }, "timeout": 5000 },
  },
  "provider": { "openai": { "options": { "apiKey": "{file:~/.secrets/openai-key}" } } },
  "logLevel": "INFO",
  "server": { "port": 4096, "hostname": "0.0.0.0", "mdns": true, "mdnsDomain": "myproject.local", "cors": ["http://localhost:5173"] },
  "subagent_depth": 2,
  "instructions": ["**/*.md", "https://raw.githubusercontent.com/my-org/shared-rules/main/style.md"],
}"#;
        let diagnostics = validate(content);
        assert!(
            diagnostics
                .iter()
                .all(|d| d.level != DiagnosticLevel::Error),
            "{diagnostics:?}"
        );
        // Unpinned OpenCode 1 shape: no OpenCode 2 findings.
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-DEP-009"));
        assert!(!diagnostics.iter().any(|d| d.rule == "OC-CFG-006"));
    }

    #[test]
    fn v2_markers_or_pin_select_ignored_field_checks() {
        let v1_fields =
            r#"{"logLevel": "INFO", "subagent_depth": 2, "compaction": {"prune": true}}"#;
        assert!(!rules_of(&validate(v1_fields)).contains(&"OC-DEP-009"));
        let with_v2 = r#"{"logLevel": "INFO", "permissions": [], "compaction": {"prune": true}}"#;
        let diagnostics = validate(with_v2);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "OC-DEP-009")
                .count(),
            2
        );
        let pinned_v2 = validate_with_config(v1_fields, &pinned("2.0.32"));
        assert_eq!(
            pinned_v2.iter().filter(|d| d.rule == "OC-DEP-009").count(),
            3
        );
        // A V1 pin keeps the V1 value checks and flags V2-only keys.
        let pinned_v1 =
            validate_with_config(r#"{"agents": {}, "logLevel": "nope"}"#, &pinned("1.19.3"));
        let found = rules_of(&pinned_v1);
        assert!(
            found.contains(&"OC-004") && found.contains(&"OC-CFG-008"),
            "{found:?}"
        );
        assert!(!found.contains(&"OC-DEP-009"));
    }

    #[test]
    fn mcp_servers_container_and_placeholders() {
        let diagnostics =
            validate(r#"{"mcp": {"servers": {"x": "bad"}, "jira": {"enabled": false}}}"#);
        let oc006: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-006")
            .collect();
        assert_eq!(oc006.len(), 1, "{diagnostics:?}");
        assert!(oc006[0].message.contains("'x'") && !oc006[0].message.contains("%{"));
        let v1_fields = validate(
            r#"{"mcp": {"servers": {"p": {"type": "local", "command": ["x"], "enabled": false, "timeout": 30000, "protocol": "v3"}}}}"#,
        );
        let found = rules_of(&v1_fields);
        assert_eq!(
            found.iter().filter(|r| **r == "OC-CFG-021").count(),
            2,
            "{found:?}"
        );
        assert!(found.contains(&"OC-CFG-007"));
        let oauth = validate(
            r#"{"mcp": {"servers": {"r": {"type": "remote", "url": "https://x.example/mcp", "oauth": {"clientId": "abc"}}}}}"#,
        );
        assert_eq!(rules_of(&oauth), ["OC-CFG-021"]);
        let oauth_true = validate(
            r#"{"mcp": {"r": {"type": "remote", "url": "https://x.example", "oauth": true}}}"#,
        );
        assert_eq!(rules_of(&oauth_true), ["OC-CFG-012"]);
    }

    #[test]
    fn secrets_and_shell_variables() {
        let content = r#"{
  "provider": { "a": { "options": { "apiKey": "sk-live-1234567890abcdef" } }, "local": { "options": { "apiKey": "ollama" } } },
  "mcp": {
    "r": { "type": "remote", "url": "https://x.example/${REGION}", "headers": { "Authorization": "Bearer ghp_abcdefghijklmnopqrstuvwxyz0123", "X-Key": "${MY_API_KEY}" } },
    "l": { "type": "local", "command": ["run", "$HOME"], "environment": { "API_TOKEN": "0123456789abcdef0123" } }
  }
}"#;
        let diagnostics = validate(content);
        // The provider key. The MCP header and environment secrets are
        // MCP-018 and MCP-028 findings (they used to be reported twice).
        let oc005: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-005")
            .collect();
        assert_eq!(oc005.len(), 1, "{diagnostics:?}");
        assert!(oc005[0].message.contains("Provider 'a'"), "{oc005:?}");
        // The MCP validator reports them once each; with MCP-018 and MCP-028
        // disabled OC-CFG-005 reports them again.
        let mcp = crate::rules::mcp::McpValidator.validate(
            Path::new("opencode.json"),
            content,
            &LintConfig::default(),
        );
        let mut mcp_rules: Vec<_> = mcp
            .iter()
            .map(|d| d.rule.as_str())
            .filter(|rule| matches!(*rule, "MCP-018" | "MCP-028"))
            .collect();
        mcp_rules.sort();
        assert_eq!(mcp_rules, ["MCP-018", "MCP-028"], "{mcp:?}");
        let mut no_mcp = LintConfig::default();
        no_mcp.rules_mut().disabled_rules = vec!["MCP-018".to_string(), "MCP-028".to_string()];
        assert_eq!(
            validate_with_config(content, &no_mcp)
                .iter()
                .filter(|d| d.rule == "OC-CFG-005")
                .count(),
            3
        );
        let shell: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-CFG-015")
            .collect();
        assert_eq!(shell.len(), 2, "command arrays are not checked: {shell:?}");
        assert!(shell.iter().any(|d| d.message.contains("{env:MY_API_KEY}")));
        assert!(
            diagnostics
                .iter()
                .all(|d| d.level != DiagnosticLevel::Error)
        );
    }

    #[test]
    fn agents_descriptions_and_legacy_fields() {
        // OpenCode 1 shape: description is "a required config option" for
        // custom agents (built-ins and disabled agents are skipped).
        let v1 = validate(
            r#"{"agent": {"build": {"model": "a/b"}, "helper": {"tools": {"write": false}}, "off": {"disable": true}}}"#,
        );
        let oc007: Vec<_> = v1.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc007.len(), 1, "{oc007:?}");
        assert!(oc007[0].message.contains("'helper'"));
        assert!(rules_of(&v1).contains(&"OC-DEP-002"));
        // OpenCode 2 shape: only subagents need one; a file with native V2
        // fields is checked against OpenCode 2 only.
        let v2 = validate(
            r#"{"agent": {"helper": {}}, "agents": {"writer": {"mode": "primary"}, "sub": {"mode": "subagent"}, "mixed": {"description": "d", "prompt": "p", "permissions": []}}}"#,
        );
        let oc007: Vec<_> = v2.iter().filter(|d| d.rule == "OC-007").collect();
        assert_eq!(oc007.len(), 1, "{oc007:?}");
        assert!(oc007[0].message.contains("'sub'"));
        assert!(rules_of(&v2).contains(&"OC-CFG-017"));
        let hidden = validate(r#"{"agent": {"plan": {"mode": "primary", "hidden": true}}}"#);
        assert_eq!(rules_of(&hidden), ["OC-AG-011"]);
        let hidden_v2 = validate(r#"{"agents": {"plan": {"mode": "primary", "hidden": true}}}"#);
        assert!(hidden_v2.is_empty());
    }

    #[test]
    fn permission_ordering_and_patterns() {
        let v1 = validate(
            r#"{"permission": {"bash": {"git status": "allow", "*": "ask"}, "external_directory": {"~/**": "allow"}, "edit": {"*": "allow", "src/*": "deny"}}}"#,
        );
        let found = rules_of(&v1);
        for rule in ["OC-PM-003", "OC-PM-007", "OC-PM-004"] {
            assert!(found.contains(&rule), "{rule}: {found:?}");
        }
        let v2 = validate(
            r#"{"permissions": [{"action": "bash", "resource": "*", "effect": "ask"}, {"action": "shell", "resource": "git status", "effect": "allow"}, {"action": "shell", "resource": "*", "effect": "deny"}, {"action": "external_directory", "resource": "*", "effect": "allow"}, {"action": "shell", "resource": "x", "effect": "maybe", "extra": 1}]}"#,
        );
        let found = rules_of(&v2);
        for rule in ["OC-PM-005", "OC-PM-003", "OC-PM-004", "OC-PM-007", "OC-008"] {
            assert!(found.contains(&rule), "{rule}: {found:?}");
        }
        let allow_all = validate(
            r#"{"permissions": [{"action": "shell", "resource": "*", "effect": "allow"}, {"action": "shell", "resource": "rm *", "effect": "deny"}]}"#,
        );
        assert_eq!(rules_of(&allow_all), ["OC-PM-006"]);
    }

    #[test]
    fn policy_statements() {
        let diagnostics = validate(
            r#"{"policies": [{"action": "provider.use", "resource": "openai", "effect": "ask"}, {"action": "integration.use", "resource": "github", "effect": "deny"}, {"action": "tool.use", "resource": "shell:sudo *", "effect": "deny", "note": "x"}]}"#,
        );
        let pol: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.rule == "OC-POL-001")
            .collect();
        assert_eq!(pol.len(), 3, "{pol:?}");
        assert_eq!(
            pol.iter()
                .filter(|d| d.level == DiagnosticLevel::Error)
                .count(),
            2
        );
        let v1 = validate_with_config(
            r#"{"experimental": {"policies": [{"action": "tool.use", "resource": "shell:*", "effect": "deny"}]}}"#,
            &pinned("1.19.3"),
        );
        assert_eq!(rules_of(&v1), ["OC-POL-001"]);
    }

    #[test]
    fn commands_need_templates_and_safe_shell_blocks() {
        let diagnostics = validate(
            r#"{"command": {"a": {"description": "d"}, "b": {"template": "Log: !`git log -$1`"}, "c": {"template": "Review $ARGUMENTS"}}, "commands": {"d": {"template": "x", "variant": "high", "subagent": true}}}"#,
        );
        let found = rules_of(&diagnostics);
        assert_eq!(found.iter().filter(|r| **r == "OC-CMD-002").count(), 1);
        assert_eq!(found.iter().filter(|r| **r == "OC-CMD-001").count(), 1);
        assert!(found.contains(&"OC-CFG-017"));
    }

    #[test]
    fn references_mcp_count_and_project_only_settings() {
        let refs = validate(
            r#"{"references": {"docs": "docs", "lib": "./lib", "effect": "Effect-TS/effect"}}"#,
        );
        assert_eq!(refs.iter().filter(|d| d.rule == "OC-CFG-019").count(), 1);
        assert_eq!(refs.iter().filter(|d| d.rule == "OC-CFG-018").count(), 3);
        let servers: Vec<String> = (0..9)
            .map(|i| format!(r#""s{i}": {{"type": "local", "command": ["x"]}}"#))
            .collect();
        let many = validate(&format!(r#"{{"mcp": {{{}}}}}"#, servers.join(",")));
        assert_eq!(rules_of(&many), ["OC-CFG-020"]);
        let update = validate(r#"{"update": "auto"}"#);
        assert_eq!(rules_of(&update), ["OC-CFG-022"]);
        let bad_update = validate(r#"{"update": "always"}"#);
        assert!(rules_of(&bad_update).contains(&"OC-CFG-002"));
    }

    #[test]
    fn default_agent_and_instructions_use_the_filesystem() {
        use crate::fs::MockFileSystem;
        use std::sync::Arc;
        let fs = MockFileSystem::new();
        fs.add_file("/p/.opencode/agents/writer.md", "---\nmode: primary\n---\n");
        fs.add_file("/p/CONTRIBUTING.md", "x");
        let mut config = LintConfig::default();
        config.set_fs(Arc::new(fs));
        let root = OpenCodeValidator.validate(
            Path::new("/p/opencode.json"),
            r#"{"default_agent": "writer", "instructions": ["CONTRIBUTING.md"]}"#,
            &config,
        );
        assert!(root.is_empty(), "{root:?}");
        let nested = OpenCodeValidator.validate(
            Path::new("/p/.opencode/opencode.json"),
            r#"{"default_agent": "writer", "instructions": ["CONTRIBUTING.md", "missing.md"]}"#,
            &config,
        );
        assert_eq!(rules_of(&nested), ["OC-002"], "{nested:?}");
        let unknown = OpenCodeValidator.validate(
            Path::new("/p/opencode.json"),
            r#"{"default_agent": "ghost"}"#,
            &config,
        );
        assert_eq!(rules_of(&unknown), ["OC-CFG-004"]);
    }

    #[test]
    fn model_references() {
        for ok in [
            r#"{"model": "anthropic/claude-sonnet-4-5"}"#,
            r#"{"agents": {"a": {"mode": "primary", "model": "anthropic/claude-sonnet-4-5#high"}}}"#,
            r#"{"agents": {"a": {"mode": "primary", "model": {"providerID": "anthropic", "model": "claude-sonnet-4-5", "variant": "high"}}}}"#,
        ] {
            assert!(!rules_of(&validate(ok)).contains(&"OC-CFG-001"), "{ok}");
        }
        let bad = validate(r#"{"model": "gpt-5", "small_model": 5}"#);
        assert_eq!(bad.iter().filter(|d| d.rule == "OC-CFG-001").count(), 2);
        assert!(bad.iter().any(|d| d.message.contains("'gpt-5'")));
    }
}
