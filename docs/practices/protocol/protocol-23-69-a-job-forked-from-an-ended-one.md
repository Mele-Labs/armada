# Protocol 23.69: a job forked from an ended one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *A forked job*; `docs/spikes/028-can-a-session-be-forked-from-a-dead-one.md`.
**Additive only**: two optional fields, one migration and no operation. 23.54 is another branch's.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session` | `StartSession.fork?`: `session_id` | Starts a job as a copy of an ended or dead one's conversation. 409 `fleet.session_fork_live` for a live one, 422 `fleet.no_such_session` for an unknown one. `manifest_id` is read as for any start |
| `SessionRecord.terminal` | `listening?`: `true` while the job's mod has asked within ten seconds | Absent is not listening. A live terminal job that is not listening is dead for Fork. Published as `session.changed` when it flips, while a window has the thread open |
| `SessionRecord.attachments` | kinds `forked_to` and `forked_from`, `target` the other job's id, `spent` | Open text as every kind is. **One migration**, `session_fork.hosted_fork_of`: `hosted_sessions.fork_of` |
