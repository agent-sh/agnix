#!/usr/bin/env bash
# Unit tests for scripts/check-glibc-floor.sh.
#
# Run:
#   bash scripts/check-glibc-floor.test.sh
#
# Drives the checker through AGNIX_GLIBC_REFS so every glibc floor scenario is
# covered without needing a binary built against that glibc. Guards the two
# things that would make the release gate useless: version-aware comparison
# (2.9 is older than 2.10, 2.4 is older than 2.39) and a non-zero exit when a
# binary sits above the floor.

set -uo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
checker="$script_dir/check-glibc-floor.sh"

pass=0
fail=0

# assert_check <expected: ok|fail> <name> <floor> <refs>
assert_check() {
    local expected="$1" name="$2" floor="$3" refs="$4" actual output

    if output=$(AGNIX_GLIBC_REFS="$refs" GLIBC_FLOOR="$floor" bash "$checker" /dev/null 2>&1); then
        actual="ok"
    else
        actual="fail"
    fi

    if [[ "$actual" == "$expected" ]]; then
        echo "  ok   - $name (expected $expected)"
        pass=$((pass + 1))
    else
        echo "  FAIL - $name (expected $expected, got $actual)"
        echo "         output: $output"
        fail=$((fail + 1))
    fi
}

echo "check-glibc-floor.sh:"

assert_check ok "all references below the floor" 2.31 "2.2.5 2.14 2.17 2.23"
assert_check ok "reference equal to the floor" 2.31 "2.17 2.31"
assert_check fail "reference above the floor" 2.31 "2.17 2.34"
assert_check fail "the v0.48.0 x86_64 reference set" 2.31 "2.2.5 2.17 2.34 2.39"
assert_check ok "the v0.48.0 aarch64 reference set" 2.31 "2.17 2.18"
assert_check fail "2.4 must not read as newer than 2.39" 2.4 "2.39"
assert_check ok "2.10 must read as newer than 2.9" 2.31 "2.9 2.10"
assert_check fail "2.9 must not read as newer than 2.10" 2.9 "2.10"
assert_check ok "no references at all (static musl)" 2.31 " "

# A named binary that cannot be read has to fail rather than pass silently, or
# a renamed release artifact would slip through the gate.
if bash "$checker" "$script_dir/does-not-exist" >/dev/null 2>&1; then
    echo "  FAIL - missing binary (expected fail, got ok)"
    fail=$((fail + 1))
else
    echo "  ok   - missing binary (expected fail)"
    pass=$((pass + 1))
fi

# No arguments is a usage error, not a vacuous pass.
bash "$checker" >/dev/null 2>&1
if [[ "$?" -eq 2 ]]; then
    echo "  ok   - no arguments exits 2 (expected fail)"
    pass=$((pass + 1))
else
    echo "  FAIL - no arguments must exit 2 with a usage message"
    fail=$((fail + 1))
fi

# A file that is not an ELF binary reports zero glibc references, which would
# otherwise look identical to a clean static build.
non_elf=$(mktemp)
printf '#!/bin/sh\necho not an elf\n' > "$non_elf"
if bash "$checker" "$non_elf" >/dev/null 2>&1; then
    echo "  FAIL - non-ELF file (expected fail, got ok)"
    fail=$((fail + 1))
else
    echo "  ok   - non-ELF file (expected fail)"
    pass=$((pass + 1))
fi
rm -f "$non_elf"

# Without readelf every binary would report zero references and pass, so the
# checker must refuse to run instead of green-lighting the release.
empty_path_dir=$(mktemp -d)
env -i "PATH=$empty_path_dir" /bin/bash "$checker" "$checker" >/dev/null 2>&1
if [[ "$?" -eq 2 ]]; then
    echo "  ok   - missing readelf exits 2 (expected fail)"
    pass=$((pass + 1))
else
    echo "  FAIL - missing readelf must exit 2 instead of passing"
    fail=$((fail + 1))
fi
rmdir "$empty_path_dir"

echo
echo "passed: $pass  failed: $fail"
[[ "$fail" -eq 0 ]]
