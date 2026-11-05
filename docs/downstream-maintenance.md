# Maintaining the PhotoCraft agent/web downstream

Updated: 2026-11-05. This is the maintenance policy for the `jixoai/photocraft` fork,
not a change to upstream's contribution or CI policy.

## Sources and branches

| Ref | Purpose |
| --- | --- |
| `upstream/main` | Fetched official `storytold/photocraft` history. The local upstream push URL is disabled. |
| `origin/main` | Official main mirrored in the fork, with no downstream commits. |
| `agent-web` / `origin/agent-web` | The enhanced product and its release source. |
| `sync/upstream-*` | Disposable integration branches, retained until their results are reviewed. |
| `feature/*` | Focused future fixes, contracts, adapters or performance improvements. |

`origin` is `https://github.com/jixoai/photocraft.git`. `upstream` is
`https://github.com/storytold/photocraft`. The initial source checkpoint is tagged
`agent-web-baseline-20261105`; it predates the first upstream merge and is **not** a
new browser acceptance or performance measurement.

Remote configuration is local and is not carried by a clone. In a fresh clone of the fork:

```sh
git remote add upstream https://github.com/storytold/photocraft.git
git remote set-url --push upstream DISABLED
git config --replace-all remote.upstream.fetch '+refs/heads/main:refs/remotes/upstream/main'
git config remote.pushDefault origin
git fetch --no-tags upstream main
git switch --track origin/agent-web
git config branch.agent-web.pushRemote origin
git config branch.agent-web.rebase false
```

Keep published product history: merge upstream into an integration branch, validate,
then fast-forward `agent-web`. Rebase only unpublished feature work. Never force-push
the product or mirror branches, automatically resolve conflicts with `ours`/`theirs`,
or reset a dirty tree. Before synchronizing, commit and publish all intended source,
including previously untracked Rust modules, tests and documentation.

## Integration boundaries

- Raster, engine and compositor fixes belong in their owning Rust layers. Keep the web
  worker availability/spawn boundary in `crates/engine/src/background.rs`; native and
  threaded-browser jobs must retain cancellation, draining, scratch release and admission
  limits when upstream changes the job system.
- Command descriptions and schemas belong in `crates/engine/src/cmd/schema/`, discovered
  through the engine registry. CLI, control, native MCP and WebMCP reuse this source.
  New upstream commands remain discoverable with honest `partial` contracts until audited.
- `apps/photocraft-web/src/agent.rs` owns the six tool envelopes;
  `apps/photocraft-web/src/webmcp.rs` adapts browser registration and transport. Do not
  duplicate the full command catalogue or put drawing/engine algorithms in this adapter.
- Composition, layer planning, iterative visual inspection and task-specific technique
  belong in skills. A pelican illustration is an acceptance example, not a special case
  in production rendering or a substitute for varied regression scenarios.
- Upstream generic fixes in focused PRs when appropriate. Once equivalent behavior is
  upstream, review and retire redundant downstream code instead of maintaining both.
  Keep shared `craft-libs` revisions immutable; do not add unrelated dependency updates
  while resolving a synchronization conflict.

No downstream workflow changes `.github/workflows/`. Local checks do not install a
scheduled sync, guarantee future merges, or configure a hosting provider's deployment branch.

## Routine upstream synchronization

Start on a clean, published `agent-web`:

```sh
git switch agent-web
bash scripts/downstream-sync.sh status
bash scripts/downstream-sync.sh prepare
```

`status` is offline and reports cached refs. `prepare` fetches official main and the
fork's product branch, refuses an unpublished/diverged local product branch, creates
`sync/upstream-<upstream-sha>`, merges without rewriting history, and runs the full local
code gate. It never resets an existing integration branch, promotes or pushes a result.

If the merge conflicts, resolve them on the integration branch, commit, then run:

```sh
bash scripts/downstream-check.sh full
```

If checks fail, keep `agent-web` at its last checkpoint. Fix integration-caused failures;
report unrelated failures rather than weakening the gate or silently skipping tests.
`git merge --abort` is an option only while the merge itself is still unresolved.

After reviewing the merge and its reports:

```sh
bash scripts/downstream-sync.sh promote
git push origin agent-web
```

Promotion requires a clean tree and a passing **full** marker for the exact current
commit. It verifies ancestry and re-fetches the remote product branch before a local
fast-forward. Publication is a separate non-force push. A marker is local workflow
evidence, not a signed attestation or a browser acceptance certificate.

Mirror official main separately, without downstream commits:

