#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
cd "$ROOT"
reject() {
  local pattern="$1"
  shift
  local result=0
  rg -n "$pattern" "$@" || result="$?"
  if [[ "$result" != 1 ]]; then
    echo 'FAIL: upstream promotion or invalid branding audit input' >&2
    exit 1
  fi
}
reject 'getartcraft|discord\.gg|Learning Machines|storyteller\.ai' README.md apps packaging book docs/user docs/releasing.md crates/ui-egui/src/links.rs crates/ui-egui/src/lib.rs
reject 'storytold/photocraft' crates/engine/src/brand.rs crates/engine/src/cmd/updates_cmds.rs crates/ui-egui/src/links.rs crates/ui-egui/src/updates_ui.rs apps/photocraft/src/updates.rs
reject '^[^#].*(ArtCraft|Discord)' crates/ui-egui/src/i18n --glob '*.tsv'
for html in apps/photocraft-web/index.html apps/photocraft-web-boot/index.html; do
  rg -q '<title>FDE2E Photocraft</title>' "$html"
done
for mark in artcraft-logo.svg artcraft-logo-white.svg artcraft-mark.svg artcraft-mark-black.svg; do
  [[ ! -e "docs/brand/$mark" ]]
done
for notice in LICENSE-MIT LICENSE-APACHE NOTICE ATTRIBUTION.md docs/brand/LICENSE-brand.txt assets/app-icon/LICENSE.txt; do
  [[ -s "$notice" ]]
done
rg -q '^# FDE2E Photocraft$' README.md
rg -q 'based on.*PhotoCraft' README.md
rg -q 'Copyright \(c\) 2026 ArtCraft Team' NOTICE
rg -q 'jixoai/photocraft' crates/engine/src/brand.rs
rg -Fq '.with_pins_path(std::path::PathBuf::new())' crates/ui-egui/src/updates_ui.rs
reject 'filesystem=xdg-data/(ArtCraft|FDE2E)/versions' packaging/linux/flatpak
reject 'FDE2E PhotocraftData' docs/user book
echo 'PASS: downstream branding, support/update destinations, excluded upstream marks and retained attribution.'
