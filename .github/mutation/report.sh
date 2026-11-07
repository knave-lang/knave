#!/usr/bin/env bash
set -euo pipefail
source .github/mutation/issue.sh

readonly SURVIVORS_HEADING='## Missed or timed out'
readonly UNVIABLE_HEADING='## Functions whose only mutants were unviable'

main() {
  if ! every_shard_completed; then
    echo "::warning::A mutation shard did not finish, so the issue keeps its last tested commit."
    return
  fi
  local issue previous survivors unviable
  issue=$(nightly_issue)
  previous=$(issue_body_text <<< "$issue")
  survivors=$(merged_unchecked "$SURVIVORS_HEADING" "$previous" "$(surviving_mutants)")
  unviable=$(merged_unchecked "$UNVIABLE_HEADING" "$previous" "$(functions_with_only_unviable_mutants)")
  publish "$(issue_number <<< "$issue")" "$(issue_body "$survivors" "$unviable")" "$survivors$unviable"
}

every_shard_completed() {
  ((CHANGED_SOURCES == 0)) && return 0
  for shard in $(seq 0 $((SHARDS - 1))); do
    [[ -f results/mutants-shard-$shard/complete ]] || return 1
  done
}

merged_unchecked() {
  local heading=$1 previous=$2 found=$3
  {
    unchecked_lines_under "$heading" <<< "$previous"
    echo "$found"
  } | awk 'NF && !seen[$0]++'
}

unchecked_lines_under() {
  awk -v heading="$1" '{ sub(/[ \t]+$/, "") } $0 == heading { inside = 1; next } /^## / { inside = 0 } inside && /^- \[ \] /'
}

surviving_mutants() {
  { cat results/mutants-shard-*/survivors.tsv 2>/dev/null || true; } | sort -u |
    while IFS=$'\t' read -r outcome name; do
      IFS=: read -r file line _ description <<< "$name"
      echo "- [ ] \`$file:$line\` $outcome:${description}"
    done
}

functions_with_only_unviable_mutants() {
  jq -rs '
    [.[].outcomes[] | .summary as $summary | .scenario.Mutant? // empty | {
      key: "`\(.file):\(.function.span.start.line)` `\(.function.function_name)`",
      unviable: ($summary == "Unviable")
    }]
    | group_by(.key)
    | map(select(all(.unviable)) | "- [ ] \(.[0].key)")
    | .[]' results/mutants-shard-*/first-stage.json 2>/dev/null || true
}

issue_body() {
  echo "Last tested: \`main\` from \`$BASE\` to \`$HEAD_COMMIT\`, by [this run]($RUN_URL). Check a box once a test covers the line or the code is simplified. Checked lines drop off on the next run."
  echo
  echo "$SURVIVORS_HEADING"
  echo
  echo "${1:-None.}"
  echo
  echo "$UNVIABLE_HEADING"
  echo
  echo "${2:-None.}"
  echo
  echo "<!-- tested: $HEAD_COMMIT -->"
}

publish() {
  local number=$1 body=$2 open_items=$3
  if [[ -z $number ]]; then
    number=$(gh issue create --title "$ISSUE_TITLE" --body "$body" | sed 's#.*/##')
  else
    gh issue edit "$number" --body "$body"
  fi
  if [[ -n $open_items ]]; then
    gh issue reopen "$number" 2>/dev/null || true
  else
    gh issue close "$number" 2>/dev/null || true
  fi
}

main
