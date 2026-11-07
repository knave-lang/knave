#!/usr/bin/env bash
set -euo pipefail

main() {
  if ! every_shard_completed; then
    echo "::warning::A mutation shard did not finish, so the issue keeps its last tested commit."
    exit 0
  fi
  local body survivors
  survivors=$(surviving_mutants)
  body=$(issue_body "$survivors" "$(functions_with_only_unviable_mutants)")
  if [[ -n $survivors ]]; then
    publish_open "$body"
  else
    publish_closed "$body"
  fi
}

every_shard_completed() {
  ((CHANGED_SOURCES == 0)) && return 0
  for shard in $(seq 0 $((SHARDS - 1))); do
    [[ -f results/mutants-shard-$shard/complete ]] || return 1
  done
}

surviving_mutants() {
  cat results/mutants-shard-*/survivors.tsv 2>/dev/null | sort -u |
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
  local survivors=$1 unviable=$2
  echo "Mutants of \`main\` from \`$BASE\` to \`$HEAD_COMMIT\`, tested by [this run]($RUN_URL)."
  echo
  echo "## Missed or timed out"
  echo
  echo "${survivors:-None.}"
  echo
  echo "## Functions whose only mutants were unviable"
  echo
  echo "${unviable:-None.}"
  echo
  echo "<!-- tested: $HEAD_COMMIT -->"
}

existing_issue() {
  gh issue list --state all --limit 100 --search "\"$ISSUE_TITLE\" in:title" --json number,title \
    --jq '[.[] | select(.title == env.ISSUE_TITLE)][0].number // empty'
}

publish_open() {
  local number
  number=$(existing_issue)
  if [[ -z $number ]]; then
    gh issue create --title "$ISSUE_TITLE" --body "$1"
  else
    gh issue edit "$number" --body "$1"
    gh issue reopen "$number" 2>/dev/null || true
  fi
}

publish_closed() {
  local number
  number=$(existing_issue)
  if [[ -n $number ]]; then
    gh issue edit "$number" --body "$1"
    gh issue close "$number" 2>/dev/null || true
  fi
}

main
