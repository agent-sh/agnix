//! Checks for one MCP server entry, whatever client config it comes from.
//!
//! - MCP-018: literal secret in a server's `env`
//! - MCP-019: dangerous stdio startup command
//! - MCP-027: stdio server runs a package without a pinned version
//! - MCP-028: literal credential in a server's headers, URL or OAuth secret
//! - MCP-029: server given the home directory or the filesystem root
//!
//! [`McpValidator`](super::mcp::McpValidator) runs these on generic MCP files
//! (`.mcp.json`, `.cursor/mcp.json`, `.vscode/mcp.json`, ...).
//! [`validate_client_config`] runs them on client files that hold MCP
//! servers inside a larger config: OpenCode (`mcp`, V2 `mcp.servers`), Gemini
//! settings and extensions (`mcpServers`), Gemini agents (`mcpServers` or
//! `mcp_servers` frontmatter), Amp settings (`amp.mcpServers`), Amp skill
//! servers (SKILL.md `mcpServers` frontmatter and the skill's `mcp.json`),
//! Codex `config.toml` (`[mcp_servers.<name>]`) and Copilot custom agents
//! (`mcp-servers` frontmatter). Inline `mcpServers` in plugin manifests go
//! through `McpValidator`'s server checks, which end here. Kiro MCP files are
//! not covered here: KR-MCP-002 checks their secrets.
//!
//! Evidence: MCP local server security guide ("Keep long-lived secrets out of
//! plaintext config", "Pin the version", "Grant the narrowest root that works
//! ... never your home directory"), and each client's MCP page, which tells
//! users to reference environment variables for secrets.

use crate::{config::PerFileLintConfig, diagnostics::Diagnostic, file_types::FileType};
use regex::Regex;
use rust_i18n::t;
use serde_json::Value;
use std::path::Path;
use std::sync::OnceLock;

/// The parts of an MCP server entry the shared checks read. Field aliases of
/// every client are folded in: `env`/`environment`, `url`/`httpUrl`/
/// `serverUrl`, `headers`/`http_headers`, `oauth`/`auth` client secrets.
#[derive(Debug, Default)]
pub(crate) struct ServerView {
    /// Server name (map key).
    pub name: String,
    /// Command and arguments as one argv: a command array, or the command
    /// followed by `args`. A single command string with spaces and no `args`
    /// is split on whitespace.
    pub argv: Vec<String>,
    /// String-valued environment entries.
    pub env: Vec<(String, String)>,
    /// Remote endpoint, if any.
    pub url: Option<String>,
    /// String-valued header entries, with the field they came from.
    pub headers: Vec<(String, String)>,
    /// Literal OAuth client secrets and bearer tokens: (field, value).
    pub secrets: Vec<(String, String)>,
    /// True for a local (stdio) server.
    pub stdio: bool,
}

fn as_string(value: &Value) -> Option<String> {
    value.as_str().map(ToOwned::to_owned)
}

fn string_map(value: Option<&Value>) -> Vec<(String, String)> {
    let Some(obj) = value.and_then(Value::as_object) else {
        return Vec::new();
    };
    obj.iter()
        .filter_map(|(key, value)| value.as_str().map(|v| (key.clone(), v.to_string())))
        .collect()
}

impl ServerView {
    /// Read a server entry. `stdio` says whether the caller resolved the
    /// transport as local; pass `None` to infer it (a command, or `type`
    /// `stdio`/`local`, means local).
    pub(crate) fn from_json(name: &str, server: &Value, stdio: Option<bool>) -> Self {
        let mut view = ServerView {
            name: name.to_string(),
            ..ServerView::default()
        };
        let Some(obj) = server.as_object() else {
            return view;
        };

        match obj.get("command") {
            Some(Value::String(command)) => {
                let has_args = obj
                    .get("args")
                    .and_then(Value::as_array)
                    .is_some_and(|a| !a.is_empty());
                if has_args {
                    view.argv.push(command.clone());
                } else {
                    view.argv
                        .extend(command.split_whitespace().map(ToOwned::to_owned));
                }
            }
            Some(Value::Array(items)) => {
                view.argv.extend(items.iter().filter_map(as_string));
            }
            _ => {}
        }
        if let Some(args) = obj.get("args").and_then(Value::as_array) {
            view.argv.extend(args.iter().filter_map(as_string));
        }

        view.env = string_map(obj.get("env").or_else(|| obj.get("environment")));
        view.url = ["url", "httpUrl", "serverUrl"]
            .iter()
            .find_map(|key| obj.get(*key).and_then(as_string));
        for key in ["headers", "http_headers"] {
            view.headers.extend(string_map(obj.get(key)));
        }

        for container in ["oauth", "auth"] {
            if let Some(auth) = obj.get(container).and_then(Value::as_object) {
                for (key, value) in auth {
                    let normalized = key.to_ascii_lowercase().replace(['_', '-'], "");
                    if normalized == "clientsecret"
                        && let Some(secret) = value.as_str()
                    {
                        view.secrets
                            .push((format!("{container}.{key}"), secret.to_string()));
                    }
                }
            }
        }
        if let Some(token) = obj.get("bearer_token").and_then(Value::as_str) {
            view.secrets
                .push(("bearer_token".to_string(), token.to_string()));
        }

        let declared_local = obj
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|t| t.eq_ignore_ascii_case("stdio") || t.eq_ignore_ascii_case("local"));
        view.stdio = stdio.unwrap_or(declared_local || !view.argv.is_empty());
        view
    }
}

// ---------------------------------------------------------------------------
// Secret references and placeholders
// ---------------------------------------------------------------------------

