---
name: photocraft-webmcp
description: Operate the PhotoCraft web editor through native WebMCP, with tracked editing requests and safe continuation after browser automation interruptions.
---

# PhotoCraft WebMCP

Use the available browser skill to connect to the requested editor. Reuse its existing
tab/target; inspect before navigating. Do not refresh a user's working canvas to recover
from an automation timeout. Follow the user's constraints on imported assets and examples.

## Discover and diagnose

- Wait for `window.photocraft.webmcp.registered === 6` and no registration error before
  using native `document.modelContext.getTools()` / `executeTool`. Do not serialize tool
  callback objects; project only names/descriptions/schemas when inspecting metadata.
- Discover the required commands with `photocraft_commands` (limit at most 200), then use
  their exact IDs and `inputSchema`. A partial contract is not full validation.
- Check `ok` in every reply. An application error includes `errorInfo`; native browser
  rejection can precede the callback. In that case diagnose the same arguments through
  `window.photocraft.validateTool(name, args)`, without mutating the document.

## Continue safely

For multi-step edits, use tracked `photocraft_execute` calls when supported. See the
[request protocol](../../../docs/webmcp.md#tracked-editing-and-safe-continuation).

1. Get `instanceId` through `photocraft_control` with `method: "requests.status"` and
   empty params. Save a local plan containing the page target, instance, unique plan ID,
   step IDs and exact command arguments **before** submitting edits. Persist returned
   object IDs rather than guessing them. Do not log file payloads in progress summaries.
2. Submit each step with its stable `requestId` and `instanceId`. The immediate reply is
   admission/status, not completion. Query that ID until `state === "succeeded"`, then
   use `reply.result` and checkpoint the completed prefix before the next dependent edit.
3. Keep each browser evaluation short (for example, a one-second polling window); return
   progress to the host instead of running an entire drawing loop under `page.evaluate`.
   A host timeout does not cancel admitted work. Query the same ID after reconnecting.
4. Resubmitting the **identical** ID and arguments within the **same** instance queries
   its existing status, never starts it again. Do not invent a new ID just because a
   reply was lost. Stop at `failed` or `unknown`; inspect the document/history/jobs before
   deciding on a new, reconciled edit. Do not advance dependent steps on an admission ack.
5. On `INSTANCE_MISMATCH`, stop the old plan. This ledger is memory-only: reload, recovery,
   undo or user edits invalidate assumptions about a saved prefix. Reconcile against the
   current document; do not blindly regenerate the old prefix using the new instance.
6. Check tracking limits before a large plan. Records are not silently evicted. When full,
   save/export and establish a deliberate new session; never refresh unsaved work to
   clear the ledger. A tracked ID is a receipt of execution, not proof its edit still exists.

If tracking is unavailable, execute small ordered chunks, persist the last confirmed
result, and inspect before retrying any ambiguous mutation. Do not claim exactly-once
execution across page reloads or transactional rollback of a sequence.

## Checkpoint and recover

Use `photocraft_control` `recovery.checkpoint` between completed milestones, then poll
`recovery.list` for the matching entry revision with no pending/queued operation or error.
An admission is not a committed checkpoint. Browser storage can be blocked or evicted;
also export `.pcraft` at important milestones. Never clear storage to fix an error.

After an intentional reload, list available snapshots and explicitly restore the selected
key using `recovery.restore`. It opens a new dirty copy and retains the source. Poll for
`restored[key]`, inspect the document and obtain the new request instance before replanning.
Do not replay old receipts. Discard requires `confirm: true`; do not delete another tab's
snapshot or retry an ambiguous restore with a different key. Restore is not undo-history
recovery. A new origin/profile cannot see the previous origin's checkpoints.

## Preserve the result

Deliver artwork PNG separately from the editor screenshot. Export `.pcraft` when editable
content is requested, keep the final tab open, and verify downloaded files rather than
equating a browser download request with a successful save. Send preview bytes to the
browser tool's image output, not the text transcript. Do not claim visual acceptance
without actually viewing the image.
