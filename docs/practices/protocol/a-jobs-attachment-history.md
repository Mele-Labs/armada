# A Job's attachment history

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 22 Oct 2026. Every version of every ledger attachment a Job holds, so Bridge can draw its artifacts as a timeline.
**Additive only**: one operation and the two types it carries; `AttachmentVersion` later gained two optional fields (below). No field of `SessionRecord`, no event kind and no error is new.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand.
The migration `20261022T1300Z-session_ledger.versions` is additive: a table, triggers, and one row seeded for each row already kept.

| Where | Side | Carries |
| --- | --- | --- |
| `get_attachment_history` (`GET /sessions/attachment_history?session_id=`) | Bridge reads | `AttachmentHistory`: `versions[]`, oldest first. `AttachmentVersion`: `kind`, `manifest_id?`, `target`, `state`, `detail?`, `at`, `from_message?`, `from_task?`. 422 `fleet.session_unnamed` for a job Fleet does not know |

**The history is not on `SessionRecord`.** That record is published whole on every `session.changed`, so a list that only grows would be sent again with every change a Job makes.
Bridge reads this when a Job's artifacts are open, and again on a `session.changed`, as it reads the Loadout.
It adds no event and no queue, so it leaves the unmeasured sink risk where it was.

**A version is the attachment as it stood from `at`.** One is written when a row is inserted and one whenever its `state` or `detail` changes, by a trigger on `ledger_attachments`, so every writer is covered and a new one needs nothing.
`at` is the row's `changed_at`. A write that restamps `changed_at` and moves neither field adds no version.
A row kept before the table existed is seeded with one version, its present state at its `since`: what came before is not known.
The versions of every attachment the Job holds are in one list, ordered by when each was written; Bridge groups them by `kind`, `manifest_id` and `target`.

**What a version followed.** `from_message` is the thread row id, the `id` of the `SessionRow::Message` Bridge draws, of the person's latest message when the version was written.
`from_task` is the id of the plan task in progress then, the one that began last where several are, and absent once it is completed or none is in progress.
Both are optional and left out when absent, on the wire and in `packages/protocol/src/sessions.ts`; a version with neither follows nothing known.
Fleet works them out; the agent says nothing.
Migrations `20261022T2000Z-session_focus.tables`, `20261022T2010Z-session_plan.task_started_at` and `20261022T2020Z-session_ledger.version_origins` are additive: two tables with a trigger, and a nullable column.

**Skew.** The route is new, so the protocol ID moves with it and a Bridge and a Fleet that differ on it never meet.
