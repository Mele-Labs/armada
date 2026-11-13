# A dispatch the Operator holds

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/operator.md`. Three operations, one event and `SessionHeld.held`.

Since 12 Nov 2026. `crates/ipc/src/job_dispatch.rs`, `crates/fleet/src/operator/dispatching.rs`, `crates/store/migrations/20261112T2000Z-operator_dispatches.table.sql` and `20261112T2100Z-parked_ideas.table.sql`.
Bridge's half is `packages/protocol/src/job-dispatch.ts` and `DispatchWay` in `operator-board.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `dispatch_job` (`POST /sessions/dispatch`) | Bridge calls | `DispatchJob`: every field of `StartSession`, flat, `kind?` (`idea`, `feature` or `bug`, a hint the Operator is told and nothing more) and `then?`, the sends made into the Job once it exists, each `{ notes: [SessionNote] }` or `{ message: { text, attachments?, mentions?, sketches? } }`. Answers `JobDispatched`, **exactly one of** `session` (the `SessionRecord` of the Job started) or `operator` (the dispatch's id while the Operator holds it). `parked` stays on `JobDispatched` and is not answered |
| `operator.dispatched` (`/events`) | Fleet publishes | `OperatorDispatched`: `dispatch_id`, `manifest_id`, `way` (`started`, `handed`, `asked` or `parked`), `session_id?` and `title?`. Not replayed |
| `list_parked_ideas` (`GET /sessions/parked`) | Bridge calls | `ParkedIdeas`: `ideas`, oldest first, each `{ id, dispatch, at }` with `dispatch` the `DispatchJob` as it was sent |
| `remove_parked_idea` (`POST /sessions/parked/remove`) | Bridge calls | `RemoveParkedIdea` `{ id }`. Answers `ParkedIdeas`. 422 `fleet.parked_idea_unknown` where none is kept under the id |

**A dispatch that is not the Operator's case is `start_session`.**
A body with `held`, `fork`, `root` or `after`, or no `request`, and every dispatch while the Operator is off or not seated, is started as `start_session` starts it, its `then` sent, and answered with `session`.
**With the Operator on and seated the work goes to it first.**
The Operator is put the dispatch with the live Jobs near it, even where none is, and rules: start a Job, fold it into the live one, keep it for later, or ask him with `Hand to <Job>` or `Keep both`.
**`kind` is a hint, and the Operator is told it in one line.**
It never picks a way, a workflow or whether a call is made, and a dispatch with none is as valid as one with any.
It stays on the stored dispatch, so it rides onto the Job or the idea kept for later.
The answer is `operator`, and `operator.dispatched` says what came of it.
`parked` is part of `JobDispatched` and is no longer answered by `dispatch_job`: an idea is kept only by the Operator's ruling, as `way` `parked`.
**A start or a fold the Operator rules is done at once** and is not a suggestion to press; only the ask is a go-ahead, in the Operator's own list.
**His work is never lost.**
A ruling that does not come in a minute, a call that fails, an act that cannot be carried out, a Skip, and a dispatch still kept when Fleet starts each start the Job as it was asked, from the whole body kept in `operator_dispatches`.

**Why Fleet keeps the ideas.**
The queued ideas Bridge keeps are in its own `localStorage` and in the native stack, so Fleet cannot write to either, and a Bridge that is closed when the Operator parks one would lose it.
`parked_ideas` is the smallest list that is not lost: the body whole, opaque to the store, and two operations to read and remove it.
Bridge merges it into its own list where it draws ideas, which is its half.

**Skew.**
An older Fleet answers 404 to `dispatch_job`, so a Bridge that must reach one starts with `start_session` instead.
An older Bridge never calls it, and Fleet starts what it starts as before.
`SessionHeld.held` rides `start_another_session` and `fork_this_session` (always `true` there).
