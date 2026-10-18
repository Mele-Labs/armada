# Protocol 21.6: a dispatched request is a Job, and the proposal names it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`ProposalMoved.job_id` names the Job at `proposing` the call is reading for. It is absent only from
an older Fleet, which created no Job until the call answered. Two escalation reasons are new on
`Reason.named`: `no_workflow_fits` and `proposer_failed`, both on `proposing -> escalated`. All of
it is additive.

**The row exists from the press** (#1714). `propose_from_request` creates the Job at `proposing`
before the call goes out, with the request as its title, `workflow_id` empty, no steps, and
`workflow_source` blank. The route still answers the plan or the refusal code. On a refusal the
Job is now escalated rather than never created: `fleet.no_workflow_fits` takes `no_workflow_fits`.
`fleet.proposer_unreachable`, `fleet.proposer_unreadable` and `fleet.proposer_model_not_held` take
`proposer_failed`, and the Job's log says which (#1716). A stop takes the Job to `killed`.

**The head of the plan is that Job, moved to the gate**, with the proposer's title replacing the
request. Its `job.state_changed` is the move, not a `job.created`. The other Jobs of a split arrive
as `job.created` with `dispatched_by` naming the head.
