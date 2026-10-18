# Protocol 16.4: a Job dispatched from a Studio says so, and reaches it

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1362`. Two `origin` values, `studio_dispatched` and `studio_helm_drafted`,
replacing the `manual` and `helm_drafted` 14.13 gave a Studio dispatch; and
`JobDetail.from_studio`, additive — `{ studio_id, name?, node_id }`, the Studio
that produced the Job and the node to select on landing.

**Minor, on `queued_reason`'s precedent.** Bridge types `origin` as `string`
and reads it through the generated vocabulary rather than matching on it, and
that map already answers `undefined` for a key it does not hold — so an older
Bridge meeting either value draws no provenance rather than the wrong one. The
values are Fleet's own writing: Bridge proposes `manual` and cannot send these,
so the strict direction is never exercised. **The condition is the same one**:
the moment either side branches on this value rather than rendering it,
widening the set is a major bump.

**Two values and not one, and that is the whole shape of the change.** One
column answers one question, and the row has to keep answering two — where the
Job came from, and who pressed. A single value meaning *a Studio* would have
put *dispatched by you* in a second place nothing checks, which is the argument
`sub_dispatched`'s own registry row already makes.

**`from_studio` is derived and no column is added.** The `produced` edge from
the Issue draft to the Job node is the record; `store::studio::tracing` reads it
backwards, the way `store::lineage` reads `redispatched_from`. So the field is
absent on a Job whose Studio was deleted — the nodes cascade with it — and
`origin` is what still says the Job came off one. Job detail draws that pair as
*that Studio is no longer there*, with no control, which is `job-board.md`'s *A
Board outlives its Workspace* one scope smaller.
