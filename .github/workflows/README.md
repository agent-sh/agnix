# GitHub Actions Workflows

This directory contains CI/CD workflows for the agnix project.

## Security Hardening

All workflows follow security best practices:

### 1. Explicit Permissions

Every workflow declares minimum required permissions at the workflow level.
Jobs that need additional permissions declare them at the job level.

- `permissions: {}` - No permissions (used when jobs specify their own)
- `permissions: contents: read` - Read-only access to repository contents

### 2. SHA-Pinned Actions

All third-party actions are pinned to specific commit SHAs to prevent
supply chain attacks. The SHA pins are documented with version comments
for maintainability.

### 3. Cache Save Restrictions

Rust caches (`Swatinem/rust-cache`) are configured with `save-if` conditions
to only save caches on protected branches (main) or tag pushes. This prevents
cache poisoning from pull requests.

## SHA Pin Reference

When updating actions, use these SHA commits. Generated from the workflow
files themselves - if this table and a `uses:` line disagree, the workflow
is authoritative (last verified: 2026-10-31).

```yaml
# GitHub Official Actions
actions/attest@v4.2.2: 1e69f48acb82d1966a394da916b4c1698aa569d6
actions/checkout@v7.0.1: 3d3c42e5aac5ba805825da76410c181273ba90b1
actions/configure-pages@v6.0.0: 45bfe0192ca1faeb007ade9deae92b16b8254a0d
actions/deploy-pages@v5.0.1: 368f82528645a54fb793d4d04e342629a3f51346
actions/download-artifact@v8.0.2: 9000827ccba6bdab643e8b6fd33ac0654aef8333
actions/setup-java@v6.1.0: b843ebf9b913146b38da2eaa7aeddf76074ae1ad
actions/setup-node@v7.1.0: 949feb2413d6458794dcd2491c4babbbce0c15c1
actions/setup-python@v7.1.0: 6d4fd22b20465ccc0f7354476588e8df7c9ca0e0
actions/stale@v11.0.0: 4391f3da665fdf50b6810c1a66712fb9ba21aa93
actions/upload-artifact@v7.0.2: cf430e030ddbb5b0abf93d22962f4752f3646cd9
actions/upload-pages-artifact@v5.0.0: fc324d3547104276b827a68afc52ff2a11cc49c9
rhysd/actionlint@v1.7.12: 914e7df21a07ef503a81201c76d2b11c789d3fca

# Rust Tooling
Swatinem/rust-cache@v2.9.3: d1527c8f67ef9af357290475d4af4b0fab8f12e0
dtolnay/rust-toolchain@stable: 4be9e76fd7c4901c61fb841f559994984270fce7
taiki-e/install-action@v2.87.32: 5488ac5a951645ce95062214902ac4b6bc1a1037

# Security
EmbarkStudios/cargo-deny-action@v2.1.1: 3c6349835b2b7b196a839186cb8b78e02f7b5f25
github/codeql-action/analyze@v4.39.1: 9b3254085494b55a0f7c4c2b3d2d52cc712ce370
github/codeql-action/init@v4.39.1: 9b3254085494b55a0f7c4c2b3d2d52cc712ce370
github/codeql-action/upload-sarif@v4.39.1: 9b3254085494b55a0f7c4c2b3d2d52cc712ce370

# Release
softprops/action-gh-release@v3.0.3: efb35369e0ad2afab669f228072c1b0d510eae64

# Claude Code
anthropics/claude-code-action@v1.0.264: d70510b174a1244958df0b03166a54912811f3e6
```

## Updating Action Versions

When a new version of an action is released:

1. Check the release notes for security implications
2. Get the full SHA of the release tag:
   ```bash
   git ls-remote --tags https://github.com/owner/repo refs/tags/vX.Y.Z
   ```
3. Update all occurrences in workflow files
4. Update this README with the new SHA
5. Test the workflows on a feature branch before merging

Local preflight and CI check every SHA-pinned action against this table.
CodeQL init, analyze and upload-sarif must also share the same SHA.

## Workflow Overview

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| ci.yml | push/PR to main | Lint, test, coverage, build |
| release.yml | tag push (v*) | Build and publish releases |
| fuzz.yml | schedule/manual | Fuzz testing with cargo-fuzz |
| security.yml | push/PR/schedule | CodeQL analysis and security audit |
| test-action.yml | push/PR (action paths) | Test the GitHub Action |
| changelog.yml | PR | Verify CHANGELOG.md is updated |
| claude.yml | issue/PR comments | Claude Code assistant |
| spec-drift.yml | schedule/manual | Monitor upstream specs for changes |
| mcp-release-watch.yml | daily/manual | Watch MCP spec repo for new releases |
| docs-site.yml | push/PR/manual | Build and deploy documentation website |
