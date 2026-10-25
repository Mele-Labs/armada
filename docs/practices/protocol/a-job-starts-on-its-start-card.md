# A Job starts on its start card

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. `docs/concepts/session.md`, *The start card*.
**Additive only**: one field on `StartSession`, one on `HostedFacts`, three types and one operation.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `StartSession.held` | Bridge sends, Fleet takes | `bool`, left out when false. Holds the Job on its start card: no agent runs and a message is kept until `begin_session`. Not read with `root` or `after`, or for Helm's start. Read with `fork` since 25 Oct |
| `HostedFacts.start` (`StartHeld`) | Fleet writes, Bridge reads | Present while the Job waits for `begin_session`, absent once it has begun and for a Job never held. Left out when absent |
| `StartHeld` | | `asks_delivery`, and `picked?` (`DeliveryPicked`) |
| `DeliveryPicked` | | `choice` (`DeliveryChoice`) and `from` (`DeliveryPickedFrom`) |
| `DeliveryPickedFrom` | | `usual` or `named` |
| `begin_session` (`POST /sessions/begin`) | Bridge sends, Fleet takes | `BeginSession`: `session_id`, `delivery?` (`DeliveryChoice`). Answers the `SessionRecord`, whose `hosted.start` is gone. `agent_access` No |

**`asks_delivery` is true where the Job's workflow has a step that asks how the work lands.**
It is false for a custom Job and for a workflow with no such step.
`picked` is absent where the person has no usual yet and the first message names none.

**`delivery` is required where `asks_delivery` holds and ignored elsewhere.**
Where it is read, Fleet keeps it as the answer to the delivery question (`a-jobs-delivery-question.md`), turns Auto land on for `auto_land`, and records the pick.
Where it is ignored, nothing is written.

| Refusal | Status | When |
| --- | --- | --- |
| `fleet.session_waiting_unheld` | 409 | The Job is not held: begun already, or never held |
| `fleet.session_waiting_empty` | 422 | `asks_delivery` holds and `delivery` is absent |
| `fleet.session_closed` | 409 | The Job has ended |
| `fleet.no_such_session` | 422 | Fleet does not host it |

A refusal changes nothing.
The press that lifts the hold is the one that begins, so a second finds the Job unheld.

**Two additive migrations.**
`delivery_picks.table` keeps each landing he chose, by repository and workflow, and the usual is read off the last ten.
`session_start_held.hosting_flag` adds `hosted_sessions.start_held`, default 0.
The hold survives a Fleet restart because the column does.

**Skew.** A Bridge and a Fleet from either side of this change have different IDs, so the two are refused regardless.
The event stream is untouched beyond the `session.changed` a hosted Job already rides on, which makes the unmeasured sink risk neither better nor worse.
Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand.

## Every new Job is held, with its first message on the card

Since 25 Oct 2026. `docs/concepts/session/the-start-card.md`.
**Additive on the wire**: two optional fields and one enum value.
One wire behaviour goes: the `unsent` attachment.

| Where | Side | Change |
| --- | --- | --- |
| `StartHeld.message` | Fleet writes, Bridge reads | `string`, left out when absent. The first turn the agent will read, as it will read it: a pinned note's with Fleet's `Pointed at:`, `Candidates:` and `Page:` lines and the handoff, another Job's with the line that names it. Absent until a first turn is kept |
| `StartHeld.picked` | Fleet writes, Bridge reads | Now present wherever `asks_delivery` holds: named, else usual, else `default` |
| `DeliveryPickedFrom::Default` | | Wire spelling `default`: neither named nor usual, so Fleet offers Auto land. Bridge marks only `named` and `usual` |
| `BeginSession.message` | Bridge sends, Fleet takes | `string`, left out when absent. The box as he left it |
| `StartSession.held` | Bridge sends, Fleet takes | Now read with `fork` too |
| `unsent` attachment | Fleet wrote, Bridge read | Gone. Nothing writes it, and the `first_message` item no longer reads it |

**The first turn waits off the thread.**
The first message, note or other Job's message to reach a held Job is kept with the Job, its files and pictures with it, and is not written to the thread.
It is what `StartHeld.message` reports.
A later turn while held is written to the thread at once and queued, as before.
`begin_session` writes the first turn to the thread and sends it first, then the rest.

**`message` on `begin_session` replaces the first turn where it differs**, trimmed, from what `StartHeld.message` reported.
The thread row and what the agent reads are then his words alone, in his voice, and the files stay with them.
Where it is the same, or absent, the turn goes as it was kept.
Where nothing was kept, a `message` with words is the first turn.
An emptied box on a turn with no files sends nothing.

**A held Job waits in Needs you.**
One item while it is held: id `unsent:<session id>`, text `First message`, source `first_message`, act `message` on the Job.
`begin_session` settles it, and so does ending the Job.
`answer_waiting` on it is a 422 `fleet.session_waiting_empty`, as it was.

**Every new Job is held**, whoever starts it.
`start_another_session` and `fork_this_session` start one held, with the agent's `message` as its first turn, and a start with `after` is held with the handoff and brief once its wait is met.
`hand_to_session` is the person's own press and is not held.

**A held Job is never without a workflow.**
Once the proposer's reading is done, or at once where none will run, a held Job with no workflow takes the repository's `feature` workflow, else its first.
A workflow the person set, custom too, is left alone.

**One additive migration.**
`held_first_turns.table` keeps the first turn, one row per Job, until Start takes it.
A Job held before this build has its first message on the thread, and it is sent as a queued turn.

**Skew.** The protocol ID moves with these files, so a Bridge and a Fleet from either side are refused regardless.
The event stream carries nothing new beyond the `session.changed` a hosted Job already rides on, which makes the unmeasured sink risk neither better nor worse.
