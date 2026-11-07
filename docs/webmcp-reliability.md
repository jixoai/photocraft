# WebMCP reliability work

Updated: 2026-11-07.

This work follows an independent browser drawing-session review. Improvements must apply
to arbitrary subjects, not a particular illustration. Desktop-only test failures are not
part of the Web feature scope. Each independently validated change gets its own commit.

## 1. Error diagnostics

Implemented in source: additive `errorInfo`, field-level tool-envelope validation, resolved
application error responses, conservative execution/retry status, and a non-mutating
`window.photocraft.validateTool` diagnostic for native browser pre-callback errors.

Regression cases include an out-of-range discovery limit, missing control method, invalid
inspect/export arguments, invalid encoded files and unsafe filenames. Error enrichment
preserves legacy error strings and other response fields. Control-dispatch abandonment
reports whether a queued operation was prevented, rather than parsing an error string.

Native tests and a Wasm check are required before committing. A rebuilt browser acceptance
must separately verify native WebMCP and same-page error delivery; source changes do not
update a running tab or an existing release package.

## 2. Resumable execution

Implemented in source: optional paired request/page IDs on execute, immediate admission,
out-of-band status queries, bounded completion receipts and duplicate/conflict detection.
The ledger is page-local and never silently evicts IDs. A new page rejects the old instance;
unknown outcomes are terminal rather than automatically replayed. This is not durable
execution state across reloads or a transaction. A reusable WebMCP skill covers host-side
plan checkpoints, short polling windows and stop-on-error reconciliation.

An outer browser automation timeout does not cancel editor work. Existing headless `batch`
is not currently exposed by the WebMCP control whitelist; do not assume it is a browser
transaction or reuse it without checking browser semantics.

Acceptance: interrupt a multi-step edit, query its completed prefix and continue without
duplicating layers or strokes. Unknown outcomes require inspection, never blind replay.

## 3. Web document recovery

Implemented: atomic IndexedDB `.pcraft` + metadata checkpoints, separate from brush presets.
The existing autosave service checks dirty revisions every 30 seconds while autosave is on;
the browser may throttle background tabs. Manual `recovery.checkpoint` is also available.
Acknowledgements mean queued, not durable: poll `recovery.list` for the matching revision,
an empty pending/queued set and no error. Encoding uses the engine job path and COW snapshots.

Storage is capped at 16 documents, 64 MiB per project and 256 MiB total, including metadata.
No silent eviction. Quota/denial/timeout keeps the old atomic checkpoint and the live editor;
the recovery panel warns the user to export. Browser eviction or cleared site data can still
remove checkpoints; they are not a substitute for downloaded project copies.

Every page owns unique snapshot keys. Explicit restore creates a fresh, dirty document copy,
keeps the original stored snapshot and synchronizes editor views before rendering. Closing
the copy never automatically removes another page's source. Owned snapshots are removed when
their document is saved or closed; foreign entries require explicit confirmed deletion.
The catalog is loaded at startup; another tab's later writes are discovered on a new page.
Recovery does not revive the old page-local execution ledger or authorize replaying an old plan.

Acceptance: reload an isolated test document and restore the latest completed checkpoint;
storage failures must leave the current document usable and visible to the user.

Verified on an isolated local origin: all six native `document.modelContext` tools registered,
a 16-bit vector checkpoint survived reload and opened as a dirty editable copy, and an injected
second-record quota exception retained the old stored metadata/revision while the live shape
remained editable. Explicit retry completed a later checkpoint. Browser testing found and fixed
a missing view synchronization after inline restore. Native recovery regressions include the
status-bar render, ownership, ordered cleanup, validation and 8/16/32-bit project roundtrips.
Final rebuilt-browser checks also cover the monotonic autosave scheduler and restored name.

## 4. Editable drawing workflow

Implemented as a general-purpose skill using existing commands, not a subject-specific engine
generator. Semantic decomposition, appropriate live primitives/Bézier paths, grouping/overlap,
returned IDs, local patches, unchanged-neighbor checks, staged image review and project-copy
delivery are agent strategy. Precise shape mutation, shared history and editable project
serialization remain application capabilities and receive real control-path regression tests.

The Web suite revises an explicitly addressed shape without changing its backdrop/accent,
checks undo/redo against exported PNG bytes, and reopens `.pcraft` at 8/16/32 bits with vector
types and exported appearance preserved. The reopened object can be edited again. No new
engine drawing feature or scene template is required for these already-supported operations.

Acceptance: revise one object without reconstructing unrelated objects; verify project
roundtrip, undo/redo and exported appearance at multiple document bit depths.

## 5. Visual feedback and independent evaluation

