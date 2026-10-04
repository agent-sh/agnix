use std::fs;

#[test]
fn ci_runs_tests_on_linux_windows_and_macos() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/ci.yml"))
        .expect("failed to read CI workflow")
        .replace("\r\n", "\n");

    assert!(
        workflow.contains("runs-on: ${{ matrix.os }}"),
        "CI test job must run on the matrix operating system"
    );
    assert!(
        workflow.contains("os: [ubuntu-latest, windows-latest, macos-latest]"),
        "CI matrix must include Linux, Windows, and macOS"
    );
    assert!(
        workflow.contains("run: cargo nextest run --locked --workspace --profile ci"),
        "CI matrix must run the full workspace test suite"
    );
}

#[test]
fn ci_keeps_linux_only_quality_gates_on_ubuntu() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/ci.yml"))
        .expect("failed to read CI workflow")
        .replace("\r\n", "\n");

    for step in [
        "Preflight regression tests",
        "Format check",
        "Clippy",
        "Rule efficacy eval",
        "Rule bookkeeping sync check",
        "Rule count table check",
        "Schema sync check",
    ] {
        let marker = format!("- name: {step}\n        if: matrix.os == 'ubuntu-latest'");
        assert!(
            workflow.contains(&marker),
            "{step} must stay scoped to Ubuntu so Windows/macOS exercise portability tests without duplicating Linux-only gates"
        );
    }
}

#[test]
fn validation_concurrency_cancels_only_superseded_pr_runs() {
    let root = env!("CARGO_MANIFEST_DIR");
    for name in ["ci.yml", "security.yml"] {
        let workflow = fs::read_to_string(format!("{root}/.github/workflows/{name}"))
            .expect("failed to read workflow");
        assert!(workflow.contains("group: ${{ github.workflow }}-${{ github.event_name }}-${{ github.event.pull_request.number || github.run_id }}"),
            "{name}: isolate workflows, PR numbers, and every non-PR run, including pending runs");
        assert!(
            workflow.contains("cancel-in-progress: ${{ github.event_name == 'pull_request' }}"),
            "{name}: main, scheduled and deployment runs must not be canceled"
        );
    }
}

#[test]
fn security_keeps_rust_extraction_and_all_security_gates() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/security.yml"))
        .expect("failed to read Security workflow");
    for required in [
        "languages: rust",
        "build-mode: none",
        "queries: security-extended",
        "category: \"/language:rust\"",
        "github/codeql-action/analyze@",
        "cargo audit --deny warnings",
        "command: check all",
        "schedule:",
        "push:",
        "pull_request:",
    ] {
        assert!(
            workflow.contains(required),
            "missing security gate: {required}"
        );
    }
    assert!(
        !workflow.contains("run: cargo build"),
        "CodeQL Rust extracts with rust-analyzer; a separate release build is redundant"
    );
}

#[test]
fn mcp_release_watch_filters_prerelease_tags() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/mcp-release-watch.yml"))
        .expect("failed to read MCP release-watch workflow")
        .replace("\r\n", "\n");

    assert!(
        workflow.contains(r#"select(test("^[0-9]{4}-[0-9]{2}-[0-9]{2}$"))"#),
        "MCP tag fallback must select only final date-based versions"
    );
    assert!(
        workflow.contains("] | max // empty"),
        "MCP tag fallback must select the newest final date-based version"
    );
    assert!(
        workflow.contains("Ignoring non-final MCP release/tag"),
        "MCP prerelease tags must be ignored instead of failing the workflow"
    );
    assert!(
        !workflow.contains("ERROR: Unsupported MCP release format"),
        "known MCP prerelease tag shapes must not hard-fail the workflow"
    );
}

#[test]
fn ci_preserves_full_suites_and_cold_runner_settings() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/ci.yml")).unwrap();
    for required in [
        "schedule:",
        "cron: '0 3 * * *'",
        "github.event_name == 'schedule'",
        "github.event_name == 'workflow_dispatch'",
        "contains(github.event.pull_request.labels.*.name, 'full-suite')",
        "github.event_name == 'pull_request'",
        "!contains(fromJSON('[\"OWNER\", \"MEMBER\", \"COLLABORATOR\"]'), github.event.pull_request.author_association)",
        "CARGO_INCREMENTAL: '0'",
        "CARGO_PROFILE_DEV_DEBUG: '0'",
        "tool: cargo-nextest@0.9.146",
        "if: env.FULL_TEST_SUITE != 'true'",
        "run: cargo nextest run --locked -p agnix-workspace-tests --profile ci",
        "- name: Test\n        if: env.FULL_TEST_SUITE == 'true'",
        "run: cargo test --locked --workspace --doc",
        "if: matrix.os == 'ubuntu-latest' && env.FULL_TEST_SUITE == 'true'",
        "--test kiro_ci_gate --run-ignored all --profile ci",
    ] {
        assert!(workflow.contains(required), "missing CI policy: {required}");
    }

    let config: toml::Value =
        toml::from_str(&fs::read_to_string(format!("{root}/.config/nextest.toml")).unwrap())
            .unwrap();
    assert_eq!(
        config["profile"]["default"]["test-threads"].as_integer(),
        Some(2)
    );
    assert_eq!(config["profile"]["ci"]["fail-fast"].as_bool(), Some(false));
    assert_eq!(config["profile"]["ci"]["retries"].as_integer(), Some(0));
}
