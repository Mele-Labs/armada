# A sub project has an orchestrator: `goal`, `turn_into_orchestrator` and the `goal_met` card

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Nov 2026. `docs/concepts/feature-catalog.md`, *The orchestrator* and *Goal met?*.
**Additive on the wire, breaking on one body**: `StartSubProject` takes a required `goal`, so a Bridge that sends none is a 422.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation files and the route table.
Only the native Bridge reads it.

| Where | Side | Carries |
| --- | --- | --- |
| `start_sub_project`, `POST /sub_projects` | Bridge sends `StartSubProject` | `goal`, his words, beside `project`, `name` and `jobs`. Fleet starts the orchestrator Job before it keeps the sub project |
| `SubProject` | Fleet answers | `goal`; `orchestrator`, the Job's session id, absent only where no start succeeded; `ended_at`, present once his Agree folded it away |
| `list_sub_projects`, `GET /sub_projects?project=&ended=` | Fleet answers `SubProjects` | The open ones, or with `ended=true` only the folded ones. The first read that lists an open one with no orchestrator starts one, once |
| `turn_into_orchestrator`, `POST /sub_projects/orchestrator` | Bridge sends `TurnIntoOrchestrator`, Fleet answers `SubProject` | `project`, `session_id` and `goal`. The Job's work moves to a new first child, which is the one Job in `jobs` |
| `WaitingSource::GoalMet` | Fleet raises it on the orchestrator's `waiting_for` | Id `goal_met:<sub project id>`, text `Goal met?`, options `Agree` (0) and `Not yet` (1) |
| `WaitingItem.goal_met`, a `GoalMet` | Fleet | `sub_project`, `goal` and `landed`, each a `LandedPull` of `number` and `title`, oldest first. Present on a `goal_met` item only |

**The card stands** while the orchestrator's `all_done` stands, newer than his last Not yet, or once every Job of the sub project merged a pull request or ended, before any Not yet.
`answer_waiting` with choice 0 sets `ended_at`, 1 keeps it open; neither sends the orchestrator a word.
Words alone are a 422 `fleet.session_waiting_empty`, a second answer a 409, and `dismiss_waiting` refuses it.

`start_sub_project` adds a 422 `fleet.sub_project_no_goal`.
`turn_into_orchestrator` is a 422 `fleet.sub_project_no_goal`, a 404 `fleet.no_such_session`, a 409 `fleet.sub_project_not_turnable` for a Job that ended, runs in a terminal or orchestrates already, and a 409 `fleet.sub_project_across_projects`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: the orchestrator's `SessionChanged` carries the card, and Fleet publishes it when one of the sub project's Jobs ends or merges.