fn reference_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // ${VAR}, ${VAR:-x}, ${env:VAR}, ${input:x}, ${file:x}, ${{ secrets.X }},
        // ${user_config.X}, ${amp:id-token}, $VAR, %VAR%, {env:VAR}, {file:x},
        // $(command), {{template}}.
        Regex::new(
            r"\$\{[^}]*\}|\$[A-Za-z_][A-Za-z0-9_]*|%[A-Za-z_][A-Za-z0-9_]*%|\{(?:env|file):[^}]+\}|\$\(|\{\{",
        )
        .expect("reference pattern must compile")
    })
}

/// True when `value` is a reference to a secret kept elsewhere (an environment
/// variable, input, file or secret store) or an obvious placeholder, so it is
/// not a literal credential. Builds on the shared
/// [`seems_plaintext_secret`](crate::rules::seems_plaintext_secret) filter
/// (placeholders, variable names, values under 8 characters) and adds the
/// reference forms MCP clients document anywhere in a value: `${...}`,
/// `{env:...}`, `{file:...}`, `%VAR%`, `$VAR`.
pub(crate) fn is_reference_or_placeholder(value: &str) -> bool {
    let trimmed = value.trim();
    if !crate::rules::seems_plaintext_secret(trimmed) || reference_pattern().is_match(trimmed) {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    const PLACEHOLDER_WORDS: &[&str] = &[
        "your",
        "<",
        "xxx",
        "***",
        "placeholder",
        "changeme",
        "replace",
        "redacted",
        "dummy",
        "insert",
        "_here",
        "-here",
        "example",
    ];
    PLACEHOLDER_WORDS.iter().any(|word| lower.contains(word))
}

/// Header names that carry a credential.
fn is_credential_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower == "authorization"
        || lower == "proxy-authorization"
        || [
            "api-key", "apikey", "api_key", "token", "secret", "password", "auth-key",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
}

/// Return the literal credential in a header value, if any.
fn literal_header_credential(name: &str, value: &str) -> Option<String> {
    if !is_credential_header(name) || is_reference_or_placeholder(value) {
        return None;
    }
    let trimmed = value.trim();
    let lower = name.to_ascii_lowercase();
    if lower == "authorization" || lower == "proxy-authorization" {
        // "Bearer <token>", "Basic <b64>", "Token <token>"; the credential
        // part must look like one (12+ non-space characters).
        let mut parts = trimmed.split_whitespace();
        let scheme = parts.next()?;
        let credential = parts.next()?;
        let known_scheme = ["bearer", "basic", "token"]
            .iter()
            .any(|s| scheme.eq_ignore_ascii_case(s));
        return (known_scheme
            && credential.len() >= 12
            && !is_reference_or_placeholder(credential))
        .then(|| scheme.to_string());
    }
    (trimmed.len() >= 16 && !trimmed.contains(char::is_whitespace)).then(|| name.to_string())
}

/// Query parameters that carry a credential.
const CREDENTIAL_QUERY_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "api-key",
    "key",
    "token",
    "access_token",
    "auth",
    "auth_token",
    "secret",
    "client_secret",
    "password",
    "sig",
];

/// Return the URL part holding a literal credential: `user:password@` or a
/// credential query parameter with a literal value.
fn literal_url_credential(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url.trim()).ok()?;
    if parsed
        .password()
        .is_some_and(|p| !is_reference_or_placeholder(p))
    {
        return Some("user:password".to_string());
    }
    parsed.query_pairs().find_map(|(key, value)| {
        let key_lower = key.to_ascii_lowercase();
        (CREDENTIAL_QUERY_KEYS.contains(&key_lower.as_str())
            && value.len() >= 8
            && !is_reference_or_placeholder(&value))
        .then(|| key.to_string())
    })
}

/// Environment keys that look like they hold a secret (MCP-018).
fn is_sensitive_env_key(key: &str) -> bool {
    let upper = key.to_ascii_uppercase();
    [
        "API_KEY",
        "APIKEY",
        "SECRET",
        "TOKEN",
        "PASSWORD",
        "PASSWD",
        "PRIVATE_KEY",
        "ACCESS_KEY",
        "CREDENTIAL",
    ]
    .iter()
    .any(|needle| upper.contains(needle))
}

// ---------------------------------------------------------------------------
// Dangerous commands (MCP-019)
// ---------------------------------------------------------------------------

pub(crate) fn is_dangerous_command(command: &str) -> bool {
    let normalized = command.to_ascii_lowercase();
    let has_remote_pipe = (normalized.contains("curl") || normalized.contains("wget"))
        && normalized.contains('|')
        && (normalized.contains("| sh")
            || normalized.contains("|sh")
            || normalized.contains("| bash")
            || normalized.contains("|bash"));
    let has_sudo_rm = normalized.contains("sudo rm");
    let has_exfil_pattern = (normalized.contains("nc ") || normalized.contains("netcat "))
        && (normalized.contains("/etc/")
            || normalized.contains(".ssh")
            || normalized.contains("token"));
    has_remote_pipe || has_sudo_rm || has_exfil_pattern
}

// ---------------------------------------------------------------------------
// Package pinning (MCP-027)
// ---------------------------------------------------------------------------

/// Why a launched package counts as unpinned.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Unpinned {
    /// No version at all (`npx -y pkg`, `uvx tool`, `docker run image`).
    Missing,
    /// A floating tag or range (`@latest`, `@^1.2.0`, `:latest`).
    Floating(String),
}

/// A launched package and why it is unpinned.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct UnpinnedPackage {
    pub package: String,
    pub reason: Unpinned,
}

fn program_name(token: &str) -> String {
    let base = token.rsplit(['/', '\\']).next().unwrap_or(token);
    base.trim_end_matches(".exe")
        .trim_end_matches(".cmd")
        .to_ascii_lowercase()
}

