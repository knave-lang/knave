#!/usr/bin/env bash
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/test-runs.sh"

main() {
  mkdir -p "$RESULTS/logs"
  : > "$RESULTS/results.tsv"
  record_baseline_failures
  for directory in $(mutants_of_shard); do
    record "$directory" "$(timed_verdict "$directory")"
  done
  touch "$RESULTS/complete"
  warn_when_slow
}

warn_when_slow() {
  if [[ -n ${SLOW_SECONDS:-} ]] && ((SECONDS > SLOW_SECONDS)); then
    echo "::warning::Shard $SHARD took $((SECONDS / 60)) minutes, past $((SLOW_SECONDS / 60)). The cost estimates in .github/mutation/shards.jq may be low."
  fi
}

mutants_of_shard() {
  grep -lx "$SHARD" "$MUTANTS"/*/shard 2> /dev/null | xargs -r -n 1 dirname | sort -V
}

record_baseline_failures() {
  if [[ -n ${BASELINE_FAILURES:-} ]]; then
    cp "$BASELINE_FAILURES" "$(baseline_failures)" || {
      echo "::error::The baseline failures file $BASELINE_FAILURES is missing, so the shard cannot tell a caught mutant from a test set that fails without one."
      exit 1
    }
  else
    test_sets_of_shard | record_failing_test_sets "$(baseline_log)" "$(baseline_failures)"
  fi
}

test_sets_of_shard() {
  for directory in $(mutants_of_shard); do
    jq -c '.[]' "$directory/tests.json"
  done | sort -u
}

timed_verdict() {
  local directory=$1
  local started=$SECONDS
  local verdict
  verdict=$(verdict_of "$directory" 2>> "$(log_of "$directory")")
  printf '%s\t%s' "$verdict" $((SECONDS - started))
}

verdict_of() {
  local directory=$1
  if [[ -f $directory/unapplied ]]; then
    echo "unapplied: $(head -n 1 "$directory/unapplied")"
  elif git apply "$directory/mutant.patch"; then
    verdict_of_applied "$directory"
    restore_checkout
  else
    echo "unapplied: git apply refused the patch"
  fi
}

verdict_of_applied() {
  local directory=$1
  local test_set_json
  while IFS= read -r test_set_json; do
    mapfile -t test_set < <(jq -r '.[]' <<< "$test_set_json")
    if grep -qxF "$test_set_json" "$(baseline_failures)"; then
      echo "invalid: ${test_set[*]} fails without the mutant"
      return
    fi
    run_test_set "$(log_of "$directory")" "${test_set[@]}" || return
  done < <(jq -c '.[]' "$directory/tests.json")
  echo missed
}

record() {
  local directory=$1 verdict_and_seconds=$2
  printf '%s\t%s\tlogs/%s.log\n' "$(cat "$directory/name")" "$verdict_and_seconds" "$(basename "$directory")" >> "$RESULTS/results.tsv"
}

log_of() {
  echo "$RESULTS/logs/$(basename "$1").log"
}

baseline_log() {
  echo "$RESULTS/logs/baseline.log"
}

baseline_failures() {
  echo "$RESULTS/baseline-failures"
}

main
