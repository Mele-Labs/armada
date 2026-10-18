# Protocol 23.68: steps added to one Job, and the repository's Draft default

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/trigger.md`, *Steps added to one Job*.
**Additive only**: two operations, one event, the DTOs of `ipc::added_steps`, one optional field each on `ApproveDispatch`, `JobDetail`, `SaveTrigger` and `ManifestDeclared`, and one edit.
The repair, the hold and the Draft default below are the same minor, landed together; `armada need` gave the number after the branches ahead of it.

| Where | Carries | Notes |
| --- | --- | --- |
| `add_job_step` (`POST /jobs/:job_id/add_job_step`) | `AddStep`: `runs` (`script` and `command`, `skill` and `skill`, or `drone` and `brief`), `when`, `step`, `block?`, `repair?` | `AddedStep`. 409 `fleet.added_step_behind` (`reason`: `started`, `passed` or `ended`), 409 `fleet.added_step_before_approval`, 422 `fleet.unacceptable_addition`. `Bridge only` |
| `remove_job_step` (`POST /jobs/:job_id/remove_job_step`) | `RemoveAddedStep`: `id` | `AddedStepRemoved`. 409 `fleet.added_step_fired`, 422 `fleet.no_such_addition`. `Bridge only` |
| `approve_dispatch` | `additions?`: a list of `AddStep` | Placed at the press. Left out keeps what was placed, `[]` clears it. 422 `fleet.unacceptable_proposal` |
| `save_trigger` | `kept_from?`: `{ job_id, addition_id }` | **Keeps an addition for every Job** with no new operation: the definition is the Trigger the editor drew, and Fleet records where it went. A Script or a Skill only |
| `job.addition_changed` (event) | `JobAdditionChanged`: `job_id`, `addition`, `removed?`, `at` | The row whole, on each add, removal, keep and state a firing reaches |
| `JobDetail.additions` | `AddedStep`: `id`, `runs`, `when`, `step`, `block`, `repair`, `placed`, `added_at`, `state`, `skipped?`, `exit_code?`, `started_at?`, `ended_at?`, `log_at?`, `kept?` | `state` reuses `TriggerFiringState`: `pending` until the moment has come. Absent from a Fleet before 23.68 and where empty |
| `edit_manifest` | the edit `set_pr_mode`: `{ "edit": "set_pr_mode", "pr_mode": "draft" \| "ready" \| null }` | `null` removes the key |
| `ManifestDeclared` | `pr_mode?` | Absent where the file defers to this machine's default |

**A place is a moment and a step**, a Trigger's.
Before a step is its `step_starts`, after it is its `step_passes`, the gap before the pull request opens is the delivering step's `step_starts` and the gap after is `pr_opened`.
A gap behind the Job's current step is refused, which is what stops a step landing where it can never fire.

**A Drone step is recorded `skipped`, and says so.** A step a Drone works needs a gate and Fleet has the frozen workflow's step rows only.
A Skill is skipped as a skill Trigger is.
Both run on a side Drone from 23.73, below.

**The event stream: slightly worse, and bounded.** At most two messages a firing and one per act a person makes, on the one drop-oldest channel, with nothing a Drone produces on it.
A Bridge that missed some re-reads `get_job`.
The rate is `[broadcast-capacity]`'s to measure.

**Skew.** A Fleet before 23.68 sends no `additions` and no `pr_mode`, which Bridge reads as none.
A Bridge before it sends no `additions` and no `kept_from`, which Fleet reads as every call before.
Nothing a 23.58 peer reads changes.
Bridge's half is `packages/protocol/src/added-steps.ts (deleted)`, written by hand like the rest.