/// True for an exact version: `1.2.3`, `v1.2.3`, `1.2.3-beta.1`.
fn is_exact_version(version: &str) -> bool {
    let version = version.strip_prefix('v').unwrap_or(version);
    let core = version.split(['-', '+']).next().unwrap_or(version);
    let parts: Vec<&str> = core.split('.').collect();
    parts.len() >= 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// A package spec that is a path, URL or other non-registry source, which
/// MCP-027 leaves alone.
fn is_local_or_url_spec(spec: &str) -> bool {
    spec.starts_with('.')
        || spec.starts_with('/')
        || spec.starts_with('~')
        || spec.starts_with('$')
        || spec.contains("://")
        || spec.starts_with("file:")
        || spec.starts_with("git+")
        || spec.starts_with("github:")
        || spec.ends_with(".js")
        || spec.ends_with(".mjs")
        || spec.ends_with(".cjs")
        || spec.ends_with(".ts")
        || spec.ends_with(".py")
        || spec.ends_with(".tgz")
        || (spec.contains('\\'))
}

/// Check an npm package spec (`pkg`, `pkg@1.2.3`, `@scope/pkg@latest`).
fn npm_spec_unpinned(spec: &str) -> Option<Unpinned> {
    if is_local_or_url_spec(spec) {
        return None;
    }
    let name_end = if let Some(rest) = spec.strip_prefix('@') {
        rest.find('@').map(|i| i + 1)
    } else {
        spec.find('@')
    };
    match name_end {
        None => Some(Unpinned::Missing),
        Some(at) => {
            let version = &spec[at + 1..];
            if is_exact_version(version) {
                None
            } else if version.is_empty() {
                Some(Unpinned::Missing)
            } else {
                Some(Unpinned::Floating(version.to_string()))
            }
        }
    }
}

/// Check a Python package spec for uvx / pipx (`tool`, `tool==1.2.3`,
/// `tool@1.2.3`).
fn python_spec_unpinned(spec: &str) -> Option<Unpinned> {
    if is_local_or_url_spec(spec) {
        return None;
    }
    if let Some((_, version)) = spec.split_once("==") {
        return (!is_exact_version(version) && version.contains('*'))
            .then(|| Unpinned::Floating(version.to_string()));
    }
    if let Some((_, version)) = spec.split_once('@') {
        return if version.eq_ignore_ascii_case("latest") {
            Some(Unpinned::Floating(version.to_string()))
        } else {
            None
        };
    }
    if spec.contains(['>', '<', '~', '^', '!']) {
        return Some(Unpinned::Floating(spec.to_string()));
    }
    Some(Unpinned::Missing)
}

/// Check a container image reference (`image`, `image:tag`, `image@sha256:...`).
fn image_unpinned(image: &str) -> Option<Unpinned> {
    if image.contains("@sha256:") || image.starts_with('$') {
        return None;
    }
    let last_segment = image.rsplit('/').next().unwrap_or(image);
    match last_segment.rsplit_once(':') {
        None => Some(Unpinned::Missing),
        Some((_, tag)) if tag.eq_ignore_ascii_case("latest") => {
            Some(Unpinned::Floating(tag.to_string()))
        }
        Some(_) => None,
    }
}

/// Flags of `docker run` / `podman run` that take a separate value.
const DOCKER_VALUE_FLAGS: &[&str] = &[
    "-e",
    "--env",
    "--env-file",
    "-v",
    "--volume",
    "--mount",
    "--name",
    "-p",
    "--publish",
    "--network",
    "--net",
    "-w",
    "--workdir",
    "-u",
    "--user",
    "--entrypoint",
    "-l",
    "--label",
    "--platform",
    "--pull",
    "--cpus",
    "-m",
    "--memory",
    "--add-host",
    "--device",
    "--cap-add",
    "--cap-drop",
    "--security-opt",
    "--tmpfs",
    "--ulimit",
    "-h",
    "--hostname",
    "--restart",
    "--log-driver",
    "--log-opt",
    "--gpus",
    "--ipc",
    "--pid",
    "--runtime",
    "--shm-size",
    "--stop-signal",
    "--dns",
    "--group-add",
    "--userns",
    "--cidfile",
];

/// Flags of npx-style launchers that take a separate value.
const NPX_VALUE_FLAGS: &[&str] = &["-p", "--package", "-c", "--call", "--registry", "--cache"];

/// Flags of uvx / pipx that take a separate value.
const PY_VALUE_FLAGS: &[&str] = &[
    "--from",
    "--with",
    "--with-requirements",
    "--python",
    "-p",
    "--index",
    "--index-url",
    "-i",
    "--extra-index-url",
    "--spec",
    "--pip-args",
    "--directory",
    "--project",
];

/// Find the package an MCP server's argv launches and report it when it is
/// not pinned to an exact version (or a container digest/tag).
pub(crate) fn unpinned_package(argv: &[String]) -> Option<UnpinnedPackage> {
    let program = program_name(argv.first()?);
    let rest: &[String] = &argv[1..];
    match program.as_str() {
        "npx" | "bunx" | "pnpx" => npm_launch(rest),
        "pnpm" | "yarn" | "bun" if rest.first().is_some_and(|a| a == "dlx" || a == "x") => {
            npm_launch(&rest[1..])
        }
        "npm" if rest.first().is_some_and(|a| a == "exec" || a == "x") => npm_launch(&rest[1..]),
        "uvx" => python_launch(rest),
        "uv" if rest.len() >= 2 && rest[0] == "tool" && rest[1] == "run" => {
            python_launch(&rest[2..])
        }
        "pipx" if rest.first().is_some_and(|a| a == "run") => python_launch(&rest[1..]),
        "docker" | "podman" => {
            let run_at = rest.iter().position(|a| a == "run")?;
            // Only `docker run ...` and `docker container run ...`.
            if run_at > 1 || (run_at == 1 && rest[0] != "container") {
                return None;
            }
            let mut iter = rest[run_at + 1..].iter();
            while let Some(arg) = iter.next() {
                if arg == "--" {
                    continue;
                }
                if arg.starts_with('-') {
                    if !arg.contains('=') && DOCKER_VALUE_FLAGS.contains(&arg.as_str()) {
                        iter.next();
                    }
                    continue;
                }
                return image_unpinned(arg).map(|reason| UnpinnedPackage {
                    package: arg.clone(),
                    reason,
                });
            }
            None
        }
        _ => None,
    }
}

fn npm_launch(args: &[String]) -> Option<UnpinnedPackage> {
    let mut iter = args.iter();
    let mut from_package_flag: Option<&String> = None;
    while let Some(arg) = iter.next() {
        if let Some(value) = arg
            .strip_prefix("--package=")
            .or_else(|| arg.strip_prefix("-p="))
        {
            return npm_spec_unpinned(value).map(|reason| UnpinnedPackage {
                package: value.to_string(),
                reason,
            });
        }
        if arg == "-p" || arg == "--package" {
            from_package_flag = iter.next();
            continue;
        }
        if arg.starts_with('-') {
            if !arg.contains('=') && NPX_VALUE_FLAGS.contains(&arg.as_str()) {
                iter.next();
            }
            continue;
        }
        let spec = from_package_flag.unwrap_or(arg);
        return npm_spec_unpinned(spec).map(|reason| UnpinnedPackage {
            package: spec.clone(),
            reason,
        });
    }
    let spec = from_package_flag?;
    npm_spec_unpinned(spec).map(|reason| UnpinnedPackage {
        package: spec.clone(),
        reason,
    })
}

fn python_launch(args: &[String]) -> Option<UnpinnedPackage> {
    let mut iter = args.iter();
    let mut from: Option<&String> = None;
    while let Some(arg) = iter.next() {
        if let Some(value) = arg
            .strip_prefix("--from=")
            .or_else(|| arg.strip_prefix("--spec="))
        {
            return python_spec_unpinned(value).map(|reason| UnpinnedPackage {
                package: value.to_string(),
                reason,
            });
        }
        if arg == "--from" || arg == "--spec" {
            from = iter.next();
            continue;
        }
        if arg.starts_with('-') {
            if !arg.contains('=') && PY_VALUE_FLAGS.contains(&arg.as_str()) {
                iter.next();
            }
            continue;
        }
        let spec = from.unwrap_or(arg);
        return python_spec_unpinned(spec).map(|reason| UnpinnedPackage {
            package: spec.clone(),
            reason,
        });
    }
    None
}

// ---------------------------------------------------------------------------
// Home directory or filesystem root (MCP-029)
// ---------------------------------------------------------------------------

fn home_root_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^(?:~|\$HOME|\$\{HOME\}|%USERPROFILE%|\$\{userHome\}|\$\{env:HOME\}|\$\{env:USERPROFILE\}|/|[a-z]:[\\/]?|/Users/[^/\\]+|/home/[^/\\]+|[a-z]:[\\/]Users[\\/][^/\\]+)[\\/]?$",
        )
        .expect("home/root pattern must compile")
    })
}

