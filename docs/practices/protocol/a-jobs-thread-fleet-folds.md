# A Job's thread, folded by Fleet

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Fleet folds a Job's thread once, and every client draws it.
Bridge folded it in TypeScript from `get_session` and `session.row`; the native Bridge and Pocket read this instead.
**Additive only**: three new operations and one new event, and no existing one changes.
Electron Bridge keeps its own TypeScript reading, so `get_session` and `session.row` are as they were.
The Rust half is `crates/ipc/src/job_thread.rs` and the fold is `crates/fleet/src/thread_view.rs`.
Bridge's half is `packages/protocol/src/job-thread.ts`, written by hand.

| Operation | Kind | Carries |
| --- | --- | --- |
| `get_job_thread` (`GET /sessions/thread?session_id=&before=&limit=`) | query | `JobThread`: the `head`, the newest `items` and `older` |
| `job_thread.tail` | event | `JobThreadTail`: `from`, the `items` from there to the end, and the `head` |
| `job_thread.changed` | event | `JobThreadChanged`: `session_id` and `from`, for a change too far back to send |

## Items and head

An item is one of `row`, `calls`, `subagent`, `checks`, `fleet_turn`, `sketch`, `file`, `document`, `change` or `not_started`.
**An item carries what a row is and how it stands, and no sentence written for a screen.**
A sentence Fleet wrote itself, a notice or the two of a Fleet act, passes through as it is.
The `head` is what stands over the thread: the `asked` call, the `open_call`, the `lapsed` questions, the `fleet_acts` and the `trail`.

**The fold is by row order alone.**
A tool row carries no call id and no result row, so a run of calls is the tool rows that came one after another, and any other row ends it.
A waiting `ask` is the head's `asked` and no item, and a `browser_reminder` is dropped.
Every other row that does not fold stays a typed `row` item, so a settled ask and a window question reach the client with their wire row.
A `plan` and a `delivery` row stay too; Electron Bridge leaves them out of its thread, and the native screen decides.

An item is held by its `id`, which is its first row's id.
A card ends a run of calls, so a `show_sketch` the ledger holds, a `show_file` the ledger holds and a document revision are items of their own.
A `change` has the id `event:<timeline id>`, read from the Job's `session.workflow`, `session.model` and `session.effort` timeline entries.
**The timeline keeps a bounded number of days**, so a change older than that is no longer an item.

## The window

`get_job_thread` folds the whole thread and then takes whole items.
**The window never cuts an item**: a run of calls, a Fleet turn or a Check run is in one answer or none.
`limit` is `THREAD_ITEMS_SERVED` (200) when absent and at most `THREAD_ITEMS_MOST` (1000).
`before` is an item's id or the id of any row folded into one, and the answer holds the items before that item.
A `before` no item holds is refused as `fleet.thread_item_unknown`, a 422, because the request is well formed and names nothing.
`older` is the first returned item's id, absent where the answer starts the thread.
It works for hosted and terminal Jobs alike.

## The tail

Fleet keeps the folded thread of each of the 32 Jobs a client read most recently, in memory.
On a row appended or replaced by id, a changed record or a timeline entry of that Job, it folds again and finds the first item that differs.
Nothing changed publishes nothing.
A Job is dropped from the held threads when it ends.

| What changed | Fleet publishes |
| --- | --- |
| Items from `from` to the end, within `THREAD_TAIL_MOST` (200) items of the end | `job_thread.tail` with `from` and the items |
| An item appended | `job_thread.tail` with `from` absent and the new items |
| The head alone | `job_thread.tail` with `from` absent and `items` empty |
| An item further back than `THREAD_TAIL_MOST` from the end | `job_thread.changed` with `from` |

**`items` replace what a client holds from the item `from` to the end.**
`from` is an item the client held, so an item inserted in the middle names the item it displaced.
A client that does not hold `from` holds only items after it, and replaces everything it holds with `items`.
A client that holds an item at or after `from` of a `job_thread.changed` reads its window again with `get_job_thread`; one that holds none ignores it.

**The cap exists because an old change must not push a whole long thread to every client.**
An old ask answered, or a long finished Check gaining a row, changes an item far from the end, and the tail from there is most of the thread.
The mark costs one id, and only a client that holds the item pays a read for it.

## Skew and the event stream

A client that missed an event reads `get_job_thread` again on the resync after `Missed`, because a tail applied to a thread it does not hold would be wrong.
Both events are published on the same stream as `session.row`.
**This makes the unbounded sink neither better nor worse**: it adds an event per change to a Job a client read, where Bridge already took `session.row` for the same change.
A tail is the changed items and not the thread, and the cap keeps it from growing with the thread.
A Bridge from before this does not call the operation and ignores both events.
A Fleet from before it refuses `/sessions/thread` as an unknown route.
