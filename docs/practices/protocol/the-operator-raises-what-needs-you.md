# The Operator raises what needs you, and briefs you once a day

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 10 Nov 2026. Fleet reviews each Needs you item and raises it to the person as a line of the Operator's conversation, with its read and a pick, and briefs him the first time each local day that Bridge's window comes to the front.
**Additive only**: two fields on a line, one new operation and one new event, and nothing Electron reads changes shape.
Electron shows a raise and a briefing as ordinary Operator lines, from `text`; a native Bridge draws a card per kind from `raise`, and the briefing from `briefing`.
The Rust half is `crates/ipc/src/operator_talk.rs`; Bridge's half is `packages/protocol/src/operator-talk.ts`, written by hand, field for field, which now holds `OperatorTalk` too.
The protocol ID moves with both, and there is no number to take.

| Where | Side | Carries |
| --- | --- | --- |
| `OperatorTalkLine.raise` | Fleet writes, Bridge reads | `OperatorRaise`: `session_id` and `job`, the Job's id and title, both absent on a suggestion no Job holds; `item`, the `WaitingItem` as it was raised; `read`, the `OperatorRead` `{ read, pick }` where a call read it, with an empty `pick` where it named an option not offered; `settled_at` once the item left the list |
| `OperatorTalkLine.briefing` | Fleet writes, Bridge reads | `OperatorBriefing` on the day's briefing, absent otherwise: `since`, each `BriefedChange` `{ kind: merged \| ended, session_id, job, pull?: { number, title }, said? }`; `waiting`, each `{ session_id?, item_id }` naming an open raise line; `on`, a line per thing the Operator has in hand; `top`, the call's one line |
| `operator_in_front` (`POST /operator/front`) | Bridge sends | No body. Answered at once with `OperatorInFront` `{ briefing }`, true where a briefing is on its way |
| `operator.talk_changed` | Fleet writes, Bridge reads | Nothing. A line was kept, or a raise settled: Bridge reads `operator_talk` again |

## What a raise carries

A raise is what a card needs, with no new act vocabulary.
Its kind is `item.source`, the thing is `item.text`, the acts are `item.act` and `item.options`, and the Operator's read and pick are `read`.
`OperatorRead` is the shape a question's `operator_read` already has, reused rather than a second one.
`text` is markdown that reads alone: `**<Job>** needs you: <item>.`, then the read on a line of its own and `Pick: **<pick>**`, or with no read only the first line.
A settled raise is served with a last line, `Settled at 08:05.`, in local time.
Its words are made in one function, `raise_text` in `crates/fleet/src/operator/raising.rs`.

## When Fleet raises and settles

Fleet taps `session.changed` for each hosted Job and diffs its list against the raises still open, keyed by the Job and the item's id.
An item not raised is reviewed and raised once; a raise whose item left the list gets `settled_at`.
A paused Job's list reads empty while it rests, so a paused Job is left alone, and an ended Job's raises all settle.
The Operator's own suggestions no Job holds are read again on each `operator.board_changed`.
At start, and when the switch comes on, every item waiting is raised, oldest `since` first.

**Fail open.** With the Operator off nothing is raised or queued, and what waits is raised when it comes on.
While it is paused for the week's plan, or where the review call fails or cannot be read, the item is still raised, with no `read`.

## What `operator_talk` serves, and what a message is handed

`operator_talk` serves the last hundred lines, raises and briefings among them.
A message to the Operator is still handed only the last twenty lines that are not raises, and the raises still open beside them, so the raises do not crowd out his conversation.

## The briefing

Fleet decides whether a time in front is the day's first, by the local day of the newest briefing kept, so a restart does not brief twice.
It briefs only while the Operator is on, one briefing out at a time.
**Fleet assembles it from what it holds**: the Jobs that ended and the pull requests that merged since the last briefing, or since the local day before began, the raises still open with their picks, and the Operator's queue.
A reading call is asked only for `top`, one line on what matters most today; paused for the week's plan, or where that call fails, the briefing goes out without it.
**The briefing is its data.** Where it is drawn is being decided with the owner, so its `text` only says it is ready, `Your briefing for Tuesday 10 November is ready.`
`OperatorBriefing` is a struct of defaulted fields so it can grow, with running Jobs, stopped Jobs, a red main or the week's plan, without moving what is there.
Electron sends it when its window gains focus or is shown, and once Fleet connects where a time in front came while it was not.

## The journal of the week

Not on the wire.
Fleet keeps a markdown file per ISO week under its own home, `operator/journal/<year>-W<week>.md`, and hands this week's and last week's to every reading call.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
`operator.talk_changed` is one new event, sent when a line is kept or a raise settles, which is far rarer than `session.changed`.
A client that missed one reads the talk again on reconnecting, as Electron does.
