//! Local resource declarations. Resolves metadata only; never loads code.
use super::{FileType, Tool, plugin_owner, small_json};
use crate::fs::FileSystem;
use serde_json::Value;
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Resource {
    PiPrompt,
    PiTheme,
    PiSkill,
    AntigravityRule,
    AntigravitySkill,
    GrokAgent,
    GrokSkill,
    GrokHooks,
    GrokMcp,
    GrokLsp,
}
impl Resource {
    pub(crate) fn file_type(self) -> FileType {
        match self {
            Self::PiPrompt => FileType::PiPrompt,
            Self::PiTheme => FileType::PiConfig,
            Self::PiSkill => FileType::Skill,
            Self::AntigravityRule => FileType::AntigravityRule,
            Self::AntigravitySkill | Self::GrokSkill => FileType::Skill,
            Self::GrokAgent => FileType::GrokAgent,
            Self::GrokHooks | Self::GrokMcp | Self::GrokLsp => FileType::GrokConfig,
        }
    }
}

fn normalized(path: &Path) -> PathBuf {
    let mut result = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir if result.file_name().is_some_and(|name| name != "..") => {
                result.pop();
            }
            _ => result.push(part.as_os_str()),
        }
    }
    result
}

fn target(base: &Path, value: &str) -> PathBuf {
    if let Some(tail) = value.strip_prefix("~/") {
        if let Some(home) = super::home_dir() {
            return normalized(&home.join(tail));
        }
    }
    normalized(&base.join(value))
}

fn exact(path: &Path, base: &Path, value: &str) -> bool {
    normalized(path) == target(base, value)
        || (path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
            && path
                .parent()
                .is_some_and(|parent| normalized(parent) == target(base, value)))
}
fn pattern(path: &Path, base: &Path, value: &str) -> bool {
    let pattern = target(base, value).to_string_lossy().replace('\\', "/");
    glob::Pattern::new(&pattern).ok().is_some_and(|pattern| {
        pattern.matches_path_with(
            &normalized(path),
            glob::MatchOptions {
                require_literal_separator: true,
                require_literal_leading_dot: true,
                case_sensitive: true,
            },
        )
    })
}

fn override_pattern(path: &Path, base: &Path, value: &str) -> bool {
    let matches_name = |path: &Path| {
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| {
                glob::Pattern::new(value)
                    .ok()
                    .is_some_and(|pattern| pattern.matches(name))
            })
    };
    pattern(path, base, value)
        || matches_name(path)
        || (path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
            && path
                .parent()
                .is_some_and(|parent| pattern(parent, base, value) || matches_name(parent)))
}

fn pi_skill_in_directory(path: &Path, root: &Path, fs: &dyn FileSystem) -> bool {
    let Some(parent) = path.parent() else {
        return false;
    };
    // The first SKILL.md terminates traversal of that skill subtree. Plain
    // Markdown files are collected only at the explicitly declared root.
    if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return parent == root && !fs.is_file(&root.join("SKILL.md"));
    }
    !parent
        .ancestors()
        .skip(1)
        .take_while(|ancestor| ancestor.starts_with(root))
        .any(|ancestor| fs.is_file(&ancestor.join("SKILL.md")))
}
fn member(path: &Path, base: &Path, value: &str, fs: &dyn FileSystem) -> bool {
    let root = target(base, value);
    let path = normalized(path);
    path == root || (fs.is_dir(&root) && path.starts_with(&root))
}

