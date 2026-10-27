# Protocol 23.92: a Job paused and resumed

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Pausing a Job*. **Additive only**: two operations, three types and one optional field. There is no number to bump: the protocol ID moves with the files below.

| Where | Carries | Notes |
| --- | --- | --- |
| `pause_session` (`POST /sessions/pause`) | `PauseSession`: `session_id` | Answers the `SessionRecord` with `hosted.paused` set. Ends the turn, commits the slot's work to the branch (WIP, no push), frees the slot. `Helm only` |
| `resume_session` (`POST /sessions/resume`) | `ResumeSession`: `session_id` | Answers the record; `hosted.paused` is absent once a slot is held, and `resuming` while it waits for one. Sends the turns kept while paused, or tells the Job it was resumed. `No`: only a person's press. |
| `HostedFacts.paused` | `SessionPaused`: `at`, `resuming` | Absent unless paused. Carried on `session.changed`, which is published on every change; no new event kind |
| 409 `fleet.session_not_pausable`, `fleet.session_already_paused`, `fleet.session_not_paused`, `fleet.session_pause_refused` | | A terminal Job is 422 `fleet.session_command_refused`, as for a stop |

**Skew.** A Fleet before 23.92 has no route and answers 404, and sends no `paused`.
A Bridge before it ignores the field and never calls the routes.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.
