# Releasing agnix

## Version Bumping

1. Update `[workspace.package].version` and the internal `agnix-rules` / `agnix-core` dependency versions in root `Cargo.toml`. Keep the unpublished root test package at `0.0.0`. Refresh `Cargo.lock`, then synchronize distribution manifests:
   ```bash
   cargo update --workspace
   cargo metadata --locked --format-version 1 --no-deps >/dev/null
   bash scripts/sync-versions.sh 0.X.Y
   ```
   Run a locked workspace check before tagging to verify all six crate versions in `Cargo.lock`. The sync script updates npm, Python, editor and plugin manifests, not Rust versions.

2. Update `CHANGELOG.md` with the new version section.

3. Commit version bump:
   ```bash
   git add Cargo.toml Cargo.lock CHANGELOG.md
   # Also stage the explicitly reviewed manifests changed by sync-versions.sh.
   git commit -m "release: v0.X.Y"
   ```

## Build Release Binaries

Release builds use LTO and stripped symbols (per project rules):

```bash
cargo build --release
```

The binaries are at:
- `target/release/agnix` (CLI)
- `target/release/agnix-lsp` (LSP server)
- `target/release/agnix-mcp` (MCP server)

## Pre-release Checks

```bash
# All tests pass
cargo test --workspace

# Doc tests
cargo test --doc --workspace

# Clippy clean
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Eval must be fully green (0 failed cases). It is gated in CI (ci.yml,
# "Rule efficacy eval"), so a red eval blocks merge - this is just a local
# pre-check.
cargo run -p agnix-cli --bin agnix -- eval tests/eval.yaml

# Self-lint (agnix validates its own config)
cargo run -p agnix-cli --bin agnix -- .

# Review RUSTSEC advisories (see docs/RUSTSEC-ADVISORIES.md for details)
# Check if ignored advisories can be removed:
# - RUSTSEC-2024-0384: instant (via notify) - waiting for notify 7.0
# - RUSTSEC-2025-0141: bincode (via iai-callgrind) - dev-only, low risk
cargo audit
cargo deny check advisories
```

## Creating a GitHub Release

1. Tag the release:
   ```bash
   git tag -a v0.X.Y -m "Release v0.X.Y"
   git push origin v0.X.Y
   ```

2. The GitHub Actions release workflow will automatically:
   - Build CLI, LSP and MCP binaries for Linux (x86_64 GNU/musl, aarch64 GNU), macOS (aarch64), and Windows (x86_64). On macOS, also cross-compile the LSP for Intel using the Apple SDK and Rust's `x86_64-apple-darwin` target.
   - Create a GitHub Release with the binaries
   - Publish the crates, the npm package, and the PyPI wheels
   - Publish VS Code and JetBrains plugins, and move the floating `v0` tag

3. Verify every publish job, not only the GitHub Release at https://github.com/agent-sh/agnix/releases. Crates and editor publications can fail independently. The workflow publishes `agnix-rules`, then `agnix-core`, then CLI/LSP/MCP; do not manually double-publish.

The Homebrew and Zed jobs open pull requests in their registry repositories.
Review their diffs, wait for CI and Revuto, and merge those PRs before treating
the distribution update as complete.

### Staging the v0.62.0 Intel macOS LSP backfill

Zed needs the archive and a checksum of the **extracted executable**. JetBrains
needs a third file, the checksum of the **archive**. Normal releases ship all
three. The two-file v0.62.0 repair below is for Zed only; it does not update a
marketplace extension or claim to restore JetBrains downloads.

On macOS with Xcode and rustup, start at the root of the reviewed fix checkout.
Use a fresh build directory and the compiler from the v0.62.0 macOS build
(job `140065461592`). The packaging helper comes from the fix; the binary and
Cargo.lock must come from the original release commit, never from the fix.
These commands stage files locally without a GitHub write token:

