#!/usr/bin/env bash
set -uo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/test-runs.sh"

main() {
  mkdir -p "$RESULTS"
  every_test_set | record_failing_test_sets "$RESULTS/baseline.log" "$RESULTS/baseline-failures"
}

every_test_set() {
  jq -c '.[]' "$MUTANTS"/*/tests.json | sort -u
}

main
