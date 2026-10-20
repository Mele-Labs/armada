# Protocol 23.86: a Job's standing, and the replied and done items

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`.
**Additive only**: one optional field, one enum, two `WaitingSource` values, one new `SessionFact` and one optional field on another.
No new operation.
**Nothing the owner sees may depend on an agent making a tool call**: Fleet reads what a Job is doing from the turn, the thread and `done_since`, and the agent's own `waiting_for` and `all_done` only add detail.

| Where | Carries | Notes |
| --- | --- | --- |
| `SessionRecord.standing?` | `SessionStanding`: `working`, `stuck`, `replied`, `done`, `idle` | The one answer the Jobs list, the Cockpit's tiles, lanes and calls all draw. Absent from a Fleet that does not say, and Bridge then reads the turn alone |
| `SessionFact::TurnStarted` (`report_session`) | nothing | The terminal mod tells a prompt went in. Fleet has the Job working and forgets the reply the turn before left. Never sent for a slash command. A Fleet before 23.86 refuses the fact, which the IDs make moot |
| `SessionFact::TurnCompleted.said?` | The close of what the turn ended on: the last 1,500 characters, from a line start (a mod before 0.3.39 sent the first line, at most 200) | Left out when the turn said nothing, never sent as `null`. Fleet keeps it with the turn's end in `session_turn_end` (store migration `20261014T1000Z`) and reads it to tell a reply from a turn that said nothing, and a reply that asks from one that only tells |
| `WaitingItem.source` | `replied`, `done` | `replied` is derived on every read, never kept: id `replied:<turn ended at>`, text the sentence of the reply that asks, no act. It is raised only for a reply whose close asks the person something and that the agent did not ask through a tool; a reply that only reports is the `replied` standing and no item. It is dismissable, dismissing sends the agent nothing, and a dismissed id never returns. `done` is derived on every read, never kept: id `done:<done_since>`, `since` that instant, text `Done`; Bridge offers End job (`close_session`) and Keep going (`dismiss_waiting`). It is dismissable, and a dismissed id never returns |
| `DismissWaiting.reason?` | Why the person removed the item | Left out where he gave none, never sent as `null`. An item the agent stated (`waiting_for`) tells it so: the Job is sent `Owner removed "<item text>"`, with `: <reason>` after it where the reason is not blank. A derived item is only dropped. |

**Skew.** A Fleet before 23.86 serves neither, and Bridge reads a Job as before.
The IDs differ, so the two are refused regardless, and the lifeboat serves.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.
**Served.** Fleet derives `standing` and the `done` item from observed facts on every read (`session_host/standing.rs`), for hosted and terminal Jobs, and none of it needs the agent to call a tool.
A running turn nothing is heard from for ten minutes reads `stuck`, which no event says, so the notice loop's tick republishes the Job when that moves: a `session.changed` per move, in both directions, and no other new traffic.
`Timeline::session_turn` no longer marks its entry `live`, since a finished turn is not moving.
A Helm Job has no `done` item, never replies and is never stuck.
A terminal turn's running state is kept in memory, so a Fleet restart reads it as idle until the next `turn_started`; a mod older than `0.3.20` reports no `turn_started` or `said`, so its Job is never read as working or stuck and has no reply, and its `done` still stands.
