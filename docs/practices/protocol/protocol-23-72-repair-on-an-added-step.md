# Protocol 23.72: repair on an added step

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md`, *Steps added to one Job*.
**Additive only**: one optional field on `AddedStep`, one on `ChooseTriggerFix`, one 422 and no new operation.
A failed added Script with `repair` on is repaired as a Trigger is, and the repair branch is deleted once its fix is placed.

| Where | Carries | Notes |
| --- | --- | --- |
| `AddedStep.repair_record?` | `TriggerRepair`, as `JobTrigger.repair` | Present from the first repair Drone. Named apart from `AddedStep.repair`, which is whether the step asks for one. `AddedStep.block` already tells a hold from a plain repair |
| `AddedStep.state` | `repairing`, `rerunning`, `fix_ready` | The states 23.68 added to `JobTrigger.state`, now reached by an added step too. A step that blocks holds the Job through each |
| `choose_trigger_fix` | `ChooseTriggerFix`: `trigger?`, `addition?`, `choice` | **Exactly one of `trigger` and `addition`**, as `HoldAct`. `trigger` was required and is now optional, which only relaxes what Fleet reads: a Bridge before 23.72 still sends it and Fleet still takes it. 422 `fleet.no_fix_named` where both or neither is sent; 409 `fleet.no_fix_waiting` where the step holds no fix |
| `list_alerts`, `JobSummary.alert` | | An added step's fix waiting on a choice, and one that failed after its repair tries, are alerts as a Trigger's are. Nothing new is read |

**A repair branch is deleted** after `this_branch` has merged it and when the repair ends `failed` or `held`, for a Trigger as for an added step.
It is a Fleet act with no wire: `branch` on `TriggerRepair` names a branch that may no longer exist once the state is `passed` with `this_branch`, `failed` or `held`.
A `new_pr` branch is the pull request's head and stays.

**Skew.** A Fleet before 23.72 sends no `repair_record` and acts on no added step's `repair`, which Bridge reads as every row before.
A Bridge before it never sends `addition`, so a fix it cannot see is one it cannot place.
Bridge's half is `packages/protocol/src/added-steps.ts (deleted)` and `triggers.ts`, written by hand like the rest.
