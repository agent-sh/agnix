#!/usr/bin/env bash
# Verify that locale files in each crate match the root locales/ directory.
# The i18n!() macro loads from crate-local locales/, so they must stay in sync
# with the canonical copies under the workspace root.

set -euo pipefail

ROOT_LOCALES="locales"
# Explicit floor: these three MUST have a locales/ directory, so a deleted one
# is a failure rather than something a glob silently stops enumerating.
CRATES=("crates/agnix-core" "crates/agnix-cli" "crates/agnix-lsp")

# Discovery pass on top of the floor: a new crate that gains locales/ is checked
# without editing this list. Union, so the floor keeps its failure mode.
for discovered in crates/*/locales; do
  [ -d "$discovered" ] || continue
  crate_dir="${discovered%/locales}"
  for known in "${CRATES[@]}"; do
    [ "$known" = "$crate_dir" ] && continue 2
  done
  CRATES+=("$crate_dir")
done

# Dynamically discover locale files from root directory
# rust-i18n loads YAML, JSON and TOML ("Use YAML (default), JSON or TOML format
# for mapping localized text"), so restricting to *.yml would let a stray
# fr.json in a crate be loaded by the macro while staying invisible here -
# the same blind spot the reverse pass below exists to close.
# docs/TRANSLATING.md mandates .yml; these extensions are checked so a
# deviation fails loudly instead of silently.
LOCALE_GLOBS=("*.yml" "*.yaml" "*.json" "*.toml")

LOCALE_FILES=()
# Root-side convention deviations, counted here so the rename advice is
# reachable from this direction too. Previously only the reverse (crate-side)
# pass counted them, so a root `zz.json` fell through to the copy hint - and
# `cp locales/*.yml` cannot move it.
root_non_yml=0
for glob in "${LOCALE_GLOBS[@]}"; do
  for f in "${ROOT_LOCALES}"/$glob; do
    [ -f "$f" ] || continue
    case "$f" in
      *.yml) ;;
      *)
        # Counted but NOT enumerated: adding it to LOCALE_FILES would make the
        # forward loop demand a copy in every crate, i.e. instruct the
        # contributor to propagate the very deviation these globs exist to
        # catch. The footer's advice is to rename it.
        root_non_yml=$((root_non_yml + 1))
        continue
        ;;
    esac
    LOCALE_FILES+=("$(basename "$f")")
  done
done

if [ ${#LOCALE_FILES[@]} -eq 0 ]; then
  echo "FAIL: No locale files (${LOCALE_GLOBS[*]}) found in ${ROOT_LOCALES}/"
  exit 1
fi

errors=0
if [ "$root_non_yml" -gt 0 ]; then
  echo "FAIL: ${ROOT_LOCALES}/ contains ${root_non_yml} locale file(s) not using the .yml extension"
  errors=$((errors + root_non_yml))
fi
# Counted separately: a crate-only file needs the opposite copy direction.
missing_from_root=0
# Counted separately: a non-.yml locale needs a rename, not a copy.
non_yml=0

for crate in "${CRATES[@]}"; do
  crate_locales="${crate}/locales"

  if [ ! -d "$crate_locales" ]; then
    echo "FAIL: ${crate_locales}/ directory missing"
    errors=$((errors + 1))
    continue
  fi

  for file in "${LOCALE_FILES[@]}"; do
    root_file="${ROOT_LOCALES}/${file}"
    crate_file="${crate_locales}/${file}"

    if [ ! -f "$crate_file" ]; then
      echo "FAIL: ${crate_file} missing (expected copy of ${root_file})"
      errors=$((errors + 1))
    elif ! diff -q "$root_file" "$crate_file" > /dev/null 2>&1; then
      echo "FAIL: ${crate_file} differs from ${root_file}"
      diff --unified=3 "$root_file" "$crate_file" || true
      errors=$((errors + 1))
    fi
  done

  # Reverse direction: a locale added to a crate but never to the root would
  # otherwise be invisible, since the loop above only walks the root's files.
  # rust_i18n loads from the crate copy, so that file WOULD ship while the
  # canonical root copy silently lacked it.
  for glob in "${LOCALE_GLOBS[@]}"; do
    for crate_file in "${crate_locales}"/$glob; do
      [ -f "$crate_file" ] || continue
      file="$(basename "$crate_file")"
      if [ ! -f "${ROOT_LOCALES}/${file}" ]; then
        echo "FAIL: ${crate_file} has no counterpart in ${ROOT_LOCALES}/ (add it there too; the root copy is canonical)"
        errors=$((errors + 1))
        missing_from_root=$((missing_from_root + 1))
        case "$crate_file" in
          *.yml) ;;
          *) non_yml=$((non_yml + 1)) ;;
        esac
      fi
    done
  done
done

if [ "$errors" -gt 0 ]; then
  echo ""
  echo "${errors} locale file(s) out of sync."

  # Derived from CRATES, not hardcoded: the discovery pass can add a crate, and
  # a literal list here would silently exclude it from the advice while the gate
  # still checked it.
  crate_targets=""
  for crate in "${CRATES[@]}"; do
    crate_targets="${crate_targets} ${crate}/locales/"
  done

  # Each applicable class prints its own step rather than one exclusive branch:
  # a tree can trip several at once, and an exclusive chain left the mixed case
  # with no runnable command.
  #
  # A non-.yml locale is a convention violation, not a sync problem - telling
  # the contributor to propagate it would spread the deviation. Counted from
  # both directions, since a root-side `zz.json` is just as unfixable by
  # `cp locales/*.yml` as a crate-side one. See docs/TRANSLATING.md.
  if [ $((non_yml + root_non_yml)) -gt 0 ]; then
    echo "A locale file uses an extension other than .yml. Rename it, e.g."
    echo "  mv <dir>/<locale>.json <dir>/<locale>.yml"
  fi

  # The outward copy cannot create a root file, so a crate-only locale needs the
  # inward copy first.
  if [ "$missing_from_root" -gt 0 ]; then
    echo "For a file present in a crate but not in ${ROOT_LOCALES}/, copy it inward first:"
    echo "  cp crates/<crate>/locales/<locale>.yml ${ROOT_LOCALES}/"
  fi

  # Always printed: the fan-out is the final step for every class above. Worded
  # as a standalone instruction when it is the only step, since plain forward
  # drift would otherwise open on "Then ..." with nothing before it.
  if [ $((non_yml + root_non_yml + missing_from_root)) -gt 0 ]; then
    echo "Then fan the canonical copies out to every crate:"
  else
    echo "Fan the canonical copies out to every crate:"
  fi
  echo "  for d in${crate_targets}; do cp ${ROOT_LOCALES}/*.yml \"\$d\"; done"
  exit 1
fi

echo "All locale files in sync."
