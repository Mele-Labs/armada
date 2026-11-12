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
| `operator_answer` (`POST /operator/page/answer`) | Bridge writes | `OperatorAnswerPress` `{ id, answer, then?, project? }`, answered with `OperatorAnswered` `{ open?, started?, filed?, handed_to?, page }` |
| `operator_later` (`POST /operator/page/later`) | Bridge writes | `OperatorLater` `{ id, until? }`, answered with `OperatorPage` |
| `operator_turn_order` (`POST /operator/page/order`) | Bridge writes | `OperatorTurnOrder` `{ ids }`, answered with `OperatorPage` |
| `operator_in_front` (`POST /operator/front`) | Bridge writes | No body. Answered at once with `OperatorInFront` `{ rewriting }` |
| `operator.page_changed` | Fleet writes, Bridge reads | Nothing. Bridge reads `operator_page` again |
| `talk_to_operator` `about` | Bridge writes | A value, not a field: `turn:<n>` names an item of Your turn |

## Your turn

An `OperatorTurnItem` is what a card needs, with no second vocabulary beside the item's own.
`id` is `turn:<n>`, which every act names.
`kind` is `decision`, his to decide, or `go_ahead`, what the Operator would do itself.
`urgency` is `urgent` on a red main, else `decision` or `go_ahead` as its kind, so a screen reads it and never works it out. No stopped Job is queued today, so none reads `urgent`.
`item` is the `WaitingItem` as it was raised: its `source` is its kind, `text` the thing, `act` and `options` what it asks.
`read` is the `OperatorRead` `{ read, pick }`, the shape a question's `operator_read` already has. It is absent where no call read the item, and an empty `pick` is none.
`session_id` and `job` name the Job whose list holds the item, both absent on a go-ahead no Job holds and on a red main. `manifest_id` and `repository` are present where the item has a repository.
`raised_at` is when it joined the queue, and `later_until` is set while it waits at the end.
`keeps` says Always here and Always are offered: for a go-ahead, which Fleet can do by itself, and not for a decision.
`also_raised_by` names what else raised the same ask and was folded into this item, each in a few words for its tooltip, oldest first. Absent where nothing was.
`group` is the Job's id where that Job has two or more asks waiting: `turn` lists them together, at the place of the first of them in his order, in the order the Job raised them, and Bridge draws them as one entry.
Absent for a single ask.

A red main nobody is on is an item: a `go_ahead` with no Job, its `item.id` `red:<repository>:<commit>`, its one option `Start a fix`.
Every act the Operator would take is a go-ahead, so a note, an error or a report ruled to start, send or file is one, asked once.
Where a rule he kept covers the act, it is done as it is raised and is never an item.

**`then`** is on a go-ahead to file an issue only.
Each `OperatorThenOffer` is `just_file`, `file_and_start` or, **only where a live Job already works on the same files**, `file_and_hand` with `hand_to` `{ session_id, job }`.
The Operator's read marks one `pick`.
Fleet finds that Job from the paths each live Job has written and its plan names, as it does for a clash.

**The order** is the order he left, with an item he put Later after the rest until its `later_until`.
That is the `until` Bridge sent, worked out from the choice he pressed, or the local day's end where it sent none.
Fleet knows nothing of the choices.
**A pick is a button that does what it says**: one of the item's own options, exactly as labelled, or `Later`, which a Yes does.
Any other pick the call names is dropped.
A review of a Job's work picks one label of its kind, and Yes does it: on a plan, `Approve the plan` approves it and `Send the plan back` sends the Job the review's read as the reason; on a pull request, `Open the pull request` opens it, `Send it back` sends the Job the read, and `Walk it first` opens the Job's walk, or asks the Job for one where none is shown; on a walk, `Open the walk`.
**One ask, one item.** A suggestion that would file or close the same issue as an earlier open one is folded into it, named in its `also_raised_by`, and answered with it.
One that repeats a suggestion already answered is dropped, with no item and no row under Answered, and Fleet's log says so.
An ask read off a Job's reply is not queued while one of Fleet's own items for that Job stands: a held Job's start, a plan, a walk, a draft, a sketch or a document.
**Yes on a held Job's start starts that Job**, with the landing its card picked; where the card still asks how the work lands, its act is opened.
**Nothing is queued while the Operator is off**, and `turn` is then empty. What waits is queued when it comes on.
Where the review call fails, or the week's plan is paused, the item is still queued, with no `read`.

## The page's other fields

