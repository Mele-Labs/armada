# Protocol 23.6: a model per task, and a person's edit to a task

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, the wire lock for the new Job, slice 3 (#1764, carrying #1657).

**Optional fields, one new DTO pair and two routes, all additive.** `PlanTask` gains `tier`, a
strict `TaskTier` (`difficult`, `medium`, `easy`), and `model`, a person's pick. `JobDetail` gains
`tiers`, a `TierModels` of `difficult?`, `medium?` and `easy?`, and `JobDrone` gains `model`, the
model that Drone was spawned as. **An empty tier is a key left out, never `null`** (answer 8): a
tier the map leaves out is Armada picking, and the Drone's `model` says what it picked. Each is
absent where empty, so a 23.5 Bridge connects behind the banner and draws what it drew.

**Every spawn resolves its model in spike 022's order**: a person's pick on the task, then
`tiers` for the task's tier, then the step's model, then the Job's. `Job::model_spawned_for`
spells it once. A Drone on no task asks neither of the first two, and still records its model.

| Route | Body | Answers | Refused |
|---|---|---|---|
| `POST /jobs/:job_id/tasks/:task_id/edit`, `edit_task` | `EditTask`, only the changed fields | `WorkPlan` | 409 `fleet.task_in_flight` on a task working or handed in, `fleet.task_already_settled` on one done or dropped; 409 on a model `list_models` does not offer, as `set_model` refuses one; 422 on a body changing nothing or a blank title, and `fleet.no_such_task` |
| `POST /jobs/:job_id/set_tiers`, `set_tiers` | `SetTiers`: `tiers` | `JobSummary` | 409 on any model `list_models` does not offer, keeping nothing |

**The edit's body is the lock's, which is Bridge's.** `EditTask` moved from `pending.ts` to
`work-plan.ts` unchanged. It reaches an open or a failed task, so it works before the plan's gate
and after a failure alike, and it is appended as a person's change: `WorkPlan.recorded_by` stays
the planner, and the history reads the edit after the recording. Bridge's pending entry goes with
the Bridge half, which answers it in the mock.

**`set_tiers` is a choice the lock did not make.** The spike sources a Job's tiers from *dispatch,
then approval*, and both are slice 4's bodies; a Job needs a map before then for slice 3's claim
to be anything but a test, so the map is a Job setting `set_model`'s way, and slice 4 fills it from
those two as well.

**`record_plan` takes an optional `tier` per task.** It is an MCP tool, not this protocol, and its
own schema says so. **Store V97** keeps a recorded task's tier, a person's edit, a Job's map and
the model each Drone ran.