/// True when `path` names a home directory or a filesystem root itself (not
/// a directory inside it). Placeholder user names (`/Users/<name>`,
/// `/home/user`) are documentation, not a real home.
pub(crate) fn is_home_or_root(path: &str) -> bool {
    let trimmed = path.trim();
    if !home_root_pattern().is_match(trimmed) {
        return false;
    }
    let user = trimmed
        .trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let under_users =
        trimmed.contains("Users") || trimmed.contains("users") || trimmed.starts_with("/home/");
    !(under_users
        && (user.contains('<')
            || [
                "user",
                "username",
                "name",
                "you",
                "yourname",
                "your-name",
                "me",
                "example",
            ]
            .contains(&user.as_str())))
}

/// Find an argument that hands the server the home directory or the
/// filesystem root: a positional argument (not the value of a flag), or the
/// source of a container volume or bind mount.
pub(crate) fn home_or_root_argument(argv: &[String]) -> Option<String> {
    let program = argv.first().map(|p| program_name(p)).unwrap_or_default();
    let is_container = program == "docker" || program == "podman";
    for (idx, arg) in argv.iter().enumerate().skip(1) {
        let previous = &argv[idx - 1];
        if is_container {
            let source = if previous == "-v" || previous == "--volume" {
                arg.split(':').next()
            } else if let Some(value) = arg
                .strip_prefix("-v=")
                .or_else(|| arg.strip_prefix("--volume="))
            {
                value.split(':').next()
            } else if previous == "--mount" || arg.starts_with("--mount=") {
                arg.trim_start_matches("--mount=")
                    .split(',')
                    .find_map(|part| {
                        part.strip_prefix("source=")
                            .or_else(|| part.strip_prefix("src="))
                    })
            } else {
                None
            };
            if let Some(source) = source
                && is_home_or_root(source)
            {
                return Some(source.to_string());
            }
            continue;
        }
        let previous_is_flag = previous.starts_with('-') && !previous.contains('=');
        if !arg.starts_with('-') && !previous_is_flag && is_home_or_root(arg) {
            return Some(arg.clone());
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Running the checks
// ---------------------------------------------------------------------------

/// Where an MCP server entry sits in the file, for diagnostic positions.
pub(crate) struct Locator<'a> {
    content: &'a str,
    line_starts: Vec<usize>,
    /// Byte range that holds the server map; lookups stay inside it.
    start: usize,
    end: usize,
    /// Byte offset of each server's key, when the caller parsed the map with
    /// spans; otherwise the name is searched for.
    server_keys: Vec<(String, usize)>,
}

impl<'a> Locator<'a> {
    pub(crate) fn new(content: &'a str) -> Self {
        Self::within(content, 0, content.len(), Vec::new())
    }

    /// A locator for a server map at `start..end` of `content` (inline
    /// servers in a larger file), with the byte offset of each server key.
    pub(crate) fn within(
        content: &'a str,
        start: usize,
        end: usize,
        server_keys: Vec<(String, usize)>,
    ) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(content.match_indices('\n').map(|(i, _)| i + 1));
        let end = end.min(content.len());
        Self {
            content,
            line_starts,
            start: start.min(end),
            end,
            server_keys,
        }
    }

    fn line_col(&self, offset: usize) -> (usize, usize) {
        let idx = self.line_starts.partition_point(|&s| s <= offset) - 1;
        (idx + 1, offset - self.line_starts[idx] + 1)
    }

    /// Offset of the server's name: its known key offset, or else a quoted
    /// key, a TOML table header, or a YAML key.
    fn server_offset(&self, name: &str) -> Option<usize> {
        if let Some((_, offset)) = self.server_keys.iter().rev().find(|(key, _)| key == name) {
            return Some(*offset);
        }
        let scope = self.content.get(self.start..self.end)?;
        [
            format!("\"{name}\""),
            format!("mcp_servers.{name}]"),
            format!("mcp_servers.\"{name}\"]"),
            format!("{name}:"),
            format!("'{name}'"),
        ]
        .iter()
        .find_map(|needle| find_key(scope, needle))
        .map(|rel| self.start + rel)
    }

    /// Line and column of `needle` after the server's name, or of the name.
    /// A quoted JSON key (`"API_TOKEN"`) is also looked up bare, as YAML
    /// frontmatter and TOML inline tables write it.
    pub(crate) fn locate(&self, server: &str, needle: &str) -> (usize, usize) {
        let start = self.server_offset(server).unwrap_or(self.start);
        let bare = needle
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))
            .filter(|key| !key.is_empty());
        let offset = (!needle.is_empty())
            .then(|| {
                let scope = self.content.get(start..self.end)?;
                scope
                    .find(needle)
                    .or_else(|| bare.and_then(|key| find_key(scope, key)))
                    .map(|rel| start + rel)
            })
            .flatten()
            .unwrap_or(start);
        self.line_col(offset)
    }
}

