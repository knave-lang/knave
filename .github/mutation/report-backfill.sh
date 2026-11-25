#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob
source .github/mutation/issue.sh
source .github/mutation/shard-results.sh

readonly ISSUE_INTRODUCTION='Each comment below records one backfill slice of the nightly mutants: its range, its run, and the mutants that no test caught. Check a box once a test covers the line or the code is simplified.'
readonly MOST_COMMENT_BYTES=60000
readonly FULL_COMMENT=slice-comment-full.md
readonly COMMENT=slice-comment.md

main() {
  local number survivors
  number=$(nightly_issue | issue_number)
  if [[ -z $number ]]; then
    number=$(gh issue create --title "$ISSUE_TITLE" --body "$ISSUE_INTRODUCTION" | sed 's#.*/##')
  fi
  survivors=$(surviving_mutants)
  slice_comment "$survivors" > "$FULL_COMMENT"
  fit_to_comment_limit "$FULL_COMMENT" > "$COMMENT"
  gh issue comment "$number" --body-file "$COMMENT"
  if [[ -n $survivors ]]; then
    gh issue reopen "$number" 2>/dev/null || true
  fi
  if ! every_shard_completed; then
    fail "A mutation shard did not finish, so this slice is incomplete."
  fi
}

slice_comment() {
  local survivors=$1
  echo "## \`$BASE\` to \`$HEAD_COMMIT\`"
  echo
  if ((${CHANGED_SOURCES:-0} == 0)); then
    echo "[This run]($RUN_URL) found no Rust source change in the slice, so it has no mutants."
    return
  elif ((${SHARD_COUNT:-0} == 0)); then
    echo "[This run]($RUN_URL) found no mutant in the slice's Rust changes."
    return
  fi
  echo "[This run]($RUN_URL) $(tested_line)"
  every_shard_completed || echo "Some shards did not finish, so the slice needs a rerun."
  echo
  if [[ -n $survivors ]]; then
    echo "$survivors"
  elif every_shard_completed; then
    echo "No mutant survived."
  fi
  invalid_section
}

invalid_section() {
  local invalid
  invalid=$(invalid_mutants)
  if [[ -n $invalid ]]; then
    echo
    echo "Invalid, because a test set failed without the mutant:"
    echo
    echo "$invalid"
  fi
}

tested_line() {
  local counts
  counts=$(verdict_counts)
  if [[ -n $counts ]]; then
    echo "tested the slice: $counts."
  else
    echo "ran no mutant of the slice."
  fi
}

fit_to_comment_limit() {
  local full=$1
  local kept total
  kept=$(LC_ALL=C awk -v limit="$MOST_COMMENT_BYTES" '{ bytes += length($0) + 1; if (bytes > limit) exit; count++ } END { print count + 0 }' "$full")
  total=$(wc -l < "$full")
  head -n "$kept" "$full"
  if ((kept < total)); then
    echo
    echo "This comment leaves out $((total - kept)) lines past GitHub's size limit. The run's shard artifacts hold every result."
  fi
}

verdict_counts() {
  cat results/mutants-shard-*/results.tsv /dev/null | cut -f 2 | cut -d : -f 1 | sort | uniq -c |
    awk '{ printf "%s%d %s", (NR > 1 ? ", " : ""), $1, $2 }'
}

fail() {
  echo "::error::$1"
  exit 1
}

main
