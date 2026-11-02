# Autopilot's answers: `get_autopilot_decisions`, `review_autopilot_decision`, `override_autopilot_decision`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/autopilot.md`, and [Autopilot's boxes](autopilot-boxes.md) for what a person checks.
While an Autopilot runs, Fleet answers a decision whose kind was checked, leaves the rest, and keeps every answer for the person to read.
**Additive only**: three operations, one event, and a table with a migration that has a name and not a number.
The protocol ID moves with `crates/ipc/src/autopilot_decisions.rs`, the operation files and `packages/protocol/src/autopilot.ts`, and there is no number to take.
Bridge's half is `packages/protocol/src/autopilot.ts`, written by hand.

| Operation | Side | Carries |
| --- | --- | --- |
| `get_autopilot_decisions` | Bridge asks, Fleet answers | `AutopilotDecisions`: `to_review`, every repository's answers not yet reviewed, in the order they were given |
| `review_autopilot_decision` | Bridge sends `{id}` | The person kept the answer. It leaves `to_review`, and the list comes back. An id nothing holds is `fleet.autopilot_no_decision`, an illegal move |
| `override_autopilot_decision` | Bridge sends `{id, text}` | The correction goes to the Job as `Re: <asked>` and then the words, and the row is kept as corrected and reviewed. Blank words are `fleet.autopilot_decision_empty`, unacceptable; an id with no Job to tell is `fleet.autopilot_no_decision` |
| `autopilot.decisions_changed` | Fleet publishes | The whole `AutopilotDecisions`, each time an answer is added, kept or corrected |

**One row is `AutopilotDecided`.** `id` is `<session>:<item>` for a Job's item, so an item is answered once.
Other rows are named by what they did: `<job>:land:<number>:<head>` for a pull request landed, `pick:<id>` for an issue started that the picker was unsure of, `red:<manifest>:<commit>` for a red main taken.
`kind` is the `AutopilotDecision` the answer was given under, `asked` is the item's words, `chose` the way taken in the item's own words, `because` the helper's reason, and `session_id` the Job to correct, absent where there is none.

**There is no wire value for a destructive decision.** Fleet reads an item's words and options for it before any call and the chosen answer after, and an item that reads as destructive waits.
Nothing is recorded for what waits.

**A correction is read the next time.** The helper is shown the repository's last twenty answers with each correction beside the answer it replaces, so a person's "no, the other one" shapes the next call for that repository.
`prompts.autopilotDecider` is the setting that holds the question.

Migration `20261102T1200Z-autopilot.decisions` adds `autopilot_decisions`, one row per id, written once.

**Skew.** The IDs differ, so a Bridge from before is refused regardless.
**The event stream** gains one event, published whole on a change.
The list holds what is not yet reviewed and shrinks as the person reviews it, which bounds it; the unbounded-sink risk is otherwise unchanged.
The lifeboat is untouched.