```bash
set -euo pipefail
pack="$PWD/scripts/package-unix-lsp.py"
repo=agent-sh/agnix
tag=v0.62.0
commit=66c82e01f10281198449b2ab1ae125d97b3e1c2f
target=x86_64-apple-darwin
stem=agnix-lsp-$target
stage=$(mktemp -d "${TMPDIR%/}/agnix-intel.XXXXXXXX")
git clone --depth 1 --branch "$tag" "https://github.com/$repo.git" "$stage/src"
cd "$stage/src"
test "$(git rev-parse "$tag^{commit}")" = "$commit"
test "$(git rev-parse HEAD)" = "$commit"
test -z "$(git status --porcelain)"
rustup toolchain install 1.100.0 --profile minimal --target "$target"
export CARGO_TARGET_DIR="$stage/target"
# Keep checkout and registry paths out of diagnostic strings in the binary.
export CARGO_ENCODED_RUSTFLAGS="--remap-path-prefix=$stage=/build"$'\x1f'"--remap-path-prefix=${CARGO_HOME:-$HOME/.cargo}=/cargo"
cargo +1.100.0 build --locked --release --target "$target" -p agnix-lsp
mkdir "$stage/assets" "$stage/check"
cd "$stage/assets"
python3 "$pack" "$target" "$CARGO_TARGET_DIR/$target/release/agnix-lsp"
shasum -a 256 -c "$stem.tar.gz.sha256"
test "$(tar -tzf "$stem.tar.gz")" = agnix-lsp
tar -xzf "$stem.tar.gz" -C ../check
(cd ../check && test -x agnix-lsp && shasum -a 256 -c "../assets/$stem.sha256")
test "$(lipo -archs ../check/agnix-lsp)" = x86_64
file ../check/agnix-lsp
```

Record `rustc +1.100.0 -vV`, `xcodebuild -version`, `xcrun --show-sdk-version`,
`python3 --version` and the digests with the staged files. Packing the same
binary with the same tools is deterministic. Do not infer native execution
from a Mach-O header; use an Intel Mac for a native `agnix-lsp --version` check.

**Publication is a separate maintainer step.** Do not rerun the release
workflow, move tags, use `--clobber`, or upload `assets/*`. Immediately before
upload, verify the remote tag and release and take an inventory:

```bash
test "$(git ls-remote "https://github.com/$repo.git" "refs/tags/$tag^{}" | cut -f1)" = "$commit"
gh api "repos/$repo/releases/tags/$tag" > "$stage/before.json"
jq -e --arg stem "$stem" '.id == 459711967 and .tag_name == "v0.62.0"
  and .draft == false and .immutable == false
  and all(.assets[]; .name != ($stem + ".tar.gz") and .name != ($stem + ".sha256"))' "$stage/before.json"
```

Only after approval, upload the two named files without replacement:

```bash
gh release upload "$tag" "$stem.sha256" "$stem.tar.gz" --repo "$repo"
```

Read back the release by ID and confirm every original asset's name, ID and
digest is unchanged and the only additions are those two files. Download both
into a fresh directory with `gh release download --repo "$repo" "$tag" -p "$stem.sha256" -p "$stem.tar.gz"`;
compare each byte-for-byte to the staged files and repeat the archive and raw
checksum checks above. If interrupted or either file already exists, stop:
compare the existing asset against the retained staging directory before
uploading only a confirmed missing file. Never replace a mismatched asset.
The `.tar.gz.sha256` sidecar is staged for verification; publishing it for
JetBrains requires expanding the historical backfill scope.

### glibc floor

