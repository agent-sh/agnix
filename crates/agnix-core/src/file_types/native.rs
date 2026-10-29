//! Native Pi, Antigravity CLI and Grok paths, before generic filename matches.
use super::FileType;
use std::path::{Component, Path};
mod resources;
pub(crate) use resources::{Resource, resource_owner};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tool {
    Pi,
    Antigravity,
    Grok,
}

pub(crate) fn home_dir() -> Option<std::path::PathBuf> {
    #[cfg(feature = "filesystem")]
    {
        dirs::home_dir()
    }
    #[cfg(not(feature = "filesystem"))]
    {
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
    }
}

pub(crate) fn selected(config: &crate::config::LintConfig) -> Option<Tool> {
    fn from_name(name: &str) -> Option<Tool> {
        match name.to_ascii_lowercase().as_str() {
            "pi" | "pi-coding-agent" => Some(Tool::Pi),
            "antigravity" | "agy" => Some(Tool::Antigravity),
            "grok-code" | "grok" => Some(Tool::Grok),
            _ => None,
        }
    }
    if config.tools().is_empty() {
        return config.target().tool_name().and_then(from_name);
    }
    let first = from_name(config.tools().first()?)?;
    config
        .tools()
        .iter()
        .all(|name| from_name(name) == Some(first))
        .then_some(first)
}

fn content_value(content: &str) -> Option<serde_json::Value> {
    let (blanked, _) =
        crate::schemas::kiro_json::blank_jsonc(content.trim_start_matches('\u{feff}'));
    serde_json::from_str(&blanked).ok()
}

pub(crate) fn hinted_basename(content: &str) -> Option<&'static str> {
    let value = content_value(content)?;
    match value.get("$schema")?.as_str()? {
        "https://pi.dev/schemas/settings.schema.json" => Some("settings.json"),
        "https://pi.dev/schemas/models.schema.json" => Some("models.json"),
        "https://pi.dev/schemas/keybindings.schema.json" => Some("keybindings.json"),
        "https://pi.dev/schemas/theme.schema.json" => Some("theme.json"),
        "https://antigravity.google/schemas/v1/settings.json" => Some("settings.json"),
        "https://antigravity.google/schemas/v1/plugin.json" => Some("plugin.json"),
        _ => None,
    }
}

/// Known schema IDs identify a bundled format. No schema URL is fetched.
pub(crate) fn refine_content(path: &Path, content: &str, detected: FileType) -> FileType {
    let ambiguous_settings = detected == FileType::Hooks
        && path.file_name().and_then(|name| name.to_str()) == Some("settings.json")
        && !components(path).iter().any(|part| part == ".claude");
    if !matches!(detected, FileType::Unknown | FileType::Plugin) && !ambiguous_settings {
        return detected;
    }
    let Some(value) = content_value(content) else {
        return detected;
    };
    if path.file_name().and_then(|p| p.to_str()) == Some("package.json")
        && (value.get("pi").is_some()
            || value
                .get("keywords")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|items| items.iter().any(|item| item.as_str() == Some("pi-package"))))
    {
        return FileType::PiConfig;
    }
    let schema = value
        .get("$schema")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("");
    if [
        "https://pi.dev/schemas/settings.schema.json",
        "https://pi.dev/schemas/models.schema.json",
        "https://pi.dev/schemas/keybindings.schema.json",
        "https://pi.dev/schemas/theme.schema.json",
    ]
    .contains(&schema)
    {
        return FileType::PiConfig;
    }
    if [
        "https://antigravity.google/schemas/v1/settings.json",
        "https://antigravity.google/schemas/v1/plugin.json",
    ]
    .contains(&schema)
    {
        return FileType::AntigravityConfig;
    }
    detected
}

fn small_json(path: &Path, fs: &dyn crate::fs::FileSystem) -> Option<serde_json::Value> {
    let metadata = fs.symlink_metadata(path).ok()?;
    if !metadata.is_file
        || metadata.is_symlink
        || metadata.len > crate::file_utils::DEFAULT_MAX_FILE_SIZE
    {
        return None;
    }
    let content = fs.read_to_string(path).ok()?;
    if content.len() as u64 > crate::file_utils::DEFAULT_MAX_FILE_SIZE {
        return None;
    }
    content_value(&content)
}

