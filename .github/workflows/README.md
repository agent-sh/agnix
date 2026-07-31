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

When updating actions, use these SHA commits (last verified: 2026-07):

```yaml
# GitHub Official Actions
actions/checkout@v4:           34e114876b0b11c390a56381ad16ebd13914f8d5
actions/upload-artifact@v4:    ea165f8d65b6e75b540449e92b4886f43607fa02
actions/download-artifact@v7.0.0: 37930b1c2abaa49bbe596cd826c3c89aef350131
actions/setup-python@v6.2.0:    a309ff8b426b58ec0e2a45f0f869d46889d02405
actions/setup-node@v6.2.0:      6044e13b5dc448c55e2357c09f80417699197238
actions/configure-pages@v5:    983d7736d9b0ae728b81ab479565c72886d7745b
actions/upload-pages-artifact@v4.0.0: 7b1f4a764d45c48632c6b24a0339c27f5614fb0b
actions/deploy-pages@v4:       d6db90164ac5ed86f2b6aed7e0febac5b3c0c03e
rhysd/actionlint@v1.7.12:      914e7df21a07ef503a81201c76d2b11c789d3fca

# Rust Tooling
dtolnay/rust-toolchain@stable: 4be9e76fd7c4901c61fb841f559994984270fce7
Swatinem/rust-cache@v2:        779680da715d629ac1d338a641029a2f4372abb5
taiki-e/install-action@v2.83.4:  07b4745e0c39a41822af610387492e3e53aa222b
taiki-e/install-action@nextest: cd05dcd6eb73067dda063b97a15b7060049dacd9

# Security
github/codeql-action@v4.37.1:   7188fc363630916deb702c7fdcf4e481b751f97a
EmbarkStudios/cargo-deny-action@v2.1.1: 3c6349835b2b7b196a839186cb8b78e02f7b5f25

# Coverage
codecov/codecov-action@v5.5.2:  671740ac38dd9b0130fbe1cec585b89eea48d3de

# Release
actions/setup-java@v5.6.0:      03ad4de0992f5dab5e18fcb136590ce7c4a0ac95
softprops/action-gh-release@v2: a06a81a03ee405af7f2048a818ed3f03bbf83c7b

# Zed Extension
huacnlee/zed-extension-action@v2: 8cd592a0d24e1e41157740f1a529aeabddc88a1b

# Claude Code
anthropics/claude-code-action@v1: e0cf66d1d257526b5d07f141838c338921cb8455
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
| claude-code-review.yml | PR | Automated code review |
| spec-drift.yml | schedule/manual | Monitor upstream specs for changes |
| mcp-release-watch.yml | daily/manual | Watch MCP spec repo for new releases |
| docs-site.yml | push/PR/manual | Build and deploy documentation website |