/// The first `needle` in `haystack` that starts a key: not preceded by a
/// letter, digit, `_` or `-`, so the YAML key `s:` is not found inside
/// `mcpServers:`.
fn find_key(haystack: &str, needle: &str) -> Option<usize> {
    let starts_word = needle
        .chars()
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-');
    haystack
        .match_indices(needle)
        .map(|(at, _)| at)
        .find(|&at| {
            !starts_word
                || !haystack[..at]
                    .chars()
                    .next_back()
                    .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
        })
}

/// MCP-018's test for one `env` entry of a local server. Client rules that
/// also look for secrets in MCP server config (OC-CFG-005) skip what this
/// reports, so a secret is reported once.
pub(crate) fn is_literal_env_secret(key: &str, value: &str) -> bool {
    is_sensitive_env_key(key) && !is_reference_or_placeholder(value)
}

/// MCP-028's test for one header of a server.
pub(crate) fn is_literal_header_secret(name: &str, value: &str) -> bool {
    literal_header_credential(name, value).is_some()
}

/// MCP-028's test for an OAuth client secret (`oauth.clientSecret`).
pub(crate) fn is_literal_oauth_secret(value: &str) -> bool {
    !is_reference_or_placeholder(value)
}

/// Whether MCP-018 or MCP-028 reports the string at `field` (a key path
/// inside the server entry, such as `["env", "API_TOKEN"]`,
/// `["http_headers", "Authorization"]` or `["bearer_token"]`) of the server
/// `name`. Client rules that scan a whole config for secrets (CDX-CFG-010)
/// skip these, so each secret in an MCP server is reported once.
pub(crate) fn mcp_reports_server_secret(
    name: &str,
    server: &Value,
    field: &[&str],
    config: &PerFileLintConfig<'_>,
) -> bool {
    let Some(value) = field
        .iter()
        .try_fold(server, |node, key| node.get(*key))
        .and_then(Value::as_str)
    else {
        return false;
    };
    let view = ServerView::from_json(name, server, None);
    match field {
        ["env" | "environment", key] => {
            view.stdio && config.is_rule_enabled("MCP-018") && is_literal_env_secret(key, value)
        }
        ["headers" | "http_headers", header] => {
            config.is_rule_enabled("MCP-028") && is_literal_header_secret(header, value)
        }
        ["bearer_token"] => config.is_rule_enabled("MCP-028") && is_literal_oauth_secret(value),
        ["oauth" | "auth", key] => {
            key.to_ascii_lowercase().replace(['_', '-'], "") == "clientsecret"
                && config.is_rule_enabled("MCP-028")
                && is_literal_oauth_secret(value)
        }
        _ => false,
    }
}

