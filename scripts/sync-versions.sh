#!/usr/bin/env bash
# Sync version from Cargo.toml to ALL package manifests.
# Run before tagging a release or as part of CI.
#
# Accepts optional argument to override version:
#   ./sync-versions.sh 0.9.0

set -euo pipefail

if [ -n "${1:-}" ]; then
  VERSION="$1"
else
  # Read the [workspace.package] version, not the first `version = ` in the file.
  # The root [package] (agnix-workspace-tests) is pinned at 0.0.0 and appears
  # first, so a naive `head -1` would pick that up. Pass an explicit version arg
  # to skip this (release.yml does: `sync-versions.sh "$TAG_VERSION"`).
  VERSION=$(awk '/^\[workspace\.package\]/{f=1; next} /^\[/{f=0} f && /^version = /{gsub(/version = "|"/, ""); print; exit}' Cargo.toml)
fi

if [ -z "$VERSION" ]; then
  echo "Error: Could not extract version from [workspace.package] in Cargo.toml"
  exit 1
fi

echo "Syncing all manifests to version: $VERSION"

# VS Code extension
if [ -f editors/vscode/package.json ]; then
  sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" editors/vscode/package.json
  rm -f editors/vscode/package.json.bak
  echo "  editors/vscode/package.json -> $VERSION"
fi

# VS Code package-lock.json (only update root-level version fields, not dependency versions)
if [ -f editors/vscode/package-lock.json ]; then
  if command -v jq &>/dev/null; then
    jq --arg v "$VERSION" '.version = $v | .packages[""].version = $v' editors/vscode/package-lock.json > editors/vscode/package-lock.json.tmp
    mv editors/vscode/package-lock.json.tmp editors/vscode/package-lock.json
  else
    # Fallback: only replace the first two occurrences (root "version" fields)
    python3 -c "
import json, sys
with open('editors/vscode/package-lock.json') as f:
    data = json.load(f)
data['version'] = '$VERSION'
if '' in data.get('packages', {}):
    data['packages']['']['version'] = '$VERSION'
with open('editors/vscode/package-lock.json', 'w') as f:
    json.dump(data, f, indent=2)
    f.write('\n')
"
  fi
  echo "  editors/vscode/package-lock.json -> $VERSION"
fi

# npm package
if [ -f npm/package.json ]; then
  sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" npm/package.json
  rm -f npm/package.json.bak
  echo "  npm/package.json -> $VERSION"
fi

# JetBrains plugin
if [ -f editors/jetbrains/gradle.properties ]; then
  sed -i.bak "s/pluginVersion = .*/pluginVersion = $VERSION/" editors/jetbrains/gradle.properties
  rm -f editors/jetbrains/gradle.properties.bak
  echo "  editors/jetbrains/gradle.properties -> $VERSION"
fi

# Zed extension
if [ -f editors/zed/extension.toml ]; then
  sed -i.bak "s/^version = \"[^\"]*\"/version = \"$VERSION\"/" editors/zed/extension.toml
  rm -f editors/zed/extension.toml.bak
  echo "  editors/zed/extension.toml -> $VERSION"
fi

# Claude Code plugin manifest
if [ -f plugin/.claude-plugin/plugin.json ]; then
  if command -v jq &>/dev/null; then
    jq --arg v "$VERSION" '.version = $v' plugin/.claude-plugin/plugin.json > plugin/.claude-plugin/plugin.json.tmp
    mv plugin/.claude-plugin/plugin.json.tmp plugin/.claude-plugin/plugin.json
  else
    sed -i.bak "s/\"version\": \"[^\"]*\"/\"version\": \"$VERSION\"/" plugin/.claude-plugin/plugin.json
    rm -f plugin/.claude-plugin/plugin.json.bak
  fi
  echo "  plugin/.claude-plugin/plugin.json -> $VERSION"
fi

echo "Done. All manifests synced to $VERSION"
