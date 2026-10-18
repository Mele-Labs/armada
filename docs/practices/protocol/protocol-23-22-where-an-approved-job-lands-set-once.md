# Protocol 23.22: where an approved Job lands, set once

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: a Job approved landing in the base may be aimed at another branch after
the press.

**A route and a body, additive.** `POST /jobs/:job_id/set_landing_target`, `set_landing_target`,
takes `SetLandingTarget { target }` and answers the Job's `JobSummary`. `JobDetail.landing.target`
reads it back, and the delivering step opens its pull request against it, as against a target the
approval set. A 23.21 Fleet has no such route, so a 23.22 Bridge behind it is refused, which is the
skew rule's own direction.

| Refused | Code |
|---|---|
| At `awaiting_approval`, where the approval sets it; ended; already landing where a person chose; its work committed or opened for review | 409 `fleet.landing_target_settled` |
| A blank branch | 422 `fleet.landing_target_blank` |
| A branch the repository does not hold | 422 `fleet.no_such_branch` |

**Once, and before the work goes out**, because the pull request opens against it: a target moved
after that is a record the forge disagrees with. **The worktree keeps what it was cut from.** No
store migration: the landing row takes the target. The event stream is untouched.
