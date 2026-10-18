# A Job's Now: what runs, what is wrong, what it waits on

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

the retired Job concept, *A Job's Now*.
**Additive only**: two fields on `JobDetail`, one on `JobDrone`, a `mode` on three answers and three operations.
Every new field is optional, `serde(default, skip_serializing_if)`, so an older reader carries on and an older Fleet sends none.
Bridge's half is `packages/protocol/src/job-issues.ts (deleted)` and the `drones`, `detail` and `asking` files beside it, written by hand.

| Where | Carries | Notes |
| --- | --- | --- |
| `JobDrone.activity` | `DroneActivity`: `line`, `tail[]` | The latest action and the few before it, off the transcript pass that yields `turns` and `cost_micros`. Absent on a Drone that has said nothing, never an empty line |
| `JobDetail.issues` | `JobIssue[]`, tagged by `kind` | `breakage` (`check`, `test`), `write_scope_overlap` (`job_id`), `failed_verdict` (`step_id`, `attempt`, `trigger?`), `stuck_drone` (`drone_id`, `step_id`, `quiet_since`). Each is a pointer to a fact `get_job` already holds; empty is none |
| `JobDetail.waiting` | `JobWaiting[]`, tagged by `kind` | `resource` (`name`), `job` (`job_id`, `title?`), `transition` (`step_id`). Empty on a Job that works and on one that waits on a person |
| `answer_question`, `answer_command`, `answer_judge` | `mode?`: `best` or `quick` | Fleet words the instruction to the Drone. A mode alone is an answer, so `chose` is then absent; with a label the label is the answer and the mode rides with it. A stale question id is still refused |
| `retry_check` (`POST /jobs/:job_id/retry_check`) | `check` | Runs that one Check again. 409 where no gate is running Checks, or the gate has no Check of that name |
| `skip_check` (`POST /jobs/:job_id/skip_check`) | `check` | Ends one Check as `skipped`, which advances the step and is not a pass. 409 as `retry_check`, and where that Check has finished |
| `skip_checks` (`POST /jobs/:job_id/skip_checks`) | nothing | Skips every Check not yet finished. 409 where no gate is running Checks |

**A `stuck_drone` is only ever a working Drone.** One at rest or waiting on an answer is quiet by design and yields no issue.
The threshold is the one Fleet already pokes at (`quiet_after`), so a Drone is named stuck at the moment Fleet would first poke it, and not on a second clock.

**Redirecting a stuck Drone and retrying a step add no operation**: they are `redirect_one_drone` and `restart_step`, pressed from the issue row.

**Skew.** A Bridge before this draws a Job with no issues and nothing waited on, and a Drone row with no line beyond what its transcript gives.
A Fleet before it sends none of the fields and refuses the three Check routes as unknown.
