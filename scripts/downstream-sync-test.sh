#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
FIXTURE="$(mktemp -d)"
trap 'find "$FIXTURE" -depth -delete' EXIT
mkdir -p "$FIXTURE/source/scripts" "$FIXTURE/source/target/downstream-validation/full"
cp "$ROOT/scripts/downstream-sync.sh" "$FIXTURE/source/scripts/"
cd "$FIXTURE/source"
git init -q -b main
git config user.name 'PhotoCraft test'
git config user.email 'test@example.invalid'
printf '/target/\n' > .gitignore
printf '#!/usr/bin/env bash\nexit 7\n' > scripts/downstream-check.sh
git add .
git commit -qm initial
git switch -qc agent-web
git init -q --bare "$FIXTURE/fork.git"
git init -q --bare "$FIXTURE/upstream.git"
git remote add origin "$FIXTURE/fork.git"
git remote add upstream "$FIXTURE/upstream.git"
git push -q origin main agent-web
git push -q upstream main
git fetch -q upstream main
BASE="$(git rev-parse agent-web)"
refuse() {
  if bash scripts/downstream-sync.sh "$1" > "$FIXTURE/last.log" 2>&1; then
    echo "FAIL: accepted unsafe $1" >&2
    exit 1
  fi
  [[ "$(git rev-parse agent-web)" == "$BASE" ]]
}
touch untracked
refuse prepare
rm untracked
git switch -qc sync/upstream-test
refuse promote
printf 'stale\n' > target/downstream-validation/full/passed-revision
refuse promote
git switch -q agent-web
bash scripts/downstream-sync.sh prepare > "$FIXTURE/current.log" 2>&1
[[ "$(git branch --show-current)" == agent-web ]]
git switch -q main
printf 'upstream\n' > upstream.txt
git add upstream.txt
git commit -qm upstream
git push -q upstream main
git switch -q agent-web
refuse prepare
[[ "$(git branch --show-current)" == sync/upstream-* ]]
git merge-base --is-ancestor upstream/main HEAD
refuse promote
printf '%s\n' "$(git rev-parse HEAD)" > target/downstream-validation/full/passed-revision
bash scripts/downstream-sync.sh promote > "$FIXTURE/promote.log" 2>&1
[[ "$(git branch --show-current)" == agent-web ]]
git merge-base --is-ancestor upstream/main agent-web
[[ "$(git --git-dir="$FIXTURE/fork.git" rev-parse agent-web)" == "$BASE" ]]
BASE="$(git rev-parse agent-web)"
git switch -qc sync/upstream-remote-change
printf '%s\n' "$(git rev-parse HEAD)" > target/downstream-validation/full/passed-revision
git push -q origin main:agent-web
refuse promote
[[ "$(git branch --show-current)" == sync/upstream-remote-change ]]
cp "$ROOT/scripts/downstream-check.sh" scripts/downstream-check.sh
mkdir -p target/downstream-validation/quick/running
if bash scripts/downstream-check.sh quick > "$FIXTURE/locked.log" 2>&1; then
  echo 'FAIL: overlapping validation was accepted' >&2
  exit 1
fi
grep -q 'Another quick gate is running' "$FIXTURE/locked.log"
if bash scripts/downstream-check.sh invalid > "$FIXTURE/arguments.log" 2>&1; then
  echo 'FAIL: invalid validation mode was accepted' >&2
  exit 1
fi
echo 'PASS: dirty/missing/stale/failed gates cannot promote; a passing revision fast-forwards locally without pushing.'