fn pi_entries(
    path: &Path,
    base: &Path,
    entries: &[Value],
    manifest: bool,
    fs: &dyn FileSystem,
    skill: bool,
) -> bool {
    let entries: Vec<&str> = entries.iter().filter_map(Value::as_str).collect();
    let glob = |s: &str| s.contains('*') || s.contains('?');
    let discovered = entries.iter().any(|entry| {
        !entry.starts_with(['!', '+', '-'])
            && if glob(entry) {
                manifest
                    && (pattern(path, base, entry)
                        || path.ancestors().skip(1).any(|parent| {
                            fs.is_dir(parent)
                                && pattern(parent, base, entry)
                                && (!skill || pi_skill_in_directory(path, parent, fs))
                        }))
            } else {
                member(path, base, entry, fs)
                    && (!skill
                        || !fs.is_dir(&target(base, entry))
                        || pi_skill_in_directory(path, &target(base, entry), fs))
            }
    });
    if !discovered {
        return false;
    }
    let includes: Vec<&&str> = entries
        .iter()
        .filter(|entry| !entry.starts_with(['!', '+', '-']) && glob(entry))
        .collect();
    let mut enabled = manifest
        || includes.is_empty()
        || includes
            .iter()
            .any(|entry| override_pattern(path, base, entry));
    if entries.iter().any(|entry| {
        entry
            .strip_prefix('!')
            .is_some_and(|value| override_pattern(path, base, value))
    }) {
        enabled = false;
    }
    if entries.iter().any(|entry| {
        entry
            .strip_prefix('+')
            .is_some_and(|value| exact(path, base, value))
    }) {
        enabled = true;
    }
    if entries.iter().any(|entry| {
        entry
            .strip_prefix('-')
            .is_some_and(|value| exact(path, base, value))
    }) {
        enabled = false;
    }
    enabled
}

/// Whether a package's `pi` object lists any resources. Since v1.5.0 a `pi`
/// object holding only metadata (`changelogPath`, gallery `image`/`video`)
/// keeps directory auto-discovery: "Packages whose `pi` object holds only
/// metadata ... still use directory auto-discovery" (package-manager.ts).
fn pi_lists_resources(value: &Value) -> bool {
    ["extensions", "skills", "prompts", "themes"]
        .iter()
        .any(|key| {
            value
                .get(*key)
                .and_then(Value::as_array)
                .is_some_and(|items| items.iter().all(Value::is_string))
        })
}

fn pi_resource(
    path: &Path,
    base: &Path,
    value: &Value,
    manifest: bool,
    fs: &dyn FileSystem,
) -> Option<Resource> {
    let extension = path.extension().and_then(|value| value.to_str());
    for (kind, resource) in [
        ("prompts", Resource::PiPrompt),
        ("themes", Resource::PiTheme),
        ("skills", Resource::PiSkill),
    ] {
        if (kind == "themes" && extension != Some("json"))
            || (kind != "themes" && extension != Some("md"))
        {
            continue;
        }
        // A package manifest keeps a resource list only when every entry is a
        // string (Pi's readPiManifest drops the rest).
        let declared = value
            .get(kind)
            .and_then(Value::as_array)
            .filter(|items| !manifest || items.iter().all(Value::is_string));
        if let Some(entries) = declared {
            if pi_entries(path, base, entries, manifest, fs, kind == "skills") {
                return Some(resource);
            }
        } else if manifest
            && !pi_lists_resources(value)
            && member(path, base, kind, fs)
            && (kind != "skills" || pi_skill_in_directory(path, &base.join(kind), fs))
        {
            return Some(resource);
        }
    }
    None
}

/// Where a discovery manifest's relative paths start (customization
/// discovery, "Paths and patterns"). In a project's manifest: "relative paths
/// start at the folder that contains the `.agents/` folder ... If a relative
/// path doesn't exist there, Antigravity tries it from the repository root."
/// In a personal manifest: "relative paths in `entries` start at
/// `~/.gemini/config/`."
fn manifest_bases(registry: &Path, fs: &dyn FileSystem) -> Vec<PathBuf> {
    let folder = normalized(registry.parent().unwrap_or_else(|| Path::new(".")));
    let project = folder
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| super::AGY_PROJECT_FOLDERS.contains(&name));
    if !project {
        return vec![folder];
    }
    let owner = folder
        .parent()
        .map_or_else(|| folder.clone(), Path::to_path_buf);
    let mut bases = vec![owner.clone()];
    if let Some(repo) = owner.ancestors().find(|dir| fs.exists(&dir.join(".git"))) {
        if repo != owner {
            bases.push(repo.to_path_buf());
        }
    }
    bases
}

