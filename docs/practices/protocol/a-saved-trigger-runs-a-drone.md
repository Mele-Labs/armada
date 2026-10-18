# A saved Trigger runs a Drone

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md`, *A Skill or a Drone step*.
**Additive only**: one variant of `TriggerRuns` and no new operation, state or error.
A saved Trigger can name a prompt, and a Drone is sent with it on a side Drone as a Skill is.

| Where | Carries | Notes |
| --- | --- | --- |
| `TriggerRuns` | `{ kind: "drone", brief }` | The prompt, named as an added Drone step's is. A Trigger file spells it `brief:`, and an empty one is refused naming `brief`. `TriggerSummary.runs` serves it |
| `JobTrigger.drone` | `true` | Now also where the Trigger names a prompt |
| `save_trigger` `kept_from` | | A Drone step can be kept for every Job. 422 `fleet.unacceptable_addition` is no longer raised for one |

**`repair` is ignored**, as for a Skill: a file with `brief:` and `repair: true` loads with it off.
