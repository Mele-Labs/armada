# A walk's notes, sent as one turn: `send_session_notes`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 19 Oct 2026. `docs/concepts/session.md`.
**Additive only**: one operation, one route, one error code and the types they carry; no event kind and no field on an existing type.
The protocol ID moves with `crates/ipc/src`, the operation file, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-notes.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `send_session_notes` (`POST /sessions/notes`, `agent_access = "No"`) | Bridge sends, Fleet answers 202 | `SendSessionNotes` `{ session_id, notes }`; answered with `NotesSent` `{ session, candidates }`, `candidates` one list per note in the notes' order. An empty `notes` is a 422 `fleet.session_notes_empty` |
| `SessionNote` | Bridge sends | `{ said, address, note_id?, place, state, crop? }`, each note with its own `address` |
| `SendSessionNote` | Bridge sends | Now `session_id` beside a flattened `SessionNote`. **Its JSON is unchanged**; `crates/ipc/src/tests/session_notes.rs` holds the bytes |

**One turn.** Each note is searched in the checkout that served its own address, as one note is.
The agent reads the notes numbered in order, each with its words and Fleet's lines for it, then the handoff line once, and every crop as a picture in note order.
The thread keeps the words alone, numbered. A batch of one note is sent as `send_session_note` sends it.

**Bridge's call** is `SessionsHost.sendNotes` in main, which folds `NotesSent.session` as `sendNote` folds its record.
The renderer reaches it as `sendSessionNotes` on `window.armada` (channel `bridge:send-session-notes`), each note with the box its picture is ringed at beside it; main burns the ring in, tagged with the note's place in the turn, and the box never reaches the wire.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
Were a pair to meet anyway, a Fleet without the route answers 404, which Bridge words as a send that failed.

**The event stream.** Untouched: the one turn publishes `session.changed` as a message does, and nothing new.
Several notes now take one turn where they took one each, so the stream carries fewer events for a walk; the open risk below is no worse.
