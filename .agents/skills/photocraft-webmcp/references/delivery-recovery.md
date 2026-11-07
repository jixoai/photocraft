# Download and reload-restoration acceptance

Use this reference when download or recovery verification is requested. These are separate
technical probes, not a reason to repeat a drawing or operate an unrelated artwork tab.

## Set up an isolated probe

Use one TaskSpace for the probe and obey its ownership/stop rules. Keep existing artwork pages
untouched. Prefer a different explicitly chosen local origin when testing shared browser
storage; `localhost` and `127.0.0.1` are different origins. A new origin cannot restore another
origin's checkpoints. For task-scope isolation acceptance, instead keep both probe pages on the
same origin with distinct explicit scopes; another origin is not evidence for scope isolation.
Include the assigned `scopeId` on export and all recovery writes. Do not change profiles, clear storage or create a replacement TaskSpace
to get around a failure. Create only a disposable document with a distinctive name, editable
text and a small multipart shape; export its project and baseline PNG before any reload.
Record the page identity, native registration, command contracts, request instance, document
metadata, component IDs and before-state. This synthetic probe is not artistic acceptance.

## Verify a durable checkpoint and explicit restore

1. Name the test document through its discovered creation contract. Call
   `photocraft_control` with `method: "recovery.checkpoint", params: {}`. There is no checkpoint
   name argument: keep the exact returned key. An accepted reply is not a completed write.
2. Query `recovery.list` in bounded calls until that key has the expected document revision and
   nonzero bytes, with `pending: null`, `queued: []` and `error: null`. Record the entry and the
   exported baseline. Stop on an error or an unresolved deadline, rather than reloading early.
3. Reload only the disposable page after those exports and the durable check. Handle any
   navigation confirmation through the browser skill; never suppress a user permission prompt.
   Wait for six native tools again. Obtain `requests.status` and `recovery.list`; the new page
   must have a different request instance and the same saved key/revision. Merely listing a
   checkpoint after reload does not prove restoration.
4. Submit `recovery.restore` with `params: {key: exactKey}` once. Poll `recovery.list` for
   `restored[exactKey]`, idle queues and no error. Inspect that document: it is a new dirty copy,
   and the source snapshot entry remains available. Document IDs are page-local; do not infer
   cross-page identity from an equal numeric ID or require the restored history/revision to
   equal the source's in-memory history/revision.
5. Inspect actual component IDs and hierarchy in the restored copy before addressing it.
   Compare the multipart group, vector geometry, paints, text, dimensions and bit depth with
   the baseline; export and compare the rendered PNG. An exported `.pcraft` reimport is a
   useful independent check but is not a substitute for this checkpoint restore.
6. Use the current page instance for any new tracked edits; never replay the old receipts.
   Preserve evidence before closing only the probe's pages. Do not discard foreign snapshots
   or delete a test snapshot unless cleanup is authorized.

Report checkpoint creation, persistence across reload, explicit restoration and appearance/
editability checks separately. A storage failure is not permission to clear site data. A
restored copy does not promise persistent undo history or durable request receipts.

## Verify a browser download, not just export bytes

Before triggering, choose a fresh unique filename and an absolute destination, and record
that no file exists there. Arm the browser skill's download-event wait, then call the native
`photocraft_export` with `download: true`. Preserve the WebMCP reply separately. Attach rejection
handling to the wait immediately so a timeout cannot become an unhandled host error. If the
event arrives, save it in the same browser round, inspect its completion/failure result and
verify the actual saved file: PNG decode/dimensions/depth or project archive/reimport, plus
size and checksum against the unchanged baseline where meaningful. Do not run simultaneous
download waits; the browser SDK may manage one shared download setting.

If the wait expires, stop and inspect before retrying. Distinguish tool rejection, a successful
trigger, pending browser permission, a missed event, a failed transfer and a valid completed
file. Let the user handle browser-owned prompts using the browser skill's handoff workflow;
never change global download policy or bypass authorization. After approval, resume only the
same authorized space and check existing files before considering another trigger. Record
whether completion was automatically observed, verified from an existing file, only confirmed
by the user, or still blocked.

`download: false` returns base64 (within the documented size limit) for host-side delivery.
Verify those bytes and label the route, but do not count them as a browser-download pass.
The report should include the trigger reply, event outcome, file path/checks, authorization
status and any remaining limitation; do not put base64 file payloads in the report.
