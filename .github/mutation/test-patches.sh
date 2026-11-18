#!/usr/bin/env bash
set -uo pipefail

readonly TIMED_OUT=124
readonly KILLED_AFTER_TIMEOUT=137
readonly KILL_GRACE=10s
readonly COMPILE_FAILURE='error\[E[0-9]+\]|could not compile'

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
  local test_set_json
  : > "$(baseline_failures)"
  while IFS= read -r test_set_json; do
    mapfile -t test_set < <(jq -r '.[]' <<< "$test_set_json")
    if ! run_test_set "$(baseline_log)" "${test_set[@]}" > /dev/null; then
      echo "$test_set_json" >> "$(baseline_failures)"
    fi
    restore_checkout
  done < <(test_sets_of_shard)
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

run_test_set() {
  local log=$1
  shift
  local assignments=() arguments=() cargo_arguments=()
  split_assignments "$@"
  split_cargo_arguments
  echo "cargo test $*" >> "$log"
  env "${assignments[@]}" timeout --kill-after="$KILL_GRACE" "$BUILD_SECONDS" cargo test --no-run "${cargo_arguments[@]}" >> "$log" 2>&1
  local build_status=$?
  if ((build_status != 0)); then
    verdict_of_build "$build_status" "$log" "$*"
    return 1
  fi
  env "${assignments[@]}" timeout --kill-after="$KILL_GRACE" "$TEST_SECONDS" cargo test "${arguments[@]}" >> "$log" 2>&1
  verdict_of_exit $? "$*"
}

split_assignments() {
  for word in "$@"; do
    if ((${#arguments[@]} == 0)) && [[ $word =~ ^[A-Z_]+= ]]; then
      assignments+=("$word")
    else
      arguments+=("$word")
    fi
  done
}

split_cargo_arguments() {
  for word in "${arguments[@]}"; do
    [[ $word == -- ]] && return
    cargo_arguments+=("$word")
  done
}

verdict_of_build() {
  local status=$1 log=$2 command_words=$3
  if is_timeout "$status"; then
    echo "timeout: build of $command_words"
  elif grep -Eq "$COMPILE_FAILURE" "$log"; then
    echo unviable
  else
    echo "invalid: $(grep '^error' "$log" | tail -n 1 | grep . || echo "cargo test exited with $status")"
  fi
}

verdict_of_exit() {
  local status=$1 command_words=$2
  if ((status == 0)); then
    return 0
  elif is_timeout "$status"; then
    echo "timeout: $command_words"
  else
    echo "caught: $command_words"
  fi
  return 1
}

is_timeout() {
  (($1 == TIMED_OUT || $1 == KILLED_AFTER_TIMEOUT))
}

restore_checkout() {
  git checkout --quiet -- .
  git clean -fdq ${TOOLS:+--exclude="/${TOOLS#"$PWD"/}"}
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
