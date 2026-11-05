# Upstream sync checkpoint — 2026-11-06

**Status: maintenance tooling delivered; upstream product integration withheld.**

## Branches and scope

- The fork default is `agent-web`. Its editor/application code and build configuration
  remain identical to the source tag `agent-web-baseline-20261105` (`7815cdfd4`). Only
  maintenance tooling, its tests, contract baseline and documentation are added separately.
  Existing browser workers, drawing fixes, command contracts and WebMCP are preserved.
- `main` / `origin/main` mirror official `upstream/main` at `834e25337`, without downstream
  code. `upstream` has a disabled local push URL; product pushes go to the fork.
- `sync/upstream-20261105` retains the integration of 205 official commits, the browser job
  lifecycle/admission merge resolution and the reviewed duplicate-layer link semantics.
  Its tested checkpoint is `a5ee69c94`; it is not an accepted product or a web release.
- The product registry remains **964 commands: 135 complete, 829 partial**. The candidate
  discovers **990 commands: 135 complete, 855 partial**. All 26 new upstream commands are
  honestly partial; no audited command was removed or downgraded. Only the reviewed
  duplicate-layer semantic description changed its complete-contract signature.

## Candidate evidence

Passed at the tested integration checkpoint:

- 11 maintenance unit tests and synthetic Git sync/promotion safety scenarios.
- 25 schema regressions, 3 general drawing regressions, 18 web tests and 50 MCP discovery/
  execution integration tests, including nested brush definitions and project roundtrips.
- Native automation: 124 unit tests, 93 main integration tests, 9 real MCP wait/progress/
  cancellation tests and the scratch-save corruption regression; engine/raster targets
  passed before the UI suite failed. This is not a claim that every native target passed.
- Layering, 24-library Wasm plus HEIF, web/boot Wasm checks, pinned threaded-Wasm
  release-profile check, adversarial command panic hunt and synthetic web packaging.

Not passed, and **not waived**:

- Full UI library: **4,931 passed, 22 failed, 21 existing ignored**. Failures include missing
  Japanese/CJK glyphs, localized labels, font commit expectations, IME and panel interaction
  assertions. No `craft-fonts` checkout was supplied. Their exact causes need a separate UI/
  test-environment investigation; they are not classified as all caused by one font issue.
- Strict changed-crate Clippy stops at `crates/ui-egui/src/canvas/tool_cursor/pen.rs:95`
  (`manual_range_contains`). That file is unchanged from the official snapshot.

No passing full-revision marker was generated. The promotion helper refused the candidate;
it was not forced onto `agent-web`. The remaining UI/lint work is separate from setting up
fork maintenance, rather than silently weakening its checks.

The code gates bound build/Rayon workers to two and use a canonical temporary directory.
An upstream cancellation unit fixture was made deterministic with a cancellable synthetic
job instead of assuming a 36 MP blur would exceed a timeout. Production memory admission
and scratch reserves were not relaxed, and no failing test was newly ignored. A low-disk
scratch refusal was resolved by pruning disposable validation incremental caches with Cargo
stopped; free space recovered to about 61 GiB, and the real scratch regression then passed.

Local evidence remains in `log/downstream/` and `target/downstream-validation/full/`, including
the native failure log, exact contract diff, provenance and `integration-extra-gates-status.log`.
These logs are local artifacts, not checked-in generated command inventories.

## Resuming safely

From a clean, published `agent-web`, run `bash scripts/downstream-sync.sh prepare` for a
fresh upstream candidate. Alternatively, resume `sync/upstream-20261105`, reconcile the
latest `agent-web`, address the UI/lint prerequisites and run the full gate. Only an exact
clean commit passing every required code gate may be promoted and then pushed. Do not
replace missing success evidence with a hand-written marker or a skipped-test list.

Source maintenance is not a release acceptance: no fresh live-browser drawing, PNG export
or browser performance comparison was performed in this task. Existing drawings/projects
are retained locally. The prior `dist/brush-contracts-release/photocraft-web-0.20.1.zip` is
unchanged (SHA256 `f52c4bd731b91427049acc040a45be58794fc99e29097fd555625a0d76be72df`).
Deploy/rebuild from `agent-web`, never from the unmodified mirror or the held candidate
without its required acceptance.
