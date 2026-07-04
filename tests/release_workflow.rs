use std::fs;

#[test]
fn release_publish_jobs_are_gated_by_tests() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/release.yml"))
        .expect("failed to read release workflow");

    assert!(
        workflow.contains("  test:\n    name: Test release commit"),
        "release workflow must define a test job"
    );
    assert!(
        workflow.contains("run: cargo fmt --check --all"),
        "release test job must run fmt"
    );
    assert!(
        workflow
            .contains("run: cargo clippy --workspace --all-targets --all-features -- -D warnings"),
        "release test job must run clippy"
    );
    assert!(
        workflow.contains("run: cargo test --workspace"),
        "release test job must run tests"
    );

    let lines: Vec<&str> = workflow.lines().collect();
    for job in ["release", "publish", "vscode"] {
        let job_line = format!("  {job}:");
        let start = lines
            .iter()
            .position(|line| *line == job_line)
            .unwrap_or_else(|| panic!("release workflow must contain job {job}"));
        let next_job = lines
            .iter()
            .skip(start + 1)
            .take_while(|line| !line.starts_with("  ") || line.starts_with("    "))
            .copied()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            next_job.contains("needs: [build, test]"),
            "{job} must depend on the release test job"
        );
    }
}

#[test]
fn release_workflow_publishes_lsp_binary_checksums() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/release.yml"))
        .expect("failed to read release workflow");

    assert!(
        workflow.contains("agnix-lsp-${{ matrix.target }}.sha256"),
        "release workflow must publish binary checksum assets for Zed"
    );
    assert!(
        workflow.contains("printf '%s  agnix-lsp\\n'"),
        "Unix LSP checksum must name the extracted binary"
    );
    assert!(
        workflow.contains("\"$lspHash  agnix-lsp.exe\""),
        "Windows LSP checksum must name the extracted binary"
    );
}

#[test]
fn action_download_script_verifies_release_checksum() {
    let root = env!("CARGO_MANIFEST_DIR");
    let script = fs::read_to_string(format!("{root}/scripts/download.sh"))
        .expect("failed to read download script");

    assert!(
        script.contains("CHECKSUM_URL=\"${DOWNLOAD_URL}.sha256\""),
        "download script must fetch release checksum sidecars"
    );
    assert!(
        script.contains("EXPECTED_SHA=") && script.contains("ACTUAL_SHA="),
        "download script must compare expected and actual SHA256 values"
    );
    assert!(
        script.contains("expected=\"${ARTIFACT_NAME}\"") && script.contains("base == expected"),
        "download script must bind checksum sidecar entries to the downloaded artifact filename"
    );
    assert!(
        script.contains("sub(/\\r$/, \"\", file)"),
        "download script must tolerate CRLF checksum sidecar entries"
    );
    assert!(
        script.contains("Checksum mismatch"),
        "download script must fail closed on checksum mismatch"
    );
}
