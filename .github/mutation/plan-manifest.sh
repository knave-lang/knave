#!/usr/bin/env bash
set -euo pipefail

readonly MANIFEST_FILE=$RUNNER_TEMP/manifest.json
readonly COMMIT_PATTERN='^[0-9a-f]{40}$'
readonly WHOLE_SECONDS_PATTERN='^[1-9][0-9]*$'

main() {
  require_inputs
  require_head_branch_of_pull_request
  require_commit_of_pull_request
  decode_manifest
  require_complete_entries
  write_mutants
  echo "shards=$(shard_list)" >> "$GITHUB_OUTPUT"
}

require_inputs() {
  [[ $COMMIT =~ $COMMIT_PATTERN ]] || fail "The commit must be a full 40-character SHA, not $COMMIT."
  [[ $TEST_SECONDS =~ $WHOLE_SECONDS_PATTERN ]] || fail "test-seconds must be a whole number of at least 1, not $TEST_SECONDS."
}

require_head_branch_of_pull_request() {
  local details head_branch
  details=$(gh api "repos/$GITHUB_REPOSITORY/pulls/$PULL_REQUEST")
  if [[ $(jq '.head.repo.full_name != .base.repo.full_name' <<< "$details") != false ]]; then
    fail "Pull request $PULL_REQUEST comes from a fork, so its code does not run here."
  fi
  head_branch=$(jq -r .head.ref <<< "$details")
  [[ -n $DEFAULT_BRANCH ]] || fail "The workflow cannot tell the default branch, so it refuses to run."
  if [[ $head_branch == "$DEFAULT_BRANCH" ]]; then
    fail "Pull request $PULL_REQUEST has $DEFAULT_BRANCH as its head branch, so its mutants cannot run here."
  fi
  if [[ $GITHUB_REF == "refs/heads/$DEFAULT_BRANCH" ]]; then
    fail "The workflow does not run on $DEFAULT_BRANCH. Dispatch with --ref $head_branch."
  fi
  if [[ $GITHUB_REF != "refs/heads/$head_branch" ]]; then
    fail "Dispatch with --ref $head_branch, the head branch of pull request $PULL_REQUEST, not $GITHUB_REF."
  fi
}

require_commit_of_pull_request() {
  local commits
  commits=$(gh api --paginate "repos/$GITHUB_REPOSITORY/pulls/$PULL_REQUEST/commits?per_page=100" --jq '.[].sha')
  if ! grep -qxF "$COMMIT" <<< "$commits"; then
    fail "Commit $COMMIT is not a commit of pull request $PULL_REQUEST."
  fi
}

decode_manifest() {
  local decoded=$RUNNER_TEMP/decoded.json
  base64 --decode <<< "$MANIFEST" 2> /dev/null | gunzip > "$decoded" 2> /dev/null ||
    fail "The manifest is not gzip compressed, base64 encoded text."
  jq -e 'type == "array"' "$decoded" > /dev/null 2>&1 || fail "The manifest is not a JSON array."
  jq 'sort_by(.tests? | tostring)' "$decoded" > "$MANIFEST_FILE"
}

require_complete_entries() {
  local incomplete
  incomplete=$(jq -r --from-file <(incomplete_entries_program) "$MANIFEST_FILE")
  [[ -z $incomplete ]] || fail "These manifest entries break the manifest format in docs/pull-requests.md: $incomplete"
  [[ $(jq length "$MANIFEST_FILE") != 0 ]] || fail "The manifest holds no mutant."
}

incomplete_entries_program() {
  cat << 'EOF'
def is_test_set: type == "array" and length > 0 and all(.[]; type == "string" and (test("[\t\n]") | not));
def is_complete:
  type == "object"
  and (.name | type == "string" and length > 0 and (test("[\t\n]") | not))
  and (.file | type == "string") and (.old | type == "string") and (.new | type == "string")
  and (.tests | type == "array" and length > 0 and all(.[]; is_test_set));
.[] | select(is_complete | not) | (.name? | strings) // "an entry without a name"
EOF
}

write_mutants() {
  local count index
  count=$(jq length "$MANIFEST_FILE")
  for ((index = 0; index < count; index++)); do
    write_mutant "$index" "$count"
  done
}

write_mutant() {
  local index=$1 count=$2
  local directory=$MUTANTS/$index
  mkdir -p "$directory"
  jq -r ".[$index].name" "$MANIFEST_FILE" > "$directory/name"
  jq -c ".[$index].tests" "$MANIFEST_FILE" > "$directory/tests.json"
  echo $((index * SHARDS / count)) > "$directory/shard"
  if write_patch "$index" "$directory/mutant.patch" 2> "$directory/unapplied"; then
    rm "$directory/unapplied"
  fi
  git checkout --quiet -- .
}

write_patch() {
  local index=$1 patch=$2
  local file
  file=$(jq -r ".[$index].file" "$MANIFEST_FILE")
  require_editable "$file" || return 1
  replace_text "$index" "$file" || return 1
  git diff > "$patch"
  [[ -s $patch ]] || { echo "the new text equals the old text, so the mutant changes nothing" >&2; return 1; }
}

require_editable() {
  local file=$1
  if [[ $file == /* || /$file/ == */../* || $file == .github/* ]]; then
    echo "$file is outside the files a mutant may change" >&2
    return 1
  fi
  if [[ $(git --literal-pathspecs ls-files --full-name -- "$file") != "$file" || ! -f $file || -L $file ]]; then
    echo "$file is not the path of a tracked regular file at $COMMIT, written from the repository root" >&2
    return 1
  fi
}

replace_text() {
  local index=$1 file=$2
  local OLD NEW
  IFS= read -r -d '' OLD < <(jq -j ".[$index].old" "$MANIFEST_FILE") || true
  IFS= read -r -d '' NEW < <(jq -j ".[$index].new" "$MANIFEST_FILE") || true
  OLD=$OLD NEW=$NEW perl -0777 -i -pe '
      my $count = () = /\Q$ENV{OLD}\E/g;
      die "the old text occurs $count times in $ARGV, not once\n" unless $count == 1;
      s/\Q$ENV{OLD}\E/$ENV{NEW}/;
    ' "$file"
}

shard_list() {
  find "$MUTANTS" -name shard -exec cat {} + | sort -nu | jq -cs .
}

fail() {
  echo "::error::$1"
  exit 1
}

main
