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
for glob in "${LOCALE_GLOBS[@]}"; do
  for f in "${ROOT_LOCALES}"/$glob; do
    [ -f "$f" ] && LOCALE_FILES+=("$(basename "$f")")
  done
done

if [ ${#LOCALE_FILES[@]} -eq 0 ]; then
  echo "FAIL: No .yml files found in ${ROOT_LOCALES}/"
  exit 1
fi

errors=0
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
  # Direction matters: the outward copy cannot fix a crate-only file, since it
  # never creates the root copy. Printing only that hint sent a contributor who
  # tripped the reverse check into a no-op and identical output on re-run.
  # Derived from CRATES, not hardcoded: the discovery pass can add a crate, and
  # a literal list here would silently exclude it from the advice while the gate
  # still checked it.
  crate_targets=""
  for crate in "${CRATES[@]}"; do
    crate_targets="${crate_targets} ${crate}/locales/"
  done

  if [ "$non_yml" -gt 0 ]; then
    # A non-.yml locale is a convention violation, not a sync problem. Telling
    # the contributor to propagate it would spread the deviation; the fix is a
    # rename. See docs/TRANSLATING.md.
    echo "A locale file uses an extension other than .yml. Rename it, e.g."
    echo "  mv crates/<crate>/locales/<locale>.json crates/<crate>/locales/<locale>.yml"
    echo "then make sure ${ROOT_LOCALES}/ has the canonical copy and fan it out."
  elif [ "$missing_from_root" -gt 0 ]; then
    echo "For a file present in a crate but not in ${ROOT_LOCALES}/: copy it inward first,"
    echo "  cp crates/<crate>/locales/<locale>.yml ${ROOT_LOCALES}/"
    echo "then fan it back out to every crate:"
    echo "  for d in${crate_targets}; do cp ${ROOT_LOCALES}/*.yml \"\$d\"; done"
  else
    echo "Run:"
    echo "  for d in${crate_targets}; do cp ${ROOT_LOCALES}/*.yml \"\$d\"; done"
  fi
  exit 1
fi

echo "All locale files in sync."