`top` is the page's one line, rewritten each time he comes back, and absent before the first.
`looked_at` is when he last looked, which the digest counts from.
`on_it` is `{ reading, since }`, present while the Operator is reading something to review.
`digest` has `landed`, the pull requests that merged since `looked_at`; `handled`, the Operator's own acts since then, newest first, each `{ at, title, outcome }`, and `log`, `{ week, path }` of the week's journal; `running`, the live Jobs; `coming_up`, the Jobs waiting to start with `needs`; `learned`, the Always and Always here rules across projects, each with its `scope` (`{ kind: "everywhere" }`, `{ kind: "repository", manifest_id }` or `{ kind: "project", project }`) and, where it holds in one, the repository's or project's name as `repository`; and `holds`, the holds that stand, each `{ id, manifest_id, repository?, said, at }`.
`learned` is rules only.
A hold's `id` is what `remove_operator_order` and the chat's lift name.
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
Pressed on an item he settled with Yes less than a minute ago, they only keep the rule, since the act is done, and `open` is absent; any other answer on a settled item, or an Always later than that, is 404 `fleet.operator_turn_unknown`.
`project` on an Always here keeps it for that project instead, which the page filtered to a project sends: 404 `fleet.no_such_project`, and 422 `fleet.project_not_member` where the item's repository is not one of its members.
A rule for the repository beats one for a project, which beats one for everywhere, and the same thing in the same place is kept once.
A rule kept before scopes read as the repository it named, or everywhere.
A rule is kept only for a go-ahead.
From then on that thing, arriving while the Operator is on, is done as it is raised, with no press.
It is in `handled` as `By a rule: <the rule's words>`, and the Log row it leaves is signed by the Operator and says by which rule.
The repository's rules are read first, then those of a project that holds it, then every repository's.
A rule is not applied while the Operator is off.
**No** drops the item, for this time only. On the Operator's own go-ahead it is a Skip.
Later is `operator_later` and Ask is Bridge opening the chat with `talk_to_operator` carrying `about: "turn:<n>"`, so neither is an `OperatorAnswer`.

| Status | Error | Where |
| --- | --- | --- |
| 404 | `fleet.operator_turn_unknown` | No open item has `id`. Nothing is done. Also on `operator_later` |
| 422 | `fleet.operator_later_past` | `operator_later`'s `until` has passed or does not read as a time. Nothing is done |
| 422 | `fleet.operator_off` | The Operator is off |
| 422 | `fleet.operator_answer_not_offered` | The answer is not offered on the item: Always on a decision, `file_and_hand` with no live Job to hand to, a `then` on anything but a file |
| 422 | `fleet.operator_turn_not_done` | The pick could not be carried out. The item is left standing |

`operator_turn_order` leaves out an id that is not an open item, and the open items it does not name follow in the order they had.
Naming any ask of a group moves the whole group, since the page lists a group at the place of whichever of its asks comes first.

## When the window comes to the front

`operator_in_front` is sent whenever Bridge's window gains focus or is shown, however often.
Fleet keeps when the window was last in front.
A time in front more than a quarter of an hour after the last sets when he last looked and rewrites `top` with one reading call.
The first ever counts from the start of the day before.
`rewriting` is true where that call is under way.
Fleet acts only while the Operator is on. Where the call fails, or the week's plan is paused, `top` is left as it was.
A time in front that comes before Fleet is connected is owed: Electron sends it on connecting.

## The event stream

`operator.page_changed` is sent when an item joins Your turn or settles, when the order, a Later or a rule moves, when a Later runs out, when `top` or `on_it` changes, when `operator_in_front` moves the top line, and when a repository's main turns red or green, once for each reading that changed it.
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

## Since 12 Nov 2026: Later until a time, and one ask folded into one item: `OperatorLater.until` and `OperatorTurnItem.also_raised_by`

**Additive only**: one optional field on a body, one optional field on an answer, and one error code; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src`, the operation files and `packages/protocol/src`, and there is no number to take.
Bridge's half is `OperatorLater.until` and `OperatorTurnItem.also_raised_by` in `packages/protocol/src/operator-page.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `OperatorLater.until` | Bridge writes | RFC 3339, the instant the item comes back, worked out by Bridge from the choice he pressed. Absent, the local day's end, as before |
| `OperatorTurnItem.also_raised_by` | Fleet writes | What else raised the same ask, folded into this item, each in a few words for its tooltip, oldest first. Absent where nothing was |

**Refused with nothing changed**: `operator_later` 422 `fleet.operator_later_past` where `until` has passed or does not read as a time.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** No new event: `operator.page_changed` is also sent when a Later runs out, read on Fleet's tick.

## Since 12 Nov 2026: a Job's asks as one entry, and Always after a Yes: `OperatorTurnItem.group`

**Additive only**: one optional field on an answer, and Always here or Always taken on an item settled with Yes within the minute; no operation, route, event kind or error code.
The protocol ID moves with `crates/ipc/src`, the operation files and `packages/protocol/src`, and there is no number to take.
Bridge's half is `OperatorTurnItem.group` in `packages/protocol/src/operator-page.ts`, written by hand.

| Where | Side | Carries |
| --- | --- | --- |
| `OperatorTurnItem.group` | Fleet writes | The Job's id, on each of its asks where it has two or more waiting. Absent for a single ask |

**Order.** A group's asks stand together in `turn`, at the place of the first of them in his order, oldest first.
To drag a group, Bridge sends its ids in `operator_turn_order`; naming any one of them moves the whole group.
Asks put Later group among themselves, after the rest.

**Always after a Yes.** `operator_answer` with `always_here` or `always` on an item he settled with `yes` less than a minute ago keeps the rule only and is answered with no `open`; Bridge offers the line under a Yes for that minute.
Any other answer on a settled item, and an Always after that minute, stays 404 `fleet.operator_turn_unknown`.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
