#!/usr/bin/env bash
# Shared by the workflow and its offline dispatch tests.
select_spec_tiers() {
  local event=$1 requested=$2 schedule=$3
  if [[ "$event" == workflow_dispatch ]]; then
    case "$requested" in
      s-tier|a-tier|b-tier|d-tier) echo "$requested" ;;
      all) echo 's-tier a-tier b-tier d-tier' ;;
      *) echo "Invalid spec tier: $requested" >&2; return 2 ;;
    esac
  elif [[ "$event" == schedule ]]; then
    case "$schedule" in
      '0 0 * * 0') echo s-tier ;;
      '0 0 * * 3') echo 'a-tier b-tier' ;;
      '0 0 1 * *') echo d-tier ;;
      *) echo "Unknown spec schedule: $schedule" >&2; return 2 ;;
    esac
  else
    echo 's-tier a-tier b-tier d-tier'
  fi
}
