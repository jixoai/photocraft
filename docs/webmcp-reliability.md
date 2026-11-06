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

Pending. Start with bounded, time-budgeted client chunks and stop on the first error.
Add durable execution progress/request identity only with explicit semantics for partial
completion and safe replay. An outer browser automation timeout does not cancel editor
work. Existing headless `batch` is not currently exposed by the WebMCP control whitelist;
do not assume it is a browser transaction or reuse it without checking browser semantics.

Acceptance: interrupt a multi-step edit, query its completed prefix and continue without
duplicating layers or strokes. Unknown outcomes require inspection, never blind replay.

## 3. Web document recovery

Pending. IndexedDB currently persists presets, not a document-recovery checkpoint stream.
Define bounded storage, dirty-state semantics, retention, quota errors and explicit restore
selection before adding checkpoints. A changed browser task/tab is not proof of an app crash.

Acceptance: reload an isolated test document and restore the latest completed checkpoint;
storage failures must leave the current document usable and visible to the user.

## 4. Editable drawing workflow

Pending. Prefer existing `shape.create`, `shape.edit`, `shape.info`, groups and transformations
over rasterizing every object into a small number of broad layers. Stable IDs are obtained
from results, not guessed. Teach the workflow in a general-purpose skill before adding
engine features already available in the command registry.

Acceptance: revise one object without reconstructing unrelated objects; verify project
roundtrip, undo/redo and exported appearance at multiple document bit depths.

## 5. Visual feedback and independent evaluation

Pending. Minimize inspect payloads, deliver images as images rather than base64 transcript
text, and review composition before adding details. Use independent, previously unseen
subjects and keep same-subject examples/old exports outside the acceptance agent's context.

Measure tool success, visual quality, editability, recovery and instruction compliance
separately. A command count or layer count alone is not a visual-quality acceptance gate.
