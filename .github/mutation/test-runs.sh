#!/usr/bin/env bash

readonly TIMED_OUT=124
readonly KILLED_AFTER_TIMEOUT=137
readonly KILL_GRACE=10s
readonly COMPILE_FAILURE='error\[E[0-9]+\]|could not compile'

record_failing_test_sets() {
  local log=$1 failures=$2 test_set_json
  : > "$failures"
  while IFS= read -r test_set_json; do
    mapfile -t test_set < <(jq -r '.[]' <<< "$test_set_json")
    if ! run_test_set "$log" "${test_set[@]}" > /dev/null; then
      echo "$test_set_json" >> "$failures"
    fi
    restore_checkout
  done
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