/// Strong plugin/package markers distinguish otherwise shared resource paths.
pub(crate) fn plugin_owner(
    path: &Path,
    fs: &dyn crate::fs::FileSystem,
    scope: Option<Tool>,
) -> Option<(Tool, std::path::PathBuf, serde_json::Value)> {
    for root in path.parent()?.ancestors() {
        for (marker, tool) in [
            (".antigravity-plugin/plugin.json", Tool::Antigravity),
            (".grok-plugin/plugin.json", Tool::Grok),
        ] {
            if let Some(value) = small_json(&root.join(marker), fs) {
                return Some((tool, root.to_path_buf(), value));
            }
        }
        if let Some(value) = small_json(&root.join("plugin.json"), fs) {
            let schema = value.get("$schema").and_then(serde_json::Value::as_str);
            let tool = match schema {
                Some("https://antigravity.google/schemas/v1/plugin.json") => {
                    Some(Tool::Antigravity)
                }
                _ if grok_relative(root)
                    .first()
                    .is_some_and(|part| part == "plugins") =>
                {
                    Some(Tool::Grok)
                }
                _ if sequence(&components(root), &[".agents", "plugins"])
                    || sequence(&components(root), &[".gemini", "config", "plugins"])
                    || sequence(
                        &components(root),
                        &[".gemini", "antigravity-cli", "plugins"],
                    ) =>
                {
                    Some(Tool::Antigravity)
                }
                _ => scope.filter(|tool| matches!(tool, Tool::Antigravity | Tool::Grok)),
            };
            if let Some(tool) = tool {
                return Some((tool, root.to_path_buf(), value));
            }
        }
        if let Some(value) = small_json(&root.join("package.json"), fs) {
            if value.get("pi").is_some()
                || value
                    .get("keywords")
                    .and_then(serde_json::Value::as_array)
                    .is_some_and(|items| {
                        items.iter().any(|item| item.as_str() == Some("pi-package"))
                    })
            {
                return Some((Tool::Pi, root.to_path_buf(), value));
            }
        }
        if fs.exists(&root.join(".git")) {
            break;
        }
    }
    None
}

pub(crate) fn refine(
    path: &Path,
    detected: FileType,
    fs: &dyn crate::fs::FileSystem,
    scope: Option<Tool>,
    project_root: Option<&Path>,
) -> FileType {
    if let Some(resource) = resource_owner(path, fs, scope, project_root) {
        return resource.file_type();
    }
    let filename = path.file_name().and_then(|p| p.to_str()).unwrap_or("");
    let json_file = path.extension().and_then(|p| p.to_str()) == Some("json");
    if detected == FileType::Unknown
        && !json_file
        && !matches!(
            filename,
            "package.json" | "plugin.json" | "settings.json" | "models.json" | "keybindings.json"
        )
    {
        return detected;
    }
    if detected == FileType::GenericMarkdown
        && scope.is_none()
        && !components(path).iter().any(|part| {
            matches!(
                part.as_str(),
                "agents" | "rules" | "skills" | "prompts" | "commands"
            )
        })
    {
        return detected;
    }
    if detected == FileType::Skill && detect(path) == Some(FileType::Skill) {
        return detected;
    }
    if !matches!(
        detected,
        FileType::Unknown
            | FileType::Plugin
            | FileType::Agent
            | FileType::Hooks
            | FileType::GenericMarkdown
            | FileType::Mcp
            | FileType::Skill
    ) {
        return detected;
    }
    let name = path.file_name().and_then(|p| p.to_str()).unwrap_or("");
    if json_file
        || matches!(
            name,
            "package.json" | "plugin.json" | "settings.json" | "models.json" | "keybindings.json"
        )
    {
        if let Some(value) = small_json(path, fs) {
            let text = value.to_string();
            let refined = refine_content(path, &text, detected);
            if refined != detected {
                return refined;
            }
        }
    }
    if let Some((tool, root, _manifest)) = plugin_owner(path, fs, scope) {
        let relative = path
            .strip_prefix(&root)
            .ok()
            .map(components)
            .unwrap_or_default();
        let first = relative.first().map(String::as_str);
        if tool == Tool::Pi && name == "package.json" && path.parent() == Some(root.as_path()) {
            return FileType::PiConfig;
        }
        if tool == Tool::Antigravity {
            if matches!(name, "plugin.json" | "mcp_config.json" | "hooks.json") {
                return FileType::AntigravityConfig;
            }
            if path.extension().and_then(|s| s.to_str()) == Some("md") {
                if first == Some("agents") {
                    return FileType::AntigravityAgent;
                }
                if first == Some("rules") {
                    return FileType::AntigravityRule;
                }
                if name == "SKILL.md" && first == Some("skills") {
                    return FileType::Skill;
                }
            }
        }
        if tool == Tool::Grok {
            if matches!(name, "plugin.json" | ".mcp.json" | ".lsp.json")
                || (first == Some("hooks")
                    && path.extension().and_then(|s| s.to_str()) == Some("json"))
            {
                return FileType::GrokConfig;
            }
            if first == Some("agents") && path.extension().and_then(|s| s.to_str()) == Some("md") {
                return FileType::GrokAgent;
            }
        }
    }
    if name == "plugin.json" {
        return match scope {
            Some(Tool::Antigravity) => FileType::AntigravityConfig,
            Some(Tool::Grok) => FileType::GrokConfig,
            _ => detected,
        };
    }
    detected
}

