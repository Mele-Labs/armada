# Sleep leaves the wire

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Autopilot is the one overnight control, and it answers and keeps what it answered ([Autopilot's answers](autopilot-decisions.md)).
**Removal only**: no operation, event kind, route or field is added.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and a pair that disagrees is refused.
`crates/ipc` and `packages/protocol` are edited together; Bridge's half comes out in its own change, so its type-check is red on these names until then.

Fleet no longer runs Sleep's pass, so nothing answers behind a switch that is no longer there.
`OwnerAway` keeps its wire word and now holds where the Job's repository has an Autopilot running; a paused or stopped one is not away.

**Operations removed (4) and one event.** Each loses its route:

| Operation | Route |
| --- | --- |
| `get_sleep` | `GET /sleep` |
| `set_sleep` | `POST /sleep` |
| `override_sleep` | `POST /sleep/override` |
| `review_sleep` | `POST /sleep/review` |
| `sleep.changed` | the event stream |

**Wire types removed.** `ipc::sleep` and its mirror (`packages/protocol/src/sleep.ts (deleted)`): `SleepState`, `SleepDecided`, `SleepBlocked`, `SleepLanded`, `SleepWalk`, `SetSleep`, `OverrideSleep` and `ReviewSleep`.
**Errors removed.** `fleet.sleep_no_decision` and `fleet.sleep_empty`.

**What the night held, landed and showed is not carried over.** Autopilot keeps what it answered and nothing for what waited, what merged or what was shown.

Migration `20261102T1500Z-sleep.folded` is **breaking**: it drops `sleep_state` and `sleep_rows`, so a build from before cannot open the file.
Before dropping, it moves each `decided` row not yet reviewed into `autopilot_decisions` as a card to review.
The repository is the answering Job's own, the kind is `recommended`, the reason is empty and the time is when the night began.
A row whose Job is gone has no repository to name, and the column cannot be empty, so it is dropped.
