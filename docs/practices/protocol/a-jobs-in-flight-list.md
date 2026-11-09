# A Job's in-flight list: `SessionRecord.in_flight`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 9 Nov 2026. Fleet lists what End job lists, and every client draws it.
Bridge worked the list out in TypeScript from the lanes' instances (`inFlight` in `LaneCanvas/build.ts`); the native Bridge and Pocket read this instead.
**Additive only**: one field on an existing type and the type it carries; no operation, route, event kind or error.
Electron Bridge keeps its own TypeScript reading, so `get_job_lifecycles` is as it was.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `ipc::InFlight` in `crates/ipc/src/lifecycles.rs`; the reading is `Fleet::lifecycles_flying`, the one End job and the agent's refusal already ask.
Bridge's half is `InFlight` in `packages/protocol/src/lifecycles.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.in_flight` | Fleet writes, Bridge reads | `InFlight[]`, absent when empty |
| `InFlight` | Fleet writes | `{ lifecycle, key, label, state }` |

## What Fleet reads

An instance is listed unless ending or handing away the Job ended it, or it stands in an end state of its lifecycle.
**One standing in no state is listed too**, because an instance nobody can place must not be let go.
`state` is the lifecycle's label for the state it is In, the word its source used where it is Unmapped, and empty where it is Unread.
The order is the order the Job took them on.
The lifecycles are the ones the Job reads its instances against: its frozen copy, else those that apply to its repository.

**Set for a live Job only.** A Job that has ended, and one that holds nothing, have none.
It is not `holds`: an unread or unmapped instance is listed here and keeps the Job open from nothing, and a Job it started that has not begun is listed and holds nothing either.
A client that wants only the instances that refuse `all_done` reads the lanes.

## The event

`SessionChanged` carries the record, so it carries the list; no new event.
**Fleet publishes the record again whenever what a live Job holds moves**, from the same place that publishes `job_lifecycles.changed`, and it does not ask whether the list differs.
Every other publish of the record carries the list as it stands, so a memory of the last list sent would go stale the first time one of them did, and a client would then miss the move back.
The cost is a record built for each move, take-on or end of a Job that holds instances, and for a rule's act or a change of its copy, which move nothing in the list.
Each reader of `SessionChanged` keeps its own look at what it cares about (the attachments, the step, the pull request, whether it ended) and skips a record where that is the same.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**This makes the unbounded sink slightly worse, and bounds how much.**
It adds one `SessionChanged` beside each `job_lifecycles.changed`, about as often as an instance moves, and never more than one per call.
The sink is still unbounded from the application side, and a client that missed events reads the Job's record again on the resync after `Missed`.
