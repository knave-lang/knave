#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob
source .github/mutation/issue.sh
source .github/mutation/shard-results.sh

readonly SURVIVORS_HEADING='## Missed or timed out'
readonly UNVIABLE_HEADING='## Functions whose only mutants were unviable'
readonly SKIPPED_HEADING='## Nightly ranges left untested'
readonly INVALID_HEADING='## Invalid in the last run, because a test set failed without the mutant'

main() {
  if ! every_shard_completed; then
    fail "A mutation shard did not finish, so the issue keeps its last tested commit."
  fi
  local issue previous found survivors unviable skipped
  issue=$(nightly_issue)
  previous=$(issue_body_text <<< "$issue")
  found=$(surviving_mutants)
  survivors=$(merged_unchecked "$SURVIVORS_HEADING" "$previous" "$found")
  unviable=$(merged_unchecked "$UNVIABLE_HEADING" "$previous" "$(functions_with_only_unviable_mutants)")
  skipped=$(merged_unchecked "$SKIPPED_HEADING" "$previous" "$(skipped_range)")
  publish "$(issue_number <<< "$issue")" "$(issue_body "$survivors" "$unviable" "$(invalid_mutants)" "$skipped")" "$survivors$unviable"
  require_no_new_survivor "$previous" "$found"
}

skipped_range() {
  if [[ -n ${SKIPPED:-} ]]; then
    read -r from to <<< "$SKIPPED"
    echo "- [ ] \`$from\` to \`$to\`: test it with \`gh workflow run mutation.yml --ref main -f base=$from -f head=$to\`"
  fi
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

functions_with_only_unviable_mutants() {
  awk -F '\t' '
    FNR == NR { function_of[$1] = $2; next }
    { key = function_of[$1]; seen[key] = 1; if ($2 != "unviable") viable[key] = 1 }
    END { for (key in seen) if (!(key in viable)) print "- [ ] " key }
  ' <(mutant_functions) results/mutants-shard-*/results.tsv 2>/dev/null | sort
}

mutant_functions() {
  for directory in "$MUTANTS"/*/; do
    printf '%s\t%s\n' "$(cat "$directory/name")" "$(cat "$directory/function")"
  done
}

require_no_new_survivor() {
  local previous=$1 found=$2
  local new
  new=$(comm -13 <(unchecked_items <<< "$previous" | sort -u) <(unchecked_items <<< "$found" | grep -F '` missed:' | sort -u))
  [[ -z $new ]] || fail "These mutants are missed and were not open in the issue: $new"
}

unchecked_items() {
  sed -n 's/^- \[ \] //p'
}

fail() {
  echo "::error::$1"
  exit 1
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
  echo "$INVALID_HEADING"
  echo
  echo "${3:-None.}"
  echo
  echo "$SKIPPED_HEADING"
  echo
  echo "${4:-None.}"
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
