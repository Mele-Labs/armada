# A Job reads and commands the Jobs it started

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A Job that started Jobs*.
**Additive throughout**; the protocol ID moves with the wire files, and nothing is bumped.
The mirror is `packages/protocol/src/started-jobs.ts`, written by hand. Bridge draws none of it.

| Where | Side | Carries |
| --- | --- | --- |
| `list_started_sessions` (`GET /sessions/started`) | An agent sends, Fleet answers | No body: the caller is placed by its connection. Answers `StartedSessions` `{ jobs }`, oldest first. A `StartedJob` is a `SessionSummary` with `standing?`, `held`, `paused`, `step?` (`index`, `label`, `waiting_on?`), `waiting_for?`, `waits?` (`SessionWait`) and `result?` (`end_reason?`, `summary?`, only once ended). `agent_access` Yes. 422 `fleet.started_unplaced` |
| `command_started_sessions` (`POST /sessions/command_started`) | An agent sends, Fleet answers | `StartedCommands` `{ commands }`, each `{ session_id, act, why?, condition?, base? }`, `act` being `withdraw`, `end`, `pause`, `resume`, `change` or `drop`. Answers 200 `StartedAnswers` `{ commands }`, one `{ done? , refused? }` for each in the order given. An empty list is 400. `agent_access` Yes. 422 `fleet.started_unplaced` refuses the whole call |
| Refusals of an entry | Fleet writes | 422 `fleet.started_not_yours` for a Job the caller did not start (a grandchild included), `fleet.started_needs_why`, `fleet.started_needs_change`; 409 `fleet.started_begun`, `fleet.started_ended`, `fleet.wait_over`; and what `pause_session`, `resume_session` and `change_waiting` refuse |
| `Notice::StartedJobEnded` (`started_job_ended`) | Fleet writes | A notice word, told to a child that had begun when its starter ends it. In `NOTICES`, so a mute or a rule can name it |

**`pause_session` and `resume_session` keep `agent_access` No.** The entries `pause` and `resume` run the same Fleet acts, after the ownership check, so no other Job is reachable through them.

**Skew.** The IDs differ, so a Bridge before this is refused regardless. A Bridge that does not know the notice word draws the Job's thread without a label for it.
