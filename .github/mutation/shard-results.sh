#!/usr/bin/env bash

surviving_mutants() {
  mutants_with_verdicts 'missed|timeout'
}

invalid_mutants() {
  mutants_with_verdicts 'invalid'
}

mutants_with_verdicts() {
  awk -F '\t' -v pattern="^($1)" '$2 ~ pattern { split($2, verdict, ":"); print verdict[1] "\t" $1 }' \
    /dev/null results/mutants-shard-*/results.tsv | sort -u |
    while IFS=$'\t' read -r outcome name; do
      IFS=: read -r file line _ description <<< "$name"
      echo "- [ ] \`$file:$line\` $outcome:${description}"
    done
}

every_shard_completed() {
  for shard in $(seq 0 $((${SHARD_COUNT:-0} - 1))); do
    [[ -f results/mutants-shard-$shard/complete ]] || return 1
  done
}