```sh
git fetch --no-tags upstream main
git fetch --no-tags origin main
git merge-base --is-ancestor origin/main upstream/main
git push origin upstream/main:refs/heads/main
```

A non-fast-forward rejection requires investigation, never `--force`. To move the local
`main`, switch to it and merge `upstream/main --ff-only`, then return to `agent-web`.
If another worktree owns `main`, update it there. Hosting and manual release builds must
select `agent-web`, not the unmodified mirror.

## Local gates and contract review

```sh
bash scripts/downstream-check.sh quick
bash scripts/downstream-check.sh full
```

The default cache is `target/web-validation`; override `CARGO_TARGET_DIR` if needed.
Logs and inventory/provenance reports go to `target/downstream-validation/<mode>/`.
Tests use a canonical, isolated temporary directory there to avoid macOS `/var` alias
differences. Existing source changes can be tested, but dirty/changed trees receive no
promotion marker. A quick marker cannot authorize promotion.
Two checks of the same mode cannot share a log directory concurrently. If a process was
forcibly terminated, confirm it is gone before removing its empty `running` directory.

- **Quick:** formatting, maintenance unit/sync-safety tests, schema/brush regressions,
  general drawing regression, browser tool tests, native MCP integration, synthetic web
  packaging, current command export/contract comparison, and crate layering.
- **Full:** quick plus raster/engine/UI/automation crate tests, changed-crate strict
  no-deps Clippy, the library Wasm gate, web app/boot Wasm checks including HEIF, and the
  threaded-Wasm release-profile check using the pinned worker toolchain, plus the
  adversarial command panic hunt. Stop at the first failure; inspect the named log.

`maintenance/command-baseline.json` protects every existing command ID and the exact
coverage of audited commands. For complete commands it hashes the canonical schema,
semantic description and journal flag, including nested `$defs`; dynamic enablement is
excluded. New commands are allowed and reported, so adding upstream partial commands
does not fail an arbitrary fixed command count or percentage floor. Removals, audited
downgrades and audited metadata changes require review. Changed metadata is not
automatically a breaking change, and unchanged metadata cannot prove unchanged execution:
runtime tests remain necessary.

The original baseline was generated from the pre-merge 964-command inventory. After
reviewing the first integration it can advance to include new command IDs and the upstream
duplicate-layer link semantics; original audited commands must not be downgraded. To
compare a fresh inventory or explicitly record a reviewed replacement:

```sh
cargo run --locked --quiet -p photocraft-cli -- commands --json > target/commands.json
cargo xtask downstream check target/commands.json maintenance/command-baseline.json target/contracts.json
cargo xtask downstream baseline target/commands.json maintenance/command-baseline.json --replace
git diff -- maintenance/command-baseline.json
```

Do not run the replacement command merely to turn a failure green. Audit changed
execution, update representative tests and semantic documentation, and explain the
baseline diff in the same commit. Contracts do not add default insertion or dispatch
rejection. Keep the existing `complete`/`partial` distinction and brush deep-patch rules.

## Release gates beyond code integration

Full code checks do **not** establish fresh visual acceptance, browser responsiveness,
or successful real threaded-Wasm linking. Before publishing a new web package:

1. Build compatibility and threaded releases using `packaging/web/README.md`; run the
   package regression and inspect the generated ZIP, worker modules, isolation headers,
   decompressed payloads and hosting size limits. Do not reuse an old ZIP as new evidence.
2. Run an independent browser agent through real WebMCP: discover tools/contracts,
   create/edit several subjects, inspect and revise, undo/redo, preserve the visible
   drawing, export PNG and `.pcraft`, then reopen the project. Cover 8/16/32-bit workflows
   and the compatibility fallback. Record browser/build identity and actual outcomes.
3. On the same machine and representative documents, compare operation latency, peak
   memory and main-thread stalls for worker and compatibility builds against the last
   accepted package. Record environment/noise; previous single-run smoke timings are
   not enforceable performance budgets. Run applicable existing perf/corpus/parity gates
   under `AGENTS.md`; report existing failures and do not claim an unmeasured improvement.
4. Archive upstream and downstream commit IDs, `Cargo.lock` hash, toolchain versions,
   build flags, contract report, acceptance evidence and package SHA256. Tag the reviewed
   source only after its required release gates pass. Keep a previous package for rollback.

Acceptance images/projects and large generated inventories stay local/archived, not in
source history: `.gitignore` excludes them without deleting them. Source, compact baseline,
tests and documentation are committed. Browser tools always discover live contracts;
do not feed a multi-megabyte generated catalogue into every agent turn.
