# A message tags a request: `TagKind::Request`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 20 Nov 2026. A message in the native Bridge can tag one of the Job's saved API Client requests, or ask for a new one.
**Additive only**: one `TagKind`; no operation, route, event kind or field.
There is no number to bump: the protocol ID moves with `crates/ipc/src` and `classic-final:packages/protocol/src`.
Bridge's half is `classic-final:packages/protocol/src/hosted-sessions.ts`, written by hand; Pocket's `apps/pocket/src/wire/hosted-sessions.ts` is generated.

| Where | Side | Carries |
| --- | --- | --- |
| `TagKind::Request` (`request`) on a `SessionTag` | Bridge sends, Fleet reads | A saved request: `id` is `<collection id>/<request id>` (a request id is unique only within its collection), `title` its name. A new one: `id` is `new`, `title` `New request` |

Fleet adds a line of its own for each, after the line that names the rest and the lines for tools.
Saved: `The person tagged the saved request "<title>": load it with `get_requests` (collection `<collection>`, request `<request>`) and send it with `send_request` by that id.`
New: `The person asked for a new request: build it, send it with `send_request`, and save it with `save_request_to_collection`.`
An `id` with a slash that leaves either half empty adds no line; the person's words still say what they meant.

The thread keeps the tag on his row.
Classic and Pocket read the tag and draw no chip for it, so the row shows the words as typed.
**The event stream:** untouched.
