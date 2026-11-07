#!/usr/bin/env bash

readonly ISSUE_AUTHOR='github-actions[bot]'

nightly_issue() {
  gh api --method GET "repos/$GITHUB_REPOSITORY/issues" \
    -f state=all -f creator="$ISSUE_AUTHOR" -f per_page=100 \
    --jq "[.[] | select(.title == env.ISSUE_TITLE and .user.login == \"$ISSUE_AUTHOR\" and (.pull_request | not))]
          | sort_by(.number) | first // empty"
}

issue_number() {
  jq -r '.number // empty'
}

issue_body_text() {
  jq -r '.body // empty'
}

tested_commit_in() {
  sed -n 's/.*<!-- tested: \([0-9a-f]\{40\}\) -->.*/\1/p' | head -n 1
}
