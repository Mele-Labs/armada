# A Job hands off to a new one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session/a-handed-off-job.md`. **Additive only**: no operation, field or event. Four ledger kinds, which are open text, and two option labels.

| Where | Carries | Notes |
| --- | --- | --- |
| `Attachment.kind` `replaces` | On the new Job. `target` is the old Job's id, `spent` | Bridge folds the old Job behind this one in the same row |
| `Attachment.kind` `replaced_by` | On the old Job. `target` is the new Job's id, `spent` | The old Job is `ended` |
| `Attachment.kind` `handoff_starting` | On the old Job. `standing` while the new Job comes up, then `given_back` | The ring |
| `Attachment.kind` `handoff_failed` | On the old Job. `standing`, `detail.reason`, `target` the instant; `given_back` at the owner's next message to the Job | The band "Failed to hand off to new job". The old Job is live and unchanged |
| `ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY` | `Carry on in a new Job`, `Stay here` | The two option labels of the Job's question. Fleet acts on the owner's choice of the first, in any question |

The new Job also has the ordinary `forked_from` row, and the old one `forked_to`.
The sequence is `session.changed` for the old Job (`handoff_starting`), then for both as the ledger moves, then the old Job ended.

**Skew.** A Bridge before this meets kinds it has no screen for, which `attachmentsOfRecord` leaves out, and draws the old Job live and the new one beside it.
Bridge's half is `packages/screens/src/sessions-wire.ts`: `replaces` to `Session.replaces`, `replaced_by` to `replacedBy`, `handoff_starting` and `handoff_failed` to `handoff`.
