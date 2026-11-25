#!/usr/bin/env bash
set -euo pipefail
shopt -s nullglob

readonly MODULES=$(dirname "${BASH_SOURCE[0]}")
readonly LOW_RATIO=0.67
readonly HIGH_RATIO=1.5
readonly WORST_SHARD_RATIO=2.0

main() {
  local summary
  summary=$(timing_summary)
  read -r actual estimated worst_shard worst_ratio <<< "$summary"
  if ((estimated == 0)); then
    return
  fi
  local ratio
  ratio=$(awk -v actual="$actual" -v estimated="$estimated" 'BEGIN { printf "%.2f", actual / estimated }')
  echo "The tested mutants took $actual seconds against an estimate of $estimated, a ratio of $ratio."
  echo "The slowest shard against its estimate is shard $worst_shard, at $worst_ratio times."
  if outside "$ratio" "$LOW_RATIO" "$HIGH_RATIO"; then
    echo "::warning::The mutants took $ratio times their estimated seconds. Refit the estimates in .github/mutation/shards.jq."
  fi
  if outside "$worst_ratio" 0 "$WORST_SHARD_RATIO"; then
    echo "::warning::Shard $worst_shard took $worst_ratio times its estimated seconds. Refit the estimates in .github/mutation/shards.jq."
  fi
}

outside() {
  awk -v value="$1" -v low="$2" -v high="$3" 'BEGIN { exit !(value < low || value > high) }'
}

timing_summary() {
  awk -F '\t' '
    FNR == NR { file_of[$1] = $2; next }
    ($1 in file_of) { shard = FILENAME; sub(/.*mutants-shard-/, "", shard); sub(/\/.*/, "", shard); print shard "\t" file_of[$1] "\t" $3 }
  ' <(mutant_files) results/mutants-shard-*/results.tsv |
    jq -R -s -r -L "$MODULES" 'include "shards";
      [split("\n")[] | select(length > 0) | split("\t")
        | {shard: .[0], seconds: (.[2] | tonumber), estimate: (.[1] | seconds)}] as $rows
      | ($rows | group_by(.shard) | map({shard: .[0].shard, ratio: ((map(.seconds) | add) / ([(map(.estimate) | add), 1] | max))})
        | max_by(.ratio) // {shard: "none", ratio: 0}) as $worst
      | "\($rows | map(.seconds) | add // 0) \($rows | map(.estimate) | add // 0) \($worst.shard) \($worst.ratio * 100 | round / 100)"'
}

mutant_files() {
  for directory in "$MUTANTS"/*/; do
    printf '%s\t%s\n' "$(cat "$directory/name")" "$(sed -n 's/^`\([^:`]*\).*/\1/p' "$directory/function")"
  done
}

main
