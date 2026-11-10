# The repair under way, on a Job's card: `SessionRecord.repairing`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 10 Nov 2026. A Job's card names the repair Fleet is running on it, in any state, as one more clause on the line it already says: `Stuck · Repairing: a command hung`, `Stopped: process ended · Repairing: its process died twice`.
**Additive only**: one field on an existing type and the type it carries; no operation, route, event kind or error.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
`armada need protocol-version.toml`, which an earlier brief named as a step, is not a verb of `armada` and no such file exists: the contributor workflow in [The protocol ID](the-protocol-id.md) has no such step.
The Rust half is `ipc::SessionRepairing` in `crates/ipc/src/sessions.rs`; the reading is `Repairs::under_way`, called in `ledger_built`.
Bridge's half is `SessionRepairing` in `packages/protocol/src/sessions.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.repairing` | Fleet writes, Bridge reads | `SessionRepairing`, absent while no repair runs |
| `SessionRepairing` | Fleet writes | `{ said, since }` |

## What Fleet says

`said` is `RepairBreak::words`, Fleet's own words for what broke (`a command hung`, `checks failed on #7`), so no client spells them.
`since` is when the repair was listed as running.
The field is set from the registry of running repairs, not from the store, so it is absent from the moment a repair leaves the list.
It is set for a Job in any state, an ended Job whose pull request a repair works on included.

## What a card says

`jobStateOf` in `packages/screens/src/session-state.ts` and `said_of` in `crates/fleet/src/standing_calls.rs` add ` · Repairing: <said>` to the words of whatever state the Job is in.
On a stopped Job the clause stands in for `Repair Job running` and for the failed restart beside it.
`Repair Job failed` reads `Not repaired`, and `No repair Job: Fleet is unhealthy` reads `No repair: Fleet is unhealthy`.
The Now act in `fleetActsOf` reads `Repairing: <said>` from this field, as `thread_view/head.rs` does from Fleet's notes.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
It publishes no new event kind: `SessionChanged` carries the record, and `repair_started` and the end of a repair already publish it.
The event stream is as it was.
