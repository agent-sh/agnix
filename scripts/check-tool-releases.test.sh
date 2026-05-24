#!/usr/bin/env bash
# Unit tests for the pure helpers in scripts/check-tool-releases.sh.
#
# Run:
#   bash scripts/check-tool-releases.test.sh
#
# Sources the script with CHECK_TOOL_RELEASES_LIB_ONLY=1 so only the helper
# functions load (the live release poll is skipped). Locks in the
# triage_has_relevant_changes contract: a daily release with no agnix-relevant
# bullets must be skipped, but anything with relevant bullets - or with no
# triage at all - must still open an issue. Regression guard for the
# "issue_body: unbound variable" crash that took the watcher down (the skip
# logic had been wedged inside the issue-body heredoc).

set -uo pipefail

script_dir=$(cd "$(dirname "$0")" && pwd)
CHECK_TOOL_RELEASES_LIB_ONLY=1 source "$script_dir/check-tool-releases.sh"

pass=0
fail=0

# assert_relevant <expected: open|skip> <name> <triage text>
assert_relevant() {
  local expected="$1" name="$2" triage="$3" actual
  if triage_has_relevant_changes "$triage"; then
    actual="open"
  else
    actual="skip"
  fi
  if [[ "$actual" == "$expected" ]]; then
    echo "  ok   - $name (expected $expected)"
    pass=$((pass + 1))
  else
    echo "  FAIL - $name (expected $expected, got $actual)"
    fail=$((fail + 1))
  fi
}

relevant_with_bullets=$(printf '%s\n' \
  '#### Agnix-relevant changes' \
  '- new MCP server field `transport` (matches: MCP server definition shape)' \
  '#### Irrelevant changes (not reviewed)' \
  '- perf improvements')

relevant_empty=$(printf '%s\n' \
  '#### Agnix-relevant changes' \
  '#### Irrelevant changes (not reviewed)' \
  '- perf improvements' \
  '- UI polish')

# Exact "nothing relevant" marker that scripts/glm-extract.js instructs GLM to emit.
relevant_says_none=$(printf '%s\n' \
  '#### Agnix-relevant changes' \
  '_None - this release is agnix-irrelevant._' \
  '#### Irrelevant changes (not reviewed)' \
  '5 items: model additions, UI polish, bug fixes')

relevant_is_last_section=$(printf '%s\n' \
  '#### Irrelevant changes (not reviewed)' \
  '- billing copy tweaks' \
  '#### Agnix-relevant changes' \
  '- hook event renamed (matches: Hook event names)')

assert_relevant open "bullets under relevant section"            "$relevant_with_bullets"
assert_relevant skip "relevant section empty"                    "$relevant_empty"
assert_relevant skip "relevant section says None"                "$relevant_says_none"
assert_relevant open "relevant section is last, has bullet"      "$relevant_is_last_section"
assert_relevant open "no triage ran (empty) - cannot rule out"   ""
assert_relevant open "triage without the relevant heading"       "free-form notes, no headings"

echo ""
echo "Summary: pass=$pass fail=$fail"
[[ "$fail" -eq 0 ]]
