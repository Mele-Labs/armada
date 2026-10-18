# A full pool asks the person: `slot_choice`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/session.md`, *A full pool asks the person*. **Additive only**: one value on `WaitingSource`, no operation, no event kind and no route. The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `WaitingSource` `slot_choice` | Fleet writes, Bridge reads | Derived on every read, never kept. Id `slot-choice`, text `Every slot is in use`, `act` `{kind: "answer", target: "slot-choice"}`, `options` exactly `Wait for a slot` (choice 0) and `Add a slot` (choice 1), labels only. Present while the job is in the line for a slot and nobody has chosen |
| `answer_waiting` | Bridge sends, Fleet takes | `choice` 0 or 1 on that item. **No message goes to the agent**: it was told to end its turn when it was queued, and is moved into its slot when one is granted. Words or a mode without a `choice` are a 422 `fleet.session_waiting_empty` |
| `dismiss_waiting` | | Refused 422 `fleet.session_waiting_empty` for a `slot_choice` item: it is answered |

**The same call from a command.** `ask_for_slot` (`POST /sessions/slot_wanted`, `agent_access` `No`) is `armada worktree lease` run by a hosted Job telling Fleet every slot is held. Body `AskForSlot`: `act` (`ask`, repeated about once a second, or `done`) and `session_id`, read by the command from `ARMADA_SESSION`. Answers `SlotAsked`: `standing` (`asking`, `waiting`, `added` or `unasked`). Bridge never sends it, and `packages/protocol/src/sessions.ts` carries both types so the wire reads whole. It raises the same `slot-choice` item, and one item where the Job is also in the line: **one call, never two**. Fleet forgets a command it has not heard from for ten seconds. *Add a slot* for a command with no line makes a slot and holds it for that command until `done` or thirty seconds; for a Job in the line, the slot goes to the Job and the command keeps waiting. A command with no `ARMADA_SESSION`, or no Fleet, does not call it. `armada need` has nothing to order here either.

**Skew.** A Bridge before this meets `slot_choice` as a source it does not know. The IDs differ, so the two are refused regardless. The event stream is untouched beyond the `session.row` and `session.changed` the item already rides on.
