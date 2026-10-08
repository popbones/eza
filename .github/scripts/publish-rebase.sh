#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Popbones
# SPDX-License-Identifier: EUPL-1.2
set -euo pipefail

: "${BEFORE_SHA:?Missing original branch head}"
: "${AFTER_SHA:?Missing validated candidate head}"
[[ $(git rev-parse HEAD) == "$AFTER_SHA" ]] || { echo 'Candidate HEAD changed.' >&2; exit 1; }

if [[ "$BEFORE_SHA" != "$AFTER_SHA" ]]; then
  previous_sha=$(git ls-remote origin refs/heads/popbones/previous | awk '{print $1}')
  git push --atomic \
    --force-with-lease="refs/heads/popbones/develop:$BEFORE_SHA" \
    --force-with-lease="refs/heads/popbones/previous:$previous_sha" \
    origin "$BEFORE_SHA:refs/heads/popbones/previous" HEAD:refs/heads/popbones/develop
fi
