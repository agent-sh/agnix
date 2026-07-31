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
is authoritative (last verified: 2026-07-31).

```yaml
# GitHub Official Actions
actions/add-to-project@v2.0.0: 5afcf98fcd03f1c2f92c3c83f58ae24323cc57fd
actions/attest@v4.1.1: a1948c3f048ba23858d222213b7c278aabede763
actions/checkout@v7.0.0: 9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0
actions/configure-pages@v6.0.0: 45bfe0192ca1faeb007ade9deae92b16b8254a0d
actions/deploy-pages@v5.0.0: cd2ce8fcbc39b97be8ca5fce6e763baed58fa128
actions/download-artifact@v8.0.1: 3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c
actions/setup-java@v5.6.0: 03ad4de0992f5dab5e18fcb136590ce7c4a0ac95
actions/setup-node@v6.4.0: 48b55a011bda9f5d6aeb4c2d9c7362e8dae4041e
actions/setup-python@v7.0.0: 5fda3b95a4ea91299a34e894583c3862153e4b97
actions/stale@v10.4.0: 1e223db275d687790206a7acac4d1a11bd6fe629
actions/upload-artifact@v7.0.1: 043fb46d1a93c77aae656e7c1c64a875d1fc6a0a
actions/upload-pages-artifact@v5.0.0: fc324d3547104276b827a68afc52ff2a11cc49c9
rhysd/actionlint@v1.7.12: 914e7df21a07ef503a81201c76d2b11c789d3fca

# Rust Tooling
Swatinem/rust-cache@v2: c19371144df3bb44fab255c43d04cbc2ab54d1c4
dtolnay/rust-toolchain@stable: 4be9e76fd7c4901c61fb841f559994984270fce7
taiki-e/install-action@v2.85.0: 7572810d7dd469b651bb7793945692cf78da5dd7

# Security
EmbarkStudios/cargo-deny-action@v2.1.1: 3c6349835b2b7b196a839186cb8b78e02f7b5f25
github/codeql-action/analyze@v4.37.3: c54b30b7df092240050e69945842bc67aee0f0f4
github/codeql-action/init@v4.37.3: c54b30b7df092240050e69945842bc67aee0f0f4
github/codeql-action/upload-sarif@v4.37.3: c54b30b7df092240050e69945842bc67aee0f0f4

# Release
softprops/action-gh-release@v2: 718ea10b132b3b2eba29c1007bb80653f286566b

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
