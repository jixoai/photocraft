# PhotoCraft command contracts

Generated on 2026-11-05 from the current engine registry:

```sh
cargo run -p photocraft-cli -- commands --json > artifacts/webmcp/command-contracts.json
```

- `command-contracts.json`: all 964 production command entries, with `description`, draft
  2020-12 `inputSchema`, `journal`, legacy `params` and an empty-session enablement snapshot.
- `contract-coverage.json`: per-family counts and the exact list of incomplete commands.
- Current coverage: 135 explicitly verified authoring contracts, 829 partial contracts.
  A schema existing for every command does **not** mean every parameter contract is complete.

Use `photocraft_commands` in the running web editor to discover matching commands and their
current enablement. Read `inputSchema["x-photocraft-coverage"]`, required fields, coordinate and
opacity units, defaults and nested variants before calling `photocraft_execute`. Do not feed
the entire inventory to a model on every turn. Keep the six WebMCP tools; search and page for
command contracts as needed. `params` is legacy prose, not a second JSON Schema.

These schemas describe supported authoring inputs. They do not insert defaults or change
the existing dispatch behavior. Document-dependent limits, fonts/profiles, channel targets
and native-only operations remain runtime checks. No output schemas are supplied here.

Nested brush contracts are self-contained via local `$defs`/`$ref`. Brush patches preserve
omitted fields; factory values used by reset are `x-photocraft-initial-default`, not patch defaults
or necessarily the current session brush.
The latest audit covers `paint.stroke`, `tools.setBrush`, brush read/preset operations,
`edit.fill`, `layer.duplicate`, `image.imageSize` and `image.canvasSize`. Other brush tools
are not implicitly complete just because their settings type has been audited.

Validation evidence is in `log/webmcp/schema-*.log` and `log/webmcp/brush-contracts-*.log`.
The latest run passes 3,389 engine tests (21 existing ignored), 24 schema regressions,
18 web tests and 83 automation integration tests, including 49 native MCP tests.
The earlier broad automation unit run has one unrelated trusted-root path failure at
`crates/automation/src/workspace/tests.rs:925`; that suite was not rerun for this increment.
See `brush-contracts.md` for the audit scope and `docs/webmcp.md` for transport details.

Updated compatibility + worker package: `dist/brush-contracts-release/photocraft-web-0.20.1.zip`.
ZIP SHA256: `f52c4bd731b91427049acc040a45be58794fc99e29097fd555625a0d76be72df`.
Both payloads contain the nested contract metadata; this is not a fresh live-browser
drawing acceptance. Existing illustration/project artifacts and the previous release ZIP are preserved.
