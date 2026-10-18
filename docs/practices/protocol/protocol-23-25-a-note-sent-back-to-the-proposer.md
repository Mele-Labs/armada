# Protocol 23.25: a note sent back to the proposer

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: from the approval gate a person can send the proposal back with a note, and
the proposer rewrites it.

**A route, a body and the one new edge, additive.** `POST /jobs/:job_id/to_proposer`, `to_proposer`,
takes `ToProposer { note, tuning?, landing? }` and answers a `ProposedPlan`. The transition registry
gains `awaiting_approval -> proposing`, a person's act, crossed by `Job::sent_back_to_the_proposer`
alone; `proposing -> awaiting_approval` is still `Job::answered`'s. The first `job.state_changed`
carries the move out, and the answer's the move back, so a Bridge that reads the stream sees the Job
leave its gate and return. A 23.24 Bridge has no such route, and a 23.25 Bridge behind it is
refused, which is the skew rule's own direction.

| What | How |
|---|---|
| The proposer is asked | The Job's request, the proposal's title, and the note |
| What the answer replaces | Title, workflow, steps, criteria, urgency and the rest `Job::answered` freezes, whole |
| The person's tuning | Carried to every step whose id the new workflow has; the rest is dropped and logged |
| The person's landing | Carried as it stands; one the repository will not take is dropped and logged |
| A split | **The whole split goes back together**, from any one of its Jobs. The proposer answers once and its answer replaces the group (below); each Job comes to the gate on its own with the matching tuning |
| A call that fails | The proposal is put back as it was, and the refusal is the proposer's |

**A split goes back whole, so no sibling is stranded.** The group is the head and every extra
naming it, by number. The answer is laid over it in order: the head and the old extras are kept as
the same Jobs and rewritten whole, an answer with more Jobs makes the rest as new extras of the
head, and an answer with fewer ends the old extras it has no place for, killed with a line in their
logs saying the revised proposal replaced them. Every rewrite is read before any is written. A call
that cannot be read puts every Job of the group back as it was.

**Refused, moving nothing:** 409 `fleet.proposal_frozen` off the gate; 422
`fleet.unacceptable_proposal` on a blank note, on a Job of the group that is not at its gate (the
message names it and its status, and nothing goes back), on a Job dispatched by an Epic's plan step,
which the Epic's plan owns, and on a tuning or landing the proposal in front of the person cannot
take. The tuning and landing in the body are laid on every Job the answer makes. **Store**: no migration. `record_answered` clears the step and write-target rows a revised
answer replaces, and the rebuild replays `awaiting_approval -> proposing` and the answer back
through the machine, from the columns the last answer wrote.
