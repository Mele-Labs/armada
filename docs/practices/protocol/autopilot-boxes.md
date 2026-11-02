# Autopilot's boxes: `on_autopilot` and `decisions`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/autopilot.md`.
The person checks what goes on autopilot and which kinds of decision a helper may answer, and Fleet keeps what was checked.
**Additive only**: two fields on two types, and a migration with a name and not a number.
The protocol ID moves with `crates/ipc/src/autopilot.rs`, the operation files and `packages/protocol/src/autopilot.ts`, and there is no number to take.
Bridge's half is `packages/protocol/src/autopilot.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `AutopilotSettings.on_autopilot` (`start_autopilot`'s body) | Bridge sends, Fleet keeps | `AutopilotWork`: `backlog` and `running`, booleans. Absent reads as `backlog` true, `running` false |
| `AutopilotSettings.decisions` | Bridge sends, Fleet keeps | A set of `AutopilotDecision`: `recommended`, `open`, `plan`, `draft`, `walk`, `land`, `unsure`, `red`, `reads`, `writes`. Absent reads as none |
| `Autopilot.on_autopilot`, `Autopilot.decisions` | Fleet writes, Bridge reads | What was checked at start, always present. A record kept before them reads as the defaults above |
| `update_autopilot` | unchanged route | Ignores both fields and keeps what the run holds, since the Cockpit's Edit caps sends none |

**There is no `destructive` kind**: it always waits for a person, so no word names it, and a body naming it or any other word is refused when it is read.
`propose_autopilot` answers the defaults, since a sentence says nothing about them.
Migration `20261102T1000Z-autopilot.boxes` adds `on_backlog`, `on_running` and `decisions` to `autopilots`, each defaulted so an old row reads as before.

**Skew.** The IDs differ, so a Bridge from before is refused regardless.
**The event stream is untouched** beyond the `autopilot.changed` each Autopilot already publishes, which now carries the two fields; this makes the unbounded-sink risk neither better nor worse.
The lifeboat is untouched.
