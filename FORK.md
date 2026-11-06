# FDE2E Photocraft fork

Updated: 2026-11-06.

FDE2E Photocraft is maintained by FDE2E（昭智羽名(厦门)科技有限公司）.
It is based on PhotoCraft from `storytold/photocraft`, under MIT OR Apache-2.0.
This is an independently branded downstream, not an official upstream distribution.

## Provenance and compatibility

- Upstream source anchor: `37a01ca87`; downstream enhancement checkpoint:
  `7815cdfd4` / `agent-web-baseline-20261105`.
- `agent-web` is the product branch; `main` is an unmodified upstream mirror.
  The integration at `sync/upstream-20261105` has not passed all gates and is not released.
- Crate names, executable names, command/tool IDs, `.pcraft`, browser storage keys and
  existing application IDs remain unchanged for compatibility. Display names, project/help
  links, icons and update destinations identify FDE2E Photocraft instead.
- The old `PhotoCraft Defaults` shortcut-export label remains an interchange identifier;
  the UI displays `FDE2E Photocraft Defaults`. Existing user-named sets are not rewritten.
- Browser workers, compatibility fallback, drawing improvements and shared command schemas
  are downstream capabilities. See [WebMCP](docs/webmcp.md) and
  [maintenance](docs/downstream-maintenance.md).

## Branding and attribution

The product name is **FDE2E Photocraft**. Upstream marketing, social/community buttons and
non-open-source ArtCraft logo files are not part of this product. No upstream sponsorship,
endorsement or support is implied. The app icon is an original geometric F mark; see
[`assets/app-icon/README.md`](assets/app-icon/README.md).

Original copyright notices, MIT/Apache license texts, third-party notices and contributor
history remain intact. Historical technical evidence and pinned dependency/corpus URLs
still name their actual upstream sources; replacing them would misstate provenance or
break builds. The retained upstream trademark license documents excluded historical marks.

Runtime help and issue links go to `jixoai/photocraft`. Native update checks must use only
this fork's releases, never replace an enhanced build with a `storytold/photocraft` release.
Publishing a source change does not rebuild old ZIPs or replace an already open browser.
