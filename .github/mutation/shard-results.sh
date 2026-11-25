#!/usr/bin/env bash

surviving_mutants() {
  mutants_with_verdicts 'missed|timeout'
}

invalid_mutants() {
  mutants_with_verdicts 'invalid' | sed 's/^- \[ \] /- /'
}

mutants_with_verdicts() {
  awk -F '\t' -v pattern="^($1)" '$2 ~ pattern { split($2, verdict, ":"); print verdict[1] "\t" $1 }' \
    /dev/null results/mutants-shard-*/results.tsv | sort -u | as_issue_items
}

unfinished_mutants() {
  for shard in $(unfinished_shards); do
    comm -23 <(names_of_shard "$shard" | sort) <(recorded_names_of_shard "$shard" | sort)
  done | sed 's/^/not finished\t/' | sort -u | as_issue_items
}

recorded_mutant_keys() {
  cut -f 1 /dev/null results/mutants-shard-*/results.tsv | sort -u |
    while IFS=: read -r file line _ description; do
      echo "\`$file:$line\`:$description"
    done
}

item_keys() {
  sed -n 's/^- \[ \] \(`[^`]*`\) [^:]*:/\1:/p'
}

as_issue_items() {
  while IFS=$'\t' read -r outcome name; do
    IFS=: read -r file line _ description <<< "$name"
    echo "- [ ] \`$file:$line\` $outcome:${description}"
  done
}

names_of_shard() {
  for directory in "$MUTANTS"/*/; do
    if [[ $(cat "$directory/shard") == "$1" ]]; then
      cat "$directory/name"
    fi
  done
}

recorded_names_of_shard() {
  local results=results/mutants-shard-$1/results.tsv
  if [[ -f $results ]]; then
    cut -f 1 "$results"
  fi
}

unfinished_shards() {
  for shard in $(seq 0 $((${SHARD_COUNT:-0} - 1))); do
    [[ -f results/mutants-shard-$shard/complete ]] || echo "$shard"
  done
}

unfinished_shards_phrase() {
  local shards
  shards=$(unfinished_shards | paste -sd ' ')
  if [[ $shards == *' '* ]]; then
    echo "shards $shards"
  else
    echo "shard $shards"
  fi
}

every_shard_completed() {
  [[ -z $(unfinished_shards) ]]
}