Implemented: additive `photocraft_inspect` scopes (`session`, compact `document`, paginated
metadata `layers`/`history`, running `jobs`) bypass full UI serialization. Default `{}` still
returns the original complete UI inspection. Layer pagination uses bounded-depth iterative
traversal and does not compute raster bounds or include vector path payloads. Responses carry
document identity/revision so agents can reject mixed-revision pages. Dedicated object-info
commands retain access to editable details. A synthetic 8-layer test measures about 200 bytes
for compact document metadata against 10.4 KB for full UI state (about 98% less payload), without
changing the document. This compares different projections, not equivalent-data compression
or a claim about renderer speed.

The skill uses existing PNG export/editor screenshots for actual image feedback rather than
adding another renderer. It teaches bounded composition/silhouette/detail reviews and actual
image delivery, not base64 transcript text. [Independent acceptance](webmcp-acceptance.md)
defines a fresh-brief prompt template, held-out comparisons and separate reliability,
integrity/recovery, visual communication, delivery and instruction-compliance evidence.
No fixed scene or previous acceptance exports are supplied to the evaluating agent.

Measure tool success, visual quality, editability, recovery and instruction compliance
separately. A command count or layer count alone is not a visual-quality acceptance gate.
The protocol and technical browser checks prepare independent acceptance; they do not replace
the user's final visual review or claim that another agent has already judged unseen artwork.

Integrated browser verification (isolated local release build): native WebMCP discovery,
tracked admission/duplicate submission/status completion, compact/full inspection and layer
pagination, original-name 16-bit restore, a new automatically scheduled checkpoint, local shape
editing with an unchanged neighbor, and undo/redo all passed. Native PNG/project export returned
bytes saved as local artifacts, and the PNG was opened and visually inspected. These simple
geometry artifacts validate transport/document behavior only, not artistic quality.

The browser automation download-event wait timed out while download authorization was pending.
The user subsequently confirmed that granting authorization completed the download. Follow-up
local verification also found the earlier browser-downloaded `download-verification.png`: a valid
240×160, 16-bit PNG whose SHA-256 matched the base64 export artifact. This distinguishes a
permission-gated download from an export failure; no export-code change was needed for this case.
The later download's completion is user-confirmed, not an automatically captured event receipt
or a separate file-content check. Browser work stopped on user takeover; no control was reclaimed
or existing user tabs closed. Browser reimport was not completed after takeover; native
8/16/32-bit project reimport/editability regressions did pass. The final independent drawing
agent and user still perform the held-out visual acceptance.

## 6. Independent drawing follow-up

An independent native-WebMCP drawing session exposed a contract/runtime mismatch:
`shape.edit` advertised `kind: "path"`, but dispatched it to the live-primitive builder,
which rejected that kind. Explicit path edits now require `path`, use its Bézier geometry
without live-primitive regeneration, and retain the normal paint/transform/history behavior.
Regression coverage includes schema/runtime agreement, unchanged neighbors and undo/redo at
8/16/32 bits, malformed or missing paths without mutation, and the shared Web control path.
This is a subject-independent authoring fix, not a scene generator or a visual-quality claim.

The same session exposed wrong-target risk when `id` was supplied instead of `layer` and
ignored. Shared engine precheck/execute/start now reject unknown top-level keys in complete,
closed schemas. Partial contracts remain advisory; intentional omission of `layer` retains
active/selected-layer defaults. WebMCP uses the same property catalogue to reject these keys
before queue/tracked admission, with a field path and `execution: "not_started"`. Tests cover
query/edit/delete mistakes, unchanged document/history/revision/journal, explicit targets,
intentional defaults, partial compatibility and direct Web control dispatch. This does not
claim full runtime JSON Schema validation of nested values or state-dependent constraints.

Follow-up validation: 3,395 engine/library/integration tests and 40 Web tests passed, along
with the opt-in adversarial command hunt, 31-crate layering and the 24-library Wasm gate
including HEIF. Native changed-crate all-target strict Clippy and Web-only Wasm strict Clippy
passed. Engine Wasm strict Clippy was blocked at that checkpoint by three unused constants in
display-colour/LUT code; the 2026-11-07 full-UI follow-up below resolves their platform gating. The reviewed 964-command
baseline changes only `shape.edit` and the previously omitted `document.inspect.document`
parameter. Source/test success does not update an already-open browser tab or prove a new
independent drawing run; retain the existing acceptance artwork and open it unchanged.

## 7. Semantic editing and isolated delivery/recovery follow-up

