#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob
source .github/mutation/issue.sh
source .github/mutation/shard-results.sh

readonly SURVIVORS_HEADING='## Missed or timed out'
readonly UNVIABLE_HEADING='## Functions whose only mutants were unviable'
readonly SKIPPED_HEADING='## Nightly ranges left untested'
readonly INVALID_HEADING='## Invalid in the last run, because a test set failed without the mutant'
readonly CHECK_INSTRUCTION='Check a box once a test covers the line or the code is simplified. Checked lines drop off on the next run.'

main() {
  local issue previous found survivors unviable skipped invalid
  issue=$(nightly_issue)
  previous=$(issue_body_text <<< "$issue")
  found=$(surviving_mutants)
  survivors=$(merged_lines "$(carried_survivors "$previous")" "$(printf '%s\n%s' "$found" "$(unfinished_mutants)")")
  unviable=$(merged_unchecked "$UNVIABLE_HEADING" "$previous" "$(functions_with_only_unviable_mutants)")
  skipped=$(merged_unchecked "$SKIPPED_HEADING" "$previous" "$(skipped_slices)")
  invalid=$(invalid_mutants)
  publish "$(issue_number <<< "$issue")" "$(issue_body "$(tested_commit "$previous")" "$survivors" "$unviable" "$invalid" "$skipped")" "$survivors$unviable$invalid$skipped$(unfinished_shards)"
  require_every_shard_completed
  require_no_new_survivor "$previous" "$found"
}

tested_commit() {
  if every_shard_completed; then
    echo "$HEAD_COMMIT"
  else
    tested_commit_in <<< "$1"
  fi
}

require_every_shard_completed() {
  every_shard_completed || fail "Not every shard finished: $(unfinished_shards_phrase) did not, so the issue keeps its last tested commit."
}

skipped_slices() {
  local from to day
  while read -r from to day; do
    [[ -n $from ]] || continue
    echo "- [ ] $day: \`$from\` to \`$to\`: test it with \`gh workflow run mutation.yml --ref main -f base=$from -f head=$to\`"
  done <<< "${SKIPPED:-}"
}

merged_unchecked() {
  local heading=$1 previous=$2 found=$3
  merged_lines "$(unchecked_lines_under "$heading" <<< "$previous")" "$found"
}

merged_lines() {
  printf '%s\n%s\n' "$1" "$2" | awk 'NF && !seen[$0]++'
}

carried_survivors() {
  unchecked_lines_under "$SURVIVORS_HEADING" <<< "$1" | without_retested_unfinished "$(recorded_mutant_keys)"
}

without_retested_unfinished() {
  local retested=$1 line
  while IFS= read -r line; do
    if [[ $line != *'` not finished:'* ]] || ! grep -qxF -- "$(item_keys <<< "$line")" <<< "$retested"; then
      echo "$line"
    fi
  done
}

unchecked_lines_under() {
  awk -v heading="$1" '{ sub(/[ \t]+$/, "") } $0 == heading { inside = 1; next } /^## / || /^<!--/ { inside = 0 } inside && /^- \[ \] /'
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
  new=$(missed_lines <<< "$found" | lines_with_keys_outside "$(item_keys <<< "$previous")" | unchecked_items)
  [[ -z $new ]] || fail "These mutants are missed and were not open in the issue: $new"
}

missed_lines() {
  grep -F '` missed:' || true
}

lines_with_keys_outside() {
  local known=$1 line
  while IFS= read -r line; do
    if ! grep -qxF -- "$(item_keys <<< "$line")" <<< "$known"; then
      echo "$line"
    fi
  done
}

unchecked_items() {
  sed -n 's/^- \[ \] //p'
}

fail() {
  echo "::error::$1"
  exit 1
}

issue_body() {
  local tested=$1
  tested_line "$tested"
  echo
  echo "$SURVIVORS_HEADING"
  echo
  echo "${2:-None.}"
  echo
  echo "$UNVIABLE_HEADING"
  echo
  echo "${3:-None.}"
  echo
  echo "$INVALID_HEADING"
  echo
  echo "${4:-None.}"
  echo
  echo "$SKIPPED_HEADING"
  echo
  echo "${5:-None.}"
  if [[ -n $tested ]]; then
    echo
    echo "<!-- tested: $tested -->"
  fi
}

tested_line() {
  if every_shard_completed; then
    echo "Last tested: \`main\` from \`$BASE\` to \`$HEAD_COMMIT\`, by [this run]($RUN_URL). $CHECK_INSTRUCTION"
  else
    echo "[This run]($RUN_URL) tested \`main\` from \`$BASE\` to \`$HEAD_COMMIT\`, but $(unfinished_shards_phrase) did not finish, so the last tested commit stays \`${1:-none}\`. $CHECK_INSTRUCTION"
  fi
  if [[ -n ${DEFERRED_HEAD:-} ]]; then
    echo
    deferred_line
  fi
}

deferred_line() {
  local deferred="The commits after \`$HEAD_COMMIT\` up to \`$DEFERRED_HEAD\` did not fit the shard budget"
  if every_shard_completed; then
    echo "$deferred, so the next run tests them."
  else
    echo "$deferred. They wait for a completed run that starts at or after \`$HEAD_COMMIT\`."
  fi
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