/// Run MCP-018, MCP-019, MCP-027, MCP-028 and MCP-029 on one server.
pub(crate) fn check_server(
    view: &ServerView,
    path: &Path,
    locator: &Locator<'_>,
    config: &PerFileLintConfig<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = view.name.as_str();

    // MCP-018: literal secret in env (local servers).
    if view.stdio && config.is_rule_enabled("MCP-018") {
        for (key, value) in &view.env {
            if is_literal_env_secret(key, value) {
                let (line, col) = locator.locate(name, &format!("\"{key}\""));
                diagnostics.push(
                    Diagnostic::warning(
                        path.to_path_buf(),
                        line,
                        col,
                        "MCP-018",
                        t!("rules.mcp_018.message", server = name, key = key.as_str()),
                    )
                    .with_suggestion(t!("rules.mcp_018.suggestion")),
                );
            }
        }
    }

    // MCP-019: dangerous startup command (local servers).
    if view.stdio && config.is_rule_enabled("MCP-019") && !view.argv.is_empty() {
        let command_text = view.argv.join(" ");
        if is_dangerous_command(&command_text) {
            let (line, col) = locator.locate(name, "");
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-019",
                    t!(
                        "rules.mcp_019.message",
                        server = name,
                        command = command_text.as_str()
                    ),
                )
                .with_suggestion(t!("rules.mcp_019.suggestion")),
            );
        }
    }

    // MCP-027: unpinned package (local servers).
    if view.stdio
        && config.is_rule_enabled("MCP-027")
        && let Some(found) = unpinned_package(&view.argv)
    {
        let (line, col) = locator.locate(name, &found.package);
        let message = match &found.reason {
            Unpinned::Missing => t!(
                "rules.mcp_027.message_missing",
                server = name,
                package = found.package.as_str()
            ),
            Unpinned::Floating(version) => t!(
                "rules.mcp_027.message_floating",
                server = name,
                package = found.package.as_str(),
                version = version.as_str()
            ),
        };
        diagnostics.push(
            Diagnostic::warning(path.to_path_buf(), line, col, "MCP-027", message)
                .with_suggestion(t!("rules.mcp_027.suggestion")),
        );
    }

    // MCP-028: literal credential in headers, URL or OAuth secret.
    if config.is_rule_enabled("MCP-028") {
        let mut push = |field: &str, needle: &str| {
            let (line, col) = locator.locate(name, needle);
            diagnostics.push(
                Diagnostic::warning(
                    path.to_path_buf(),
                    line,
                    col,
                    "MCP-028",
                    t!("rules.mcp_028.message", server = name, field = field),
                )
                .with_suggestion(t!("rules.mcp_028.suggestion")),
            );
        };
        for (header, value) in &view.headers {
            if is_literal_header_secret(header, value) {
                push(&format!("header '{header}'"), header);
            }
        }
        if let Some(url) = &view.url
            && let Some(part) = literal_url_credential(url)
        {
            push(&format!("URL ({part})"), url);
        }
        // CDX-PL-019 reports a `clientSecret` in a Codex plugin's MCP file
        // as a field the manifest rejects; it is not reported twice.
        let codex_plugin_secrets = crate::rules::codex_plugin::reports_client_secrets(path, config);
        for (field, value) in &view.secrets {
            let key = field.rsplit('.').next().unwrap_or(field);
            let client_secret = key.to_ascii_lowercase().replace(['_', '-'], "") == "clientsecret";
            if is_literal_oauth_secret(value) && !(codex_plugin_secrets && client_secret) {
                push(field, key);
            }
        }
    }

    // MCP-029: home directory or filesystem root.
    if view.stdio
        && config.is_rule_enabled("MCP-029")
        && let Some(root) = home_or_root_argument(&view.argv)
    {
        let (line, col) = locator.locate(name, &root);
        diagnostics.push(
            Diagnostic::warning(
                path.to_path_buf(),
                line,
                col,
                "MCP-029",
                t!("rules.mcp_029.message", server = name, path = root.as_str()),
            )
            .with_suggestion(t!("rules.mcp_029.suggestion")),
        );
    }
}

// ---------------------------------------------------------------------------
// Client configs that embed MCP servers
// ---------------------------------------------------------------------------

/// Parse JSON that may carry comments and trailing commas (JSONC).
fn parse_jsonc(content: &str) -> Option<Value> {
    let (stripped, _) = crate::schemas::kiro_agent::strip_jsonc(content);
    serde_json::from_str(&stripped).ok()
}

