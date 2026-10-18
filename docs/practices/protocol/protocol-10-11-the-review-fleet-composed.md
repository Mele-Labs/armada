# Protocol 10.11: the review Fleet composed

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Job detail's review area and the pull request's description said nearly the
same thing from two independent builders — `#665`. `ipc::JobReview`, additive
on `get_job`'s `JobDetail`, is the fix: Fleet composes one review from the one
reading of the record, in four parts — `why` (the brief, in the requester's
own words), `outcome` (what the worktree changed, as far as a diff can say
it), `risks` (what nothing checked, and what the base carries that this Job
did not write), `evidence` (every step and every Check that ran, with its
outcome) — and renders it two ways: the pull request's Markdown body,
unchanged in content, and this DTO. `crates/fleet/src/review.rs` is the one
builder both readings come from.

Composed at the delivering step's entry, as before, and now also at any other
`human_always` gate, so a workflow that never opens a pull request still
reaches a person with the same four sections. **Absent is a Job that has not
reached a gate yet**, not an empty review — a Job still running, or one that
finished with no `human_always` step at all, carries nothing here, and so does
every Job read from a Fleet older than 10.11.

`VerdictSheet` draws `risks` as its own card, "What was not checked", less the
paragraph Fleet opens it with. `why` it no longer draws: Overview's Brief card
already carries the request, and the record repeated it whole (#1680). The
Drone's own claims stay their own cards, "What was done" and "What was skipped"
— the owner's names of 2 Oct 2026, because several Drones now share one Job.
The pull request leaves them out on purpose; the old labels, "What the Drone
says it did", were what marked them as a self-report, and the new ones no
longer say whose account it is.
