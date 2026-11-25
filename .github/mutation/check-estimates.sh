#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

readonly MODULES=$(dirname "${BASH_SOURCE[0]}")
readonly LOW_RATIO=0.67
readonly HIGH_RATIO=1.5

main() {
  local totals
  totals=$(actual_and_estimated_seconds)
  read -r actual estimated <<< "$totals"
  if ((estimated == 0)); then
    return
  fi
  local ratio
  ratio=$(awk -v actual="$actual" -v estimated="$estimated" 'BEGIN { printf "%.2f", actual / estimated }')
  echo "The tested mutants took $actual seconds against an estimate of $estimated, a ratio of $ratio."
  if awk -v ratio="$ratio" -v low="$LOW_RATIO" -v high="$HIGH_RATIO" 'BEGIN { exit !(ratio < low || ratio > high) }'; then
    echo "::warning::The mutants took $ratio times their estimated seconds. Refit the estimates in .github/mutation/shards.jq."
  fi
}

actual_and_estimated_seconds() {
  awk -F '\t' 'FNR == NR { file_of[$1] = $2; next } ($1 in file_of) { print file_of[$1] "\t" $3 }' \
    <(mutant_files) results/mutants-shard-*/results.tsv |
    jq -R -s -L "$MODULES" 'include "shards";
      [split("\n")[] | select(length > 0) | split("\t") | {file: .[0], seconds: (.[1] | tonumber)}]
      | "\(map(.seconds) | add // 0) \(map(.file | seconds) | add // 0)"' -r
}

mutant_files() {
  for directory in "$MUTANTS"/*/; do
    printf '%s\t%s\n' "$(cat "$directory/name")" "$(sed -n 's/^`\([^:`]*\).*/\1/p' "$directory/function")"
  done
}

main
