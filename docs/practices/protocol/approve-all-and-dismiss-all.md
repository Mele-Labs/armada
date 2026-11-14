# Approve all and Dismiss all: `answer_items` and `dismiss_items`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 13 Nov 2026. His review panel answers or dismisses every item with an Approve, across Jobs, in one press. `docs/concepts/artifact.md`, *Several at once*.
**Additive only**: two operations, two routes and their body and answer types. No field of an existing type, no event kind and no error kind is new.
Bridge's half is `packages/protocol/src/job-artifacts.ts`, written by hand. Classic does not call them, and native's review panel calls them later.

| Where | Side | Carries |
| --- | --- | --- |
| `answer_items` (`POST /sessions/items/answer`, `agent_access = "No"`) | Bridge writes | `AnswerItems` `{ items }`, each `ItemAnswer` `{ session_id, item, how, notes? }`, `how` `approve` or `send_back` |
| `dismiss_items` (`POST /sessions/items/dismiss`, `agent_access = "No"`) | Bridge writes | `DismissItems` `{ items }`, each `ItemRef` `{ session_id, item }` |
| Both answers | Fleet writes, Bridge reads | 200 `ItemsSettled` `{ outcomes }`, one `ItemOutcome` `{ session_id, item, outcome }` per item in the order asked. `outcome` is `ItemSettled`, tagged by `kind`: `settled`, or `refused` with `code` and `message` |
| Refusals, per item | Fleet | `fleet.items_not_waiting`, `fleet.items_not_approvable`, `fleet.items_send_back_empty`, `fleet.items_on_another_mac`, or the code the item's own answer refuses with |

**A body that does not decode is a 422**, as every route's is, and nothing is settled.
Any other refusal is one item's, and the call still answers 200.

**A Job another of his Macs holds is refused, not sent on.** A single answer is relayed whole to that Mac, and a batch is not split by Mac.

**The event stream.** No new event: each item publishes `session.changed` on its Job, as its own answer does.
One press of Approve all is as many presses as it has items, in one burst, so the unmeasured sink risk is a little worse and stays unmeasured.

**Skew.** The routes are new, so the protocol ID moves and a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
