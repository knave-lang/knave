#!/usr/bin/env bash
set -euo pipefail

fixture_tests=(--test-package=knave -- --test stages -- expand_fixtures check_fixtures emit_fixtures)
every_test=(--test-workspace=true)

test_mutants() {
  local timeout=$1
  shift
  local exit_code=0
  cargo mutants --workspace --in-place --no-shuffle --cap-lints=true --baseline=skip \
    --timeout "$timeout" --in-diff "$DIFF" -vV "$@" || exit_code=$?
  case "$exit_code" in
    0 | 2 | 3) ;;
    *) exit "$exit_code" ;;
  esac
}

outcome_lines() {
  cat mutants.out/missed.txt mutants.out/timeout.txt 2>/dev/null || true
}

survivor_patterns() {
  outcome_lines | sed -e 's/[][\\.*^$+?(){}|]/\\&/g' -e 's/^/--re=^/' -e 's/$/$/'
}

keep_first_stage_outcomes() {
  if [[ -f mutants.out/outcomes.json ]]; then
    cp mutants.out/outcomes.json "$RESULTS/first-stage.json"
  else
    echo '{"outcomes": []}' > "$RESULTS/first-stage.json"
  fi
}

keep_survivors() {
  for outcome in missed timeout; do
    sed "s/^/$outcome\t/" "mutants.out/$outcome.txt" 2>/dev/null || true
  done > "$RESULTS/survivors.tsv"
}

mkdir -p "$RESULTS"
test_mutants 240 --shard "$SHARD" "${fixture_tests[@]}"
keep_first_stage_outcomes
mapfile -t survivors < <(survivor_patterns)
if ((${#survivors[@]} > 0)); then
  test_mutants 600 "${survivors[@]}" "${every_test[@]}"
  keep_survivors
else
  : > "$RESULTS/survivors.tsv"
fi
touch "$RESULTS/complete"
