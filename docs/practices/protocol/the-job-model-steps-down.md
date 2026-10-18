# The Job model steps down

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: five operations, one event kind, two types. Bridge's half is `packages/protocol/src/model-downshift.ts` and `events.ts`, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_model_downshift` (`GET /model_downshift`) | `ModelDownshiftState`: `downshift?`, tagged by `phase` | `offered` (`percent`, `was`, `to`), `shifted` (`was`, `to`, `until`) or `reset` (`was`, `to`). Absent is none |
| `model_downshift.changed` (event) | `ModelDownshiftState`, whole | Published when the state moves, the offer's percent included. Served and published, so a Bridge opened later asks |
| `accept_downshift` (`POST /model_downshift/accept`) | no body | Valid while `offered`. Writes `models.session` through `save_settings`' path; a refused write keeps the state |
| `restore_downshift` (`POST /model_downshift/restore`) | no body | Valid while `shifted` or `reset`. Puts the saved word back, or removes the key |
| `keep_downshift` (`POST /model_downshift/keep`) | no body | Valid while `reset`. Leaves the lower model as the default |
| `settle_downshift` (`POST /model_downshift/settle`) | no body | Valid while `offered`. Makes no second offer in this window |

**A command in the wrong phase is a 409** (`fleet.downshift_wrong_phase`), never a silent no-op, and each command answers the state it left. All four are `agent_access` `No`: a step down answers an offer Bridge draws, and Helm moves `models.session` with `save_settings`. Settings: `plan.downshiftAt` and `plan.downshiftTo` in a new `Plan` section, and `models.default` and `models.session` are `live`. The step down moves `models.session` and never `models.default`, which Drones and Jobs keep.

**The event stream.** `model_downshift.changed` is rare: an offer, an accept, a reset, a restore, and the percent moving a whole point while an offer stands. It adds no queue and assumes no delivery is complete: a Bridge that missed one reads `get_model_downshift` on the resync that follows a `Missed`.

**Bridge's side.** Main passes the event to every window and to `downshift-notice.ts`, which tells the OS when the state enters `offered` or `reset`, once for each entry; the state read on a connection is remembered and told to nobody. A window's source (`fleet-downshift.ts`) reads once Fleet is connected, and a 409 from an act makes it read again rather than show an error.
