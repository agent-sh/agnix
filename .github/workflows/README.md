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
is authoritative (last verified: 2026-10-24).

```yaml
# GitHub Official Actions
actions/attest@v4.2.2: 1e69f48acb82d1966a394da916b4c1698aa569d6
actions/checkout@v7.0.1: 3d3c42e5aac5ba805825da76410c181273ba90b1
actions/configure-pages@v6.0.0: 45bfe0192ca1faeb007ade9deae92b16b8254a0d
actions/deploy-pages@v5.0.1: 368f82528645a54fb793d4d04e342629a3f51346
actions/download-artifact@v8.0.2: 9000827ccba6bdab643e8b6fd33ac0654aef8333
actions/setup-java@v6.0.1: de7274f081f381c8f8158605e0321c36c376e2e6
actions/setup-node@v7.1.0: 949feb2413d6458794dcd2491c4babbbce0c15c1
actions/setup-python@v7.0.0: 5fda3b95a4ea91299a34e894583c3862153e4b97
actions/stale@v11.0.0: 4391f3da665fdf50b6810c1a66712fb9ba21aa93
actions/upload-artifact@v7.0.2: cf430e030ddbb5b0abf93d22962f4752f3646cd9
actions/upload-pages-artifact@v5.0.0: fc324d3547104276b827a68afc52ff2a11cc49c9
rhysd/actionlint@v1.7.12: 914e7df21a07ef503a81201c76d2b11c789d3fca

# Rust Tooling
Swatinem/rust-cache@v2.9.3: d1527c8f67ef9af357290475d4af4b0fab8f12e0
dtolnay/rust-toolchain@stable: 4be9e76fd7c4901c61fb841f559994984270fce7
taiki-e/install-action@v2.87.29: 8cca0918f8471c33515202b73f56ab27f2bed283

# Security
EmbarkStudios/cargo-deny-action@v2.1.1: 3c6349835b2b7b196a839186cb8b78e02f7b5f25
github/codeql-action/analyze@v4.39.0: 6f2ac81f5cd81fe351a197ba0b00d91d03cd2aee
github/codeql-action/init@v4.39.0: 6f2ac81f5cd81fe351a197ba0b00d91d03cd2aee
github/codeql-action/upload-sarif@v4.39.0: 6f2ac81f5cd81fe351a197ba0b00d91d03cd2aee

# Release
softprops/action-gh-release@v3.0.3: efb35369e0ad2afab669f228072c1b0d510eae64

# Claude Code
anthropics/claude-code-action@v1.0.255: e4a6faad497d33dbc7144135c75e41f82f0bf6c0
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
