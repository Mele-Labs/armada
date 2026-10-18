# Protocol 23.39: pause and resume a Job

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`park_job` and `resume_job` are Fleet's pause and resume as commands, `POST /jobs/:job_id/park_job` and
`/resume_job`, no body, each answering the Job's `JobSummary`. `../concepts/job.md`, *Pausing a Job*.

**The names are `park_job` and `resume_job`**, because `pause_job` is a retired row for a different act
(a hold on a healthy Drone) that `xtask`'s unserved-operation allowance keeps on purpose.

**Additive only.** A new field, a new queued reason, two commands, two events and six refusal codes; a
Bridge before 23.39 reads past the field and the event, and draws a paused Job by its status.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `paused` | `JobSummary` | `{ "by", "at", "resuming" }`: `person` or `fleet`, when, and whether a resume waits for a slot | A Job that is not paused, and every row before 23.39 |
| `paused` | `queued_reason` | A running Job that was paused reads `queued` with this reason | Every other queued Job |
| `park_job`, `resume_job` | commands | The Job's row, `paused` set or lifted | |
| `job.paused`, `job.resumed` | events | `{ "job", "actor", "at" }`, the row whole; a resume that waits carries `paused.resuming` | |
| `fleet.not_pausable` | 409 | A status that cannot hold a pause, or no slot to park | |
| `fleet.already_paused`, `fleet.not_paused` | 409 | A pause on a paused Job, a resume on one that is not | |
| `fleet.checks_running` | 409 | Checks are running again on the worktree; wait | |
| `fleet.pause_refused` | 409 | The pool or git would not park the work, or take it back, and why | |
| `fleet.paused` | 409 | A person's act on a paused Job: resume it first | |

**The marker rides beside the status.** A paused Job at a review gate reads `awaiting_review` with
`paused` set, so `queued_reason` is no place to look for it; it is only the reading of a running Job
that went back to `queued`. `job.state_changed` is silent for a gate Job, which is why a pause has its
own event.

`fleet.checks_running` was `fleet.not_resumable` for a re-run of the Checks and a merge gating a moved
base before this, and is now its own code for every act that meets it. No Bridge code read the old one.
