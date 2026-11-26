#!/usr/bin/env bash
# Exercise the Unix release asset contract without a Rust build.
set -euo pipefail
pack="$(cd "$(dirname "$0")" && pwd)/package-unix-lsp.py"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
cd "$stage"
cp /bin/sh agnix-lsp
binary="$PWD/agnix-lsp"
target=x86_64-unknown-linux-gnu
stem=agnix-lsp-$target
for build in one two; do
  mkdir "$build"
  (
    cd "$build"
    python3 "$pack" "$target" "$binary"
    shasum -a 256 -c "$stem.tar.gz.sha256"
    test "$(tar -tzf "$stem.tar.gz")" = agnix-lsp
    tar -xzf "$stem.tar.gz"
    test -x agnix-lsp
    cmp "$binary" agnix-lsp
    shasum -a 256 -c "$stem.sha256"
    if python3 "$pack" "$target" "$binary"; then exit 1; fi
  )
done
cmp "one/$stem.tar.gz" "two/$stem.tar.gz"
