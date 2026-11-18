#!/usr/bin/env bash
set -euo pipefail

readonly LIST=$RUNNER_TEMP/mutants.json
readonly SELECTION=.github/mutation
readonly MUTANTS_PER_SHARD=24
readonly MOST_SHARDS=48

main() {
  cargo mutants --workspace --list --json --diff --in-diff "$DIFF" > "$LIST"
  local count shard_count
  count=$(jq length "$LIST")
  shard_count=$(shard_count_for "$count")
  mkdir -p "$MUTANTS"
  echo "$count" > "$MUTANTS/count"
  write_mutants "$shard_count"
  {
    echo "shard-count=$shard_count"
    echo "shards=$(jq -cn --argjson count "$shard_count" '[range($count)]')"
  } >> "$GITHUB_OUTPUT"
}

shard_count_for() {
  local count=$1
  local needed=$(((count + MUTANTS_PER_SHARD - 1) / MUTANTS_PER_SHARD))
  echo $((needed < MOST_SHARDS ? needed : MOST_SHARDS))
}

write_mutants() {
  local shard_count=$1
  local index=0 name function tests patch
  while IFS= read -r -d '' name && IFS= read -r -d '' function &&
    IFS= read -r -d '' tests && IFS= read -r -d '' patch; do
    write_mutant "$MUTANTS/$index" "$((index % shard_count))"
    index=$((index + 1))
  done < <(mutant_fields)
}

mutant_fields() {
  jq --raw-output0 -L "$SELECTION" '
    include "test-sets";
    .[]
    | .name,
      "`\(.file):\(.function.span.start.line // 1)` `\(.function.function_name // "")`",
      (.file | test_sets | tojson),
      "--- a/\(.file)\n+++ b/\(.file)\n" + (.diff | split("\n")[2:] | join("\n"))
  ' "$LIST"
}

write_mutant() {
  local directory=$1 shard=$2
  mkdir -p "$directory"
  printf '%s\n' "$name" > "$directory/name"
  printf '%s\n' "$function" > "$directory/function"
  printf '%s\n' "$tests" > "$directory/tests.json"
  printf '%s' "$patch" > "$directory/mutant.patch"
  echo "$shard" > "$directory/shard"
}

main
