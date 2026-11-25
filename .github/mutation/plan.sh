#!/usr/bin/env bash
set -euo pipefail
source .github/mutation/issue.sh

readonly OLDEST_BASE='3 days ago'

last_tested_commit() {
  nightly_issue | issue_body_text | tested_commit_in
}

is_usable_base() {
  [[ -n $1 ]] && git merge-base --is-ancestor "$1" HEAD && is_recent "$1"
}

announce_skipped_commits() {
  if [[ -n $1 ]] && git merge-base --is-ancestor "$1" HEAD && (($(git rev-list --count "$1..$2") > 0)); then
    echo "::notice::The last tested commit is older than $OLDEST_BASE, so the commits from $1 to $2 are not tested."
    skipped=$(daily_slices "$1" "$2")
  fi
}

daily_slices() {
  local from=$1 to=$2
  TZ=UTC git log --first-parent --reverse --date=format-local:%F --format='%H %cd' "$from..$to" |
    awk -v from="$from" '
      $2 != day && last { print from, last, day; from = last }
      { day = $2; last = $1 }
      END { if (last) print from, last, day }'
}

is_recent() {
  (($(git log -1 --format=%ct "$1") >= $(date -d "$OLDEST_BASE" +%s)))
}

commit_a_day_old() {
  git rev-list -1 --first-parent --before="24 hours ago" HEAD
}

nightly_range() {
  local tested
  tested=$(last_tested_commit)
  if is_usable_base "$tested"; then
    base=$tested
  else
    base=$(commit_a_day_old)
    announce_skipped_commits "$tested" "$base"
  fi
  head=$(git rev-parse HEAD)
}

requested_range() {
  base=$(git rev-parse --verify "$BASE_INPUT^{commit}")
  head=$(git rev-parse --verify "${HEAD_INPUT:-HEAD}^{commit}")
  git merge-base --is-ancestor "$base" "$head" || {
    echo "::error::The base $base is not an ancestor of the head $head."
    exit 1
  }
}

if [[ -n ${HEAD_INPUT:-} && -z ${BASE_INPUT:-} ]]; then
  echo "::error::A head input needs a base input: give both to test a backfill slice, or neither for the nightly range."
  exit 1
elif [[ -n ${BASE_INPUT:-} ]]; then
  requested_range
else
  nightly_range
fi
git diff --find-renames "$base" "$head" > "$DIFF"
changed_sources=$(git diff --name-only "$base" "$head" -- 'src/*.rs' 'expand/src/*.rs' 'xtask/src/*.rs' 'xtask/agree/src/*.rs' | wc -l)
{
  echo "base=$base"
  echo "skipped<<SKIPPED_SLICES"
  echo "${skipped:-}"
  echo "SKIPPED_SLICES"
  echo "head=$head"
  echo "changed-sources=$changed_sources"
} >> "$GITHUB_OUTPUT"
