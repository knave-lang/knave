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
  if [[ -n $1 ]] && git merge-base --is-ancestor "$1" HEAD; then
    echo "::notice::The last tested commit is older than $OLDEST_BASE, so the commits from $1 to $2 are not tested."
  fi
}

is_recent() {
  (($(git log -1 --format=%ct "$1") >= $(date -d "$OLDEST_BASE" +%s)))
}

commit_a_day_old() {
  git rev-list -1 --first-parent --before="24 hours ago" HEAD
}

shard_list() {
  jq -cn --argjson count "$SHARDS" '[range($count)]'
}

tested=$(last_tested_commit)
if is_usable_base "$tested"; then
  base=$tested
else
  base=$(commit_a_day_old)
  announce_skipped_commits "$tested" "$base"
fi
head=$(git rev-parse HEAD)
git diff --find-renames "$base" "$head" > "$DIFF"
changed_sources=$(git diff --name-only "$base" "$head" -- 'src/*.rs' 'expand/src/*.rs' 'xtask/src/*.rs' | wc -l)
{
  echo "base=$base"
  echo "head=$head"
  echo "changed-sources=$changed_sources"
  echo "shards=$(shard_list)"
} >> "$GITHUB_OUTPUT"
