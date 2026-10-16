# agnix Release Checklist

Use [docs/RELEASING.md](../../docs/RELEASING.md) as the canonical procedure. The tag-triggered `.github/workflows/release.yml` owns publication; do not manually double-publish.

This repository is a Rust workspace:
- `crates/agnix-core` (library)
- `crates/agnix-cli` (binary: `agnix`)

## Pre-Release Checklist

- CI is green on `main`
- `CHANGELOG.md` updated with the release notes
- Workspace version bumped in `Cargo.toml` (`[workspace.package].version`)
- `cargo fmt --check` passes
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- `cargo test` passes

## Tag and Release

```bash
git checkout main
git pull --ff-only

# if you bumped versions / changelog
git add Cargo.toml Cargo.lock CHANGELOG.md
# Also stage the explicitly reviewed distribution manifests.
git commit -m "chore: release vX.Y.Z"

git tag vX.Y.Z
git push origin main
git push origin refs/tags/vX.Y.Z
```

## Automated publication

The release workflow publishes `agnix-rules`, then `agnix-core`, then `agnix-cli`, `agnix-lsp`, and `agnix-mcp`. It also publishes GitHub assets, npm, PyPI, VS Code, and JetBrains packages and moves the floating `v0` tag. Homebrew, Zed, and versioned-docs updates open follow-up PRs that need review and merge.

Verify every publish job. A GitHub Release alone does not prove the registry or marketplace publications succeeded.

## Post-Release Verification

- GitHub Release created from the tag
- `cargo install agnix-cli` (or your chosen distribution method) installs successfully
- `agnix --help` runs and shows expected flags
