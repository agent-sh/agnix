#!/usr/bin/env bash
# Polls GitHub releases for every tracked tool in .github/tool-release-baselines.json
# and opens (or comments on) a per-tool issue when a new release is detected.
#
# Environment:
#   GH_TOKEN          - required (passed implicitly by gh CLI)
#   UPDATE_BASELINES  - "true" to skip issue creation and emit the JSON snippet only
#   TOOL_FILTER       - optional: limit run to one tool id from the baselines file
#   GITHUB_STEP_SUMMARY - optional: workflow summary file (set by GitHub Actions)
#   RUN_URL           - optional: URL to include in issue footers
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
    latest_version=$(echo "$page_content" | grep -oE "$version_regex" | head -1)
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
    release_body="_Auto-detected via scrape of [$html_url]($html_url). See the linked page for full release notes - they are not machine-extractable from this source._"
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

  # Optional release-notes upgrade: replace the stub release_body with extracted
  # content when the tool defines a notes_extractor (glm | rss_cdata). Failures
  # log a warning and fall back to whatever release_body was already set to.
  notes_extractor=$(jq -r --arg id "$tool_id" '.tools[$id].notes_extractor // "stub"' "$BASELINES_FILE")
  case "$notes_extractor" in
    glm)
      script_dir=$(dirname "$0")
      if ! command -v node >/dev/null 2>&1; then
        echo "  WARN: notes_extractor=glm but node is not on PATH - using stub"
      elif [[ -z "${GLM_API_KEY:-}" ]]; then
        echo "  WARN: notes_extractor=glm but GLM_API_KEY env var is unset - using stub"
      elif [[ -z "${page_content:-}" ]]; then
        echo "  WARN: notes_extractor=glm but no page_content captured - using stub"
      else
        glm_stderr=$(mktemp)
        extracted=$(printf '%s' "$page_content" \
          | node "$script_dir/glm-extract.js" "$display_name" "$latest_version" "$release_url" 2>"$glm_stderr" || true)
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
        extracted=$(printf '%s' "$page_content" | python3 -c '
import re, sys
content = sys.stdin.read()
m = re.search(r"<item>.*?<description>\s*<!\[CDATA\[(.*?)\]\]>\s*</description>", content, re.DOTALL)
sys.stdout.write(m.group(1).strip() if m else "")
')
        if [[ -n "$extracted" ]]; then
          release_body="${extracted}"$'\n\n---\n*Notes extracted from the first `<item>` description in ['"$release_url"']('"$release_url"').*'
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

  # Truncate release notes if too long
  if [[ ${#release_body} -gt $ISSUE_BODY_LIMIT ]]; then
    release_body="${release_body:0:$ISSUE_BODY_LIMIT}"$'\n\n'"_(release notes truncated at ${ISSUE_BODY_LIMIT} characters; see ${release_url} for the full text)_"
  fi

  # Compose issue body
  issue_body=$(cat <<BODY
## $display_name $latest_version

**Tier**: \`$tier\` &nbsp;&nbsp; **Previous baseline**: \`$baseline_version\` &nbsp;&nbsp; **Published**: $published_at
**Release**: $release_url
**Repository**: https://github.com/$repo

---

### Release notes

$release_body

---

### Action required

1. Review the release notes for changes that may affect agnix validation rules.
2. Update [\`crates/agnix-core/src/config.rs\`](../blob/main/crates/agnix-core/src/config.rs) (\`ToolVersions\` / \`SpecRevisions\`) if the new version changes a validated field.
3. Update [\`knowledge-base/RESEARCH-TRACKING.md\`](../blob/main/knowledge-base/RESEARCH-TRACKING.md) "Last Reviewed" for $display_name.
4. After triage, run the workflow with \`update_baselines: true\` (and optionally \`tool: $tool_id\`) and copy the JSON snippet from the job summary into [\`.github/tool-release-baselines.json\`](../blob/main/.github/tool-release-baselines.json).

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
    echo "$issue_body" | gh issue comment "$existing_issue" --body-file -
  else
    issue_title="Tool release: $display_name $latest_version (was $baseline_version)"
    echo "  Creating new issue: $issue_title"
    echo "$issue_body" | gh issue create \
      --title "$issue_title" \
      --label "$GENERAL_LABEL" \
      --label "$tool_label" \
      --body-file -
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
