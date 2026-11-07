# Editable drawing workflow

## Plan semantic objects, not a flattened scene

Translate the brief into a small object/relationship list: focal subject, supporting objects,
background planes, overlaps and negative space. Decide aspect ratio, camera/viewpoint, visual
hierarchy, palette and style before detailed geometry. This planning is an agent skill, not
a reason to add scene-specific commands or generators to the editor.

Use live primitives only when they fit the intended silhouette. For expressive or irregular
contours, discover `shape.create` with `kind: "path"` and its complete Bézier contract. Use
few meaningful anchor points and tangent handles rather than hundreds of noisy points.
Retain text as text and shapes as shapes; reserve paint layers for texture, shading or marks
that actually benefit from raster brushes. Do not import a finished whole-scene raster to
claim that the editor authored editable content.

Create background-to-foreground in a deliberate order, using semantic names. Group related
parts through discovered layer/group commands if helpful. Use returned IDs and inspect actual
hierarchy; do not assume a creation index, selection or group target. A path uses document
pixels, not screen coordinates; rectangle fields are extents, not the opposite corner.
Discover each command's schema rather than inferring fields from these workflow descriptions.

## Build and inspect at useful milestones

1. Block major silhouettes and relationships. Export a PNG preview and actually view it.
   Check readability at thumbnail scale, composition, proportions, overlap and negative space.
   For an action, identify the contact, support and direction of force that make it readable;
   for depth, check consistent plane directions, scale and ground contact. A named object or
   connected pair of paths is not by itself convincing action or perspective.
   Fix the weakest large relationship before decorating. Geometry density is not quality.
2. Refine the focal contour, values and supporting objects. Keep object IDs in the host plan.
   Query `shape.info` only for objects being revised; prefer scoped inspection when supported.
3. Add only details that survive the intended viewing size. Review one final image round,
   batch any concrete fixes, then stop. Do not spend an unbounded loop polishing tiny marks.
   Send image bytes to image output, never a base64 wall in the text transcript.

Use short ordered tool calls and request receipts. A visual milestone deserves both a completed
browser checkpoint and an exported project copy. Image review does not prove checkpoint
completion; a checkpoint does not prove that the image looks correct.

## Revise locally and prove editability

Read the selected object's `shape.info` and a neighboring object's info. Apply a minimal
`shape.edit` patch with an explicit layer ID. Preserve omitted fields and path handles; do not
recreate all layers or rewrite unrelated contours to change one color/position. Live-shape
geometry edits can regenerate a path; explicit path edits can remove live parameters, so
inspect which representation is present before choosing the patch. Use transforms only after
reading their units and matrix conventions.

Compare the neighbor before/after, inspect the changed object and view the resulting PNG.
Demonstrate shared undo/redo without mistakenly replaying a previously acknowledged request ID
as a new edit. An execution receipt remains succeeded after undo; it is not the current canvas.
Finally export `.pcraft`, reopen a copy and verify shape/text types, IDs within that copy,
bit depth and appearance. The original editor stays open for the user. Reopening an editable
document does not imply that its historical undo stack was serialized.

Assess tool reliability, editability and visual quality separately. These workflows improve
control and correction; they do not replace the model's design judgment or guarantee a style.
