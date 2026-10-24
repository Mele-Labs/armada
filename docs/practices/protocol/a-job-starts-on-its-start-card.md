# A Job starts on its start card

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 23 Oct 2026. `docs/concepts/session.md`, *The start card*.
**Additive only**: one field on `StartSession`, one on `HostedFacts`, three types and one operation.
The protocol ID moves with the files below, and there is no number to take.

| Where | Side | Change |
| --- | --- | --- |
| `StartSession.held` | Bridge sends, Fleet takes | `bool`, left out when false. Holds the Job on its start card: no agent runs and a message is kept until `begin_session`. Not read with `fork`, `root` or `after`, or for Helm's start |
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
