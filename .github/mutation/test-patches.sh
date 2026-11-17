#!/usr/bin/env bash
set -uo pipefail

readonly TIMED_OUT=124
readonly KILL_GRACE=10s
readonly COMPILE_FAILURE='error\[E[0-9]+\]|could not compile'

main() {
  mkdir -p "$RESULTS/logs"
  : > "$RESULTS/results.tsv"
  for directory in $(mutants_of_shard); do
    record "$directory" "$(timed_verdict "$directory")"
  done
  touch "$RESULTS/complete"
}

mutants_of_shard() {
  grep -lx "$SHARD" "$MUTANTS"/*/shard | xargs -r -n 1 dirname | sort -V
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
    verdict_of_applied "$directory" >> "$(log_of "$directory")"
    tail -n 1 "$(log_of "$directory")"
    git checkout --quiet -- .
  else
    echo "unapplied: git apply refused the patch"
  fi
}

verdict_of_applied() {
  local directory=$1
  local count index test_set
  count=$(jq length "$directory/tests.json")
  for ((index = 0; index < count; index++)); do
    mapfile -t test_set < <(jq -r ".[$index][]" "$directory/tests.json")
    run_test_set "${test_set[@]}" || return 0
  done
  echo missed
}

run_test_set() {
  local assignments=() arguments=()
  split_assignments "$@"
  echo "cargo test ${*}"
  local build_output build_status
  build_output=$(env "${assignments[@]}" timeout --kill-after="$KILL_GRACE" "$BUILD_SECONDS" cargo test --no-run "${arguments[@]}" 2>&1)
  build_status=$?
  echo "$build_output"
  if ((build_status != 0)); then
    verdict_of_build "$build_status" "$build_output" "$*"
    return 1
  fi
  env "${assignments[@]}" timeout --kill-after="$KILL_GRACE" "$TEST_SECONDS" cargo test "${arguments[@]}" 2>&1
  verdict_of_exit $? "$*"
}

verdict_of_build() {
  local status=$1 output=$2 command_words=$3
  if ((status == TIMED_OUT)); then
    echo "timeout: build of $command_words"
  elif grep -Eq "$COMPILE_FAILURE" <<< "$output"; then
    echo unviable
  else
    echo "invalid: $(grep -m 1 '^error' <<< "$output" || echo "cargo test exited with $status")"
  fi
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

verdict_of_exit() {
  local status=$1 command_words=$2
  case $status in
    0) return 0 ;;
    "$TIMED_OUT") echo "timeout: $command_words" ;;
    *) echo "caught: $command_words" ;;
  esac
  return 1
}

record() {
  local directory=$1 verdict_and_seconds=$2
  printf '%s\t%s\tlogs/%s.log\n' "$(cat "$directory/name")" "$verdict_and_seconds" "$(basename "$directory")" >> "$RESULTS/results.tsv"
}

log_of() {
  echo "$RESULTS/logs/$(basename "$1").log"
}

main
