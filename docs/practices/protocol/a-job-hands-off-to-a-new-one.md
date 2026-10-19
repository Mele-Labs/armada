# A Job hands off to a new one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session/a-handed-off-job.md`. One operation, `retry_handoff`, four ledger kinds, which are open text, and two option labels.

| Where | Carries | Notes |
| --- | --- | --- |
| `Attachment.kind` `replaces` | On the new Job. `target` is the old Job's id, `spent`, `detail.context` the handoff and brief the new Job starts from | Bridge folds the old Job behind this one in the same row, and reads only a `spent` one: a rolled-back attempt gives it back |
| `Attachment.kind` `replaced_by` | On the old Job. `target` is the new Job's id, `spent` | The old Job is `ended` |
| `Attachment.kind` `handoff_starting` | On the old Job. `standing` while the new Job comes up, then `given_back` | The ring |
| `Attachment.kind` `handoff_failed` | On the old Job. `standing`, `detail.reason` a bare fact in lowercase, `detail.brief` the brief kept for a retry, `target` the instant; `given_back` at the owner's next message to the Job, or by a retry | The band "Failed to hand off to new job". The old Job is live and unchanged, and the only live one |
| `retry_handoff` | `POST /sessions/handoff/retry`, body `{ "session_id": <old Job> }`, answers the new Job's `SessionRecord` | Retry on the band. 409 `fleet.handoff_not_failed` where no `handoff_failed` row stands, 409 `fleet.handoff_running` while a hand runs, 422 `fleet.handoff_failed` (message the reason) where it fails again |
| `ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY` | `Carry on in a new Job`, `Stay here` | The two option labels of the Job's question. Fleet acts on the owner's choice of the first, in any question |

The new Job is not a fork and has no `forked_from` row.
The sequence is `session.changed` for the old Job (`handoff_starting`), then for both as the ledger moves, then the old Job ended.

**Skew.** A Bridge before this meets kinds it has no screen for, which `attachmentsOfRecord` leaves out, and draws the old Job live and the new one beside it.
Bridge's half is `packages/screens/src/sessions-wire.ts`: `replaces` to `Session.replaces`, `replaced_by` to `replacedBy`, `handoff_starting` and `handoff_failed` to `handoff`.
