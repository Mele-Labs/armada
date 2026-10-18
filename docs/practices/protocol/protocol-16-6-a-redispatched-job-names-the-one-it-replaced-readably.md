# Protocol 16.6: a redispatched Job names the one it replaced, readably

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1474`. `JobDetail.replaces`, additive — `{ job_id, handle }`, the Job this one
replaced, beside `replaced_by` and shaped the same.

**The same column, followed the other way.** 16.2 read `redispatched_from` as a
predicate to answer *which Job replaced this one*; this follows it as a column
to answer *which Job this one replaced*. Still no second record, still nothing
written, and V82's index is the other direction's — this join is `job_id` at
both ends. `store::lineage` holds both reads, and Fleet composes both handles
with `core_model::handle_of` at the seam.

**A DTO of its own rather than `ReplacedBy` reused**, though the two fields
match today. They answer opposite questions of one record, and the direction is
the whole content of the answer.

**Why anything crossed at all.** The id already crossed as
`JobSummary.redispatched_from`, and the header drew it: a bare ULID, which is
the one fact on the screen about where the Job came from and the one fact a
person can neither read nor press. What was missing is the predecessor's number
and title, which live on a row Bridge does not hold.

**Additive, so a Fleet ahead of a Bridge sends a field it ignores** and the
header draws the ULID it already drew.
