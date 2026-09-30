# Where does a person answer the thing the lead is about?

**Decided 2026-09-30.** The next turn of
`2026-09-29-the-review-gate-sits-under-the-lead.md`, which put the controls
directly under the lead rather than at a destination.

Under it was still two panels. On `job/runningWaitingOnACommand` the lead read
*A Drone wants to run a command it was not given / pnpm add -D reselect@5.1.1*
and the box immediately below it read *The drone is waiting on you / The drone
wants to run pnpm add -D reselect@5.1.1* before its three radios. The owner, on
the lead:

> "This panel duplicates what is shown in the panel below. Why can't we just
> have one panel?"

Offered dropping the duplicated lines, or moving the controls into the lead.

## The controls move into the lead

One bordered panel. The waiting slot renders inside the lead's own region
rather than as its sibling; the boxes lose their own heading and their
restatement of what the lead just said, and the elapsed moves to the lead's top
right — where it is the panel's, not a second box's.

**Cost he took:** the panel gets tall where the controls are big, and the
review gate is the worst case — the lead, the folded record, the decision well
and the pull request's comments are all one panel now.

**The record stays folded.** Unfolding it to make the merge look tidier is
reversing yesterday's decision, which the other record already argues.

**Where it landed:** `packages/screens/src/JobLead.tsx` draws the slot,
`OverviewBoard` hands it over, and `step.tsx` is where the two boxes lost their
head. `apps/desktop/src/renderer/src/mock/overview-boards.test.tsx` measures
containment off the DOM.
