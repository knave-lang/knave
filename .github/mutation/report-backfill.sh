#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob
source .github/mutation/issue.sh
source .github/mutation/shard-results.sh

readonly ISSUE_INTRODUCTION='Each comment below records one backfill slice of the nightly mutants: its range, its run, and the mutants that no test caught. Check a box once a test covers the line or the code is simplified.'
readonly MOST_COMMENT_CHARACTERS=60000

main() {
  local number
  number=$(nightly_issue | issue_number)
  if [[ -z $number ]]; then
    number=$(gh issue create --title "$ISSUE_TITLE" --body "$ISSUE_INTRODUCTION" | sed 's#.*/##')
  fi
  slice_comment | head -c "$MOST_COMMENT_CHARACTERS" > slice-comment.md
  gh issue comment "$number" --body-file slice-comment.md
  if ! every_shard_completed; then
    fail "A mutation shard did not finish, so this slice is incomplete."
  fi
}

slice_comment() {
  echo "## \`$BASE\` to \`$HEAD_COMMIT\`"
  echo
  echo "[This run]($RUN_URL) tested the slice: $(verdict_counts)."
  every_shard_completed || echo "Some shards did not finish, so the slice needs a rerun."
  echo
  surviving_mutants | grep . || echo "No mutant survived."
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
