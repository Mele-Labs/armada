# A Job's row mark: `SessionRecord.mark`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 9 Nov 2026. Fleet reads the state a Jobs-list row draws, and the native Bridge and Pocket read it instead of working it out.
Electron Bridge works it out in TypeScript from a dozen fields of the record and the thread, and keeps doing so for now.
**Additive only**: one field on an existing type and the types it carries; no operation, route, event kind or error.
**States only.** The words a row says beside its mark stay with the client.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/job_mark.rs`; the reading is `crates/fleet/src/sessioning/marking.rs`, called last in `ledger_built`.
Bridge's half is `packages/protocol/src/job-mark.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.mark` | Fleet writes, Bridge reads | `JobMark`, absent where Fleet did not read one |
| `JobMark` | Fleet writes | `{ state, lamp, group, once_read? }` |
| `JobReading` | Fleet writes | `{ state, lamp, group }` |
| `JobState` | | Bridge's `SessionState`, value for value |
| `JobLamp` | | `needs`, `failed`, `running`, `done`, `idle` |
| `JobGroup` | | `needs`, `working`, `held`, `done` |

## What Fleet reads

The reading is a port of Bridge's, check for check, and the order of the checks is the rule:
`jobStateOf` in `packages/screens/src/session-state.ts`, `bucketOf` in `SessionFrame.tsx`, `sessionMark` and the row in `cockpit/deck/ship.ts`, and `toneOf`, `lampOf` and `groupOf` in `cockpit/deck/order.ts`.
A change to one side is a change to the other until Electron reads this field too.

**`once_read` is set only where `state` is `replied`.** It is what the row reads as once the person has read the reply.
Only the client keeps whether he has, so Fleet sends both and the client picks.

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

## The event

`SessionChanged` carries the record, so it carries the mark; no new event.
Every input moves on a path that already publishes the record: a turn, an item, a pause, a stop, an ask arriving or lapsing, a Check's row, a pull request's reading.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
It publishes nothing new, so the event stream is as it was.
