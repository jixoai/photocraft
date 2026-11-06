# WebMCP reliability work

Updated: 2026-11-06.

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

Browser-download triggering returned success, but the browser automation download-event wait
timed out, so receipt of that download is **not verified**. The user's takeover then stopped
browser work; no control was reclaimed or existing user tabs closed. Browser reimport was not
completed after takeover; native 8/16/32-bit project reimport/editability regressions did pass.
The final independent drawing agent and user still perform the held-out visual acceptance.
