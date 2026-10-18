# Protocol 23.4: groups, a task that failed, and two acts on them

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, the wire lock for the new Job, slice 2 (#1763, carrying #1652, #1656 and #1685).

**Optional fields, one new DTO pair and two routes, all additive.** `WorkPlan` gains `groups`,
each a `PlanGroup` naming its tasks, its `state` (`GroupState`, the registry's eight words) and
every run as a `PlanGroupRun`: its own number, the step's run it was filed under, its verdict and
the commit a green run made. `PlanTask` gains `group` and `failed_reason`, `reason` staying a
drop's alone. `CheckRun` and `Recorded` gain `group` and `group_attempt`, absent where no group's
gate made the row, which means exactly that and never "unknown" (#1652). `JobPlanChanged` gains
`group`, on the change a group's verdict made.

**Fleet writes `failed`, and only once a group's retries run out** (answer 9). A red group goes
round on its own, its tasks staying `handed_in`; the last red run fails every task in the group
with a reason naming the group and the run. A Judge refusal stops the group for a person, as it
stops a step (answer 3). the retired Plan concept, *Groups*.

**Restart this task also answers a done task in a group the Judge refused** (2 Oct 2026), with
no change to the wire: Bridge reads the group's last run, `verdict.trigger` `gate_failure` over
tasks still `done`, with the step that run was filed under `stopped`, as Fleet does. **A group the
Judge only questioned is refused**: its step holds at `awaiting_human` until a person answers.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/tasks/:task_id/restart`, `restart_task` | `RestartTask`, an optional `note`; no body is valid | `JobSummary` | 409 `fleet.task_not_failed` on a task that has not failed, except a done one in a group the Judge refused |
| `POST /jobs/:job_id/plan/move`, `move_plan` | `MovePlan`: `group`, `task?`, `after?` | `WorkPlan` | 409 `fleet.task_in_flight` on a task, or a group holding one, still in its run; 422 `fleet.no_such_group`, `fleet.no_such_task` |

**The bodies are the lock's.** Bridge sent `to`, an index; the wire takes `after`, as `add_task`
places a task, because an index counted at the drag is stale the moment a Drone adds or drops one.
`MovePlan` moved from `pending.ts` to `work-plan.ts` with that shape. Bridge's two pending entries
go with the Bridge half, which draws the groups and answers both acts in the mock.

**`record_plan` takes an optional `group` per task**, a number in the order groups run; a task
naming none joins the group before it, so a plan recorded without groups is one group, which is
how every plan before this reads.

**Store V95** keeps each group's runs, the group and run in `job_step_checks`' key so two groups
gated on one run of a step keep both their rows, and a plan's two moves and a failed task's reason.
