# The title bar's views, read by Fleet

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 9 Nov 2026. Fleet works out what the Electron Bridge derived in TypeScript for the title bar and the write-clash marks, and the native Bridge and Pocket read it instead.
**Additive only**: every change is a new field, one new operation and one new event, and nothing Electron reads changes shape.
Electron keeps its own TypeScript for now, so a field here has no reader in it yet.
**States, not words**, as [the merge line](the-merge-line-fleet-reads.md) is: no field is a sentence.
The Rust halves are `crates/ipc/src/ring_band.rs`, `job_compact.rs`, `building.rs` and `clashes.rs`; Bridge's halves are `ring-band.ts`, `job-compact.ts`, `building.ts` and `clashes.ts` in `packages/protocol/src`, written by hand, field for field.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.

| Where | Side | Carries |
| --- | --- | --- |
| `PlanWindow.band` | Fleet writes, Bridge reads | `RingBand`: `normal` below 80, `warning` from 80, `danger` from 95. Fleet's reading of `percent_used`; a session's report is overwritten, and a reading kept before the field is read again |
| `SessionUsage.context_percent`, `context_band` | Fleet writes, Bridge reads | The context's whole percent, rounded half up and clamped 0 to 100, and its band. Absent with either figure missing or a window of nothing |
| `SessionRecord.compact` | Fleet writes, Bridge reads | `CompactOffer`: `{ state: "offered", how: "terminal" \| "message" }` or `{ state: "off", why }`, with `why` one of `ended`, `not_listening`, `terminal_not_listed`, `agent_not_listed` |
| `FleetBuildReport.switcher` | Fleet writes, Bridge reads | `BuildSwitcher`: `aligned`, `idle_on_main` and `restart`, either `refresh_preview` or `update_to_main` |
| `list_all_write_clashes` (`GET /clashes/all`) | Bridge reads | `AllWriteClashes`: a `RepositoryClashes` `{ manifest_id, clashes }` for each served repository that has a clash |
| `write_clashes.changed` | Fleet writes, Bridge reads | `AllWriteClashes`, whole |

## What Fleet reads

**The ring band** is `usageLevel` in `packages/components/src/usage.ts`, the two lines held once in Rust.
A plan window is banded where Fleet takes the reading, before it is compared with the one held, so a repeat of a banded reading still says nothing.
A Job's context is banded off its two figures when the record is built, so every `session.changed` carries it.

**Compact** is `offeredOf` in `cockpit/deck/session-tiles.ts`, check for check: an ended Job is off first; a hosted Job is offered as a `message` when its agent listed `compact` and is `agent_not_listed` otherwise; a terminal Job is `not_listening` unless its mod is, then offered as a `terminal` command when the mod listed `compact` and `terminal_not_listed` otherwise.
Two departures.
Electron also asks whether the window can send a command at all, which only a client knows.
A hosted Job reads its own list of commands, where Electron reads the first list any Job has; Fleet gives a Job whose process has not started the last list any Job read, so the two agree wherever a list exists.

**The build switcher** is `FleetBuildSection` in `FleetBuild.tsx`.
`aligned` is a position that is known with none ahead and none behind, so a position that cannot be counted is not aligned.
`idle_on_main` is nothing restarting, `on` being `main` and `aligned`, which disables the restart button.
`restart` follows the build being restarted onto, `restarting` else `on`: the preview is `refresh_preview` and main is `update_to_main`.
The present-tense words, Refreshing and Updating, follow `restarting`, which the report already carries.

## The Jobs a restart adopts: already on the wire

`FleetBuildReport.carried` is the list, and no field is added.
It is the titles of the hosted Jobs with a turn under way, sorted, which `scripts/restart-build --adopt` carries across, and Fleet fills it on every `get_fleet_build`.
Electron draws it only in the Revert dialog and sets the restart's list to nothing in `fleet-build.ts`, so its confirm never opens.
That is Electron's reading, and the native Bridge reads `carried` for both confirms.

## Write clashes across every repository

`GET /clashes?manifest_id=` is as it was.
`GET /clashes/all` reads the same table for every served repository, in the order Fleet serves them, and leaves out one with no clash.
Fleet publishes `write_clashes.changed` on the notice loop's tick that checks the clashes, **only when the set differs from the last one sent**, and the first read sets the baseline.
A clash found, one cleared, a clash's files changing, a holder ending and a Job's title changing are each a change.
Fleet finds a clash about 30 seconds after the writes that made it, so the push is that late and no later; Electron's poll added up to 35 seconds to it.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
A client that missed an event reads `list_all_write_clashes` again on the resync after `Missed`; the event is whole, so a missed one costs one read and no gap.
**This makes the unbounded sink slightly worse, and bounds how much.**
`write_clashes.changed` is one new event, and it is sent only when the set moves, which is a write that makes or ends a clash and not every write.
`session.changed` and `plan_usage.changed` carry the new fields on events that were already sent, and add none.
The sink is still unbounded from the application side, and the recovery is the resync.
