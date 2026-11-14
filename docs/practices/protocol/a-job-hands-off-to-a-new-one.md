# A Job hands off to a new one

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session/a-handed-off-job.md`. Two operations, `handoff_brief` and `retry_handoff`, seven ledger kinds, which are open text, one row kind, one waiting source and a setting.

| Where | Carries | Notes |
| --- | --- | --- |
| `Attachment.kind` `replaces` | On the new Job. `target` is the old Job's id, `spent`, `detail.context` the handoff and brief the new Job starts from | Bridge folds the old Job behind this one in the same row, and reads only a `spent` one: a rolled-back attempt gives it back |
| `Attachment.kind` `replaced_by` | On the old Job. `target` is the new Job's id, `spent` | The old Job is `ended` |
| `Attachment.kind` `handoff_starting` | On the old Job. `standing` while the new Job comes up, then `given_back`. `detail.brief` the brief, `detail.new` the new Job's id once it is made. One standing at Fleet's start is settled then | The ring |
| `Attachment.kind` `handoff_failed` | On the old Job. `standing`, `detail.reason` a bare fact in lowercase, `detail.brief` the brief kept for a retry, `target` the instant; `given_back` at the owner's next message to the Job, or by a retry | The band "Failed to hand off to new job". The old Job is live and unchanged, and the only live one |
| `retry_handoff` | `POST /sessions/handoff/retry`, body `{ "session_id": <old Job> }`, answers the new Job's `SessionRecord` | Retry on the band. 409 `fleet.handoff_not_failed` where no `handoff_failed` row stands, 409 `fleet.handoff_running` while a hand runs, 422 `fleet.handoff_failed` (message the reason) where it fails again |
| `handoff_brief` | `POST /sessions/handoff_brief`, body `HandoffBrief` `{ "brief": <text>, "session_id"?: <Job> }`, answers the `SessionRecord`; through the agent's door a receipt `{ "session_id", "handoff": "handoff:<row>" }` | `crates/ipc/operations/handoff_brief.toml`. 422 `fleet.handoff_not_asked` where no `handoff_asked` row stands, 422 `fleet.handoff_brief_empty` for a blank brief. The brief is trimmed and cut to 6,000 characters |
| `Attachment.kind` `handoff_asked` | On the Job. `standing`, `target` the Job's id, from the Context rule's telling; `spent` once a brief raises the card | What `handoff_brief` checks |
| `SessionRow` `handoff` | `id`, `at`, `tokens`, `limit`, `brief`, `remind_at` (three figures in tokens), and `answer` once pressed: `HandoffPick`, `{ "kind": "carry_on" }`, `stay`, `quiet`, or `{ "kind": "remind_at", "tokens": 450000 }` | Replaced by id once answered. Mirrored in `packages/protocol/src/hosted-sessions.ts`; Classic's `sessions-wire.ts` leaves the kind out |
| `WaitingSource` `handoff` | Item `handoff:<row>`, text `Carry on in a new Job?`, act `answer`, options `Carry on in a new Job`, `Stay here`, `Don't ask again`, then `Remind at <figure>` for each figure (`ipc::HANDOFF_YES`, `ipc::HANDOFF_STAY`, `ipc::HANDOFF_QUIET`, `ipc::HANDOFF_REMIND` with `ipc::handoff_figure`) | `answer_waiting` with a `choice`, by him only; `dismiss_waiting` is Stay. The agent is sent nothing |
| `Attachment.kind` `handoff_quiet` | On the Job. `spent`, `target` the Job's id | The owner chose `Don't ask again`. The Context rule never tells the Job again |
| `Attachment.kind` `handoff_remind_at` | On the Job. `standing`, `target` the figure in tokens (`500000`); `spent` once the Job is told past it, `given_back` by a later figure | The Context rule tells the Job nothing until its context passes the figure |
| `plan.contextHandoff` | `ask`, `carry on automatically` or `never`, in `get_settings` with an empty `description` and `tip` "Context limit" | `carry on automatically` runs the hand with no card; `never` tells no Job |

The new Job is not a fork and has no `forked_from` row.
The sequence is `session.changed` for the old Job (`handoff_starting`), then for both as they are linked, then for both as the ledger moves, then the old Job ended, then the new Job's first turn.

**Skew.** A Bridge before this meets kinds it has no screen for, which `attachmentsOfRecord` leaves out, and draws the old Job live and the new one beside it.
Bridge's half is `packages/screens/src/sessions-wire.ts`: `replaces` to `Session.replaces`, `replaced_by` to `replacedBy`, `handoff_starting` and `handoff_failed` to `handoff`.
`handoff_asked`, `handoff_quiet` and `handoff_remind_at` are Fleet's to read, and Bridge draws none of them.
A Bridge before `handoff_brief` meets a `handoff` row and a `handoff` waiting source it has no screen for; Classic leaves the row out.
