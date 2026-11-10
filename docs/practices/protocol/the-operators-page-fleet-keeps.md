# The Operator's page, kept by Fleet: `operator_page`, `operator_answer` and `operator.page_changed`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 10 Nov 2026. Fleet reviews each Needs you item and queues it in Your turn with the Operator's read and pick, keeps what Since you last looked counts, and serves both as one page.
Bridge draws it.
**Additive only**: five operations, one event and the types they carry; no field on an existing type.
Electron has shown no Operator since 10 Nov and sends none of this.
The native Bridge sends `operator_in_front`.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/operator_page.rs`; Fleet's is `crates/fleet/src/operator/` (`raising.rs` queues, `paging.rs` serves, `turning.rs` acts, `thening.rs` offers the Thens, `briefing.rs` keeps when he looked).
Bridge's half is `packages/protocol/src/operator-page.ts`, written by hand, field for field.

| Where | Side | Carries |
| --- | --- | --- |
| `operator_page` (`GET /operator/page`) | Bridge reads | `OperatorPage`: `on`, `top`, `looked_at`, `on_it`, `turn`, `digest`, `mains`, `slots` |
| `operator_answer` (`POST /operator/page/answer`) | Bridge writes | `OperatorAnswerPress` `{ id, answer, then? }`, answered with `OperatorAnswered` `{ open?, started?, filed?, handed_to?, page }` |
| `operator_later` (`POST /operator/page/later`) | Bridge writes | `OperatorLater` `{ id }`, answered with `OperatorPage` |
| `operator_turn_order` (`POST /operator/page/order`) | Bridge writes | `OperatorTurnOrder` `{ ids }`, answered with `OperatorPage` |
| `operator_in_front` (`POST /operator/front`) | Bridge writes | No body. Answered at once with `OperatorInFront` `{ rewriting }` |
| `operator.page_changed` | Fleet writes, Bridge reads | Nothing. Bridge reads `operator_page` again |
| `talk_to_operator` `about` | Bridge writes | A value, not a field: `turn:<n>` names an item of Your turn |

## Your turn

An `OperatorTurnItem` is what a card needs, with no second vocabulary beside the item's own.
`id` is `turn:<n>`, which every act names.
`kind` is `decision`, his to decide, or `go_ahead`, what the Operator would do itself.
`item` is the `WaitingItem` as it was raised: its `source` is its kind, `text` the thing, `act` and `options` what it asks.
`read` is the `OperatorRead` `{ read, pick }`, the shape a question's `operator_read` already has. It is absent where no call read the item, and an empty `pick` is none.
`session_id` and `job` name the Job whose list holds the item, both absent on a go-ahead no Job holds and on a red main. `manifest_id` and `repository` are present where the item has a repository.
`raised_at` is when it joined the queue, and `later_until` is set while it waits at the end.
`keeps` says Always here and Always are offered: for a go-ahead and a red main, which Fleet can do by itself, and not for a decision.

A red main nobody is on is an item: a `go_ahead` with no Job, its `item.id` `red:<repository>:<commit>`, its one option `Start a fix`.

**`then`** is on a go-ahead to file an issue only.
Each `OperatorThenOffer` is `just_file`, `file_and_start` or, **only where a live Job already works on the same files**, `file_and_hand` with `hand_to` `{ session_id, job }`.
The Operator's read marks one `pick`.
Fleet finds that Job from the paths each live Job has written and its plan names, as it does for a clash.

**The order** is the order he left, with an item he put Later after the rest until the local day ends.
**Nothing is queued while the Operator is off**, and `turn` is then empty. What waits is queued when it comes on.
Where the review call fails, or the week's plan is paused, the item is still queued, with no `read`.

## The page's other fields

`top` is the page's one line, rewritten each time he comes back, and absent before the first.
`looked_at` is when he last looked, which the digest counts from.
`on_it` is `{ reading, since }`, present while the Operator is reading something to review.
`digest` has `landed`, the pull requests that merged since `looked_at`; `handled`, the Operator's own acts since then, newest first, each `{ at, title, outcome }`, and `log`, `{ week, path }` of the week's journal; `running`, the live Jobs; `coming_up`, the Jobs waiting to start with `needs`; and `learned`, the Always and Always here rules across projects, each with its repository where it holds in one.
A digest section with nothing in it is absent from the JSON, except `handled`.
Each `landed` row is `{ session_id, job, number, title, at }`, with `at` the time the pull request merged.

## The head

`mains` is one `OperatorMain` for each repository Fleet serves, first first, which the head draws as a pill: red when main is red, plain when it is not.
Each is `{ manifest_id, repository, state?, commit?, since? }`.
`repository` is the name an item gives.
`state` is `green`, `red`, `running` or `nothing_ran`, as the merge line's `main_ci_state` has it, and a red that a newer commit's run is still on is `red`.
`commit` is whole: the commit the state was decided from, and the red's own on a red.
`since` is `red` only, when main first read red.
`state` and `commit` are absent where Fleet has not read that main yet.
It is read from what main's CI already kept, the row that makes a red main a queue item, so the page makes no call and adds no poll.

`slots` is `{ used, total }` across every pool, read from the list Cleanup draws, which a keeper holds in memory.
`used` counts the slots a Job or a process holds, work stranded in one, and a take or a release under way.
`total` counts every slot, free, unmade or closed ones too.
It is absent where no pool is read.

## A press

`operator_answer` takes `yes`, `always_here`, `always` or `no`.
**Yes does the pick.** On a go-ahead that is what the Operator would have done: for a note or an error to file, the filing and the Then; for a red main, a Job titled `Fix main` started at once with no plan step.
On a question or a card with options, the option the Operator picked is answered on the Job's list.
**An item Fleet cannot perform itself**, such as a walk to open or a pull request, is left to Bridge: `open` is its `WaitingAct`, and the item stays in Your turn until its Job's list drops it.
**Always here** and **Always** do the pick and keep it as a rule, for that item's repository or for every one.
A rule is kept only for a go-ahead and a red main.
From then on that thing, arriving while the Operator is on, is done with no press and is in `handled`.
A rule is not applied while the Operator is off.
**No** drops the item. On the Operator's own go-ahead it is a Skip.
Later is `operator_later` and Ask is Bridge opening the chat with `talk_to_operator` carrying `about: "turn:<n>"`, so neither is an `OperatorAnswer`.

| Status | Error | Where |
| --- | --- | --- |
| 404 | `fleet.operator_turn_unknown` | No open item has `id`. Nothing is done. Also on `operator_later` |
| 422 | `fleet.operator_off` | The Operator is off |
| 422 | `fleet.operator_answer_not_offered` | The answer is not offered on the item: Always on a decision, `file_and_hand` with no live Job to hand to, a `then` on anything but a file |
| 422 | `fleet.operator_turn_not_done` | The pick could not be carried out. The item is left standing |

`operator_turn_order` leaves out an id that is not an open item, and the open items it does not name follow in the order they had.

## When the window comes to the front

`operator_in_front` is sent whenever Bridge's window gains focus or is shown, however often.
Fleet keeps when the window was last in front.
A time in front more than a quarter of an hour after the last sets when he last looked and rewrites `top` with one reading call.
The first ever counts from the start of the day before.
`rewriting` is true where that call is under way.
Fleet acts only while the Operator is on. Where the call fails, or the week's plan is paused, `top` is left as it was.
A time in front that comes before Fleet is connected is owed: Electron sends it on connecting.

## The event stream

`operator.page_changed` is sent when an item joins Your turn or settles, when the order, a Later or a rule moves, when `top` or `on_it` changes, when `operator_in_front` moves the top line, and when a repository's main turns red or green, once for each reading that changed it.
**It is not sent when the slot count moves**: slots move with every lease and every release, and Bridge reads `slots` with the page whenever it reads the page.
**It carries nothing** beyond the stream's cursor: Bridge reads `operator_page` again, and a new item is what raises its toast.
It is not sent for the digest's other sections, which are read.
A client that missed one reads the page again on reconnecting.
It is one empty event per change, far rarer than `session.changed`, so the stream's unbounded sink takes little more from it.

## The journal of the week

Not on the wire.
Fleet keeps a markdown file per ISO week under its own home, `operator/journal/<year>-W<week>.md`, and hands this week's and last week's to every reading call. `digest.handled.log` names the file.

## Skew

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
`OperatorTalk` and its lines are as they were, so the talk reads the same on both sides of it.
