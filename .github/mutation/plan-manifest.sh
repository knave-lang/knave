#!/usr/bin/env bash
set -euo pipefail

readonly MANIFEST_FILE=manifest.json

main() {
  require_commit_of_pull_request
  decode_manifest > "$MANIFEST_FILE"
  require_complete_entries
  write_mutants
  echo "shards=$(shard_list)" >> "$GITHUB_OUTPUT"
}

require_commit_of_pull_request() {
  local cross_repository
  cross_repository=$(gh api "repos/$GITHUB_REPOSITORY/pulls/$PULL_REQUEST" --jq '.head.repo.full_name != .base.repo.full_name')
  if [[ $cross_repository != false ]]; then
    fail "Pull request $PULL_REQUEST comes from a fork, so its code does not run here."
  fi
  if ! gh api --paginate "repos/$GITHUB_REPOSITORY/pulls/$PULL_REQUEST/commits" --jq '.[].sha' | grep -qx "$COMMIT"; then
    fail "Commit $COMMIT is not a commit of pull request $PULL_REQUEST."
  fi
}

decode_manifest() {
  base64 --decode <<< "$MANIFEST" | gunzip | jq 'sort_by(.tests | tostring)'
}

require_complete_entries() {
  local incomplete
  incomplete=$(jq -r '.[] | select((.name | type) != "string" or (.file | type) != "string"
      or (.old | type) != "string" or (.new | type) != "string"
      or (.tests | type) != "array" or (.tests | length) == 0) | .name // "an entry without a name"' "$MANIFEST_FILE")
  if [[ -n $incomplete ]]; then
    fail "These manifest entries need a name, file, old, new and at least one test set: $incomplete"
  fi
  if [[ $(jq length "$MANIFEST_FILE") == 0 ]]; then
    fail "The manifest holds no mutant."
  fi
}

write_mutants() {
  local count index
  count=$(jq length "$MANIFEST_FILE")
  for ((index = 0; index < count; index++)); do
    write_mutant "$index" "$count"
  done
}

write_mutant() {
  local index=$1 count=$2
  local directory=$MUTANTS/$index
  mkdir -p "$directory"
  jq -r ".[$index].name" "$MANIFEST_FILE" > "$directory/name"
  jq -c ".[$index].tests" "$MANIFEST_FILE" > "$directory/tests.json"
  echo $((index * SHARDS / count)) > "$directory/shard"
  replace_text "$index" 2> "$directory/unapplied" && rm "$directory/unapplied"
  git diff > "$directory/mutant.patch"
  git checkout --quiet -- .
}

replace_text() {
  local index=$1
  local file
  file=$(jq -r ".[$index].file" "$MANIFEST_FILE")
  [[ -f $file ]] || { echo "$file does not exist at $COMMIT"; return 1; }
  local OLD NEW
  IFS= read -r -d '' OLD < <(jq -j ".[$index].old" "$MANIFEST_FILE") || true
  IFS= read -r -d '' NEW < <(jq -j ".[$index].new" "$MANIFEST_FILE") || true
  OLD=$OLD NEW=$NEW perl -0777 -i -pe '
      my $count = () = /\Q$ENV{OLD}\E/g;
      die "the old text occurs $count times in $ARGV, not once\n" unless $count == 1;
      s/\Q$ENV{OLD}\E/$ENV{NEW}/;
    ' "$file"
}

shard_list() {
  find "$MUTANTS" -name shard -exec cat {} + | sort -nu | jq -cs .
}

fail() {
  echo "::error::$1"
  exit 1
}

main
