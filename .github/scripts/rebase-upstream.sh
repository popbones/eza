#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Popbones
# SPDX-License-Identifier: EUPL-1.2
set -euo pipefail

# Prepare a candidate only. The workflow publishes it after tests and a build.
upstream_url=${UPSTREAM_URL:-https://github.com/eza-community/eza.git}
upstream_branch=${UPSTREAM_BRANCH:-main}
output=${GITHUB_OUTPUT:-/dev/stdout}
summary=${GITHUB_STEP_SUMMARY:-/dev/null}

if [[ -n $(git status --porcelain) ]]; then
  echo 'Refusing to rebase a dirty checkout.' >&2
  exit 1
fi

before_sha=$(git rev-parse HEAD)
git fetch --no-tags "$upstream_url" "+refs/heads/$upstream_branch:refs/remotes/upstream/$upstream_branch"
upstream_sha=$(git rev-parse "refs/remotes/upstream/$upstream_branch")
printf 'before_sha=%s\nupstream_sha=%s\n' "$before_sha" "$upstream_sha" >> "$output"

if ! git rebase --no-fork-point "$upstream_sha"; then
  {
    echo '### Upstream rebase failed'
    echo
    echo 'The remote branch was not changed. Resolve the conflicts locally and push the repaired branch.'
    echo
    echo 'Conflicting paths:'
    git diff --name-only --diff-filter=U
  } >> "$summary"
  git rebase --abort || true
  echo '::error::Upstream rebase failed; the remote branch was not changed.' >&2
  exit 1
fi

after_sha=$(git rev-parse HEAD)
printf 'after_sha=%s\n' "$after_sha" >> "$output"
{
  echo '### Upstream rebase candidate'
  printf '\nBefore: `%s`\n\nUpstream: `%s`\n\nCandidate: `%s`\n' "$before_sha" "$upstream_sha" "$after_sha"
} >> "$summary"
