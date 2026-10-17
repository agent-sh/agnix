//! GitHub Copilot setup workflow schema helpers.
//!
//! Hook files are validated in `rules::copilot_hooks`. This module reads the
//! `copilot-setup-steps` job of `.github/workflows/copilot-setup-steps.yml`.

use serde_yaml::{Mapping, Value as YamlValue};

/// The `copilot-setup-steps` job settings Copilot honors: "you can only
/// customize the following settings of the `copilot-setup-steps` job. If you
/// try to customize other settings, your changes will be ignored."
pub const SETUP_STEPS_JOB_KEYS: &[&str] = &[
    "steps",
    "permissions",
    "runs-on",
    "services",
    "snapshot",
    "timeout-minutes",
];

/// Maximum `timeout-minutes` of the `copilot-setup-steps` job.
pub const SETUP_STEPS_MAX_TIMEOUT_MINUTES: f64 = 59.0;

/// Parse Copilot setup workflow YAML.
pub fn parse_setup_steps_yaml(content: &str) -> Result<YamlValue, String> {
    serde_yaml::from_str(content).map_err(|e| e.to_string())
}

/// The `jobs.copilot-setup-steps` job mapping, if the workflow defines it.
pub fn setup_steps_job(workflow: &YamlValue) -> Option<&Mapping> {
    let jobs = yaml_get(workflow.as_mapping()?, "jobs")?.as_mapping()?;
    yaml_get(jobs, "copilot-setup-steps")?.as_mapping()
}

/// The runner labels of a `runs-on` value: a label, a list of labels, or a
/// `{ group, labels }` mapping.
pub fn runs_on_labels(runs_on: &YamlValue) -> Vec<&str> {
    match runs_on {
        YamlValue::String(label) => vec![label.as_str()],
        YamlValue::Sequence(labels) => labels.iter().filter_map(YamlValue::as_str).collect(),
        YamlValue::Mapping(map) => match yaml_get(map, "labels") {
            Some(YamlValue::String(label)) => vec![label.as_str()],
            Some(YamlValue::Sequence(labels)) => {
                labels.iter().filter_map(YamlValue::as_str).collect()
            }
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// True for a runner label that names macOS, which Copilot cloud agent does
/// not support ("only compatible with Ubuntu x64 and Windows 64-bit runners").
/// Expressions (`${{ ... }}`) are not resolved and never match.
pub fn is_macos_runner_label(label: &str) -> bool {
    let lower = label.to_ascii_lowercase();
    !lower.contains("${{") && (lower.starts_with("macos") || lower == "macos" || lower == "osx")
}

fn yaml_get<'a>(map: &'a Mapping, key: &str) -> Option<&'a YamlValue> {
    map.get(YamlValue::String(key.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setup_job_detection() {
        let yaml = parse_setup_steps_yaml(
            r#"
name: Copilot Setup Steps
jobs:
  copilot-setup-steps:
    runs-on: ubuntu-latest
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(setup_steps_job(&yaml).is_some());
    }

    #[test]
    fn wrong_job_name_not_accepted() {
        let yaml = parse_setup_steps_yaml(
            r#"
jobs:
  setup:
    runs-on: ubuntu-latest
    steps:
      - run: echo hi
"#,
        )
        .expect("expected valid yaml");
        assert!(setup_steps_job(&yaml).is_none());
    }

    // The previous helper also required a runner label containing "ubuntu"
    // and a non-empty `steps` list. The docs document Windows, larger and
    // ARC runners, and `steps` is an optional setting, so the job is found
    // whatever its runner and steps.
    #[test]
    fn job_without_steps_or_ubuntu_runner_is_found() {
        let yaml = parse_setup_steps_yaml(
            r#"
jobs:
  copilot-setup-steps:
    runs-on: windows-latest
"#,
        )
        .expect("expected valid yaml");
        assert!(setup_steps_job(&yaml).is_some());
    }

    #[test]
    fn runs_on_label_forms() {
        let single: YamlValue = serde_yaml::from_str("ubuntu-4-core").unwrap();
        assert_eq!(runs_on_labels(&single), vec!["ubuntu-4-core"]);
        let list: YamlValue = serde_yaml::from_str("[self-hosted, macOS]").unwrap();
        assert_eq!(runs_on_labels(&list), vec!["self-hosted", "macOS"]);
        let group: YamlValue = serde_yaml::from_str("{group: large, labels: [macos-14]}").unwrap();
        assert_eq!(runs_on_labels(&group), vec!["macos-14"]);
    }

    #[test]
    fn macos_labels() {
        for label in ["macos-latest", "macos-14", "macOS", "MACOS-13-large"] {
            assert!(is_macos_runner_label(label), "{label}");
        }
        for label in [
            "ubuntu-latest",
            "windows-latest",
            "arc-scale-set-name",
            "self-hosted",
            "${{ matrix.os }}",
        ] {
            assert!(!is_macos_runner_label(label), "{label}");
        }
    }
}
