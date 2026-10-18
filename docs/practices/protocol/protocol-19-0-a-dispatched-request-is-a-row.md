# Protocol 19.0: a dispatched request is a row

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1159`. `JobStatus` gains `proposing` — the interval while the Job proposer is
reading a request — and **widening that set is a major bump by this document's
own table**, so the major moves and the minor resets. The table's row is *add a
new enum variant the other side is expected to `match` on*, and the caveat row
above it does not reach this set: `held_by` and `queued_reason` are open because
Bridge renders them through the generated vocabulary without branching, and this
paragraph's own *Minor vs. major* section already names `JobStatus` as the
counter-example — **Bridge picks a screen from it**. A Bridge built before this
looks `proposing` up in `packages/components/src/generated/vocabulary.ts`, finds
nothing, and has no screen to draw for a row that is now on every Board; the
Rust deserializer is stricter still and refuses the spelling outright
(`crates/ipc/src/enums.rs`'s `wire_enum!`). A minor bump would have promised that
an older peer parses every message the same way, and that is the promise this
breaks.

**Nothing else on the wire changed.** No DTO gains or loses a field, no route is
added, and `dispatched_by` (14.2) is already how a Job the proposer split names
the Job that dispatched it — so the extras need no second relation. `ProposalId`,
`ProposalInFlight`, `proposal.moved` and `stop_proposal` are untouched: a client
still watches the *call* through them and now has a Job's row to come back to as
well.

**And Fleet does not write it yet.** `domain/job-statuses.toml` carries the row
at `in_code = "Not yet"`, beside `awaiting_approval`, `awaiting_repair` and
`awaiting_attestation`; `propose_from_request` still answers with the Jobs the
request became. So the refusal a major buys is paid before anything can send the
word — which is the right way round, because the alternative is a Fleet that can
send a status and a Bridge that connects and cannot draw it.

**The event stream is neither better nor worse for this.** No kind is added, no
payload grows, and nothing here assumes delivery is complete: a `proposing` row
reaches a client through the `job.created` kind and the resync that already carry
every other row, and the bounded drop-oldest broadcast in
`crates/api/src/stream.rs` is unchanged. When Fleet does create a Job at
dispatch, several requests sent at once will publish one `job.created` each,
which is the same one-message-per-row the approval gate already published.
