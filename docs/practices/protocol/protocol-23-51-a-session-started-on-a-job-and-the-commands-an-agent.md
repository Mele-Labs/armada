# Protocol 23.51: a session started on a Job, and the commands an agent names

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Started on a piloted Job*. *`StartSession.pilot` and the `handoff` row are removed, see "Sessions absorb Jobs, group 1".* **Additive only**: two optional fields, one row kind, and no operation. 23.50 is the take over and its exits.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession.pilot?`: `job_id`, `outcome` (`take_over` or `restart_step`) | Takes the Job over and starts the session on its worktree in one call. **`manifest_id` is not read** where `pilot` is set: the repository is the Job's. Refused with `take_over`'s own 409s and no session is written |
| `session.row`, `get_session` | `SessionRow` kind `handoff`: `job_id`, `number`, `title`, `reason`, `slot?`, `branch`, `step?`, `attempts`, `refusals`, `plan` (`declared`, `outside`, `unwritten`), `narrative?` | First in a piloted session's thread, once. **`plan` is Fleet's comparison and not two lists to compare**, since a declared path covers everything beneath it |
| `SessionRecord.hosted` | `commands?`: names, slash commands then skills | Off the stream's `init` line. Empty before an agent has started anywhere, and after a Fleet restart |

**A bundle that is the agent's first context is not a row.** Fleet renders it as prose and puts it ahead of the person's first message. A session closed or released from a pilot clears its own lease, so its next write leases a slot as any session's does. Bridge's half is in `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest.
