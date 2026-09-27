#!/usr/bin/env bash
# Read-only local checks, ordered to report inexpensive failures before builds.
set -euo pipefail

usage() {
  echo 'Usage: bash scripts/preflight.sh [--quick|--full]'
  echo 'Default: --quick (format, bookkeeping, counts, locales, schema copies).'
  echo '--full also runs Clippy, workspace tests, eval, Kiro, packaging tests,'
  echo 'generated schema comparison, and agent configuration validation.'
}

mode=${1:---quick}
if [[ $# -gt 1 ]]; then usage >&2; exit 2; fi
case "$mode" in
  --help|-h) usage; exit 0 ;;
  --quick|--full) ;;
  *) usage >&2; exit 2 ;;
esac

cd "$(dirname "${BASH_SOURCE[0]}")/.."
for tool in cargo node python3 cmp; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[FAIL] Missing prerequisite: $tool" >&2
    exit 1
  fi
done

# Conservative local defaults; explicit developer settings still take precedence.
export CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-2}
export RUST_TEST_THREADS=${RUST_TEST_THREADS:-2}
export RAYON_NUM_THREADS=${RAYON_NUM_THREADS:-2}
SECONDS=0
run() {
  local label=$1 started=$SECONDS
  shift
  echo "[RUN] $label"
  if "$@"; then
    echo "[PASS] $label ($((SECONDS - started))s; elapsed ${SECONDS}s)"
  else
    local status=$?
    echo "[FAIL] $label ($((SECONDS - started))s; elapsed ${SECONDS}s)" >&2
    exit "$status"
  fi
}

run 'Format' cargo fmt --check --all
run 'Rule bookkeeping' node scripts/sync-rule-bookkeeping.js --check --skip-docs
run 'Rule count tables' python3 scripts/check-rule-counts.py
run 'Locale copies' bash scripts/check-locale-sync.sh
run 'Canonical agent instructions' bash -c '[[ -f AGENTS.md && ! -L AGENTS.md && -s AGENTS.md && ! -e CLAUDE.md && ! -L CLAUDE.md ]]'
run 'Schema copies' cmp schemas/agnix.json editors/vscode/schemas/agnix.json

if [[ "$mode" == --quick ]]; then
  echo "[PASS] Quick preflight (${SECONDS}s). Run --full before pushing."
  exit 0
fi

run 'Preflight regression tests' node --test scripts/preflight.test.js
run 'Clippy' cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
run 'Workspace tests' cargo test --locked --workspace
run 'Rule efficacy eval' cargo run --locked -p agnix-cli --bin agnix -- eval tests/eval.yaml
run 'Kiro S-tier gate' cargo test --locked -p agnix-cli --test kiro_ci_gate -- --include-ignored
run 'glibc floor tests' bash scripts/check-glibc-floor.test.sh
run 'npm installer tests' node npm/test/install-helpers.test.js
run 'PyPI wheel tests' python3 -m unittest discover -s pypi/test
run 'GLM extraction tests' node --test scripts/glm-extract.test.js

# Generate outside the checkout so validation never repairs or overwrites edits.
schema=$(mktemp)
trap 'rm -f "$schema"' EXIT
run 'Generate schema' cargo run --locked -p agnix-cli --bin agnix -- schema --output "$schema"
run 'Generated schema matches' cmp "$schema" schemas/agnix.json
run 'Agent configurations' cargo run --locked -p agnix-cli --bin agnix -- . --config .agnix.toml
echo "[PASS] Full local preflight (${SECONDS}s). Hosted platform/security/review gates still apply."
