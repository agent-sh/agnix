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
        workflow.contains("run: cargo test --workspace"),
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
        "Format check",
        "Clippy",
        "Rule efficacy eval",
        "Kiro S-tier Gate",
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
        workflow.contains("Ignoring non-final MCP release/tag"),
        "MCP prerelease tags must be ignored instead of failing the workflow"
    );
    assert!(
        !workflow.contains("ERROR: Unsupported MCP release format"),
        "known MCP prerelease tag shapes must not hard-fail the workflow"
    );
}
