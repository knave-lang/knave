#!/usr/bin/env bash
set -euo pipefail

readonly FAILING_VERDICTS='^(missed|unapplied|invalid)'

main() {
  mkdir -p "$REPORT/logs"
  merged_results > "$REPORT/results.tsv"
  find "$SHARD_RESULTS" -path '*/logs/*.log' -exec cp {} "$REPORT/logs/" \;
  summary > "$REPORT/summary.md"
  cat "$REPORT/summary.md" >> "$GITHUB_STEP_SUMMARY"
  require_every_shard
  require_no_failing_verdict
}

merged_results() {
  cat "$SHARD_RESULTS"/*/results.tsv 2>/dev/null | sort -t $'\t' -k 1,1V || true
}

summary() {
  echo "Mutants of pull request $PULL_REQUEST at \`$COMMIT\`: $(verdict_counts)."
  echo
  echo '| Mutant | Verdict | Seconds | Log |'
  echo '| --- | --- | --- | --- |'
  awk -F '\t' '{ gsub(/\|/, "\\|"); printf "| %s | %s | %s | `%s` |\n", $1, $2, $3, $4 }' "$REPORT/results.tsv"
  missing_shards_line
}

verdict_counts() {
  cut -f 2 "$REPORT/results.tsv" | cut -d : -f 1 | sort | uniq -c | awk '{ printf "%s%d %s", (NR > 1 ? ", " : ""), $1, $2 }'
}

missing_shards_line() {
  local missing
  missing=$(missing_shards)
  [[ -z $missing ]] || printf '\nThese shards did not finish, so their mutants are missing above: %s\n' "$missing"
}

missing_shards() {
  for shard in $(jq -r '.[]' <<< "$SHARD_LIST"); do
    [[ -f $SHARD_RESULTS/mutant-results-shard-$shard/complete ]] || printf '%s ' "$shard"
  done
}

require_every_shard() {
  [[ -z $(missing_shards) ]] || fail "A shard did not finish."
}

require_no_failing_verdict() {
  if grep -Eq "$FAILING_VERDICTS" < <(cut -f 2 "$REPORT/results.tsv"); then
    fail "A mutant was missed, unapplied or invalid. See the summary."
  fi
}

fail() {
  echo "::error::$1"
  exit 1
}

main
