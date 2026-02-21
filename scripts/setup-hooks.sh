#!/bin/sh
# Setup git hooks for agent-sh repos
# Detects project type and installs the appropriate pre-push hook
#
# Usage: sh scripts/setup-hooks.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Verify we are inside a git repository
if ! git -C "$REPO_ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  echo "[ERROR] Not a git repository: $REPO_ROOT"
  exit 1
fi

# Resolve hooks directory (handles worktrees where .git is a file)
if [ -f "$REPO_ROOT/.git" ]; then
  COMMON_DIR="$(git -C "$REPO_ROOT" rev-parse --git-common-dir)"
  HOOKS_DIR="$COMMON_DIR/hooks"
else
  HOOKS_DIR="$REPO_ROOT/.git/hooks"
fi

mkdir -p "$HOOKS_DIR"

# Detect project type and install hook
if [ -f "$REPO_ROOT/Cargo.toml" ]; then
  echo "[INFO] Detected Rust project"
  if [ ! -f "$SCRIPT_DIR/pre-push-rust" ]; then
    echo "[ERROR] Missing hook script: $SCRIPT_DIR/pre-push-rust"
    exit 1
  fi
  cp "$SCRIPT_DIR/pre-push-rust" "$HOOKS_DIR/pre-push"
  chmod +x "$HOOKS_DIR/pre-push"
  echo "[OK] Installed pre-push hook (Rust)"
elif [ -f "$REPO_ROOT/package.json" ]; then
  echo "[INFO] Detected Node.js project"
  if [ ! -f "$SCRIPT_DIR/pre-push-node" ]; then
    echo "[ERROR] Missing hook script: $SCRIPT_DIR/pre-push-node"
    exit 1
  fi
  cp "$SCRIPT_DIR/pre-push-node" "$HOOKS_DIR/pre-push"
  chmod +x "$HOOKS_DIR/pre-push"
  echo "[OK] Installed pre-push hook (Node.js)"
else
  echo "[WARN] No package.json or Cargo.toml found - skipping hook install"
  exit 0
fi
