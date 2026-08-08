use std::fs;
use std::path::PathBuf;

#[test]
fn release_publish_jobs_are_gated_by_tests() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/release.yml"))
        .expect("failed to read release workflow")
        .replace("\r\n", "\n");

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
        .expect("failed to read release workflow")
        .replace("\r\n", "\n");

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
fn release_workflow_scopes_attestation_and_release_permissions() {
    let root = env!("CARGO_MANIFEST_DIR");
    let workflow = fs::read_to_string(format!("{root}/.github/workflows/release.yml"))
        .expect("failed to read release workflow")
        .replace("\r\n", "\n");

    assert!(
        workflow.contains("permissions:\n  contents: read"),
        "release workflow default token permissions must be read-only"
    );
    assert!(
        !workflow.contains("permissions:\n  contents: write"),
        "release workflow must not grant contents: write at workflow scope"
    );
    assert!(
        workflow.contains(
            "  build:\n    name: Build (${{ matrix.target }})\n    runs-on: ${{ matrix.os }}\n    permissions:\n      contents: read\n      id-token: write\n      attestations: write"
        ),
        "build job must carry only the permissions needed to attest built artifacts"
    );
    assert!(
        workflow.contains(
            "  release:\n    name: Create Release\n    needs: [build, test]\n    runs-on: ubuntu-latest\n    permissions:\n      contents: write"
        ),
        "release job must receive contents: write only where assets are published"
    );
    assert!(
        workflow.contains("uses: actions/attest@f7c74d28b9d84cb8768d0b8ca14a4bac6ef463e6 # v4.2.0"),
        "release workflow must use the SHA-pinned GitHub attestation action"
    );
    assert!(
        workflow.contains("agnix-*${{ matrix.target }}.tar.gz")
            && workflow.contains("agnix-*${{ matrix.target }}.zip"),
        "attestation subject paths must cover release archives"
    );
}

#[test]
fn release_workflow_pins_latest_versioned_docs_to_the_release_tag() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workflow = fs::read_to_string(root.join(".github/workflows/release.yml"))
        .expect("failed to read release workflow")
        .replace("\r\n", "\n");

    assert!(
        workflow.contains("- name: Pin versioned docs links to the release tag"),
        "release workflow must pin repository links after cutting a docs snapshot"
    );
    assert!(
        workflow.contains("agnix/blob/v{version}/") && workflow.contains("agnix/tree/v{version}/"),
        "release workflow must retarget both file and directory links"
    );

    let versions: Vec<String> = serde_json::from_str(
        &fs::read_to_string(root.join("website/versions.json"))
            .expect("failed to read docs versions"),
    )
    .expect("website/versions.json must contain a JSON string array");
    let latest = versions
        .first()
        .expect("docs must contain a latest version");
    let snapshot = root.join(format!("website/versioned_docs/version-{latest}"));
    let moving_links = [
        "https://github.com/agent-sh/agnix/blob/main/",
        "https://github.com/agent-sh/agnix/tree/main/",
    ];
    let pinned_links = [
        format!("https://github.com/agent-sh/agnix/blob/v{latest}/"),
        format!("https://github.com/agent-sh/agnix/tree/v{latest}/"),
    ];
    let mut stack = vec![snapshot];
    let mut pinned_link_count = 0;

    while let Some(path) = stack.pop() {
        for entry in fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
        {
            let entry = entry.expect("failed to read versioned docs entry");
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                let content = fs::read_to_string(&path)
                    .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
                assert!(
                    moving_links.iter().all(|link| !content.contains(link)),
                    "{} contains a repository link to moving main",
                    path.display()
                );
                pinned_link_count += pinned_links
                    .iter()
                    .filter(|link| content.contains(link.as_str()))
                    .count();
            }
        }
    }

    assert!(
        pinned_link_count > 0,
        "latest versioned docs must contain release-tagged repository links"
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
