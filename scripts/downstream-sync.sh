#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
if [[ "$#" != 1 ]]; then
  echo 'usage: bash scripts/downstream-sync.sh status|prepare|promote' >&2
  exit 2
fi
clean_tree() {
  if [[ -n "$(git status --porcelain)" ]]; then
    echo 'Refusing to synchronize a dirty tree; commit source, including untracked files, first.' >&2
    exit 1
  fi
}
case "$1" in
  status)
    git remote -v
    git status --short --branch
    git rev-list --left-right --count agent-web...upstream/main
    echo 'Divergence uses cached refs; prepare fetches current main and agent-web refs.'
    ;;
  prepare)
    clean_tree
    if [[ "$(git branch --show-current)" != agent-web ]]; then
      echo 'Start prepare on agent-web; an existing integration branch is never reset or overwritten.' >&2
      exit 1
    fi
    git fetch --no-tags upstream main
    git fetch --no-tags origin agent-web
    if [[ "$(git rev-parse HEAD)" != "$(git rev-parse origin/agent-web)" ]]; then
      echo 'agent-web and origin/agent-web differ; reconcile or publish the source checkpoint first.' >&2
      exit 1
    fi
    if git merge-base --is-ancestor upstream/main HEAD; then
      echo 'The fetched upstream/main is already included.'
      exit 0
    fi
    integration="sync/upstream-$(git rev-parse --short=12 upstream/main)"
    git switch -c "$integration"
    if ! git merge --no-ff --no-edit upstream/main; then
      echo 'Merge stopped. Resolve conflicts on this branch, commit, then run downstream-check.sh full. agent-web is unchanged.' >&2
      exit 1
    fi
    bash scripts/downstream-check.sh full
    echo 'Review the integration, then run: bash scripts/downstream-sync.sh promote'
    ;;
  promote)
    clean_tree
    integration="$(git branch --show-current)"
    if [[ "$integration" != sync/upstream-* ]]; then
      echo 'Promotion must start on a sync/upstream-* integration branch.' >&2
      exit 1
    fi
    marker="$ROOT/target/downstream-validation/full/passed-revision"
    if [[ ! -f "$marker" || "$(cat "$marker")" != "$(git rev-parse HEAD)" ]]; then
      echo 'No passing full gate for this exact clean revision; run downstream-check.sh full.' >&2
      exit 1
    fi
    git merge-base --is-ancestor agent-web HEAD
    git merge-base --is-ancestor upstream/main HEAD
    git fetch --no-tags origin agent-web
    if [[ "$(git rev-parse agent-web)" != "$(git rev-parse origin/agent-web)" ]]; then
      echo 'origin/agent-web changed; reconcile it and repeat validation before promotion.' >&2
      exit 1
    fi
    git switch agent-web
    git merge --ff-only "$integration"
    echo 'Promoted locally. Review, then publish with: git push origin agent-web'
    ;;
  *)
    echo 'usage: bash scripts/downstream-sync.sh status|prepare|promote' >&2
    exit 2
    ;;
esac
