# How a Job started, and when each step began: `SessionRecord.started`, `SessionRecord.workflow_visits`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Oct 2026. The story spine opens its Brief on how the Job started, and gives each workflow step a zone with the items that happened in it.
**Additive**: two optional fields on `SessionRecord`, two new types; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take. An older Fleet sends neither field and Bridge draws the Job from what it holds now.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.started` | Fleet answers, on every read of a hosted Job | `SessionStarted`: `workflow` (its name as it read then), `delivery`, `model`, `effort`, `mode` and `by` (`SessionLink`, the Job that started it). Each part is absent where it was not set |
| `SessionRecord.workflow_visits` | Fleet answers | `StepVisit` list, oldest first: `step` (the index) and `since` (when it began). A step the Job went back to appears again. Empty for a custom Job |

**`started` is written once, as the process first runs**, in the store's `session_started`.
A later `tune_session` or workflow change leaves it alone, which is the point: the live values stay on `hosted` and `workflow`.
The model is the one the process runs on, the machine's own where none was picked. `delivery` is the landing chosen by then, so a Job that is asked later has none.
`by` is read at each answer from the child's `started_by` attachment and the parent's title; a Job with a parent and no snapshot still answers `started` with `by` alone.

**`workflow_visits` is kept by the rail**, in `session_rail.visits`, at the two places that already enter a step on the timeline (`session.step`).
The timeline entry carries words and not the index, so it is not read back.

Migrations: `session_rail.visits` (a column) and `session_started` (a table), both additive. A Job started before them has no `started` and no visits.
Bridge reads `started` in `sessionOfRecord` and the visits in `workOf` (`packages/screens/src/sessions-wire.ts`, `session-work.ts`), as `Session.started` and `work.workflow.visits`.
The event stream is untouched, and nothing changes in the WebSocket's load: both fields ride the `session.changed` row that already goes whole.
