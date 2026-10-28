# Protocol 23.68: a Trigger that blocks holds the Job

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md (deleted)`, *A failed Trigger with `block` on*. **Additive only**: two operations, one state, one skip reason, one escalation reason and two optional fields.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.state`, `AddedStep.state` | `held` | A failure with `block` on and no repair, or a repair that did not fix it. A Bridge that does not know it draws an unknown state, and the Job's own status says it is waiting |
| `JobTrigger.blocks?` | `true` where the Trigger blocks | Left out where it does not. It tells `repairing`, `rerunning` and `fix_ready` on a Trigger that holds the Job from one that does not |
| `TriggerSkipReason`, `AddedSkipReason` | `by_owner` | The owner skipped it while it held the Job |
| `escalation_reason` (`JobSummary.reason`) | `trigger_held` | A Job stopped before a Drone by a hold at `step_starts` or `step_passes`. Job-level, so no step is named |
| `rerun_trigger` (`POST /jobs/:job_id/rerun_trigger`) | `HoldAct`: `trigger` or `addition`, one | `HoldSettled`: `state`, `released`. 409 `fleet.no_hold`, `fleet.hold_repairing`, `fleet.hold_has_a_fix`, `fleet.hold_nothing_to_run`, `fleet.hold_no_worktree`, `fleet.hold_job_working`, 422 `fleet.no_hold_named`. `Bridge only`. It may take as long as the Command |
| `skip_trigger` (`POST /jobs/:job_id/skip_trigger`) | `HoldAct` | `HoldSettled`. 409 `fleet.no_hold`, `fleet.hold_repairing`. `Bridge only` |
| `approve_review`, `merge_pull_request` | | **A new 409**, `fleet.trigger_holds`, at a gate a `pr_opened` hold stands in front of. Nothing moves |
| `JobSummary.alert?` | `JobAlert`: `kind` (`held`, `fix_ready`, `failed`), `trigger`, `when`, `step` | **The Board row's bell**: a hold, a repair fix waiting on the owner's choice, a Trigger that failed after its repair tries. Today the bell shows only on the open Job. Absent is none |

**`list_alerts` says which Trigger**: a Job a hold stopped is already in `blocked`, and its `why` names the Trigger. A `pr_opened` hold on a Job at its gate is in `waiting` with the same words.

**The event stream: no change.** Every state a hold passes through is a `job.trigger_changed` or `job.addition_changed` that already existed, and a rerun adds one transient `rerunning` that is not stored.

**Skew.** A Fleet before 23.68 holds nothing and sends no `blocks`, no `alert` and no `held`, which Bridge reads as every row before.
A Bridge before it draws `held` as an unknown state and has no Rerun or Skip, and the Job it cannot answer is `escalated`, whose acts it already offers.
Bridge's half is `packages/protocol/src/trigger-holds.ts (deleted)`, written by hand like the rest.