pub(crate) fn components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|part| match part {
            Component::Normal(name) => name.to_str().map(str::to_ascii_lowercase),
            _ => None,
        })
        .collect()
}

fn remap_home(path: &Path, home: &Path, namespace: &Path) -> Option<std::path::PathBuf> {
    // Vendor home overrides identify an exact directory. A relative setting
    // cannot establish ownership of arbitrary workspace paths.
    if !home.is_absolute() || !path.is_absolute() {
        return None;
    }
    path.strip_prefix(home)
        .ok()
        .map(|tail| namespace.join(tail))
}

fn native_path(path: &Path) -> std::path::PathBuf {
    for (variable, namespace) in [("PI_CODING_AGENT_DIR", ".pi/agent"), ("GROK_HOME", ".grok")] {
        if let Some(remapped) = std::env::var_os(variable)
            .and_then(|home| remap_home(path, Path::new(&home), Path::new(namespace)))
        {
            return remapped;
        }
    }
    path.to_path_buf()
}

pub(crate) fn sequence(parts: &[String], names: &[&str]) -> bool {
    parts
        .windows(names.len())
        .any(|window| window.iter().zip(names).all(|(a, b)| a == b))
}

pub(crate) fn pi_project(path: &Path) -> bool {
    let parts = components(&native_path(path));
    parts.iter().any(|part| part == ".pi") && !sequence(&parts, &[".pi", "agent"])
}

pub(crate) fn pi_resource(path: &Path, resource: &str) -> bool {
    let parts = components(&native_path(path));
    let Some(index) = parts.iter().rposition(|part| part == ".pi") else {
        return false;
    };
    let tail = &parts[index + 1..];
    let tail = if tail.first().is_some_and(|part| part == "agent") {
        &tail[1..]
    } else {
        tail
    };
    tail.first().is_some_and(|part| part == resource)
        && (!matches!(resource, "prompts" | "themes") || tail.len() == 2)
}

/// Components beneath the actual Grok namespace, not unrelated ancestors.
pub(crate) fn grok_relative(path: &Path) -> Vec<String> {
    let path = native_path(path);
    let parts = components(&path);
    let index = parts
        .iter()
        .enumerate()
        .filter_map(|(index, part)| {
            if part == ".grok" {
                Some(index)
            } else if path.starts_with("/etc/grok")
                && index == 0
                && part == "etc"
                && parts.get(index + 1).is_some_and(|next| next == "grok")
            {
                Some(index + 1)
            } else {
                None
            }
        })
        .max();
    index.map_or_else(Vec::new, |index| parts[index + 1..].to_vec())
}

