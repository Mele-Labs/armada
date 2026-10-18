# A Job's rail and its judges

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/session.md`, *The rail* and *The judges*.
**Additive only**: three operations, four fields on a record, two on a waiting item, one source and one timeline kind.
The protocol ID is derived from the wire files and needs no bump.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRecord` | `workflow_step?`, `judging?`, `judge?`, `settled?` | `workflow_step` is the index of the step Fleet observed the Job on; absent for a custom Job, past the last step, and while `features.workflows` is off. `judging` is a judge reading now (in memory). `judge` is `{ at, concerns }` of the last reading, `concerns` 0 being *No concerns*. `settled` is the concerns the person settled, oldest first, at most 40, each a `judge` item with `concern.settled` |
| `WaitingItem` | `concern?`, `cites?` | `concern` is `{ from, settled?, expected?, produced?, consequence? }`, `from` being `{ kind: "tile" }` or `{ kind: "step", index, label }`; `settled` is `{ how: "sent" or "dismissed", at }`; the three parts are the judge's refusal, each one short sentence, which `text` joins for the agent, and are absent on a concern kept before they were. `cites` is `[{ path, line? }]`, the files of the diff the judge's quotation landed on |
| `WaitingSource` | `judge` | id `judge:<n>`, `act` a `message` naming the Job. Sent or dismissed from its own card; `answer_waiting` and `dismiss_waiting` refuse it with `fleet.session_waiting_empty` |
| `judge_session` (`POST /sessions/judge`) | `JudgeSession`: `session_id`. Answers 200 `SessionRecord` | Answers at once with `judging` true; the reading runs in a task and the row is published when it ends. 409 `fleet.session_closed`, 422 `fleet.session_command_refused` for a terminal Job. `agent_access: No` |
| `send_concern` (`POST /sessions/judge/send`) | `SendConcern`: `session_id`, `item_id`. Answers 200 `SessionRecord` | The concern becomes the person's message to the agent, sent before it is settled. 409 `fleet.session_waiting_unheld` for an item not waiting. `agent_access: No` |
| `dismiss_concern` (`POST /sessions/judge/dismiss`) | `DismissConcern`: `session_id`, `item_id`. Answers 200 `SessionRecord` | Settled; the agent is told nothing. Same 409 |
| `timeline.entry` | `TimelineKind` gains `session.step` | `says` is `step: <old> → <new>` by label, `start` before the first and `done` past the last |
| `set_session_workflow` | Behaviour only | 422 `fleet.session_workflows_off` to set a workflow while `features.workflows` is off; clearing one still works |
| Settings | `features.workflows` (boolean, live, shipped on) | Served with the rest; Bridge hides the workflow picker and the steps when it is off |

**Skew.** A Fleet before this answers 404 for the three routes; a Bridge before it ignores the record fields and draws a `judge` source as an item it has no card for.
The IDs differ, so the two are refused regardless and the lifeboat serves.
Bridge's half is `packages/protocol/src/{session-rail,sessions,timeline}.ts`, written by hand like the rest.
