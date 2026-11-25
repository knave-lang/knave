#!/usr/bin/env bash
set -euo pipefail

readonly LIST=$RUNNER_TEMP/mutants.json
readonly MODULES=$(dirname "${BASH_SOURCE[0]}")
readonly SHARD_SECONDS=2700
readonly MOST_SHARDS=48
readonly NIGHTLY_PACKING_PERCENT=80

main() {
  list_mutants_up_to "$HEAD_COMMIT"
  if is_over_budget; then
    refuse_backfill_slice
    fit_nightly_range
  fi
  local shard_count
  shard_count=$(planned_shard_count)
  warn_when_over_budget "$shard_count"
  mkdir -p "$MUTANTS"
  planned length > "$MUTANTS/count"
  write_mutants "$shard_count"
  {
    echo "head=$(git rev-parse HEAD)"
    echo "shard-count=$shard_count"
    echo "shards=$(jq -cn --argjson count "$shard_count" '[range($count)]')"
  } >> "$GITHUB_OUTPUT"
}

list_mutants_up_to() {
  git checkout --quiet "$1"
  git diff --find-renames "$BASE" "$1" > "$DIFF"
  cargo mutants --workspace --list --json --diff --in-diff "$DIFF" > "$LIST"
}

planned() {
  jq -L "$MODULES" "include \"shards\"; $1" "$LIST"
}

planned_shard_count() {
  planned "shard_count($(packed_shard_seconds); $MOST_SHARDS)"
}

packed_shard_seconds() {
  if [[ -n ${BASE_INPUT:-} ]]; then
    echo "$SHARD_SECONDS"
  else
    echo $((SHARD_SECONDS * NIGHTLY_PACKING_PERCENT / 100))
  fi
}

is_over_budget() {
  (($(planned estimated_total) > MOST_SHARDS * $(packed_shard_seconds)))
}

refuse_backfill_slice() {
  if [[ -n ${BASE_INPUT:-} ]]; then
    echo "::error::This slice needs about $(minutes_per_shard "$MOST_SHARDS") minutes per shard, over the budget of $((SHARD_SECONDS / 60)). Split it in two at a commit near its middle, and dispatch each half as its own slice."
    exit 1
  fi
}

fit_nightly_range() {
  local commits fitted
  mapfile -t commits < <(git rev-list --first-parent --reverse "$BASE..$HEAD_COMMIT")
  if ((${#commits[@]} < 2)); then
    return
  fi
  fitted=$(last_fitting_commit "${commits[@]}")
  list_mutants_up_to "$fitted"
  echo "deferred-head=$HEAD_COMMIT" >> "$GITHUB_OUTPUT"
  echo "::notice::The range up to $HEAD_COMMIT is over the shard budget, so this run tests up to $fitted and the next run tests the rest."
}

last_fitting_commit() {
  local commits=("$@")
  local low=0 high=$((${#commits[@]} - 2)) middle
  while ((low < high)); do
    middle=$(((low + high + 1) / 2))
    list_mutants_up_to "${commits[middle]}"
    if is_over_budget; then
      high=$((middle - 1))
    else
      low=$middle
    fi
  done
  echo "${commits[low]}"
}

warn_when_over_budget() {
  local shard_count=$1
  if is_over_budget; then
    echo "::warning::The mutants need about $(minutes_per_shard "$shard_count") minutes per shard, over the packing of $(($(packed_shard_seconds) / 60)), so a shard may not finish."
  fi
}

minutes_per_shard() {
  local seconds_of_a_minute_on_every_shard=$(($1 * 60))
  echo $((($(planned estimated_total) + seconds_of_a_minute_on_every_shard - 1) / seconds_of_a_minute_on_every_shard))
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
