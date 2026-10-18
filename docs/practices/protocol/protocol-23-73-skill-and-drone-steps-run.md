# Protocol 23.73: Skill and Drone steps run

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md`, *A Skill or a Drone step*. **Additive only**: one optional field on `JobTrigger`. A Skill Trigger, a Skill added step and a Drone added step now run on a side Drone, on a branch cut from the Job's, and no state, operation or error is new.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobTrigger.drone?` | `true` where the Trigger names a skill | Left out for a Command. Lets Bridge name a Skill's branch apart from a repair's. An added step says the same through `AddedStep.runs.kind` |
| `JobTrigger.state`, `AddedStep.state` | `running` with `repair` / `repair_record` present | **A Drone is on it.** A Command that is running has no repair record. It ends `fix_ready` (the Drone committed), `passed` (it changed nothing) or `failed`, `held` where it blocks |
| `choose_trigger_fix`, `rerun_trigger`, `skip_trigger` | | Unchanged. A Skill's or Drone step's fix is placed as a repair's is, with no Command to run again, so `this_branch` ends `passed` once merged. Rerun on a held one puts the Drone on again and answers `running` |
| `TriggerSkipReason.skill_not_run`, `AddedSkipReason.skill_not_run`, `drone_step_not_run` | | **No longer produced.** The values stay, so a row an earlier build wrote still reads and removing them is not a major bump |

**`repair` is ignored for a Skill or a Drone step.** A Drone already fixes its own failures, so Fleet reads it as off: a Trigger file naming a skill loads with `repair` false, and `add_job_step` and the approval store `repair` false for a Skill or a Drone step. `block` stands.

**Skew.** A Fleet before 23.73 records a Skill `skipped` and sends no `drone`; Bridge draws it as it did. A Bridge before it sees a Skill run as a `running` Trigger that then ends `fix_ready`, which it draws as it draws a repair's fix. Bridge's half is `packages/protocol/src/triggers.ts`, written by hand like the rest.