Both `*-unknown-linux-gnu` targets build through `cross`, whose sysroot is much
older than the runner image, and the `Verify glibc floor` step fails the build
if any of the three binaries references a glibc newer than `GLIBC_FLOOR` in
`scripts/check-glibc-floor.sh`. Building natively on `ubuntu-latest` instead
pinned the x86_64 binary to `GLIBC_2.39`, which made it unloadable on Debian
bookworm, RHEL 9, and Ubuntu 22.04 (#1371).

Do not switch a gnu target back to a native build to shave build time. To check
a binary by hand:

```bash
bash scripts/check-glibc-floor.sh target/x86_64-unknown-linux-gnu/release/agnix
```

## Post-release Verification

After the release workflow completes, verify all install targets work. This should be automated via a post-release CI workflow.

### Install Targets to Verify

| Target | Install Command | Verify Command |
|--------|----------------|----------------|
| **Cargo** | `cargo install agnix-cli` | `agnix --version` |
| **Homebrew** | `brew install agnix` | `agnix --version` |
| **npm** | `npm install -g agnix` | `agnix --version` |
| **PyPI** | `pip install agnix` | `agnix --version` |
| **GitHub Release** | Download from releases page | Run binary directly |

### Editor Extensions to Verify

| Editor | Install Method | Verify |
|--------|---------------|--------|
| **VS Code** | Marketplace or `code --install-extension` | Open a CLAUDE.md, check diagnostics appear |
| **JetBrains** | Plugin marketplace | Open a CLAUDE.md, check diagnostics appear |
| **Neovim** | Plugin manager (lazy.nvim, etc.) | `:LspInfo` shows agnix-lsp attached |
| **Zed** | Extension marketplace | Open a CLAUDE.md, check diagnostics appear |

### Post-release CI (Ideal)

A `post-release.yml` workflow triggered on release publication should:
1. Install from each distribution channel (cargo, brew, npm, pip)
2. Run `agnix --version` to verify correct version
3. Run `agnix` against a small test fixture to verify basic functionality
4. Verify editor extension marketplace listings are updated
5. Verify documentation website is deployed with new version

### Manual Checklist

- [ ] GitHub Release page shows all platform binaries
- [ ] `cargo install agnix-cli` installs the new version
- [ ] VS Code extension downloads the new LSP binary
- [ ] Documentation website shows the new version
- [ ] CHANGELOG.md is up to date
- [ ] Announce on relevant channels
- [ ] Close any milestone issues tied to this release

## Documentation & Website

Documentation and website updates are **automated** by the `version-docs`
job in `.github/workflows/release.yml`. On every non-prerelease tag push
the job will:

1. Regenerate `website/src/data/siteData.json` and rule docs from `rules.json`
2. Cut a Docusaurus versioned docs snapshot for the release
3. Open a `docs/version-X.Y.Z` PR against `main` (labeled `documentation` +
   `skip-changelog`) with the snapshot - it does **not** push to `main`
   directly. Merge that PR to land the docs.

The docs-site workflow then deploys automatically once the docs PR is merged
to main.

After release, verify at https://agent-sh.github.io/agnix/ that:
- New version docs are live in the version dropdown
- Rule reference pages match the current rules.json
- Landing page stats reflect the latest rule count

## Versioning Policy

See also: [Backward-Compatibility Policy](../CONTRIBUTING.md#backward-compatibility-policy) for the full stability tier definitions.

### Patch Release (0.X.Y)

No API changes. Examples:

- Bug fixes, false positive/negative improvements, diagnostic message quality
- Performance improvements with no public API change
- Documentation updates

### Minor Release (0.X.0)

Additive changes to Public/Stable tier. Before 1.0, necessary breaking changes also use a minor bump with explicit changelog migration instructions, as defined in CONTRIBUTING.md. Examples:

- New validation rules (e.g., adding CC-SK-016)
- New `FileType` enum variants
- New public functions or types
- New optional fields with `#[serde(default)]` on existing structs
- Changes to Public/Unstable modules (`authoring`, `eval`, `parsers`, `i18n`, `validation`)

### Major Release (X.0.0)

After 1.0, breaking changes to Public/Stable tier require a major bump. Before 1.0, they require a minor bump and documented migration. Examples:

- Removing or renaming a public type or function
- Changing a function signature (parameter types, return type)
- Removing enum variants or struct fields
- Changing `.agnix.toml` config format in an incompatible way
- Changing CLI interface in an incompatible way
- Renaming or removing rule IDs
