#!/usr/bin/env bash
set -euo pipefail

last_tested_commit() {
  gh issue list --state all --limit 100 --search "\"$ISSUE_TITLE\" in:title" --json title,body \
    --jq '.[] | select(.title == env.ISSUE_TITLE) | .body' |
    sed -n 's/.*<!-- tested: \([0-9a-f]\{40\}\) -->.*/\1/p' | head -n 1
}

commit_a_day_old() {
  git rev-list -1 --first-parent --before="24 hours ago" HEAD
}

base_commit() {
  local tested
  tested=$(last_tested_commit)
  if [[ -n $tested ]] && git merge-base --is-ancestor "$tested" HEAD; then
    echo "$tested"
  else
    commit_a_day_old
  fi
}

base=$(base_commit)
head=$(git rev-parse HEAD)
git diff --find-renames "$base" "$head" > "$DIFF"
changed_sources=$(git diff --name-only "$base" "$head" -- 'src/*.rs' 'expand/src/*.rs' | wc -l)
{
  echo "base=$base"
  echo "head=$head"
  echo "changed-sources=$changed_sources"
} >> "$GITHUB_OUTPUT"
