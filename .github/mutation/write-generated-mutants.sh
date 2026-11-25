#!/usr/bin/env bash
set -euo pipefail

readonly LIST=$RUNNER_TEMP/mutants.json
readonly MODULES=$(dirname "${BASH_SOURCE[0]}")
readonly SHARD_SECONDS=2700
readonly MOST_SHARDS=48

main() {
  cargo mutants --workspace --list --json --diff --in-diff "$DIFF" > "$LIST"
  local shard_count
  shard_count=$(planned "shard_count($SHARD_SECONDS; $MOST_SHARDS)")
  warn_when_over_budget "$shard_count"
  mkdir -p "$MUTANTS"
  planned length > "$MUTANTS/count"
  write_mutants "$shard_count"
  {
    echo "shard-count=$shard_count"
    echo "shards=$(jq -cn --argjson count "$shard_count" '[range($count)]')"
  } >> "$GITHUB_OUTPUT"
}

planned() {
  jq -L "$MODULES" "include \"shards\"; $1" "$LIST"
}

warn_when_over_budget() {
  local shard_count=$1
  local total
  total=$(planned estimated_total)
  if ((total > shard_count * SHARD_SECONDS)); then
    echo "::warning::The mutants need about $((total / shard_count / 60)) minutes per shard, over the budget of $((SHARD_SECONDS / 60)), so a shard may not finish."
  fi
}

write_mutants() {
  local shard_count=$1
  local index=0 name function tests patch shard
  while IFS= read -r -d '' name && IFS= read -r -d '' function && IFS= read -r -d '' tests &&
    IFS= read -r -d '' patch && IFS= read -r -d '' shard; do
    write_mutant "$MUTANTS/$index"
    index=$((index + 1))
  done < <(mutant_fields "$shard_count")
}

mutant_fields() {
  jq --raw-output0 -L "$MODULES" --argjson count "$1" '
    include "test-sets";
    include "shards";
    shard_assignment($count) as $shards
    | range(length) as $index
    | .[$index]
    | .name,
      "`\(.file):\(.function.span.start.line // 1)` `\(.function.function_name // "")`",
      (.file | test_sets | tojson),
      "--- a/\(.file)\n+++ b/\(.file)\n" + (.diff | split("\n")[2:] | join("\n")),
      ($shards[$index] | tostring)
  ' "$LIST"
}

write_mutant() {
  local directory=$1
  mkdir -p "$directory"
  printf '%s\n' "$name" > "$directory/name"
  printf '%s\n' "$function" > "$directory/function"
  printf '%s\n' "$tests" > "$directory/tests.json"
  printf '%s' "$patch" > "$directory/mutant.patch"
  echo "$shard" > "$directory/shard"
}

main