The skill now records compact multipart-object membership and component roles, distinguishes
whole-object from component-only edits, preserves tonal variants, and checks protected
neighbours outside the intended edit closure. Real grouping is verified where appropriate;
a host manifest is not claimed to be an application group. Coordinated requests are not
atomic: undo/redo must cover the actual completed history steps in reverse/forward order.
This is reusable agent strategy, not a new scene-specific engine feature.

On 2026-11-07, an isolated `http://localhost:8788` native-WebMCP probe used a 480×320 RGB16
document with editable text, a real three-child group, a separate occluder and a support.
Three coordinated recolour requests preserved geometry and protected neighbours. Three
undos restored the original PNG exactly; three redos restored the revised PNG exactly.
The normal-operation plan completed all 20 tracked requests without duplicate IDs.

A revision-20 checkpoint was confirmed committed with idle queues and no error. Reloading
only the disposable page retained that exact key/revision and changed the page instance.
Explicit `recovery.restore` opened a dirty copy, retained the source snapshot and preserved
the full layer hierarchy, vector data, editable text and 16-bit format. The source undo stack
was not restored. A read-only query with the old instance returned `INSTANCE_MISMATCH`.
Four new tracked requests edited and undid text and shape paint in the restored copy; PNGs
before reload, after restore and after those undo checks were byte-identical. Native project
exports before/after restore had identical layer manifests and valid ZIP contents.

For PNG, a download wait was armed before the native `download: true` export. The event was
observed, the artifact saved in the same browser round, and completion reported no failure.
The downloaded file decoded as 480×320 RGB16 and matched the base64 baseline byte-for-byte.
No permission bypass or browser policy change was used. This proves the observed PNG path,
not every browser's permissions or an approval flow.

The subsequent `.pcraft` browser export returned success, but its event wait timed out.
Browser automation then reported user control; no takeover, retry, replacement space or
cleanup was attempted. No completed project download was verified, and the timeout's cause
was not established. The valid host-saved project is separate delivery evidence, not a pass
for that browser download. The reusable [probe procedure](../.agents/skills/photocraft-webmcp/references/delivery-recovery.md)
keeps these observations separate. Local evidence is in
`target/acceptance/webmcp-semantic-reliability-20261107/`; it is synthetic technical evidence,
not an independent illustration or a claim of general artistic improvement.

## 8. Independent parallel task workspaces

Implemented opt-in validated URL scopes, per-scope recovery/preset databases and preferences,
and an exclusive Web Lock acquired before storage/editor startup. Conflicting writers block
without takeover; invalid/duplicate scopes or missing Web Locks fail closed. Legacy unscoped
storage and calls remain compatible, without claiming that legacy pages are isolated.
Native `scope.status` and all six tool schemas expose scope identity. Explicit-scoped writes
require matching `scopeId`; supplied mismatches reject even discovery calls before ledger
admission. `ifDocument: {id, revision?}` on execute is checked at actual command dispatch and
included in tracked identity. It is not a transaction, job-completion guard or universal UI guard.

On 2026-11-07, two native-WebMCP pages on the same `http://127.0.0.1:8791` origin created and
edited independent 160×120 RGB16 documents concurrently. Both intentionally had document ID 1
and vector layer ID 3; task/page identities kept them separate. Wrong/missing scopes rejected
without mutation, and a rejected tracked ID was absent from the ledger. Stale revision guards
rejected before running; changed tracked guards conflicted. A's undo/redo PNGs matched their
respective baselines exactly while B's PNG/revision stayed unchanged.

Distinct preference values and brush presets were saved by real commands. Each scope showed
only its own library/checkpoint. Both committed checkpoints survived separate reloads with
rotated request instances, retained preferences/presets and explicit dirty-copy restoration.
Restored PNGs were byte-identical to their pre-reload counterparts; project ZIPs validated and
full editable-layer manifests matched. A third same-scope page blocked while the owner lived,
then acquired the lease after that disposable owner closed. Invalid/duplicate URLs and a
page-local simulated missing Web Locks API registered no editor/tools or fallback database.
The final release rebuild also verified an actionable unsupported-API message, successful
legacy unscoped editing without Web Locks, and same-scope contention across the rebuilt page.
Actual editor screenshots and restored PNGs were viewed. No existing artwork or user-owned
browser space was operated on. Local evidence: `target/acceptance/parallel-tasks-20261107/`.

Validation: 45 Web tests, 77 focused control tests, adversarial command hunt, native changed-crate
all-target strict Clippy, Web-only Wasm strict Clippy, layering and workspace Wasm/HEIF pass.
At that checkpoint the full UI library run was **not green**: 4740 pass, 29 fail, 21 ignored,
including font/glyph, README/title and stale-parameter fixtures. UI Wasm strict Clippy also
reported four cfg-specific warnings; ordinary Wasm checks passed. The follow-up below resolves
these failures rather than treating the focused controls as a substitute for the full suite.

