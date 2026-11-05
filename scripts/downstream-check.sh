#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
MODE="${1:-quick}"
if [[ "$#" -gt 1 || ( "$MODE" != quick && "$MODE" != full ) ]]; then
  echo 'usage: bash scripts/downstream-check.sh [quick|full]' >&2
  exit 2
fi
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target/web-validation}"
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export RAYON_NUM_THREADS="${RAYON_NUM_THREADS:-2}"
LOG_DIR="$ROOT/target/downstream-validation/$MODE"
mkdir -p "$LOG_DIR/tmp"
if ! mkdir "$LOG_DIR/running" 2>/dev/null; then
  echo "Another $MODE gate is running; if it crashed, remove the empty $LOG_DIR/running directory before retrying." >&2
  exit 1
fi
trap 'rmdir "$LOG_DIR/running"' EXIT
export TMPDIR="$LOG_DIR/tmp/"
rm -f "$LOG_DIR/passed-revision"
REVISION="$(git rev-parse HEAD)"
INITIAL_STATUS="$(git status --porcelain)"
{
  printf 'mode=%s\nrevision=%s\n' "$MODE" "$REVISION"
  printf 'upstream=%s\n' "$(git rev-parse upstream/main 2>/dev/null || printf unavailable)"
  printf 'platform=%s\n' "$(uname -sm)"
  printf 'build_jobs=%s\nrayon_threads=%s\n' "$CARGO_BUILD_JOBS" "$RAYON_NUM_THREADS"
  df -Pk "$ROOT"
  rustc -vV
  cargo --version
  shasum -a 256 Cargo.lock maintenance/command-baseline.json
} > "$LOG_DIR/provenance.txt"
step() {
  local name="$1"
  shift
  printf '\n== %s ==\n' "$name"
  if "$@" > "$LOG_DIR/$name.log" 2>&1; then
    printf 'PASS %s\n' "$name"
  else
    local result="$?"
    tail -60 "$LOG_DIR/$name.log" >&2
    printf 'FAIL %s (exit %s); logs: %s\n' "$name" "$result" "$LOG_DIR" >&2
    return "$result"
  fi
}
step fmt cargo fmt --all -- --check
step maintenance cargo test --locked -p xtask downstream::
step sync-safety bash scripts/downstream-sync-test.sh
step contracts cargo test --locked -p photocraft-engine --lib cmd::schema::
step drawing cargo test --locked -p photocraft-engine --test background_drawing
step web cargo test --locked -p photocraft-web --bin photocraft-web
step mcp cargo test --locked -p photocraft-automation --test it mcp::
step packaging bash packaging/web/package-test.sh
export_inventory() {
  cargo run --locked --quiet -p photocraft-cli -- commands --json > "$LOG_DIR/commands.json"
}
step export export_inventory
step contract-baseline cargo xtask downstream check "$LOG_DIR/commands.json" maintenance/command-baseline.json "$LOG_DIR/contracts.json"
step layers cargo xtask layers
if [[ "$MODE" == full ]]; then
  step native cargo test --locked -p photocraft-raster -p photocraft-engine -p photocraft-ui-egui -p photocraft-automation
  step clippy cargo clippy --locked -p xtask -p photocraft-raster -p photocraft-engine -p photocraft-ui-egui -p photocraft-web -p photocraft-automation --all-targets --no-deps -- -D warnings
  step wasm cargo xtask wasm
  step wasm-app cargo check --locked --target wasm32-unknown-unknown -p photocraft-web -p photocraft-web-boot --features photocraft-web/heif
  threaded_check() {
    env RUSTUP_TOOLCHAIN="${PHOTOCRAFT_WEB_THREADS_TOOLCHAIN:-nightly-2026-08-17}" \
      CARGO_TARGET_DIR="${PHOTOCRAFT_WEB_THREADS_TARGET_DIR:-$ROOT/target/web-threaded}" \
      RUSTFLAGS='-C target-feature=+atomics,+bulk-memory -C link-arg=--max-memory=2147483648' \
      cargo check --locked --release -Z build-std=std,panic_abort --target wasm32-unknown-unknown -p photocraft-web --features heif,threads
  }
  step wasm-workers threaded_check
  step panic-hunt cargo test --locked -p photocraft-engine --test panic_hunt -- --ignored
fi
if [[ -z "$INITIAL_STATUS" && -z "$(git status --porcelain)" && "$REVISION" == "$(git rev-parse HEAD)" ]]; then
  printf '%s\n' "$REVISION" > "$LOG_DIR/passed-revision"
  printf '\nPASS %s at %s\n' "$MODE" "$REVISION"
else
  echo 'Tests passed, but the source tree is dirty or changed; commit and rerun to authorize promotion.'
fi
printf 'Reports: %s\nBrowser drawing/PNG acceptance and measured browser performance remain separate release gates.\n' "$LOG_DIR"