/// A manifest `path` resolved against the first base where it exists, else
/// the first base. "Antigravity doesn't expand `~`", so `~/x` stays relative.
fn manifest_path(bases: &[PathBuf], value: &str, fs: &dyn FileSystem) -> PathBuf {
    let candidates: Vec<PathBuf> = bases
        .iter()
        .map(|base| normalized(&base.join(value)))
        .collect();
    candidates
        .iter()
        .find(|candidate| fs.exists(candidate))
        .unwrap_or(&candidates[0])
        .clone()
}

/// An `include_only` or `exclude` item names a rule "by name or by relative
/// path": "A file name, with or without `.md`" or "A relative path for nested
/// items". "Wildcards aren't supported".
fn manifest_item(relative: &Path, item: &str) -> bool {
    let item = normalized(Path::new(item.trim_end_matches('/')));
    relative == item || relative.with_extension("") == item
}

/// Whether a `rules.json` (or a manifest it inherits) registers `path`. A
/// folder entry is scanned "one level deep"; nested rules load only when
/// `include_only` names them. Entry and top-level `exclude` lists hide items.
fn registry_owns(path: &Path, registry: &Path, fs: &dyn FileSystem) -> bool {
    let file = normalized(path);
    let mut pending = vec![registry.to_path_buf()];
    let mut visited = HashSet::new();
    let mut bytes = 0u64;
    while let Some(registry) = pending.pop() {
        let identity = fs
            .canonicalize(&registry)
            .unwrap_or_else(|_| normalized(&registry));
        if !visited.insert(identity) {
            continue;
        }
        let Ok(metadata) = fs.symlink_metadata(&registry) else {
            continue;
        };
        bytes = bytes.saturating_add(metadata.len);
        if bytes > crate::file_utils::DEFAULT_MAX_FILE_SIZE {
            break;
        }
        let Some(manifest) = small_json(&registry, fs) else {
            continue;
        };
        let bases = manifest_bases(&registry, fs);
        if let Some(inherits) = manifest.get("inherits").and_then(Value::as_array) {
            pending.extend(
                inherits
                    .iter()
                    .filter_map(|inherit| inherit.get("path").and_then(Value::as_str))
                    .map(|value| manifest_path(&bases, value, fs)),
            );
        }
        let hidden = |relative: &Path, list: Option<&Value>| {
            let name = relative.file_name().map(Path::new);
            list.and_then(Value::as_array).is_some_and(|items| {
                items.iter().filter_map(Value::as_str).any(|item| {
                    manifest_item(relative, item)
                        || name.is_some_and(|name| manifest_item(name, item))
                        || relative.starts_with(normalized(Path::new(item.trim_end_matches('/'))))
                })
            })
        };
        for entry in manifest
            .get("entries")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(value) = entry.get("path").and_then(Value::as_str) else {
                continue;
            };
            let root = manifest_path(&bases, value, fs);
            let relative = if fs.is_dir(&root) {
                let Ok(relative) = file.strip_prefix(&root) else {
                    continue;
                };
                let included = match entry.get("include_only").and_then(Value::as_array) {
                    Some(items) => items
                        .iter()
                        .filter_map(Value::as_str)
                        .any(|item| manifest_item(relative, item)),
                    None => relative.components().count() == 1,
                };
                if !included {
                    continue;
                }
                relative.to_path_buf()
            } else if file == root {
                let name = PathBuf::from(root.file_name().unwrap_or_default());
                // "Load only the matching items" holds for a single file too.
                let included = entry
                    .get("include_only")
                    .and_then(Value::as_array)
                    .is_none_or(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .any(|item| manifest_item(&name, item))
                    });
                if !included {
                    continue;
                }
                name
            } else {
                continue;
            };
            if !hidden(&relative, entry.get("exclude"))
                && !hidden(&relative, manifest.get("exclude"))
            {
                return true;
            }
        }
    }
    false
}

