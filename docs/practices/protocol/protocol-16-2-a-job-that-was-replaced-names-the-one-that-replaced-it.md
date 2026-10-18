# Protocol 16.2: a job that was replaced names the one that replaced it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1439`. `JobDetail.replaced_by`, additive: `{ job_id, handle }` for the Job a redispatch minted to
replace this one, and absent on nearly every Job. A killed and redispatched Job was a dead end —
it said killed, and nothing on any surface said the work had carried on somewhere else.

**No new record, and deliberately no second column.** `JobSummary.redispatched_from` already
crosses and is the one fact; Fleet reads it as a predicate over `jobs` rather than writing the
forward edge, so the two directions cannot disagree and forgetting a replacement takes the link
with it. `store::lineage` is the read and `V82` is its index.

**On the detail and not on the summary.** The Board draws a row per Job and already folds a
lineage into one; a field here is one indexed read on the open of a Job, where a read per row
would be a query per row on a list that redraws on every event.

**The direct successor, never the end of a chain.** A replacement that was itself redispatched
carries its own, so a chain is walked by opening Jobs rather than by anything on the wire — and
there is no walk to loop. Two Jobs naming one predecessor is legal today and answers with the
newer.

**Additive, so a Fleet ahead of a Bridge sends a field it ignores** and the callout simply does
not draw, which is what an older Bridge already does with every Job.

**This was written as 16.1 and is 16.2**, because 16.1 landed underneath it while the branch was
open — the same collision 16.0's own note records, caught at the merge this time.
