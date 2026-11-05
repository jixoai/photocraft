# Nested brush contract audit

The 2026-11-05 increment audits 13 additional existing commands without changing
their dispatch, renderer, tool count or mutation validation:

- `paint.stroke`
- `tools.setBrush`
- `brush.get`
- `brush.presets.list`
- `brush.presets.save`
- `brush.presets.update`
- `brush.presets.delete`
- `brush.presets.setCurrent`
- `brush.defineFromSelection`
- `edit.fill`
- `layer.duplicate`
- `image.imageSize`
- `image.canvasSize`

The full inventory is `command-contracts.json`. Search by command ID rather than
loading the entire inventory into every model context. Brush-taking audited
commands carry local `$defs` for settings, dynamics, response curves, tips,
patterns and bitmap payloads, making every `$ref` self-contained.

## Semantics that must survive discovery

- Brush objects are recursive patches. Omitted fields preserve the existing
  setting; arrays replace it. Factory values used by reset are in
  `x-photocraft-initial-default`, not `default`; the live brush may differ.
- `tools.setBrush` applies preset, reset, nested patch, then root settings.
  `paint.stroke` applies preset, nested patch, then scalar overrides; stroke
  colours, erase and seed have command-specific overrides even when omitted.
- Section `enabled` flags are independent of child edits. Pose needs override
  flags; texture depth dynamics need `eachTip`; time-based behavior needs sample
  timestamps. Typed nested enums are case-sensitive serialized Rust variants.
- Bitmap/pattern same-variant patches may preserve omitted fields; new variants
  require complete payloads. Bitmap sample counts and variant merge validity
  depend on the current brush and remain runtime checks. Stored keys are not
  portable assets or URLs.
- Nested mixer percentages are fractions; the still-partial `paint.mixerBrush`
  command uses top-level percent overrides. One complete settings type does not
  certify all paint tools or preset import/export commands.

## Regression coverage

Eleven new engine tests walk serialized fields and variants, compare contracts
to real merge/type/range validation, verify unchanged state on errors, and execute
fill/copy/size workflows at 8-, 16- and 32-bit depths. The existing thirteen schema
tests continue to check the whole registry's metaschema, examples and defaults.
Browser-tool and native MCP tests verify preservation of the nested definitions.

No composition, subject-specific drawing logic or visual aesthetic policy is
encoded in these contracts. Those are reusable skill/workflow responsibilities.
This audit does not claim full registry coverage, output schemas, browser runtime
acceptance or improved illustration quality.
