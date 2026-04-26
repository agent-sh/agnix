#!/usr/bin/env bash
# Polls release feeds for every tracked tool in .github/tool-release-baselines.json
# and opens (or comments on) a per-tool issue when a new release is detected.
#
# Per-tool source type is declared in the baselines file:
#   - github_repo                       -> GitHub releases API (latest stable, tag fallback)
#   - html_url + version_regex          -> HTTP GET + first regex match (HTML/JSON/RSS)
# Optional release-notes upgrade (when [NEW] fires):
#   - notes_extractor: "glm"            -> GLM chat completion via scripts/glm-extract.js
#   - notes_extractor: "rss_cdata"      -> first <item><description> CDATA via python3
#   - notes_extractor: "stub" (default) -> link-only body
#
# Environment:
#   GH_TOKEN            - required (passed implicitly by gh CLI)
#   GLM_API_KEY         - optional; enables notes_extractor=glm tools
#   UPDATE_BASELINES    - "true" to skip issue creation and emit JSON snippet only
#   TOOL_FILTER         - optional: limit run to one tool id from the baselines file
#   GITHUB_REPOSITORY   - owner/repo for absolute-URL construction (Actions sets this);
#                         falls back to agent-sh/agnix when unset
#   GITHUB_STEP_SUMMARY - optional: workflow summary file (set by GitHub Actions)
#   RUN_URL             - optional: URL to include in issue footers
#
# Exit codes:
#   0 - completed successfully (whether or not new releases were found)
#   1 - configuration error (invalid baselines file, unknown tool filter, etc.)

set -euo pipefail

BASELINES_FILE="${BASELINES_FILE:-.github/tool-release-baselines.json}"
GENERAL_LABEL="tool-release"
ISSUE_BODY_LIMIT=50000  # GitHub issue body cap is 65535; leave room for header/footer
UPDATE_BASELINES="${UPDATE_BASELINES:-false}"
TOOL_FILTER="${TOOL_FILTER:-}"
RUN_URL="${RUN_URL:-}"

if ! jq -e '.tools | type == "object"' "$BASELINES_FILE" > /dev/null; then
  echo "ERROR: invalid $BASELINES_FILE (.tools must be an object)" >&2
  exit 1
fi

# Ensure the general label exists (per-tool labels created on demand)
if [[ "$UPDATE_BASELINES" != "true" ]]; then
  gh label create "$GENERAL_LABEL" --description "New release detected for a supported tool" --color "1F77B4" 2>/dev/null || true
fi

# Resolve which tools to check
if [[ -n "$TOOL_FILTER" ]]; then
  if ! jq -e --arg t "$TOOL_FILTER" '.tools | has($t)' "$BASELINES_FILE" > /dev/null; then
    echo "ERROR: tool filter '$TOOL_FILTER' does not match any key in $BASELINES_FILE" >&2
    exit 1
  fi
  TOOL_IDS=("$TOOL_FILTER")
else
  mapfile -t TOOL_IDS < <(jq -r '.tools | keys[]' "$BASELINES_FILE")
fi

UPDATES_JSON='[]'
NEW_COUNT=0
SKIP_COUNT=0
OK_COUNT=0

