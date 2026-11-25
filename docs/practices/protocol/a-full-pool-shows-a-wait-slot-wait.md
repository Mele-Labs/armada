# A full pool shows a wait: `slot_wait`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session/a-job-fleet-hosts.md`, *A full pool queues the job*.
**Additive only**: one field on `SessionRecord`, `slot_wait`, and one type, `SlotWait`.
No event kind.
The protocol ID moves with the files below, and there is no number to take.

**Nobody is asked.**
A Job that finds every slot held waits, and Fleet raises no waiting item for it.
The `slot_choice` item, id `slot-choice`, is no longer derived.
`WaitingSource` keeps the `slot_choice` value, and Fleet no longer writes it.

| Where | Side | Change |
| --- | --- | --- |
| `SessionRecord.slot_wait` | Fleet writes, Bridge reads | `SlotWait`, absent when the Job waits for no slot. Set while the Job is in its repository's line for a slot, or its `armada worktree lease` is still asking, and cleared when it is moved in or leaves. Never kept: derived with the record |
| `SlotWait` | Fleet writes, Bridge reads | `{since, ahead, holders?, creating_since?, create_refused?}`. `ahead` is how many Jobs are ahead of it in the line |
| `SlotWait.holders` | Fleet writes, Bridge reads | Absent when empty. `SlotHolder` `{slot, job: {id, title}, quiet_since, keeps?}`, longest quiet first, one for each slot a Job Fleet hosts holds; a terminal's or a person's has none. `keeps` is `running` (its turn runs) or `uncommitted` (its checkout holds files no commit has), and is why Free is off: **Fleet decides it, Bridge only draws it**. Read for every Job in the line when it joins and every 15 seconds while any waits, never on a publish, and a Job is published only when its rows changed |
| `SlotWait.creating_since` | Fleet writes, Bridge reads | When Fleet began the slot Add asked for, while it is being made. Bridge does not count the Job as waiting on the person meanwhile |
| `SlotWait.create_refused` | Fleet writes, Bridge reads | Fleet's words for why the slot was not made. Cleared when Add is sent again |
| `answer_waiting` | Bridge sends, Fleet takes | **Add** is `item_id` `slot-choice` with `choice` 1, accepted though no item lists it. It answers at once and makes the slot after: `creating_since` stays on the record until the Job is moved in. A second Add while one is out changes nothing. Choice 0 (Wait) is accepted and changes nothing, and nothing offers it. **No message goes to the agent**: it was told to end its turn when it was queued, and is moved into its slot when one is granted. Any other `choice`, words, or a mode without a `choice` is a 422 `fleet.session_waiting_empty`. With no wait, 409 `fleet.session_waiting_unheld` |
| `free_slot` | Bridge sends, Fleet takes | `POST /sessions/slot_choice/free`, body `FreeSlot {session_id, item_id: "slot-choice", slot}`, answers the waiting Job's `SessionRecord`, `agent_access` `No`: the person's press and no path of Fleet's. Refused with nothing changed: the Job no longer waiting, 409 `fleet.session_waiting_unheld`; holder running, 409 `fleet.slot_session_working`; uncommitted work, 409 `fleet.slot_uncommitted`; slot not held by that Job any more, 409 `fleet.slot_holder_changed`. Works while Add is under way |
| `dismiss_waiting` | | Nothing to dismiss: no item stands for the wait |

**The same wait from a command.** `ask_for_slot` (`POST /sessions/slot_wanted`, `agent_access` `No`) is `armada worktree lease` run by a hosted Job telling Fleet every slot is held.
Body `AskForSlot`: `act` (`ask`, repeated about once a second, or `done`) and `session_id`, read by the command from `ARMADA_SESSION`.
Answers `SlotAsked`: `standing` (`asking`, `waiting`, `added` or `unasked`).
`asking` means the command waits and the Job's `slot_wait` says so; the command prints that every slot is held, that it waits for one, and that the owner can add or free one.
`waiting` is the answer to an older Wait, and the command keeps looking.
Bridge never sends it, and `classic-final:packages/protocol/src/sessions.ts` carries both types so the wire reads whole.
It sets the same `slot_wait`, and one where the Job is also in the line: **one wait, never two**.
Fleet forgets a command it has not heard from for ten seconds.
Add for a command with no line makes a slot and holds it for that command until `done` or thirty seconds; for a Job in the line, the slot goes to the Job and the command keeps waiting.
A command with no `ARMADA_SESSION`, or no Fleet, does not call it.
`armada need` has nothing to order here either.

**Skew.** A Bridge before this sees no `slot-choice` item and no `slot_wait`, which it does not read.
A Job waiting for a slot then shows no sign of it, and the Job's record is otherwise unchanged.
Classic reads neither, so it no longer shows the wait.
The event stream is untouched beyond the `session.row` and `session.changed` the record already rides on.
