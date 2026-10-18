# Protocol 23.52: a job's name

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`. **Additive only**: one operation and one optional field. 23.51 is a session started on a Job.

| Where | Carries | Notes |
| --- | --- | --- |
| `rename_session` (`POST /sessions/rename`) | `RenameSession`: `session_id`, `title` | Answers the `SessionRecord` and publishes `session.changed` when the title changed. Refused with 422, nothing written, for a blank title or a job Fleet has no row for. Not offered to agents |
| `report_session` | `SessionFact` `titled.named?` | `true` is a person's name from the terminal's `/rename`: it replaces the title. Absent, it is the first prompt's line, kept only where the job has no title |

**A name from Bridge stands until the next terminal `/rename`.** Both are the same column, so the later one is the one shown.
Bridge's half is in `packages/protocol/src/sessions.ts`, written by hand like the rest.
