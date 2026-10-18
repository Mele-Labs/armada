# Protocol 23.68: a failed Trigger's repair on the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md`, *A failed Trigger with `repair` on*. **Additive only**: one operation, three states and one optional field.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.state`, `JobTriggerChanged.trigger.state` | `repairing`, `rerunning`, `fix_ready` | A failure with `repair` on stops at `repairing`, not `failed`. A Bridge that does not know them draws an unknown state |
| `JobTrigger.repair?` | `TriggerRepair`: `attempt`, `branch?`, `files`, `choice?`, `pull_request?` (`url`, `number?`) | Present from the first repair Drone. `attempt` is 1 or 2 and no client draws it as a count. `files` is what the fix changes, set when it is held |
| `choose_trigger_fix` (`POST /jobs/:job_id/choose_trigger_fix`) | `ChooseTriggerFix`: `trigger`, `choice` (`this_branch` or `new_pr`) | `TriggerFixChosen`: `state`, `pull_request?`. 409 `fleet.no_fix_waiting`, `fleet.fix_not_placed`, `fleet.fix_conflicts`, `fleet.fix_waiting`. `Bridge only` |

**A fix waiting on the owner is an alert already**: `list_alerts` carries it, and a Trigger that failed after both tries.
Nothing new is read for it.
Bridge's half is `packages/protocol/src/triggers.ts`, written by hand like the rest.