pub(crate) fn detect(path: &Path) -> Option<FileType> {
    let path = native_path(path);
    let parts = components(&path);
    let filename = parts.last()?.as_str();
    let is_md = filename.ends_with(".md");
    let is_json = filename.ends_with(".json");
    if parts
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|part| part == ".grok-plugin")
        && matches!(filename, "plugin.json" | "marketplace.json")
    {
        return Some(FileType::GrokConfig);
    }
    if let Some(index) = parts.iter().rposition(|part| part == ".pi") {
        let tail = &parts[index + 1..];
        let tail = if tail.first().is_some_and(|part| part == "agent") {
            &tail[1..]
        } else {
            tail
        };
        if filename == "skill.md" && tail.first().is_some_and(|part| part == "skills") {
            return Some(FileType::Skill);
        }
        if (tail.len() == 1
            && matches!(
                filename,
                "settings.json" | "models.json" | "keybindings.json" | "mcp.json"
            ))
            || (is_json && tail.len() == 2 && tail.first().is_some_and(|part| part == "themes"))
        {
            return Some(FileType::PiConfig);
        }
        if is_md
            && ((tail.len() == 2 && tail.first().is_some_and(|part| part == "prompts"))
                || (tail.len() == 1 && matches!(filename, "system.md" | "append_system.md")))
        {
            return Some(FileType::PiPrompt);
        }
    }
    let agy_cli = sequence(&parts, &[".gemini", "antigravity-cli"]);
    let agy_global = sequence(&parts, &[".gemini", "config"]);
    let agy_project = parts
        .iter()
        .any(|part| matches!(part.as_str(), ".agents" | ".agent"));
    let agy_plugin = parts.iter().any(|part| part == ".antigravity-plugin");
    if agy_cli || agy_global || agy_project || agy_plugin {
        let index = parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                if matches!(part.as_str(), ".agents" | ".agent" | ".antigravity-plugin") {
                    Some(index)
                } else if part == ".gemini"
                    && parts
                        .get(index + 1)
                        .is_some_and(|next| matches!(next.as_str(), "config" | "antigravity-cli"))
                {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .max()?;
        let tail = &parts[index + 1..];
        let plugin = tail.first().is_some_and(|part| part == "plugins") && tail.len() >= 3;
        let relative = if plugin { &tail[2..] } else { tail };
        if filename == "skill.md"
            && relative.first().is_some_and(|part| part == "skills")
            && (agy_cli || agy_global || plugin || parts.iter().any(|part| part == ".agent"))
        {
            return Some(FileType::Skill);
        }
        if is_md && relative.len() == 2 && relative.first().is_some_and(|part| part == "rules") {
            return Some(FileType::AntigravityRule);
        }
        if is_md && relative.first().is_some_and(|part| part == "agents") {
            return Some(FileType::AntigravityAgent);
        }
        if (relative.len() == 1
            && matches!(filename, "mcp_config.json" | "hooks.json" | "rules.json"))
            || (agy_cli
                && relative.len() == 1
                && matches!(filename, "settings.json" | "keybindings.json"))
            || (agy_global && relative.len() == 1 && filename == "config.json")
            || (filename == "plugin.json" && (agy_plugin || (plugin && relative.len() == 1)))
        {
            return Some(FileType::AntigravityConfig);
        }
    }
    let grok = parts.iter().any(|part| part == ".grok") || path.starts_with("/etc/grok");
    if grok {
        let index = parts
            .iter()
            .enumerate()
            .filter_map(|(index, part)| {
                if part == ".grok" {
                    Some(index)
                } else if path.starts_with("/etc/grok")
                    && index == 0
                    && part == "etc"
                    && parts.get(index + 1).is_some_and(|next| next == "grok")
                {
                    Some(index + 1)
                } else {
                    None
                }
            })
            .max()?;
        let tail = &parts[index + 1..];
        let plugin = tail.first().is_some_and(|part| part == "plugins") && tail.len() >= 3;
        let resource = if plugin { &tail[2..] } else { tail };
        if filename == "skill.md" && tail.iter().any(|part| part == "skills") {
            return Some(FileType::Skill);
        }
        if (tail.len() == 1
            && matches!(
                filename,
                "config.toml"
                    | "managed_config.toml"
                    | "requirements.toml"
                    | "sandbox.toml"
                    | "lsp.json"
            ))
            || (filename.ends_with(".toml")
                && tail
                    .first()
                    .is_some_and(|part| part == "personas" || part == "roles"))
            || (is_json && tail.first().is_some_and(|part| part == "hooks"))
            || (is_json
                && matches!(filename, "plugin.json" | "marketplace.json" | ".lsp.json")
                && tail.first().is_some_and(|part| part == "plugins"))
        {
            return Some(FileType::GrokConfig);
        }
        if is_md && resource.first().is_some_and(|part| part == "agents") {
            return Some(FileType::GrokAgent);
        }
        if is_md && resource.first().is_some_and(|part| part == "rules") {
            return Some(FileType::GrokRule);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_overrides_require_exact_absolute_directory_membership() {
        let root = std::env::temp_dir().join("agnix-native-home-test");
        assert_eq!(
            remap_home(
                &root.join("pi/skills/review/SKILL.md"),
                &root.join("pi"),
                Path::new(".pi/agent")
            ),
            Some(Path::new(".pi/agent/skills/review/SKILL.md").to_path_buf())
        );
        assert_eq!(
            remap_home(
                &root.join("piano/settings.json"),
                &root.join("pi"),
                Path::new(".pi/agent")
            ),
            None
        );
        assert_eq!(
            remap_home(
                &root.join("pi/settings.json"),
                Path::new("pi"),
                Path::new(".pi/agent")
            ),
            None
        );
        assert_eq!(detect(Path::new("/work/etc/grok/requirements.toml")), None);
        assert_eq!(
            detect(Path::new("/etc/grok/requirements.toml")),
            Some(FileType::GrokConfig)
        );
    }
}
