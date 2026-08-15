#!/usr/bin/env bash
# Fail if a released glibc-linked binary requires a newer glibc than the
# declared floor.
#
# Usage:
#   bash scripts/check-glibc-floor.sh <binary> [binary...]
#   GLIBC_FLOOR=2.28 bash scripts/check-glibc-floor.sh <binary>
#
# The floor exists because release binaries are consumed on distros far older
# than the GitHub runner image: Debian bookworm ships glibc 2.36, RHEL 9 and
# Amazon Linux 2023 ship 2.34, Ubuntu 22.04 ships 2.35. Building natively on
# ubuntu-latest pinned the floor to the runner's glibc and broke all of them
# (issue #1371), so the gnu targets build through `cross` and this check keeps
# a runner-image bump from silently raising the floor again.
#
# Only ELF binaries linked against glibc are meaningful here. Static musl
# builds have no version references at all and pass trivially.

set -euo pipefail

# Debian bullseye / Ubuntu 20.04. The cross 0.2.5 gnu images are older still
# (Ubuntu 16.04, glibc 2.23), so this leaves headroom for an image bump without
# giving up any distro the project supports.
GLIBC_FLOOR="${GLIBC_FLOOR:-2.31}"

if [ "$#" -eq 0 ]; then
    echo "Usage: $0 <binary> [binary...]" >&2
    exit 2
fi

# Without readelf every binary would report zero version references and pass,
# turning the release gate into a no-op.
if [ -z "${AGNIX_GLIBC_REFS:-}" ] && ! command -v readelf >/dev/null 2>&1; then
    echo "Error: readelf is required (install binutils)" >&2
    exit 2
fi

# Highest glibc version referenced by a binary, empty when none are.
# AGNIX_GLIBC_REFS overrides the extraction for the unit tests, which need to
# exercise the comparison without producing binaries for each glibc version.
glibc_refs() {
    local binary="$1"

    if [ -n "${AGNIX_GLIBC_REFS:-}" ]; then
        # Word splitting is the point: the override is a space-separated list.
        # shellcheck disable=SC2086
        printf '%s\n' ${AGNIX_GLIBC_REFS}
        return 0
    fi

    if [ ! -f "$binary" ]; then
        echo "Error: no such binary: $binary" >&2
        return 1
    fi

    # A non-ELF file also yields zero references, so reject it instead of
    # reporting it as a clean static build.
    if ! readelf -h "$binary" >/dev/null 2>&1; then
        echo "Error: not an ELF binary: $binary" >&2
        return 1
    fi

    readelf -V "$binary" 2>/dev/null | grep -o 'GLIBC_[0-9][0-9.]*' | sed 's/^GLIBC_//' || true
}

# 0 when $1 is newer than $2. sort -V orders 2.9 before 2.10, which a plain
# string or float comparison gets wrong.
version_gt() {
    [ "$1" != "$2" ] && [ "$(printf '%s\n%s\n' "$1" "$2" | sort -V | tail -1)" = "$1" ]
}

status=0

for binary in "$@"; do
    # Release archives skip binaries a target does not produce, so a missing
    # path is only an error when it was named explicitly and cannot be read.
    refs="$(glibc_refs "$binary")" || {
        status=1
        continue
    }

    if [ -z "$refs" ]; then
        echo "ok   $binary: no glibc version references (static or non-glibc)"
        continue
    fi

    highest="$(printf '%s\n' "$refs" | sort -uV | tail -1)"

    if version_gt "$highest" "$GLIBC_FLOOR"; then
        echo "FAIL $binary: requires GLIBC_$highest, floor is GLIBC_$GLIBC_FLOOR" >&2
        echo "     symbols above the floor:" >&2
        readelf --dyn-syms -W "$binary" 2>/dev/null |
            grep -oE '[A-Za-z_0-9]+@GLIBC_[0-9][0-9.]*' |
            sort -u |
            while IFS= read -r symbol; do
                version="${symbol##*@GLIBC_}"
                if version_gt "$version" "$GLIBC_FLOOR"; then
                    echo "       $symbol" >&2
                fi
            done
        status=1
    else
        echo "ok   $binary: requires GLIBC_$highest, floor is GLIBC_$GLIBC_FLOOR"
    fi
done

exit "$status"
