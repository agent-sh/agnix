//! CC-SET-037: sandbox settings that weaken the sandbox for everyone who
//! clones the repository (code.claude.com/docs/en/sandboxing, page dated
//! 2026-10-17). Checked in the shared `.claude/settings.json` only:
//! "A repository's `.claude/settings.json` can too ... Each one weakens the
//! sandbox".

use crate::{
    diagnostics::Diagnostic,
    schemas::kiro_json::{JsonMember, JsonNode, LineIndex},
};
use rust_i18n::t;
use std::path::Path;

/// "Unless managed settings set them, ... A repository's `.claude/settings.json`
/// can too, unless the sandbox is admin-required. Each one weakens the
/// sandbox".
const WEAKENING_FLAGS: &[&str] = &[
    "enableWeakerNestedSandbox",
    "enableWeakerNetworkIsolation",
    "network.allowAllUnixSockets",
    "network.allowLocalBinding",
];

/// `excludedCommands` entries that cover an interpreter or a tool that runs
/// files: "If you write a pattern that covers an interpreter, a script inside
/// your working directory, or a tool that acts on a file there ... Claude can
/// write that file and then run it outside the sandbox." The page's own
/// example of a broad entry is `docker *`.
const BROAD_EXCLUDED_PROGRAMS: &[&str] = &[
    "bash", "deno", "docker", "node", "npm", "npx", "perl", "php", "pnpm", "podman", "python",
    "python3", "ruby", "sh", "yarn", "zsh",
];

/// Write targets the "Security limitations" section names: "directories
/// containing executables in `$PATH`, system configuration directories, or
/// user shell configuration files such as `.bashrc` or `.zshrc`".
const RISKY_WRITE_TARGETS: &[&str] = &[
    "~",
    "~/",
    "/",
    "~/.bashrc",
    "~/.bash_profile",
    "~/.profile",
    "~/.zshrc",
    "~/.zprofile",
    "~/.zshenv",
    "~/.config/fish",
    "~/bin",
    "~/.local/bin",
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/etc",
    "/opt/homebrew/bin",
];

fn member_at<'a>(node: &'a JsonNode, dotted: &str) -> Option<&'a JsonMember> {
    let mut current = node;
    let mut found = None;
    for segment in dotted.split('.') {
        let member = current.value.get(segment)?;
        current = &member.value;
        found = Some(member);
    }
    found
}

fn string_items(member: &JsonMember) -> Vec<(&str, usize)> {
    member
        .value
        .value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.value.as_str().map(|s| (s, item.start)))
                .collect()
        })
        .unwrap_or_default()
}

fn excluded_command_is_broad(entry: &str) -> bool {
    let mut words = entry.split_whitespace();
    let Some(program) = words.next() else {
        return false;
    };
    let program = program.trim_end_matches('*');
    let rest: Vec<&str> = words.collect();
    // `docker *` covers everything; `docker compose *` is narrower, as the
    // page recommends ("A narrower pattern takes fewer commands out").
    (BROAD_EXCLUDED_PROGRAMS.contains(&program) && (rest.is_empty() || rest == ["*"]))
        || entry.trim() == "*"
}

fn risky_write_target(entry: &str) -> bool {
    let normalized = entry.trim().trim_end_matches("/**").trim_end_matches('/');
    let normalized = if normalized.is_empty() {
        "/"
    } else {
        normalized
    };
    RISKY_WRITE_TARGETS.contains(&normalized)
}

pub(super) fn check_sandbox_weakening(
    path: &Path,
    content: &str,
    root: &JsonNode,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(sandbox) = root.value.get("sandbox") else {
        return;
    };
    let sandbox = &sandbox.value;
    let lines = LineIndex::new(content);
    let mut push = |offset: usize, message: String, suggestion: String| {
        let (line, column) = lines.line_col(offset);
        diagnostics.push(
            Diagnostic::warning(path.to_path_buf(), line, column, "CC-SET-037", message)
                .with_suggestion(suggestion),
        );
    };

    for flag in WEAKENING_FLAGS {
        if let Some(member) = member_at(sandbox, flag)
            && member.value.value.as_bool() == Some(true)
        {
            push(
                member.key_start,
                t!("rules.cc_set_037.flag_message", key = *flag).to_string(),
                t!("rules.cc_set_037.flag_suggestion", key = *flag).to_string(),
            );
        }
    }

    if let Some(member) = member_at(sandbox, "network.allowUnixSockets") {
        for (socket, offset) in string_items(member) {
            if socket.contains("docker.sock") {
                push(
                    offset,
                    t!("rules.cc_set_037.docker_socket_message", socket = socket).to_string(),
                    t!("rules.cc_set_037.docker_socket_suggestion").to_string(),
                );
            }
        }
    }

    if let Some(member) = member_at(sandbox, "excludedCommands") {
        for (command, offset) in string_items(member) {
            if excluded_command_is_broad(command) {
                push(
                    offset,
                    t!("rules.cc_set_037.excluded_message", command = command).to_string(),
                    t!("rules.cc_set_037.excluded_suggestion").to_string(),
                );
            }
        }
    }

    if let Some(member) = member_at(sandbox, "filesystem.allowWrite") {
        for (target, offset) in string_items(member) {
            if risky_write_target(target) {
                push(
                    offset,
                    t!("rules.cc_set_037.allow_write_message", target = target).to_string(),
                    t!("rules.cc_set_037.allow_write_suggestion").to_string(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broad_excluded_commands() {
        assert!(excluded_command_is_broad("docker *"));
        assert!(excluded_command_is_broad("python*"));
        assert!(excluded_command_is_broad("bash"));
        assert!(excluded_command_is_broad("*"));
        assert!(!excluded_command_is_broad("docker compose *"));
        assert!(!excluded_command_is_broad("gh *"));
    }

    #[test]
    fn risky_write_targets() {
        assert!(risky_write_target("~/.zshrc"));
        assert!(risky_write_target("/usr/local/bin/"));
        assert!(risky_write_target("~/.local/bin/**"));
        assert!(risky_write_target("~"));
        assert!(!risky_write_target("~/.kube"));
        assert!(!risky_write_target("/tmp/build"));
    }
}
