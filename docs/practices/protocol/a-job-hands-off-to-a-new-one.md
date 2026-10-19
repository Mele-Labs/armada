# A Job hands off to a new one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session/a-handed-off-job.md`. One operation, `retry_handoff`, six ledger kinds, which are open text, four option labels, a follow-on question and a setting.

| Where | Carries | Notes |
| --- | --- | --- |
| `Attachment.kind` `replaces` | On the new Job. `target` is the old Job's id, `spent`, `detail.context` the handoff and brief the new Job starts from | Bridge folds the old Job behind this one in the same row, and reads only a `spent` one: a rolled-back attempt gives it back |
| `Attachment.kind` `replaced_by` | On the old Job. `target` is the new Job's id, `spent` | The old Job is `ended` |
| `Attachment.kind` `handoff_starting` | On the old Job. `standing` while the new Job comes up, then `given_back`. `detail.brief` the brief, `detail.new` the new Job's id once it is made. One standing at Fleet's start is settled then | The ring |
| `Attachment.kind` `handoff_failed` | On the old Job. `standing`, `detail.reason` a bare fact in lowercase, `detail.brief` the brief kept for a retry, `target` the instant; `given_back` at the owner's next message to the Job, or by a retry | The band "Failed to hand off to new job". The old Job is live and unchanged, and the only live one |
| `retry_handoff` | `POST /sessions/handoff/retry`, body `{ "session_id": <old Job> }`, answers the new Job's `SessionRecord` | Retry on the band. 409 `fleet.handoff_not_failed` where no `handoff_failed` row stands, 409 `fleet.handoff_running` while a hand runs, 422 `fleet.handoff_failed` (message the reason) where it fails again |
| `Attachment.kind` `handoff_quiet` | On the Job. `spent`, `target` the Job's id | The owner chose `Stay, don't ask again this Job`. The Context rule never tells the Job again |
| `Attachment.kind` `handoff_remind_at` | On the Job. `standing`, `target` the figure in tokens (`500000`); `spent` once the Job is told past it, `given_back` by a later figure | The Context rule tells the Job nothing until its context passes the figure |
| `ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY`, `ipc::HANDOFF_QUIET`, `ipc::HANDOFF_REMIND` | `Carry on in a new Job`, `Stay here`, `Stay, don't ask again this Job`, `Remind me at a higher limit` | The four option labels of the Job's question, mirrored in `packages/protocol/src/helm-calls.ts`. Fleet acts on the owner's choice of the first, third and fourth, in any question |
| `ipc::HANDOFF_REMIND_QUESTION`, `ipc::HANDOFF_REMIND_AT` | `Ask again at`; 450000, 500000, 600000, labelled `450k` | Fleet's own question after the fourth, an ordinary ask on the same Job's card (`hosted.asked`, an `ask` row), while the Job's call waits. Its answer is added to the call's `answers` |
| `plan.contextHandoff` | `ask`, `carry on automatically` or `never`, in `get_settings` with an empty `description` and `tip` "Context limit" | `carry on automatically` runs the hand with no card; `never` tells no Job |

The new Job is not a fork and has no `forked_from` row.
The sequence is `session.changed` for the old Job (`handoff_starting`), then for both as they are linked, then for both as the ledger moves, then the old Job ended, then the new Job's first turn.

**Skew.** A Bridge before this meets kinds it has no screen for, which `attachmentsOfRecord` leaves out, and draws the old Job live and the new one beside it.
Bridge's half is `packages/screens/src/sessions-wire.ts`: `replaces` to `Session.replaces`, `replaced_by` to `replacedBy`, `handoff_starting` and `handoff_failed` to `handoff`.
`handoff_quiet` and `handoff_remind_at` are Fleet's to read, and Bridge draws neither.
