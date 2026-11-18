#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob
source .github/mutation/issue.sh

readonly SURVIVORS_HEADING='## Missed or timed out'
readonly UNVIABLE_HEADING='## Functions whose only mutants were unviable'

main() {
  if ! every_shard_completed; then
    fail "A mutation shard did not finish, so the issue keeps its last tested commit."
  fi
  local issue previous found survivors unviable
  issue=$(nightly_issue)
  previous=$(issue_body_text <<< "$issue")
  found=$(surviving_mutants)
  survivors=$(merged_unchecked "$SURVIVORS_HEADING" "$previous" "$found")
  unviable=$(merged_unchecked "$UNVIABLE_HEADING" "$previous" "$(functions_with_only_unviable_mutants)")
  publish "$(issue_number <<< "$issue")" "$(issue_body "$survivors" "$unviable")" "$survivors$unviable"
  require_no_new_survivor "$previous" "$found"
}

every_shard_completed() {
  for shard in $(seq 0 $((${SHARD_COUNT:-0} - 1))); do
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
  awk -F '\t' '$2 ~ /^(missed|timeout)/ { split($2, verdict, ":"); print verdict[1] "\t" $1 }' \
    /dev/null results/mutants-shard-*/results.tsv | sort -u |
    while IFS=$'\t' read -r outcome name; do
      IFS=: read -r file line _ description <<< "$name"
      echo "- [ ] \`$file:$line\` $outcome:${description}"
    done
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