pub(crate) fn resource_owner(
    path: &Path,
    fs: &dyn FileSystem,
    scope: Option<Tool>,
    project_root: Option<&Path>,
) -> Option<Resource> {
    if let Some((tool, root, manifest)) = plugin_owner(path, fs, scope) {
        if tool == Tool::Pi {
            if let Some(resource) = pi_resource(
                path,
                &root,
                manifest.get("pi").unwrap_or(&Value::Null),
                true,
                fs,
            ) {
                return Some(resource);
            }
        } else {
            for (field, fallback, resource) in if tool == Tool::Grok {
                vec![
                    ("agents", "agents", Resource::GrokAgent),
                    ("skills", "skills", Resource::GrokSkill),
                    ("hooks", "hooks/hooks.json", Resource::GrokHooks),
                    ("mcpServers", ".mcp.json", Resource::GrokMcp),
                    ("lspServers", ".lsp.json", Resource::GrokLsp),
                ]
            } else {
                vec![("skills", "skills", Resource::AntigravitySkill)]
            } {
                let paths: Vec<&str> = match manifest.get(field) {
                    Some(Value::String(value)) => vec![value],
                    Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
                    None => vec![fallback],
                    _ => Vec::new(),
                };
                for value in paths {
                    let declared = target(&root, value);
                    let contained = tool != Tool::Grok
                        || fs
                            .canonicalize(&declared)
                            .unwrap_or_else(|_| declared.clone())
                            .starts_with(
                                fs.canonicalize(&root).unwrap_or_else(|_| normalized(&root)),
                            );
                    let skill =
                        matches!(resource, Resource::GrokSkill | Resource::AntigravitySkill);
                    let agent = resource != Resource::GrokAgent
                        || path.extension().and_then(|extension| extension.to_str()) == Some("md");
                    let component_file = !matches!(
                        resource,
                        Resource::GrokHooks | Resource::GrokMcp | Resource::GrokLsp
                    ) || normalized(path) == declared;
                    if contained
                        && agent
                        && component_file
                        && member(path, &root, value, fs)
                        && (!skill
                            || path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md"))
                    {
                        return Some(resource);
                    }
                }
            }
        }
    }
    if !matches!(
        path.extension().and_then(|value| value.to_str()),
        Some("md" | "json")
    ) {
        return None;
    }
    let mut roots: Vec<PathBuf> = path
        .parent()
        .into_iter()
        .flat_map(Path::ancestors)
        .map(Path::to_path_buf)
        .collect();
    roots.extend(project_root.map(Path::to_path_buf));
    if let Some(home) = super::home_dir() {
        roots.push(home);
    }
    let mut settings = Vec::new();
    let mut registries = Vec::new();
    for root in roots {
        settings.extend([
            root.join(".pi/settings.json"),
            root.join(".pi/agent/settings.json"),
        ]);
        registries.extend(
            super::AGY_PROJECT_FOLDERS
                .iter()
                .map(|folder| root.join(folder).join("rules.json")),
        );
        registries.push(root.join(".gemini/config/rules.json"));
    }
    if let Some(home) = std::env::var_os("PI_CODING_AGENT_DIR") {
        settings.push(PathBuf::from(home).join("settings.json"));
    }
    let mut visited = HashSet::new();
    for settings in settings {
        if !visited.insert(settings.clone()) {
            continue;
        }
        let Some(value) = small_json(&settings, fs) else {
            continue;
        };
        let base = settings.parent().unwrap_or_else(|| Path::new("."));
        if let Some(resource) = pi_resource(path, base, &value, false, fs) {
            return Some(resource);
        }
    }
    if path.extension().and_then(|value| value.to_str()) == Some("md")
        && registries
            .iter()
            .any(|registry| registry_owns(path, registry, fs))
    {
        return Some(Resource::AntigravityRule);
    }
    None
}