This establishes the observed independent-task path, not hostile-agent security isolation,
unbounded capacity, distributed collaboration or improved artistic judgment. Machine/browser
resource and physical storage quotas remain shared. This probe uses `download: false` exports
to separate host directories; it does not claim browser-download completion. Serialize
download-event waits and obey browser-owned permission prompts. See the
[contract](webmcp.md#independent-parallel-tasks) and
[repeatable acceptance](webmcp-acceptance.md#parallel-task-acceptance).

## 9. Full UI reliability follow-up (2026-11-07)

The full UI suite now passes: 4,773 library tests and 177 integration tests, with 22 existing
opt-in tests ignored. The original 29 failures were not all model or environment problems:

- The Layers-panel trash marker leaked into strict engine parameters, breaking confirmation,
  remembered skip and automation paths. The shell consumes only its own marker; explicit layer
  targeting, stale-document protection and unknown-engine-parameter rejection remain tested.
- System glyph fallback was incorrectly disabled whenever optional embedded craft-fonts were
  absent. The per-character provider now works without that optional build input. Synthetic
  system-only first-frame/rebuild/weight tests keep strict missing-glyph policy; no fonts were
  committed and no tofu policy was substituted for glyph coverage.
- Font acceptance fixtures assumed a substring search had only one result. Keyboard acceptance
  now explicitly targets the requested family; production arrow navigation remains unchanged.
- New-document preset titles retain ordinary word wrapping but retry character wrapping before
  eliding longer localized product names. Danish and Chinese offscreen captures were inspected.
  The exact tool inventory/count contract lives in a README-linked reference instead of an
  obsolete upstream marketing section.
- Generated ICC profiles and XMP toolkit markers use FDE2E Photocraft. Legacy own-profile names
  and exact legacy XMP ownership markers remain supported; foreign sidecars are still protected.
  The generated CMYK profile was regenerated with its original CC0 attribution retained.
- Integration fixtures now query translated crop-overlay labels and menu access-key suffixes,
  and compare the downstream SVG icon with its actual editable shapes. The synthetic clip-mask
  import and independent pixel comparison still exercise the original SVG clipping behavior.

A separate low-disk rerun produced scratch warnings that replaced status text under test.
After reclaiming only inactive generated build caches, all 88 affected checks passed without
changing assertions or suppressing real disk warnings. Native scratch requires the larger of
1 GiB or 5% of volume capacity to remain free; see [development](development.md#faster-test-loops).

Strict native all-target Clippy and strict Wasm library/Web-binary Clippy now pass with dependency
linting enabled. Fixes gate native-only helpers/constants to their platform and end a real file
handle's scope before rename; they do not disable lints or remove native durability barriers.
Layering, the 24-library Wasm/HEIF gate, the adversarial command hunt and 3,395 full engine tests
also pass (21 existing opt-in engine tests ignored). The pinned corpus fetch verified every
configured digest. Its first release run exposed a genuine SVG parser stack overflow on an
explicit 1 MiB thread, which increasing the outer test-runner stack did not repair. The shared
SVG XML nesting bound is now 128 instead of 512. Files beyond it fail before parsing; embedded
SVG images share the remaining depth. The accepted boundary, previous crashing depths, shared
image budget and deepest image chain all pass small-stack release regressions (39 SVG tests).
The XML-only parser for KRA/ORA/XMP keeps an independent 512-level bound, verified on a 1 MiB
stack. SVG conversion's stricter recursion budget does not change these formats' own
unsupported-group-depth errors.
The final pinned IO corpus release run passes 552 library and 578 integration tests, with
12 existing opt-in tests ignored (including the separate multi-gigapixel export target).
It uses the normal test-runner stack, not a `RUST_MIN_STACK` workaround. The full UI suite
also passes again after this parser fix: 4,950 passed, zero failed, 22 existing ignored.

The quick performance gate also completes without scenario failures; the earlier missing-glyph
benchmark panic and absent P50/P80 rows are gone. This run reports P50 p95 3.01 ms and P80 p95
46.7 ms on Apple M2. Regression comparison is skipped against the different-machine M4 Pro
baseline; no baseline, parity floor or budget-enforcement flag was changed.

Evidence and detailed logs are local under `target/ui-repair/`. Offscreen verification does not
claim a new agent drawing, completed browser download or refreshed existing artwork session.
The final Web release was rebuilt and the local site's index switched atomically after copying
its assets; older hashed assets remain available for already-open artwork tabs.
