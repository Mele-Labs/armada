# To review, across Jobs: `list_to_review`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Nov 2026. `docs/concepts/session/waiting-on-the-person.md`, *To review, across Jobs*.
**Additive only**: one operation, its route and the types in `crates/ipc/src/to_review.rs`; no field on an existing type, no event kind and no error.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation file and the route table.
The native Bridge, Pocket and the Operator page read it.
Pocket reads it through the Phone Gateway's `/api/sessions/to_review`, and answers its items through `/api/sessions/items/answer` (`answer_items`) and `/api/sessions/waiting/done` (`done_waiting`) besides the routes a Job's thread already uses.
The Operator's read and pick ride on the item as `operator_read`, copied from its `WaitingItem`; `NeedsYouRow` does not carry them, so `get_needs_you` is unchanged. Pocket draws them above the item's form.

| Where | Side | Carries |
| --- | --- | --- |
| `list_to_review`, `GET /sessions/to_review?project=` (`agent_access = "No"`) | Fleet answers `ToReview` | `project?`, `groups[]`; every project's live Jobs, a Job in no project among them, where `project` is left out |
| `ToReviewGroup` | Fleet writes | `{ head, items[] }`, none empty: each sub project, oldest started first, then each feature in its library's order, then `neither` |
| `ToReviewHead` | Fleet writes | Tagged by `kind`: `sub_project { id, name, goal }`, `feature` (a `SessionFeature`) or `neither` |
| `ToReviewItem` | Fleet writes | `{ kind, row, order, why?, job { id, title }, project?, sub_project?, features[], answer, operator_read? }`; `operator_read` is the item's `WaitingItem.operator_read`; `row` is the `NeedsYouRow` `get_needs_you` answers for the item |
| `ToReviewKind` | Fleet writes | `plan`, `sketch`, `document`, `draft`, `ready_to_try`, `clash`, `goal_met`, `question`, `to_do`, `walk`, `command`, `pull_request`, `restart` or `done` |
| `ToReviewAnswer` | Fleet writes | Tagged by `op`, each an existing operation: `answer_items`, `answer_waiting`, `answer_ready_to_try { cards[] }` with each card `{ branch, tip, about }`, `done_waiting`, `begin_session { session_id }`, `dismiss_waiting`, or `press`, where the row's act is the press |

**Fed from `get_needs_you`, never beside it.** Each Job's rows are `needs_you_of` over the `waiting_for` its record carries, so every item of a listed Job is one item here and its `row` is the one the band draws.
`get_needs_you` is unchanged.
Within a group, items stand by `order`, the place `order_waiting` gave them in their Job, an unplaced item after the placed, then oldest first.

**A sub project's group is headed by its `goal`**, its `name` where it was kept before goals, and holds its orchestrator's items with its Jobs'.
A `goal_met` item is the orchestrator's Goal met? card, answered with `answer_waiting`.
**One seam.** A clash is the `job_conflict` item today; the kind and answer rules in `crates/fleet/src/session_host/to_review.rs` match every source with no fallback, so a source added for a clash does not build until it is placed.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: the list is read, not pushed, so a client that draws it reads it again on `session.changed`.
