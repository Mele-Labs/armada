# A dispatch the Operator holds

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/operator.md`. Three operations, one event and `SessionHeld.held`.

Since 12 Nov 2026. `crates/ipc/src/job_dispatch.rs`, `crates/fleet/src/operator/dispatching.rs`, `crates/store/migrations/20261112T2000Z-operator_dispatches.table.sql` and `20261112T2100Z-parked_ideas.table.sql`.
Bridge's half is `classic-final:packages/protocol/src/job-dispatch.ts` and `DispatchWay` in `operator-board.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `dispatch_job` (`POST /sessions/dispatch`) | Bridge calls | `DispatchJob`: every field of `StartSession`, flat, `kind?` (`idea`, `feature` or `bug`, a plain field that nothing reads) and `then?`, the sends made into the Job once it exists, each `{ notes: [SessionNote] }` or `{ message: { text, attachments?, mentions?, sketches? } }`. Answers `JobDispatched`, **exactly one of** `session` (the `SessionRecord` of the Job started) or `operator` (the dispatch's id while the Operator holds it). `parked` stays on `JobDispatched` and is not answered |
| `operator.dispatched` (`/events`) | Fleet publishes | `OperatorDispatched`: `dispatch_id`, `manifest_id`, `way` (`started`; the other three are no longer sent), `session_id?` and `title?`. Not replayed |
| `list_parked_ideas` (`GET /sessions/parked`) | Bridge calls | `ParkedIdeas`: `ideas`, oldest first, each `{ id, dispatch, at }` with `dispatch` the `DispatchJob` as it was sent |
| `remove_parked_idea` (`POST /sessions/parked/remove`) | Bridge calls | `RemoveParkedIdea` `{ id }`. Answers `ParkedIdeas`. 422 `fleet.parked_idea_unknown` where none is kept under the id |

**A dispatch that is not the Operator's case is `start_session`.**
A body with `held`, `fork`, `root` or `after`, or no `request`, and every dispatch while the Operator is off or not seated, is started as `start_session` starts it, its `then` sent, and answered with `session`.
**With the Operator's session running the work goes to it first.**
The dispatch is told to the session as a message, and the Operator starts the Job itself, hands the words to a live Job, or raises a card, by the project's `operator_acts`.
The answer is `operator`, the dispatch's id, and Fleet keeps nothing more of it.
A dispatch carrying `then` is started as asked, since its sends need the Job they were set up for.
A session that is not running, or cannot be told, starts the Job as he asked.
**`kind` is a plain field.**
It is not told to the session, picks no way and no workflow, and a dispatch with none is as valid as one with any.
It stays on the stored dispatch, so it rides onto the Job or the idea kept for later.
`parked` is part of `JobDispatched` and is never answered by `dispatch_job`: nothing parks an idea now, and `list_parked_ideas` lists those an earlier Fleet kept.
**`operator.dispatched`** is published only when a dispatch an earlier Fleet held is started as asked, with `way` `started`.
**His work is never lost.**
A session that cannot be told, and a dispatch still kept when Fleet starts, each start the Job as it was asked, from the whole body kept in `operator_dispatches`.

**Why Fleet keeps the ideas.**
The queued ideas Bridge keeps are in its own `localStorage` and in the native stack, so Fleet cannot write to either, and a Bridge that is closed when the Operator parks one would lose it.
`parked_ideas` is the smallest list that is not lost: the body whole, opaque to the store, and two operations to read and remove it.
Bridge merges it into its own list where it draws ideas, which is its half.

**Skew.**
An older Fleet answers 404 to `dispatch_job`, so a Bridge that must reach one starts with `start_session` instead.
An older Bridge never calls it, and Fleet starts what it starts as before.
`SessionHeld.held` rides `start_another_session` and `fork_this_session` (always `true` there).
