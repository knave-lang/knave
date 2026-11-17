#!/usr/bin/env bash
set -euo pipefail

readonly LIST=$RUNNER_TEMP/mutants.json
readonly SELECTION=.github/mutation

main() {
  cargo mutants --workspace --list --json --diff --in-diff "$DIFF" > "$LIST"
  local count index
  count=$(jq length "$LIST")
  mkdir -p "$MUTANTS"
  echo "$count" > "$MUTANTS/count"
  for ((index = 0; index < count; index++)); do
    write_mutant "$index"
  done
}

write_mutant() {
  local index=$1
  local directory=$MUTANTS/$index
  mkdir -p "$directory"
  jq -r ".[$index].name" "$LIST" > "$directory/name"
  jq -r ".[$index] | \"\`\(.file):\(.function.span.start.line // 1)\` \`\(.function.function_name // \"\")\`\"" "$LIST" > "$directory/function"
  jq -c -L "$SELECTION" "include \"test-sets\"; .[$index].file | test_sets" "$LIST" > "$directory/tests.json"
  jq -r ".[$index] | \"--- a/\(.file)\n+++ b/\(.file)\n\" + (.diff | split(\"\n\")[2:] | join(\"\n\"))" "$LIST" > "$directory/mutant.patch"
  echo $((index % SHARDS)) > "$directory/shard"
}

main