/// The MCP server map in a client config, as (name, server) pairs.
fn client_servers(file_type: FileType, path: &Path, content: &str) -> Vec<(String, Value)> {
    let map_entries = |value: Option<&Value>| -> Vec<(String, Value)> {
        value
            .and_then(Value::as_object)
            .map(|obj| {
                obj.iter()
                    .filter(|(_, v)| v.is_object())
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };
    match file_type {
        FileType::OpenCodeConfig => {
            let Some(root) = parse_jsonc(content) else {
                return Vec::new();
            };
            let mcp = root.get("mcp");
            // V2 nests servers under `mcp.servers`; V1 puts them under `mcp`.
            match mcp.and_then(|m| m.get("servers")).filter(|s| s.is_object()) {
                Some(servers) => map_entries(Some(servers)),
                None => map_entries(mcp),
            }
        }
        FileType::GeminiSettings | FileType::GeminiExtension => map_entries(
            parse_jsonc(content)
                .as_ref()
                .and_then(|r| r.get("mcpServers")),
        ),
        FileType::AmpSettings => {
            let Some(root) = parse_jsonc(content) else {
                return Vec::new();
            };
            map_entries(
                root.get("amp.mcpServers")
                    .or_else(|| root.get("amp").and_then(|a| a.get("mcpServers"))),
            )
        }
        FileType::CodexConfig => {
            let is_toml = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("toml"));
            if !is_toml {
                return Vec::new();
            }
            let Ok(root) = toml::from_str::<toml::Value>(content) else {
                return Vec::new();
            };
            let Ok(json) = serde_json::to_value(root) else {
                return Vec::new();
            };
            map_entries(json.get("mcp_servers"))
        }
        FileType::CopilotAgent => {
            let Some(json) = frontmatter_json(content) else {
                return Vec::new();
            };
            map_entries(json.get("mcp-servers").or_else(|| json.get("mcpServers")))
        }
        // Gemini CLI agents: `mcpServers` as the subagents page shows, or
        // `mcp_servers` as the agent loader's schema reads it.
        FileType::GeminiAgent => {
            let Some(json) = frontmatter_json(content) else {
                return Vec::new();
            };
            map_entries(json.get("mcpServers").or_else(|| json.get("mcp_servers")))
        }
        // Amp skill servers in SKILL.md frontmatter (`mcpServers`).
        FileType::Skill => {
            let Some(json) = frontmatter_json(content) else {
                return Vec::new();
            };
            map_entries(json.get("mcpServers"))
        }
        // A skill's sibling `mcp.json`: Amp's flat server map, or the same
        // servers under an `mcpServers` key.
        FileType::AmpSkillMcp => {
            let Some(root) = parse_jsonc(content) else {
                return Vec::new();
            };
            match root.get("mcpServers").filter(|servers| servers.is_object()) {
                Some(servers) => map_entries(Some(servers)),
                None => map_entries(Some(&root)),
            }
        }
        _ => Vec::new(),
    }
}

/// The YAML frontmatter of a Markdown file as JSON.
fn frontmatter_json(content: &str) -> Option<Value> {
    let parts = crate::parsers::frontmatter::split_frontmatter(content);
    if !parts.has_frontmatter || !parts.has_closing {
        return None;
    }
    let yaml = serde_yaml::from_str::<serde_yaml::Value>(&parts.frontmatter).ok()?;
    serde_json::to_value(yaml).ok()
}

/// The literal secret values MCP-018 and MCP-028 report for the servers of a
/// client file (`file_type` as [`validate_client_config`] reads it). XP-SK-003
/// scans skill files as text and skips credentials inside these values, so a
/// secret in a skill's MCP server is reported once, by the MCP rule that
/// names the server and field.
pub(crate) fn reported_secret_values(
    file_type: FileType,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<String> {
    let mut values = Vec::new();
    if !is_client_config_with_mcp(file_type) {
        return values;
    }
    let env_rule = config.is_rule_enabled("MCP-018");
    let credential_rule = config.is_rule_enabled("MCP-028");
    if !env_rule && !credential_rule {
        return values;
    }
    let codex_plugin_secrets = crate::rules::codex_plugin::reports_client_secrets(path, config);
    for (name, server) in client_servers(file_type, path, content) {
        let view = ServerView::from_json(&name, &server, None);
        if env_rule && view.stdio {
            values.extend(
                view.env
                    .iter()
                    .filter(|(key, value)| is_literal_env_secret(key, value))
                    .map(|(_, value)| value.clone()),
            );
        }
        if credential_rule {
            values.extend(
                view.headers
                    .iter()
                    .filter(|(header, value)| is_literal_header_secret(header, value))
                    .map(|(_, value)| value.clone()),
            );
            if let Some(url) = &view.url
                && literal_url_credential(url).is_some()
            {
                values.push(url.clone());
            }
            values.extend(
                view.secrets
                    .iter()
                    .filter(|(field, value)| {
                        let key = field.rsplit('.').next().unwrap_or(field);
                        let client_secret =
                            key.to_ascii_lowercase().replace(['_', '-'], "") == "clientsecret";
                        is_literal_oauth_secret(value) && !(codex_plugin_secrets && client_secret)
                    })
                    .map(|(_, value)| value.clone()),
            );
        }
    }
    values
}

/// File types that hold MCP servers inside a larger client config.
pub(crate) fn is_client_config_with_mcp(file_type: FileType) -> bool {
    matches!(
        file_type,
        FileType::OpenCodeConfig
            | FileType::GeminiSettings
            | FileType::GeminiExtension
            | FileType::AmpSettings
            | FileType::CodexConfig
            | FileType::CopilotAgent
            | FileType::GeminiAgent
            | FileType::Skill
            | FileType::AmpSkillMcp
    )
}

/// The server-level MCP checks on a client config that embeds MCP servers.
/// [`McpValidator`](super::mcp::McpValidator) dispatches here for the file
/// types [`is_client_config_with_mcp`] accepts; the client's own validator
/// checks the rest of the file.
pub(crate) fn validate_client_config(
    file_type: FileType,
    path: &Path,
    content: &str,
    config: &PerFileLintConfig<'_>,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let servers = client_servers(file_type, path, content);
    if servers.is_empty() {
        return diagnostics;
    }
    let locator = Locator::new(content);
    for (name, server) in &servers {
        let view = ServerView::from_json(name, server, None);
        check_server(&view, path, &locator, config, &mut diagnostics);
    }
    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::LintConfig;
    use crate::rules::Validator;

    fn argv(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn references_and_placeholders_are_not_secrets() {
        for value in [
            "${API_KEY}",
            "${env:TOKEN}",
            "{env:TOKEN}",
            "$TOKEN",
            "%TOKEN%",
            "${input:api-key}",
            "${{ secrets.COPILOT_MCP_KEY }}",
            "Bearer ${TOKEN}",
            "YOUR_API_KEY",
            "your-token-here",
            "<token>",
            "GITHUB_TOKEN",
            "",
        ] {
            assert!(is_reference_or_placeholder(value), "{value}");
        }
        assert!(!is_reference_or_placeholder(
            "ghp_abcdefghijklmnopqrstuvwxyz0123"
        ));
        assert!(!is_reference_or_placeholder("hardcoded-secret"));
    }

    #[test]
    fn header_credentials() {
        assert!(literal_header_credential("Authorization", "Bearer abcdef0123456789xyz").is_some());
        assert!(literal_header_credential("authorization", "Bearer ${TOKEN}").is_none());
        assert!(literal_header_credential("Authorization", "Bearer your-token-here").is_none());
        assert!(literal_header_credential("X-API-Key", "sk_live_0123456789abcdef").is_some());
        assert!(literal_header_credential("X-API-Key", "short").is_none());
        assert!(literal_header_credential("Accept", "application/json-0123456789").is_none());
    }

    #[test]
    fn url_credentials() {
        assert!(
            literal_url_credential("https://mcp.example.com/sse?api_key=abcdef0123456789")
                .is_some()
        );
        assert!(literal_url_credential("https://user:s3cr3tpassw0rd@mcp.example.com/").is_some());
        assert!(literal_url_credential("https://mcp.example.com/sse?api_key=${KEY}").is_none());
        assert!(literal_url_credential("https://mcp.example.com/mcp?region=us").is_none());
    }

    #[test]
    fn unpinned_npx_uvx_docker() {
        let cases: &[(&[&str], Option<Unpinned>)] = &[
            (
                &["npx", "-y", "@modelcontextprotocol/server-github"],
                Some(Unpinned::Missing),
            ),
            (
                &["npx", "-y", "@playwright/mcp@latest"],
                Some(Unpinned::Floating("latest".into())),
            ),
            (
                &["npx", "-y", "pkg@^1.2.0"],
                Some(Unpinned::Floating("^1.2.0".into())),
            ),
            (&["npx", "-y", "@scope/pkg@1.2.3"], None),
            (&["npx", "--package=pkg@2.0.0", "bin"], None),
            (&["npx", "-p", "pkg", "bin"], Some(Unpinned::Missing)),
            (&["npx", "./local/server.js"], None),
            (&["node", "server.js"], None),
            (&["pnpm", "dlx", "pkg@1.0.0"], None),
            (&["uvx", "mcp-server-fetch"], Some(Unpinned::Missing)),
            (&["uvx", "mcp-server-fetch==1.2.3"], None),
            (&["uvx", "--from", "pkg==0.4.1", "tool"], None),
            (&["uvx", "--python", "3.12", "tool@1.0.0"], None),
            (&["uv", "tool", "run", "tool"], Some(Unpinned::Missing)),
            (&["pipx", "run", "tool"], Some(Unpinned::Missing)),
            (
                &[
                    "docker",
                    "run",
                    "-i",
                    "--rm",
                    "-e",
                    "TOKEN",
                    "ghcr.io/github/github-mcp-server",
                ],
                Some(Unpinned::Missing),
            ),
            (
                &["docker", "run", "-i", "image:latest"],
                Some(Unpinned::Floating("latest".into())),
            ),
            (&["docker", "run", "-i", "image:1.4"], None),
            (&["docker", "run", "image@sha256:abc"], None),
            (
                &["/usr/local/bin/npx", "-y", "pkg"],
                Some(Unpinned::Missing),
            ),
        ];
        for (parts, expected) in cases {
            let found = unpinned_package(&argv(parts)).map(|u| u.reason);
            assert_eq!(&found, expected, "{parts:?}");
        }
    }

    #[test]
    fn home_and_root_arguments() {
        assert_eq!(
            home_or_root_argument(&argv(&[
                "npx",
                "-y",
                "@modelcontextprotocol/server-filesystem@1.0.0",
                "/Users/alice"
            ])),
            Some("/Users/alice".into())
        );
        assert_eq!(
            home_or_root_argument(&argv(&["npx", "pkg", "~"])),
            Some("~".into())
        );
        assert_eq!(
            home_or_root_argument(&argv(&["npx", "pkg", "/"])),
            Some("/".into())
        );
        assert_eq!(
            home_or_root_argument(&argv(&["npx", "pkg", "/Users/alice/project"])),
            None
        );
        assert_eq!(
            home_or_root_argument(&argv(&["tool", "--cache-dir", "~"])),
            None
        );
        assert_eq!(
            home_or_root_argument(&argv(&["docker", "run", "-v", "/home/bob:/data", "img:1"])),
            Some("/home/bob".into())
        );
        assert_eq!(
            home_or_root_argument(&argv(&[
                "docker",
                "run",
                "--mount",
                "type=bind,source=$HOME,target=/x",
                "img:1"
            ])),
            Some("$HOME".into())
        );
    }

    #[test]
    fn client_configs_expose_their_servers() {
        let open_code = r#"{
  // comment
  "mcp": { "gh": { "type": "local", "command": ["npx", "-y", "pkg"] } },
}"#;
        let servers = client_servers(
            FileType::OpenCodeConfig,
            Path::new("opencode.jsonc"),
            open_code,
        );
        assert_eq!(servers.len(), 1);
        let v2 = r#"{"mcp": {"servers": {"a": {"type": "remote", "url": "https://x"}}}}"#;
        assert_eq!(
            client_servers(FileType::OpenCodeConfig, Path::new("opencode.json"), v2).len(),
            1
        );
        let amp = r#"{"amp.mcpServers": {"x": {"command": "npx", "args": ["pkg"]}}}"#;
        assert_eq!(
            client_servers(FileType::AmpSettings, Path::new(".amp/settings.json"), amp).len(),
            1
        );
        let codex = "[mcp_servers.docs]\ncommand = \"npx\"\nargs = [\"-y\", \"pkg\"]\n";
        assert_eq!(
            client_servers(
                FileType::CodexConfig,
                Path::new(".codex/config.toml"),
                codex
            )
            .len(),
            1
        );
        let agent = "---\nname: a\nmcp-servers:\n  srv:\n    type: local\n    command: npx\n    args: [\"-y\", \"pkg\"]\n---\nBody\n";
        assert_eq!(
            client_servers(
                FileType::CopilotAgent,
                Path::new(".github/agents/a.agent.md"),
                agent
            )
            .len(),
            1
        );
    }

    #[test]
    fn validator_reports_on_client_configs() {
        let config = LintConfig::default();
        let content = r#"{
  "mcpServers": {
    "remote": {
      "httpUrl": "https://mcp.example.com/mcp",
      "headers": { "Authorization": "Bearer abcdefghijklmnop0123" }
    },
    "local": { "command": "npx", "args": ["-y", "some-mcp-server"], "env": { "API_TOKEN": "literal-value-123" } }
  }
}"#;
        let diagnostics = crate::rules::mcp::McpValidator.validate(
            Path::new(".gemini/settings.json"),
            content,
            &config,
        );
        let rules: Vec<_> = diagnostics.iter().map(|d| d.rule.as_str()).collect();
        assert!(rules.contains(&"MCP-028"), "{rules:?}");
        assert!(rules.contains(&"MCP-027"), "{rules:?}");
        assert!(rules.contains(&"MCP-018"), "{rules:?}");
        let mcp_028 = diagnostics.iter().find(|d| d.rule == "MCP-028").unwrap();
        assert_eq!(mcp_028.line, 5, "{mcp_028:?}");
    }
}
