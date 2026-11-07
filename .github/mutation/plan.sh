#!/usr/bin/env bash
set -euo pipefail
source .github/mutation/issue.sh

readonly OLDEST_BASE='3 days ago'

base_commit() {
  local tested
  tested=$(nightly_issue | issue_body_text | tested_commit_in)
  if [[ -n $tested ]] && git merge-base --is-ancestor "$tested" HEAD && is_recent "$tested"; then
    echo "$tested"
  else
    commit_a_day_old
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

base=$(base_commit)
head=$(git rev-parse HEAD)
git diff --find-renames "$base" "$head" > "$DIFF"
changed_sources=$(git diff --name-only "$base" "$head" -- 'src/*.rs' 'expand/src/*.rs' | wc -l)
{
  echo "base=$base"
  echo "head=$head"
  echo "changed-sources=$changed_sources"
  echo "shards=$(shard_list)"
} >> "$GITHUB_OUTPUT"
