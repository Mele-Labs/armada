# A Job's lanes, worked out by Fleet: `SessionRecord.lanes`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 11 Nov 2026. Fleet works out the lanes a Job draws, and every client draws what it serves.
Bridge worked them out in TypeScript: `lanesOfJob` in `packages/screens/src/session-lanes.ts` read what the Job holds, and `jobLanes` in `LaneCanvas/build.ts` drew lanes and edges from it.
The native Bridge and Pocket read this instead.
**Additive only**: one field on an existing type and the types it carries; no operation, route, event kind or error.
Electron Bridge keeps its own TypeScript reading, so `get_job_lifecycles` and `job_lifecycles.changed` are as they were.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take: `armada need` has nothing to order.
The Rust half is `crates/ipc/src/job_lanes.rs`; the reading is `crates/fleet/src/job_lanes.rs` with `read.rs` and `draw.rs` under it, and `crates/fleet/src/lane_keeping.rs` hands it what the store holds.
Bridge's half is `packages/protocol/src/job-lanes.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.lanes` | Fleet writes, Bridge reads | `JobLanesView`, absent for a Job that has ended and for one that holds nothing and follows no workflow |
| `JobLanesView` | Fleet writes | `{ workflow, at, working?, skipped?, job, lanes, rules }` |
| `JobLanesView.job` | Fleet writes | `JobLanesHeld`: `{ copy, instances, fired, changes, waiting? }`, what a Job holds and what its rules did, for the panels |
| `JobLanesView.lanes` | Fleet writes | `LaneView[]`: the workflow's lane first where the Job follows one, then a lane for each instance it holds, each with its `LaneNode`s |
| `JobLanesView.rules` | Fleet writes | `LaneEdge[]`: a rule that fired, or one still waiting, between two nodes |

## Why a field, and not a route with an event

The lanes stand on the Job's record: the step it is on, the steps it skipped and when each began, what its step waits on, and whether the agent is working.
The record is published whole on every change, so a field is current wherever one of those moves, and no new place has to remember to raise an event.
A route with its own event would have raised it from each of those places and from every move of what a child Job holds, and kept a memory of the last lanes sent that every other publish of the record would have made stale.

## What Fleet reads

The reading is a port, branch for branch and in the same order, and `crates/fleet/src/tests/job_lanes.rs` holds the cases of `session-lanes.test.ts` on the same inputs.
A change to one side is a change to the other until Electron reads this field too.

| Input | Read from |
| --- | --- |
| What the Job holds | `JobLifecycles`, as `get_job_lifecycles` serves it |
| The workflow, its steps and the step the Job is on | `workflow`, `workflow_copy`, `workflow_step`, `workflow_visits` and `skipped_steps` of the record; a workflow Fleet does not hold has a name and no steps |
| What the step waits on | `workflow_waiting_on` |
| The rules | the Job's copy (`JobLifecycles.rules`), else those `list_lifecycle_rules` serves for its repository |
| A ticket handed to a Job it started | that Job's `JobLifecycles`, and the issue titles the Job and the Jobs it started hold |
| Working | the hosted turn is working, and Fleet names the Job's step |

**Workflows off drops the workflow and keeps what the Job holds**, as `features.workflows` does in the TypeScript.
A Job that holds nothing and follows no workflow with a step has none, and so does a Job that ended.

**A ticket handed to a Job this one started has no lane here**: an ended ticket that a `job` instance links to is on that Job's lane, in the state the Job reads it in.
Fleet republishes the record of the Job that started a Job whenever a lifecycle of the started one moves, since the parent's lanes draw that state.

## Where it departs from the TypeScript

- **A time is an instant, never a clock.** Fleet knows no time zone. `LaneFired.at`, `LaneFiredAct.at` and `LaneEdge.fired_at` are instants, and the client words them, as the lanes' chips word them.
- **An edge says what it waits for apart from when it fired.** The TypeScript put either in one `meta`. `waits` is set while a rule has not fired, `fired_at` once it has, and never both.
- **What no real Fleet sets is not on the wire.** The mock's `triage`, a fired rule's `needs`, an instance's `assignee` and `says`, a step's `endsOn` (so no `wait` or `document` node kind), and a rule's `ifs`, `does` and `scope`.
- **`inFlight` is not here.** `SessionRecord.in_flight` is that list, and Fleet reads it from the store: [a Job's in-flight list](a-jobs-in-flight-list.md).

## The event

`SessionChanged` carries the record, so it carries the lanes; no new event.
The step, the turn and what the step waits on move on paths that already publish the record, and what a Job holds, its rules' acts and the changes to its copy publish it from `job_lifecycles_published` for a Job that holds an instance.
A Job that holds none and changes its copy has its record published by the path that changed the copy, and not by this one.
**Fleet publishes the record again when what a Job it started holds moves**, from the same place that publishes `job_lifecycles.changed`.

## Cost

Every record built for a live Job that holds an instance or follows a workflow reads `JobLifecycles` for it and for each Job it started.
A Job whose copy holds no rule also reads its repository's rules from the pieces on disk, as `list_lifecycle_rules` does; nothing is cached.
**This is unmeasured.** A lane tree is a few KB for a Job with a ticket and a pull request, and it rides every `session.changed` and every Job in a list.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**This makes the unbounded sink worse, and does not bound how much.**
No event is added, but each `session.changed` of a Job with lanes is larger, and a started Job's move now publishes its parent's record as well.
The sink is still unbounded from the application side, and a client that missed events reads the Job's record again on the resync after `Missed`.
