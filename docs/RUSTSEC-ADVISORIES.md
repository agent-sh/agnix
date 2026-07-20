# RUSTSEC Advisory Tracking

This document tracks RUSTSEC security advisories that are currently ignored in the project and explains why they are ignored and when they should be reviewed.

Related: [Issue #346](https://github.com/agent-sh/agnix/issues/346) (this tracking system resolves that issue)

## Currently Ignored Advisories

### RUSTSEC-2025-0141 - `bincode` (via `iai-callgrind`)

**Status**: Dev-only dependency used for benchmarks

**Details**:
- The `bincode` crate has a security advisory
- It's only used via `iai-callgrind`, which is a dev dependency for benchmarks
- Not included in release binaries

**Risk Level**: Low
- Dev-only dependency (not in production code)
- Used only for benchmark serialization
- Not exposed to untrusted input

**Action Items**:
- Monitor `iai-callgrind` updates for a version that uses a patched `bincode`
- Check periodically if `iai-callgrind` has switched to a different serialization library
- Remove this advisory ignore from:
  - `deny.toml` in the `[advisories]` ignore list
  - `.github/workflows/security.yml` in the `cargo audit` command

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0141
- iai-callgrind repository: https://github.com/iai-callgrind/iai-callgrind

### RUSTSEC-2026-0173 - `proc-macro-error2` (via `iai-callgrind`)

**Status**: Dev-only dependency used for benchmarks; unmaintained, no safe upgrade

**Details**:
- Published 2026-06-07. The `proc-macro-error2` author confirmed the crate is unmaintained and recommends migrating away. No patched version exists (`patched = []`).
- It's only used via `iai-callgrind` -> `iai-callgrind-macros`, which is a dev dependency for benchmarks.
- Not included in release binaries.

**Risk Level**: Low
- Dev-only dependency (not in production code)
- Unmaintained advisory only - no known exploitable vulnerability
- Not exposed to untrusted input

**Action Items**:
- Monitor `iai-callgrind` updates for a version that drops `proc-macro-error2` (e.g. migrates to `manyhow` / `proc-macro2-diagnostics`)
- Remove this advisory ignore from:
  - `deny.toml` in the `[advisories]` ignore list
  - `.github/workflows/security.yml` in the `cargo audit` command

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0173
- Announcement: https://github.com/GnomedDev/proc-macro-error-2/issues/17

## Review Schedule

These advisories should be reviewed:

1. **Before each release** as part of the [Pre-release Checks](RELEASING.md#pre-release-checks) (highest priority)
2. **When running `cargo update`** to check if dependencies have been updated (opportunistic)
3. **Monthly** as part of the [Monthly Review](../knowledge-base/MONTHLY-REVIEW.md) process (regular cadence)

### Review Process

To review these advisories:

```bash
# Update dependencies
cargo update

# Run cargo audit without ignores to see current status
cargo audit

# Run cargo deny to check advisories (validates deny.toml ignores)
cargo deny check advisories

# Check if any of the ignored advisories have been resolved
cargo tree -i bincode -e dev              # Check if iai-callgrind still depends on bincode
cargo tree -i proc-macro-error2 -e dev    # Check if iai-callgrind still depends on proc-macro-error2

# If a dependency has been updated and no longer triggers the advisory:
# 1. Remove the advisory ID from deny.toml [advisories] ignore list
# 2. Remove the --ignore flag from .github/workflows/security.yml
# 3. Update this document to mark the advisory as resolved
# 4. Verify cargo audit and cargo deny still enforce the same ignore set
# 5. Close or update the related tracking issue
```

## Adding New Advisory Ignores

If a new advisory needs to be temporarily ignored:

1. **Document the reason** in this file with:
   - Advisory ID and affected crate
   - Why it's being ignored (waiting for upstream fix, low risk, etc.)
   - Risk assessment
   - Clear action items for removal
   - Reference links

2. **Update `deny.toml`**:
   - Add the advisory ID to the `[advisories] ignore` list
   - Add an inline comment explaining the ignore

3. **Update CI**:
   - Add `--ignore RUSTSEC-YYYY-NNNN` to the `cargo audit` command in `.github/workflows/security.yml`

4. **Create or update tracking issue** with the advisory details

5. **Set a reminder** to review the advisory in the next monthly review

### Template for New Advisory

Copy this template when adding a new ignored advisory:

```markdown
### RUSTSEC-YYYY-NNNN - `crate-name` (via `parent-crate`)

**Status**: [One sentence describing current state]

**Details**:
- [Why is this advisory triggered?]
- [What is the dependency chain?]
- [What is the plan to resolve?]

**Risk Level**: [High/Medium/Low]
- [Justify the risk level]
- [Describe exposure/impact]
- [Note any mitigations]

**Action Items**:
- [What should be monitored?]
- [What triggers removal?]
- [Where to remove the ignore?]

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-YYYY-NNNN
- Upstream tracker: [link]
```

## Future Automation

The review process could be partially automated:
- A scheduled CI job could run the current `cargo tree -i ... -e dev` review commands weekly
- Results could be posted as a comment on the tracking issue
- Manual review would still be required to decide when to remove ignores

## Resolved Advisories

### RUSTSEC-2024-0384 - `instant`

**Resolved**: 2026-07-04

**Resolution**:
- `instant` is no longer present in `Cargo.lock`.
- The stale CI-only `cargo audit` ignore was removed.

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2024-0384

### RUSTSEC-2025-0067 - `libyml`

**Resolved**: 2026-07-04

**Resolution**:
- `libyml` is no longer present in `Cargo.lock`.
- The stale CI-only `cargo audit` ignore was removed.

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0067

### RUSTSEC-2025-0068 - `serde_yml`

**Resolved**: 2026-07-04

**Resolution**:
- `serde_yml` is no longer present in `Cargo.lock`.
- The stale CI-only `cargo audit` ignore was removed.

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2025-0068

### RUSTSEC-2026-0009 - `time`

**Resolved**: 2026-07-04

**Resolution**:
- `time` is no longer present in `Cargo.lock`.
- The advisory was ignored only in CI and was missing from this tracking document and `deny.toml`.
- The stale CI-only `cargo audit` ignore was removed instead of documenting an advisory that no longer applies.

**References**:
- Advisory: https://rustsec.org/advisories/RUSTSEC-2026-0009
