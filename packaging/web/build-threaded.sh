#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
TOOLCHAIN="${PHOTOCRAFT_WEB_THREADS_TOOLCHAIN:-nightly-2026-08-17}"
if ! rustup run "$TOOLCHAIN" cargo --version >/dev/null 2>&1; then
  echo "Install the worker build toolchain: rustup toolchain install $TOOLCHAIN --profile minimal --component rust-src" >&2
  exit 1
fi
export RUSTUP_TOOLCHAIN="$TOOLCHAIN"
export CARGO_TARGET_DIR="${PHOTOCRAFT_WEB_THREADS_TARGET_DIR:-$ROOT/target/web-threaded}"
export CARGO_UNSTABLE_BUILD_STD=std,panic_abort
export RUSTFLAGS="-C target-feature=+atomics,+bulk-memory -C link-arg=--max-memory=2147483648"
cd "$ROOT/apps/photocraft-web"
ENTRY="$(mktemp "$ROOT/apps/photocraft-web/.threaded-XXXXXX")"
HTML="$ENTRY.html"
trap 'rm -f "$ENTRY" "$HTML"' EXIT
sed 's/data-cargo-features="heif"/data-cargo-features="heif,threads"/' index.html > "$HTML"
grep -q 'data-cargo-features="heif,threads"' "$HTML" || { echo "error: threaded entry point has no worker feature" >&2; exit 1; }
trunk build "$HTML" --release --dist "$ROOT/dist/web-threaded"
grep -q 'wbg_rayon_start_worker' "$ROOT"/dist/web-threaded/photocraft-web*.js || { echo "error: worker exports missing from threaded build" >&2; exit 1; }
grep -Eq 'shared[[:space:]]*:[[:space:]]*true' "$ROOT"/dist/web-threaded/photocraft-web*.js || { echo "error: threaded build has no shared memory" >&2; exit 1; }
