use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn repo_file(path: &str) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(root.join(path)).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

fn is_rustsec_id(token: &str) -> bool {
    let Some(rest) = token.strip_prefix("RUSTSEC-") else {
        return false;
    };
    let Some((year, number)) = rest.split_once('-') else {
        return false;
    };

    year.len() == 4
        && year.chars().all(|c| c.is_ascii_digit())
        && number.len() == 4
        && number.chars().all(|c| c.is_ascii_digit())
}

fn rustsec_ids(content: &str) -> BTreeSet<String> {
    content
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .filter(|token| is_rustsec_id(token))
        .map(str::to_owned)
        .collect()
}

fn current_ignored_advisory_ids() -> BTreeSet<String> {
    ["RUSTSEC-2025-0141", "RUSTSEC-2026-0173"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn rustsec_ignore_lists_match_docs() {
    let expected = current_ignored_advisory_ids();

    let security_workflow = repo_file(".github/workflows/security.yml");
    assert!(
        security_workflow.contains("cargo audit"),
        "security.yml must run cargo audit"
    );
    let audit_ignores: BTreeSet<String> = security_workflow
        .lines()
        .filter(|line| line.contains("RUSTSEC-"))
        .flat_map(rustsec_ids)
        .collect();

    let deny_toml: toml::Value =
        toml::from_str(&repo_file("deny.toml")).expect("deny.toml must parse");
    let deny_ignores: BTreeSet<String> = deny_toml
        .get("advisories")
        .and_then(|advisories| advisories.get("ignore"))
        .and_then(toml::Value::as_array)
        .expect("deny.toml must define [advisories].ignore")
        .iter()
        .map(|id| {
            id.as_str()
                .expect("deny.toml advisory ignore entries must be strings")
                .to_owned()
        })
        .collect();

    let advisory_docs = repo_file("docs/RUSTSEC-ADVISORIES.md");
    let currently_ignored_section = advisory_docs
        .split("## Review Schedule")
        .next()
        .expect("advisory docs must contain a current ignored section");
    let documented_ignores = rustsec_ids(currently_ignored_section);

    assert_eq!(audit_ignores, expected, "cargo audit ignores drifted");
    assert_eq!(deny_ignores, expected, "cargo deny ignores drifted");
    assert_eq!(
        documented_ignores, expected,
        "RUSTSEC advisory docs drifted from enforced ignores"
    );
}

#[test]
fn stale_absent_crate_advisories_are_not_ignored() {
    let security_workflow = repo_file(".github/workflows/security.yml");
    let deny_toml = repo_file("deny.toml");
    let advisory_docs = repo_file("docs/RUSTSEC-ADVISORIES.md");
    let current_docs = advisory_docs
        .split("## Review Schedule")
        .next()
        .expect("advisory docs must contain a current ignored section");

    for stale_id in [
        "RUSTSEC-2024-0384",
        "RUSTSEC-2025-0067",
        "RUSTSEC-2025-0068",
        "RUSTSEC-2026-0009",
    ] {
        assert!(
            !security_workflow.contains(stale_id),
            "{stale_id} must not be ignored by cargo audit"
        );
        assert!(
            !deny_toml.contains(stale_id),
            "{stale_id} must not be ignored by cargo deny"
        );
        assert!(
            !current_docs.contains(stale_id),
            "{stale_id} must not appear as a current ignored advisory"
        );
    }

    let lockfile = repo_file("Cargo.lock");
    for absent_crate in ["instant", "libyml", "serde_yml", "time"] {
        assert!(
            !lockfile.contains(&format!("name = \"{absent_crate}\"")),
            "{absent_crate} is expected to be absent before removing its advisory ignore"
        );
    }
}

#[test]
fn direct_yaml_parser_uses_maintained_package() {
    let root_manifest: toml::Value =
        toml::from_str(&repo_file("Cargo.toml")).expect("Cargo.toml must parse");
    let serde_yaml = root_manifest
        .get("workspace")
        .and_then(|workspace| workspace.get("dependencies"))
        .and_then(|deps| deps.get("serde_yaml"))
        .and_then(toml::Value::as_table)
        .expect("workspace serde_yaml dependency must be an aliased table");

    assert_eq!(
        serde_yaml.get("package").and_then(toml::Value::as_str),
        Some("serde_norway"),
        "agnix's direct serde_yaml crate alias must use the maintained serde_norway package"
    );

    let fuzz_manifest: toml::Value =
        toml::from_str(&repo_file("crates/agnix-core/fuzz/Cargo.toml"))
            .expect("fuzz Cargo.toml must parse");
    let fuzz_serde_yaml = fuzz_manifest
        .get("dependencies")
        .and_then(|deps| deps.get("serde_yaml"))
        .and_then(toml::Value::as_table)
        .expect("fuzz serde_yaml dependency must be an aliased table");

    assert_eq!(
        fuzz_serde_yaml.get("package").and_then(toml::Value::as_str),
        Some("serde_norway"),
        "fuzz targets must exercise the same maintained YAML parser package"
    );
}

#[test]
fn deprecated_transitive_yaml_parser_is_tracked_until_removed() {
    let lockfile = repo_file("Cargo.lock");
    let advisory_docs = repo_file("docs/RUSTSEC-ADVISORIES.md");

    if lockfile.contains("name = \"serde_yaml\"") {
        assert!(
            advisory_docs.contains("serde_yaml 0.9.34+deprecated")
                && advisory_docs.contains("via `rust-i18n`"),
            "deprecated transitive serde_yaml must be tracked while it remains in Cargo.lock"
        );
    }
}
