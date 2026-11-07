# General WebMCP drawing acceptance

Updated: 2026-11-07. This is a reusable evaluation protocol, not a reference illustration.

## Separate capability from strategy and model quality

Report these dimensions independently; never collapse them into a command or layer count:

| Dimension | Evidence | Failure owner |
| --- | --- | --- |
| Tool reliability | Discovered schemas, valid/invalid calls, tracked receipts, errors | Application/transport |
| Document integrity | Local edits, unchanged neighbors, undo/redo, editable project roundtrip | Application |
| Continuation/recovery | Reconnect without duplicate edits, explicit checkpoint restore | Application + workflow |
| Visual communication | Recognizable subject/action, silhouette, hierarchy, coherent style | Model + skill + available tools |
| Delivery | Real PNG and project bytes, retained result tab, image actually reviewed | Agent workflow |
| Instruction compliance | Constraints honored; prohibited assets/examples not used | Agent |

A reliable editor cannot supply the model's composition judgment automatically. Editable
objects, precise patches, shared history and feedback let a capable agent iterate and retain
control; they are advantages over a single flattened image, not a guarantee of better art.
Use primitives for control, not as an excuse to replace every silhouette with an ellipse.

## Semantic-object edit acceptance

Layer count is not semantic editability. For major multipart objects, save a compact manifest
of semantic names, component layer IDs and roles, plus bounds, front/back order, contact/support
relationships and protected neighbours where relevant to the edit.
This manifest is workflow evidence; it does not require a new editor command or a particular
grouping implementation.

For a whole-object edit, verify the complete intended component closure and its tonal variants,
rather than checking only the first matching layer or painting away shading. For a deliberately
component-only edit, name the component and explain why the other components remain separate.
Read the edited components and at least one protected neighbour outside the closure before
and after the change. Compare kind, path or live geometry, paint, transform and bounds, then inspect the
rendered result. A protected neighbour staying unchanged proves isolation, not that the
semantic object was edited completely. If the application exposes a real group hierarchy,
verify it from inspection; otherwise use the manifest and semantic names without claiming that
flat layers are grouped.
For multi-step edits, demonstrate undo/redo of the full completed closure in the actual history
order; several successful requests are not automatically an atomic edit.

## Avoid fitting to one request

Choose fresh briefs only after freezing the implementation and skill. Vary subject, action,
viewpoint, object overlap, aspect ratio, palette and target style. Include different families
such as an object diagram, environmental vignette, stylized character interaction and graphic
layout. Keep the actual prompts, previous scene scripts, exports and reference illustrations
out of the implementation context and out of the acceptance agent's examples. Do not publish
a fixed "golden scene" whose pixel similarity becomes the target.

For a comparison, hold model/version, budget, seed when applicable, browser build and brief
constant. Compare the baseline and improved workflow across multiple held-out briefs. Record
first-pass and revised results separately, elapsed time, failed calls, ambiguous retries,
inspection bytes and artifact size. Randomize/blind visual comparisons where feasible.
Report failures rather than replacing unsuccessful runs with a favorable sample.

Native synthetic geometry tests are technical regressions, not artistic acceptance. A new
acceptance agent and the human reviewer judge held-out artwork; development screenshots do
not count as an independent review. Desktop-only platform failures are reported separately.

## Agent prompt template

Replace the placeholders with a new brief and the actual editor URL/artifact folder:

```text
Use the already-open FDE2E Photocraft Web editor at <URL>. Read the photocraft-webmcp
skill and your browser skill. Operate through the browser's real native WebMCP tools;
verify the six registered tools. Do not substitute an external image generator, a
script-rendered finished bitmap, a hidden renderer or an imported finished illustration.
If native WebMCP is unavailable, report that blocker rather than silently substituting
the same-page API. Preserve all unrelated user tabs/documents and existing artifacts.
This is a drawing-only acceptance run: do not change project code or browser tool
registrations to make a failure disappear. On a missing TaskSpace or user takeover,
follow the browser skill's stop/recovery rules; do not create a replacement space to
bypass the failure. A generic instruction to continue does not override those rules.

Draw this new brief: <SUBJECT, ACTION, VIEWPOINT, STYLE, CONSTRAINTS, CANVAS SIZE>.
No previous scene examples or exports are supplied. Decide composition and silhouette,
then discover exact command schemas. Keep major semantic objects individually editable;
use paths/live shapes when appropriate and paint layers only where their material needs it.
Store actual returned IDs. Use tracked editing receipts, short bounded calls and checkpoints;
do not equate admission with completion or replay ambiguous mutations.
For major multipart objects, save a compact semantic component manifest. Define the full
component closure of the requested local edit; distinguish it from a component-only edit.
Read each needed command's inputSchema before its first call. On the current document API,
pass the discovered tool object to executeTool(toolObject, args), not a name/arguments wrapper.

Review actual image outputs after composition, main silhouettes and finishing. Correct
the weakest large-scale issue before adding tiny details. Make one requested local change
to a named object, verify its intended components and an unchanged protected neighbor outside
the edit closure, and demonstrate undo/redo of every completed step in that edit.
Export an editable .pcraft, reopen a copy and verify the main objects remain editable.
Verify a completed browser checkpoint and its restore in an isolated test page only;
never reload the only unsaved result or delete another page's recovery snapshot.

Keep the final artwork visible in its editor tab. Export and verify <FOLDER>/artwork.png,
<FOLDER>/artwork.pcraft and <FOLDER>/editor.png. The PNG must be the canvas artwork,
not an editor screenshot. Show the actual image preview, not base64 text. Report what was
verified, any remaining limitation, and score tool reliability, visual quality, editability,
recovery and instruction compliance separately. Do not claim human acceptance for yourself.
```

## Technical acceptance checklist

- Native `document.modelContext` or supported browser `navigator.modelContext` registers all
  six tools; no counterfeit implementation is installed. Same-page API tests are labeled.
- Bad envelope/command arguments fail without a crash; `errorInfo` identifies known fields
  and never promises a replay is safe when an operation may already have run.
- A lost host reply is reconciled by the same tracked ID/instance. Completed steps are not
  duplicated, and a changed page instance invalidates the old execution plan.
- Checkpoint acknowledgement is followed by a matching durable revision. Reload restoration
  opens a fresh dirty document copy, keeps the source, and does not recover stale receipts.
- Quota/denied storage keeps the active canvas usable and warns the user. No silent eviction;
  concurrent tabs use different owner keys. Test fault injection only on an isolated origin.
- Scoped inspection pages have coherent document ID/revision. Restart pagination if the
  document changes; metadata queries do not need full UI state, path data or raster scans.
- Local object edits preserve unrelated objects; 8/16/32-bit project roundtrips keep vector
  types and exported appearance. A saved project need not retain the in-memory undo stack.
- Actual downloaded PNG/project files decode, dimensions match, no error is hidden as an
  empty export, and the final requested editor tab stays open for human review.

For visual review, score 0–4 per category: subject/action readability, composition/negative
space, silhouette/proportion, overlap/depth, palette/value hierarchy, and stylistic coherence.
Write a concrete observation for each score. A total is descriptive, not a universal pass
threshold; the human brief decides acceptance. Keep application failures separate from a
model that used working tools poorly.
