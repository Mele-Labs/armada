# A Job's row mark: `SessionRecord.mark`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 9 Nov 2026. Fleet reads the state a Jobs-list row draws, and the native Bridge and Pocket read it instead of working it out.
Electron Bridge works it out in TypeScript from a dozen fields of the record and the thread, and keeps doing so for now.
It came in two changes the same day.
The first was **additive only**: one field on an existing type and the types it carries.
The second added one operation, one route and one field, `viewed_at`, so Fleet knows when the person was last in a Job, and took `once_read` off `JobMark`, which nothing read: see *When he was last in a Job* below.
**States only.** The words a row says beside its mark stay with the client.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/job_mark.rs`; the reading is `crates/fleet/src/sessioning/marking.rs`, called last in `ledger_built`.
Bridge's half is `packages/protocol/src/job-mark.ts`, written by hand, field for field.
`view_session` is `crates/ipc/src/job_viewing.rs` and `packages/protocol/src/job-viewing.ts`, kept by `crates/fleet/src/session_host/viewing.rs`.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.mark` | Fleet writes, Bridge reads | `JobMark`, absent where Fleet did not read one |
| `JobMark` | Fleet writes | `{ state, lamp, group }` |
| `SessionRecord.viewed_at` | Fleet writes, Bridge reads | the last instant he was in the Job; absent where never |
| `view_session` | Bridge writes | `POST /sessions/view`, `ViewSession { session_id, on }`, answered with the `SessionRecord` |
| `JobState` | | Bridge's `SessionState`, value for value |
| `JobLamp` | | `needs`, `failed`, `running`, `done`, `idle` |
| `JobGroup` | | `needs`, `working`, `held`, `done` |

## What Fleet reads

The reading is a port of Bridge's, check for check, and the order of the checks is the rule:
`jobStateOf` in `packages/screens/src/session-state.ts`, `bucketOf` in `SessionFrame.tsx`, `sessionMark` and the row in `cockpit/deck/ship.ts`, and `toneOf`, `lampOf` and `groupOf` in `cockpit/deck/order.ts`.
A change to one side is a change to the other until Electron reads this field too.

**`replied` reads only until he has been in the Job since its last turn ended**: `viewed_at` at or past `last_turn_at`, or the Job on screen now.
That is `readSince` in Electron's `session-state.ts`, check for check.

Three inputs come from the thread rather than the record: whether a question lapsed, whether a Check of the Job's is running, and whether the thread holds a row at all.
Fleet takes them from what the ledger already read: the lapsed question is the one the Waiting band reads, which says what `ThreadHead.lapsed` says, and the running Check is the store's, which writes the thread's `check` row.

Where Fleet's reading differs from Bridge's:

- **Its own pull request** is read from the Job's `pr` attachments, which Fleet's pull reading fills. Bridge prefers the merge line's listing of the same pull request where it has one.
- **An owed walk** is a `walk` item in `waiting_for`. Bridge also counts a mock walk he has not approved, which only Bridge keeps.
- **A Custom tab's Mod Job that asks** does not mark the Jobs that hold the tab. Mods come after the switch to the native app.
- **A lapsed question** is read from the whole thread, where Bridge reads it only from a thread it has open.
- **A blank Job** counts every attachment that is not given back, where Bridge leaves out a kind it has no screen for, and takes a title of only space as none.

**`failed` and the quarter-hour quiet rule are never reached for a Job today**: `sessionMark` never says a Job's tone is bad.
Both are ported for the day it does, and `failed` stays in `JobLamp`.

## When he was last in a Job: `view_session` and `viewed_at`

A client calls `view_session` at the moments Electron's `cockpit/viewed.ts` records a visit: `on: true` when a Job comes on screen in a visible window or that window shows again, `on: false` when it leaves, or the window hides or closes.
Each call keeps Fleet's clock as `viewed_at`, later only, in the store's `session_viewed`, and publishes `session.changed`.
Fleet keeps a visit of its own when a client's `send_session_message` or `answer_waiting` reaches the Job, so every client counts writing to a Job or answering its card as being in it; a message Fleet sends a Job itself does not.
`viewed_at` is the instant of the last call, so while he stays in a Job it is when he arrived.
Fleet also counts in memory the places that sent `on` and not yet `off`, and a Job with any reads as viewed now, so a turn that ends while he is in it is not `replied`.
Two clients showing one Job keep it on screen until both have left.
A restart forgets the count and keeps `viewed_at`.
A Job Fleet does not know is 404 `fleet.no_such_session`, with nothing kept.

Electron calls it from `useViewing` and still reads its own `localStorage` for the unread mark, the Cockpit's calls and the Dock count.
`once_read`, which carried what a `replied` row reads as once read, left `JobMark` with it, and `JobReading` with that; nothing read either.

## The event

`SessionChanged` carries the record, so it carries the mark; no new event.
Every input moves on a path that already publishes the record: a turn, an item, a pause, a stop, an ask arriving or lapsing, a Check's row, a pull request's reading, a `view_session`.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
It publishes no new event kind, so the event stream is as it was.
An Electron Bridge older than `view_session` never calls it, so Fleet reads every reply it shows as unread until a client that does call it is in the Job.
