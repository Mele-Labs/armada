# Two Jobs settle a clash: `settle_clash`, `SessionRecord.clashes` and `WaitingSource::ClashOrder`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 27 Nov 2026. `docs/concepts/fleet.md`, *Claims a Job holds*.
**Additive only**: one operation and its route, the types `SettleClash`, `ClashFirst`, `JobClash` and `ClashStanding` in `crates/ipc/src/job_asks.rs`, one optional field on `SessionRecord`, one `WaitingSource` value and the `job_clashes` table; no event kind.
There is no number to bump: the protocol ID moves with `crates/ipc/src`, the operation file and the route table.

| Where | Side | Carries |
| --- | --- | --- |
| `settle_clash` (`POST /sessions/settle_clash`, `agent_access = "Yes"`) | An agent sends, Fleet answers 200 | `SettleClash` `{ with, files, first, why, session_id? }`, `first` being `me`, `them` or `both_go_on`; answers the `JobClash`. Refused with `fleet.clash_refused` and a sentence naming the part |
| `JobClash` | Fleet writes, Bridge reads | `{ id, jobs, files, standing, first?, why?, at }`: `id` `clash:<a>:<b>` with the two addresses sorted, `jobs` two `OperatorAskJob`s (the Job that settled or raised it, then the other), `standing` `agreed`, `ruled` or `waiting`, `first` the session that lands first, absent where both go on or while it waits |
| `SessionRecord.clashes` (`list_sessions`, `report_session`, `session.changed`) | Fleet writes, Bridge reads | Each `JobClash` the Job is one of two in, the same on both Jobs' records. Absent when empty |
| `clash_order` (`WaitingSource`) | Bridge reads | The raising Job's Needs you item while its clash waits, raised whether the Operator is on or off. Two options, the two Jobs' titles. Answered by `answer_waiting` with a `choice`; never dismissed |

**What the claims notice says changes, not the wire**: it names the other Job's address and asks the two to agree, record it with `settle_clash`, or call `raise_conflict`.
A `raise_conflict` with `files` keeps the pair's clash as `waiting` beside the ask it already kept; its answer and refusals are unchanged.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
An older Fleet sends no `clashes` and no `clash_order` item, which a newer Bridge reads as none.
**The event stream.** The record is carried whole, so it carries `clashes`; no new event. A settling, a raise and his choice publish `session.changed` for both Jobs.