for raw_id in "${TOOL_IDS[@]}"; do
  tool_id="${raw_id%$'\r'}"  # strip stray CR (cross-platform tolerance)
  [[ -z "$tool_id" ]] && continue

  tracked=$(jq -r --arg id "$tool_id" '.tools[$id].tracked' "$BASELINES_FILE")
  display_name=$(jq -r --arg id "$tool_id" '.tools[$id].display_name' "$BASELINES_FILE")
  tier=$(jq -r --arg id "$tool_id" '.tools[$id].tier' "$BASELINES_FILE")

  if [[ "$tracked" != "true" ]]; then
    reason=$(jq -r --arg id "$tool_id" '.tools[$id].untracked_reason // "untracked"' "$BASELINES_FILE")
    echo "[skip] $tool_id (tier=$tier): $reason"
    SKIP_COUNT=$((SKIP_COUNT+1))
    continue
  fi

  repo=$(jq -r --arg id "$tool_id" '.tools[$id].github_repo // ""' "$BASELINES_FILE")
  html_url=$(jq -r --arg id "$tool_id" '.tools[$id].html_url // ""' "$BASELINES_FILE")
  version_regex=$(jq -r --arg id "$tool_id" '.tools[$id].version_regex // ""' "$BASELINES_FILE")
  baseline_version=$(jq -r --arg id "$tool_id" '.tools[$id].last_known_version' "$BASELINES_FILE")
  tool_label="tool-release:$tool_id"

  if [[ -n "$repo" ]]; then
    # GitHub releases path
    echo "[check] $tool_id (tier=$tier) repo=$repo baseline=$baseline_version"
    release_json=$(gh api "repos/$repo/releases/latest" 2>/dev/null || echo "")

    if [[ -z "$release_json" ]]; then
      latest_tag=$(gh api "repos/$repo/tags" --jq '.[0].name // empty' 2>/dev/null || echo "")
      if [[ -z "$latest_tag" ]]; then
        echo "  WARN: no releases or tags found for $repo (will retry on next run)"
        continue
      fi
      latest_version="$latest_tag"
      release_url="https://github.com/$repo/releases/tag/$latest_tag"
      published_at=""
      release_body="_No release notes available - this version was detected via the tags API only._"
    else
      latest_version=$(echo "$release_json" | jq -r '.tag_name // empty')
      release_url=$(echo "$release_json" | jq -r '.html_url // empty')
      published_at=$(echo "$release_json" | jq -r '.published_at // ""')
      release_body=$(echo "$release_json" | jq -r '.body // ""')
    fi
  elif [[ -n "$html_url" && -n "$version_regex" ]]; then
    # HTML scrape path - fetch page and extract first regex match
    echo "[check] $tool_id (tier=$tier) html=$html_url baseline=$baseline_version"
    page_content=$(curl -sL --max-time 30 "$html_url" 2>/dev/null || echo "")
    if [[ -z "$page_content" ]]; then
      echo "  WARN: failed to fetch $html_url (will retry on next run)"
      continue
    fi
    # `|| true` rescues the pipeline when grep matches nothing (exit 1 under
    # pipefail would abort the whole script before the WARN/continue below).
    latest_version=$(echo "$page_content" | grep -oE "$version_regex" | head -1 || true)
    if [[ -z "$latest_version" ]]; then
      echo "  WARN: regex '$version_regex' matched nothing on $html_url (page format may have changed - update version_regex in baselines)"
      continue
    fi
    # If the regex captured a URL (e.g., RSS item link), promote it to release_url and use
    # the trailing path segment as the human-readable version label.
    if [[ "$latest_version" =~ ^https?:// ]]; then
      release_url="$latest_version"
      latest_version="${latest_version##*/}"
    else
      release_url="$html_url"
    fi
    published_at=""
    # Default body for the html_url branch. Tools with notes_extractor=glm or
    # rss_cdata replace this below with extracted content. The notes_extractor=stub
    # tools (e.g. cursor's JSON update endpoint) keep this neutral fallback - no
    # claim that the source is unscrapable, since some sources are intentionally
    # link-only.
    release_body="_This source provides a version marker only. See [$release_url]($release_url) for the full release notes._"
  else
    echo "  ERROR: $tool_id is marked tracked but has neither github_repo nor (html_url + version_regex)" >&2
    continue
  fi

  if [[ -z "$latest_version" ]]; then
    echo "  WARN: could not determine latest version for $tool_id"
    continue
  fi

  UPDATES_JSON=$(echo "$UPDATES_JSON" | jq --arg id "$tool_id" --arg v "$latest_version" '. += [{"id": $id, "version": $v}]')

  if [[ "$latest_version" == "$baseline_version" ]]; then
    echo "  [ok] $latest_version matches baseline"
    OK_COUNT=$((OK_COUNT+1))
    continue
  fi

  echo "  [NEW] $latest_version (was $baseline_version)"
  NEW_COUNT=$((NEW_COUNT+1))

  # Script directory - used by notes_extractor=glm (to find glm-extract.js)
  # and by the later agnix-triage step. Resolve to an absolute path so
  # `node "$script_dir/glm-extract.js"` works even when this script is
  # invoked via a relative path from another working directory.
  script_dir=$(cd "$(dirname "$0")" && pwd)

  # Optional release-notes upgrade: replace the stub release_body with extracted
  # content when the tool defines a notes_extractor (glm | rss_cdata). Failures
  # log a warning and fall back to whatever release_body was already set to.
  notes_extractor=$(jq -r --arg id "$tool_id" '.tools[$id].notes_extractor // "stub"' "$BASELINES_FILE")
  case "$notes_extractor" in
    glm)
      if ! command -v node >/dev/null 2>&1; then
        echo "  WARN: notes_extractor=glm but node is not on PATH - using stub"
      elif [[ -z "${GLM_API_KEY:-}" ]]; then
        echo "  WARN: notes_extractor=glm but GLM_API_KEY env var is unset - using stub"
      elif [[ -z "${page_content:-}" ]]; then
        echo "  WARN: notes_extractor=glm but no page_content captured - using stub"
      else
        glm_stderr=$(mktemp)
        # Here-string instead of `printf|cmd` so large $page_content does not
        # hit shell ARG_MAX during pipeline expansion.
        extracted=$(node "$script_dir/glm-extract.js" "$display_name" "$latest_version" "$release_url" 2>"$glm_stderr" <<< "$page_content" || true)
        if [[ -n "$extracted" ]]; then
          release_body="${extracted}"$'\n\n---\n*Notes auto-extracted via GLM from ['"$release_url"']('"$release_url"').*'
          echo "  [glm] extracted $(echo "$extracted" | wc -c) chars of release notes"
        else
          echo "  WARN: GLM extraction returned empty (stderr: $(head -1 "$glm_stderr" 2>/dev/null)) - using stub"
        fi
        rm -f "$glm_stderr"
      fi
      ;;
    rss_cdata)
      if ! command -v python3 >/dev/null 2>&1; then
        echo "  WARN: notes_extractor=rss_cdata but python3 is not on PATH - using stub"
      elif [[ -z "${page_content:-}" ]]; then
        echo "  WARN: notes_extractor=rss_cdata but no page_content captured - using stub"
      else
        # Here-string keeps large RSS payloads out of the command line.
        extracted=$(python3 -c '
import re, sys
content = sys.stdin.read()
m = re.search(r"<item>.*?<description>\s*<!\[CDATA\[(.*?)\]\]>\s*</description>", content, re.DOTALL)
sys.stdout.write(m.group(1).strip() if m else "")
' <<< "$page_content")
        if [[ -n "$extracted" ]]; then
          # Attribute to html_url (the RSS feed - where extraction happened),
          # not release_url (the per-article URL that the version_regex pulled
          # out and rewrote earlier). The two differ for amp.
          release_body="${extracted}"$'\n\n---\n*Notes extracted from the first `<item>` description in ['"$html_url"']('"$html_url"').*'
          echo "  [rss_cdata] extracted $(echo "$extracted" | wc -c) chars of release notes"
        else
          echo "  WARN: rss_cdata regex matched nothing in $html_url - using stub"
        fi
      fi
      ;;
    stub|*)
      :  # leave existing release_body untouched
      ;;
  esac

  # Optional agnix-focused triage: when the tool declares changes_of_interest
  # and GLM_API_KEY is available, run glm-extract.js --mode=agnix-triage on
  # the release notes. The LLM filters changes down to validator-relevant
  # items + rule candidates. Result is prepended as ## Agnix Triage so a
  # human reviewer sees the filtered summary first and the full changelog
  # below. On any failure (missing key, empty result, HTTP error), behavior
  # is identical to before this flag existed.
  #
  # Runs BEFORE truncation so the LLM sees the complete changelog, not a
  # half-sentence stub. Skips when release_body is itself a stub link (no
  # meaningful content to classify) to avoid wasting GLM quota.
  agnix_triage=""
  interests_json=$(jq -c --arg id "$tool_id" '.tools[$id].changes_of_interest // empty' "$BASELINES_FILE")
  if [[ -n "$interests_json" ]]; then
    # Skip triage on stub-shaped bodies (the fallback shapes in the
    # html_url branch and the tags-fallback branch both produce
    # marker-only text that LLM has nothing to filter).
    is_stub_body=false
    if [[ "$release_body" == *"This source provides a version marker only"* ]] \
       || [[ "$release_body" == *"No release notes available"* ]]; then
      is_stub_body=true
    fi

    if ! command -v node >/dev/null 2>&1; then
      echo "  [triage] changes_of_interest set but node is not on PATH - skipping LLM triage"
    elif [[ -z "${GLM_API_KEY:-}" ]]; then
      echo "  [triage] changes_of_interest set but GLM_API_KEY env var is unset - skipping LLM triage"
    elif [[ -z "${release_body:-}" ]]; then
      echo "  [triage] no release_body to triage - skipping"
    elif [[ "$is_stub_body" == "true" ]]; then
      echo "  [triage] release_body is a stub marker - nothing to classify, skipping LLM call"
    else
      # Use a trap to guarantee cleanup of temp files even if the command
      # below fails mid-way (CI interruption, SIGTERM, etc.).
      interests_tmp=$(mktemp)
      triage_stderr=$(mktemp)
      # shellcheck disable=SC2064  # intentional early expansion of tmp paths
      trap "rm -f '$interests_tmp' '$triage_stderr'" EXIT INT TERM
      printf '%s' "$interests_json" > "$interests_tmp"
      triage_out=$(node "$script_dir/glm-extract.js" \
        "--mode=agnix-triage" \
        "--interests-json=$interests_tmp" \
        "$display_name" "$latest_version" "$release_url" \
        2>"$triage_stderr" <<< "$release_body" || true)
      if [[ -n "$triage_out" ]]; then
        agnix_triage="$triage_out"
        echo "  [triage] produced $(echo "$triage_out" | wc -c) chars of agnix-focused summary"
      else
        echo "  WARN: GLM triage returned empty (stderr: $(head -1 "$triage_stderr" 2>/dev/null)) - posting raw changelog only"
      fi
      rm -f "$interests_tmp" "$triage_stderr"
      trap - EXIT INT TERM
    fi
  fi

  # Truncate release notes if too long. When triage produced content, share
  # the ISSUE_BODY_LIMIT budget between triage and raw body so the composed
  # issue fits under GitHub's ~65KB issue-body cap. The triage summary is
  # the more important view; budget it first and fit raw body into the
  # remainder.
  if [[ -n "$agnix_triage" ]]; then
    triage_limit=$((ISSUE_BODY_LIMIT / 2))
    if [[ ${#agnix_triage} -gt $triage_limit ]]; then
      agnix_triage="${agnix_triage:0:$triage_limit}"$'\n\n'"_(triage truncated)_"
    fi
    remaining=$((ISSUE_BODY_LIMIT - ${#agnix_triage}))
    if [[ ${#release_body} -gt $remaining ]]; then
      release_body="${release_body:0:$remaining}"$'\n\n'"_(release notes truncated to fit; see ${release_url} for the full text)_"
    fi
  elif [[ ${#release_body} -gt $ISSUE_BODY_LIMIT ]]; then
    release_body="${release_body:0:$ISSUE_BODY_LIMIT}"$'\n\n'"_(release notes truncated at ${ISSUE_BODY_LIMIT} characters; see ${release_url} for the full text)_"
  fi

  # Compose issue body. Use absolute file links keyed off GITHUB_REPOSITORY so
  # they resolve correctly inside an issue (relative `../blob/main/...` resolves
  # against the issue URL and 404s).
  agnix_repo="${GITHUB_REPOSITORY:-agent-sh/agnix}"
  file_base="https://github.com/${agnix_repo}/blob/main"
  meta_line="**Tier**: \`$tier\` &nbsp;&nbsp; **Previous baseline**: \`$baseline_version\`"
  if [[ -n "$published_at" ]]; then
    meta_line="$meta_line &nbsp;&nbsp; **Published**: $published_at"
  fi
  source_line="**Source**: $release_url"
  if [[ -n "$repo" ]]; then
    source_line="$source_line"$'\n'"**Repository**: https://github.com/$repo"
  fi
  if [[ -n "$agnix_triage" ]]; then
    # Triage succeeded: show the LLM summary first, collapse the raw
    # changelog under <details> so maintainers read filtered items up top
    # but still have the full text one click away for verification.
    release_section=$(cat <<RELEASE_NOTES
### Agnix Triage (auto-filtered)

$agnix_triage

<details><summary>Full upstream release notes</summary>

$release_body

</details>
RELEASE_NOTES
)
  else
    # No triage (tool has no changes_of_interest, or LLM failed). Keep
    # the original layout.
    release_section=$(cat <<RELEASE_NOTES
### Release notes

$release_body
RELEASE_NOTES
)
  fi

  issue_body=$(cat <<BODY
## $display_name $latest_version

$meta_line
$source_line

---

$release_section

---

### Action required

1. Review the release notes for changes that may affect agnix validation rules.
2. Update [\`crates/agnix-core/src/config.rs\`](${file_base}/crates/agnix-core/src/config.rs) (\`ToolVersions\` / \`SpecRevisions\`) if the new version changes a validated field.
3. Update [\`knowledge-base/RESEARCH-TRACKING.md\`](${file_base}/knowledge-base/RESEARCH-TRACKING.md) "Last Reviewed" for $display_name.
4. After triage, run the workflow with \`update_baselines: true\` (and optionally \`tool: $tool_id\`) and copy the JSON snippet from the job summary into [\`.github/tool-release-baselines.json\`](${file_base}/.github/tool-release-baselines.json).

---
*Auto-opened by \`.github/workflows/tool-release-watch.yml\`.${RUN_URL:+ Run: $RUN_URL}*
BODY
)

  if [[ "$UPDATE_BASELINES" == "true" ]]; then
    echo "  (baseline-update mode: not creating/commenting on issues)"
    continue
  fi

  gh label create "$tool_label" --description "New release for $display_name" --color "5319E7" 2>/dev/null || true

  existing_issue=$(gh issue list --state open --label "$tool_label" --json number --jq '.[0].number // empty')

  if [[ -n "$existing_issue" ]]; then
    echo "  Updating existing issue #$existing_issue"
    gh issue comment "$existing_issue" --body-file - <<< "$issue_body"
  else
    issue_title="Tool release: $display_name $latest_version (was $baseline_version)"
    echo "  Creating new issue: $issue_title"
    gh issue create \
      --title "$issue_title" \
      --label "$GENERAL_LABEL" \
      --label "$tool_label" \
      --body-file - <<< "$issue_body"
  fi
done

echo ""
echo "Summary: ok=$OK_COUNT new=$NEW_COUNT skipped=$SKIP_COUNT"

# Emit baseline updates to the job summary for easy copy/paste
if [[ "$UPDATE_BASELINES" == "true" && -n "${GITHUB_STEP_SUMMARY:-}" ]]; then
  {
    echo ""
    echo "## Suggested baseline updates"
    echo ""
    echo "Apply each \`last_known_version\` update below to \`.github/tool-release-baselines.json\` for any tool whose release issue is now closed:"
    echo ""
    echo '```json'
    echo "$UPDATES_JSON" | jq '.'
    echo '```'
  } >> "$GITHUB_STEP_SUMMARY"
fi
