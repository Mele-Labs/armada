# Protocol 23.43: the job ledger

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`. **Additive only**: three operations, one event kind and the DTOs of `ipc::sessions`, and nothing an older Bridge reads changes.

| Operation | Carries | Notes |
| --- | --- | --- |
| `report_session` (`POST /sessions/report`) | `SessionReport`: `harness`, `session_id` and one `fact`: `started`, `titled`, `moved`, `attached`, `settled`, `measured`, `tuned` (since 23.53), `turn_started` (since 23.86), `turn_completed` (with `said?` since 23.86), `touched`, `read`, `ran`, `wrote`, `permission_asked`, `permission_settled` or `ended` | Answers the whole `SessionRecord`. `agent_access` `No`: a harness reports, never the agent inside it |
| `list_sessions` (`GET /sessions?manifest_id=&q=&state=`) | `SessionList` | `q` finds a job by title, branch, pull request number or slot |
| `who_owns` (`GET /sessions/owner?kind=&target=&manifest_id=`) | `Owners`: every holder, standing ones first | `kind` is `branch`, `pr`, `job` or `slot` |
| `session.changed` (event) | `SessionRecord`, whole | Published on a fact that changed something; a repeat publishes nothing |

**`kind` on an attachment is open text, and `state` is not.** A kind Fleet has not met is kept as it arrived; every kind's state is `standing`, `spent` or `given_back`, so a released slot and a spent need are read the same way.
A holder is `{ kind: session | job, id }` and no foreign key to either.

**One migration**, `session_ledger.tables`: `sessions` and `ledger_attachments`. Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.
