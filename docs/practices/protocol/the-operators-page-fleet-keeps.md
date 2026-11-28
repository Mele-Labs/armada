# The Operator's page, kept by Fleet: `operator_page`, the card acts and `operator.page_changed`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. The Operator is one agent session for each project, and its page is one card in front with the cards queued behind it and a feed beside them.
Fleet keeps the cards, the feed and each project's level, and serves them as one page.
Bridge draws it.
`docs/concepts/operator.md` says what the page is for.
The rules the session runs under are `crates/adapters/src/operator_rules.md`.
Electron shows no Operator and sends none of this.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `classic-final:packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/operator_page.rs`; Bridge's half is `classic-final:packages/protocol/src/operator-page.ts`, written by hand, field for field.

## What the wire carries

| Where | Side | Carries |
| --- | --- | --- |
| `operator_page` (`GET /operator/page`) | Bridge reads | `OperatorPage`. Gains `feed`, `session_id`, each project's `operator_acts` and `working`. `turn` is the card in front, then the cards queued behind it; `working` is the cards he picked that are not settled |
| `raise_card` (`POST /operator/cards`) | The Operator's session writes | `RaiseCard` `{ kind, title, read, options, sessions?, repository?, talk_of? }`. Answers `CardRaised` `{ card }` |
| `revise_card` (`POST /operator/cards/revise`) | The Operator's session writes | `ReviseCard` `{ card, read?, options? }`. Answers `CardRevised` `{ card }` |
| `report_card` (`POST /operator/cards/report`) | The Operator's session writes | `ReportCard` `{ card, step?, failed? }`, exactly one of `step` and `failed`. Answers `CardRevised` `{ card }` |
| `settle_card` (`POST /operator/cards/settle`) | The Operator's session writes | `SettleCard` `{ card, said }`. Answers `OperatorCardSettled` `{ card }` |
| `list_operator_acts` (`GET /operator/acts`) | The Operator's session reads | `OperatorActsList` `{ projects: [{ project, name, acts? }] }` |
| `set_operator_acts` (`POST /operator/acts`) | The Operator's session and Bridge write | `SetOperatorActs` `{ project, acts? }`, `acts` one of `act_then_tell`, `ask_before_starting`, `only_when_told`, absent to unset. Answers `OperatorActsList` |
| `close_session` (`POST /sessions/close`) | The Operator's session writes | Ends a Job, as Bridge's Close does |
| `operator.page_changed` | Fleet writes, Bridge reads | Nothing. Bridge reads `operator_page` again |

**The card acts are the Operator's alone.**
Fleet refuses them from any other caller with `fleet.not_the_operator`, refuses a card it cannot take with `fleet.card_refused`, an unknown one with `fleet.card_unknown`, and a press on a card while the session is not running with `fleet.operator_session_not_running`.
`report_card` is refused with `fleet.card_refused` on a card he has not picked or that came back, and for both or neither of `step` and `failed`.

## A card

**A card is one decision.**
`kind` is `deal` or `raise`.
`title` is the thing.
`read` is the facts needed to decide.
`options` is two to four, each `{ label, cost }`, and the first is the Operator's pick.
A card has two to four options, and each has a `cost`.
`sessions` names the Jobs the card is about and `repository` the project.
`talk_of` names the card a new one follows from, so a talk that ends in a new card is one thread.

**Every later act names the card.**
`revise_card`, `report_card` and `settle_card` name it, and so do his press, Snooze and the composer.
`revise_card` changes the read or the options of a card that stands, and the same card stays in front.
`settle_card` closes it with `said`, what he chose or what was done, and it leaves the page.

**A card he pressed carries three more fields.**
`picked` is `{ option, at }`, the label he pressed and when.
`steps` is `[{ at, said }]`, what the Operator reported doing since, oldest first, and is empty before a press.
`returned` is `{ why, at }`, present while a picked card is back in front.
A press clears `returned` and `steps` and sets `picked` anew.
A card raised before these fields reads as not picked.

## The page

**`turn` is the card in front, then the cards queued behind it.**
The first element is the card in front, and the rest are queued behind it, with a card he sent back by Snooze left out until its time.
The right side draws them as Next up, folded, with their count beside it and the ideas the Operator kept after them, and opens their list.
A press on one brings it to the front.
A card has what it was raised with, its `id`, when it was raised, and `later_until` while it is sent back.
It carries no count of other cards.

**`working` is the cards he picked that are not settled, newest pick first.**
Bridge draws each at the top of the feed, with its title, `picked.option` and a line for each of `steps`.
A Snooze does not hide one.
**A picked card is never lost**: until it is settled it is in `working` or back in `turn`.

**A picked card comes back first in `turn`.**
It comes back when the Operator calls `report_card` with `failed`, which is `returned.why`, or when nothing settles it within 15 minutes of `picked.at`, with `returned.why` `Nothing settled in 15 min`.
Fleet checks the 15 minutes on the Operator's tick, and tells the session which card came back.
A card that came back stands ahead of every other card and stays out of `working` until he presses again.

**`feed` is what happened since he last looked.**
Each entry is information with no decision in it: what the Operator dispatched, a merge, a Job that ended, and what a Job asked or told the Operator.
A project at `act_then_tell` has each act it did unasked told here.
`looked_at` is when he last looked, and the feed counts from there.

**`session_id` is the Operator's own session.**
It changes when the Operator hands on to a fresh copy, and the page follows it.

**The head is as it was.**
`mains` is one `OperatorMain` for each repository Fleet serves, and `slots` is `{ used, total }`.
Both are read from what Fleet already keeps, so the page makes no call and adds no poll.

## What his hands do

**His press, Snooze and each composer send reach the session as messages that name the card.**
Fleet writes the message and puts it in the Operator's session, and the Operator acts on it.
There is no answer that does the pick for him, no Always and no rule kept: a press is a message, and the Operator does what the option said and settles the card.
**A press also picks the card**: it leaves `turn` for `working`, so the next card is in front at once.
A press on a card that is in `working` is refused with `fleet.operator_answer_not_offered`.

| He does | The session is told |
| --- | --- |
| Presses an option, or keys 1 to 4 | The card and the option, and to act now: report each step, settle when done, report `failed` the moment it cannot be done |
| Presses Discuss (`/`) and sends | The card and his words. The card folds to one line and the talk runs below it |
| Presses Snooze (`S`) | The card and when it comes back. The card is hidden until then |

**A talk ends by settling the card or putting up a new one.**
Until then the page keeps the card folded in front, and the talk below it.

## `operator_acts`

**It is a value for each project.**
`act_then_tell` does the act and says so in the feed.
`ask_before_starting` raises a card before any Job starts.
`only_when_told` does nothing unasked.

**A project with no value reads as unset.**
The Operator asks once with a card, and his answer is kept as that project's value through the route above.
Armada starts at `act_then_tell`.
Whatever the level, a change he would see or a decision that is his is a card.

## What reaches the session

**A note he sends, a Job's `raise_conflict` and a Job's `ask_held` arrive in the session as messages.**
`raise_conflict` carries `with`, `files`, `wire`, `mine`, `theirs` and `breaks`, and `ask_held` carries exactly one of `pull` and `branch`.
Fleet checks every part and refuses with `fleet.conflict_refused` or `fleet.held_refused`, in a sentence that names the part.
A Job's question stays in that Job's own Needs you and is not sent.

## The event stream

`operator.page_changed` is sent when a card is raised, revised, picked, reported on, returned or settled, when a card is snoozed or comes back, when the feed gains an entry, when a project's value changes, when a project's Operator is set, and when a repository's main turns red or green.
It is not sent when the slot count moves, since Bridge reads `slots` with the page.
It carries nothing beyond the stream's cursor: Bridge reads `operator_page` again.
A client that missed one reads the page again on reconnecting.

## What goes from the wire

These leave with the old machinery, and a later change removes them: `operator_answer` and its Yes, Always here, Always and No, `operator_undo_ruling`, `operator_turn_order`, the Then offers, `follows` and `ruled`, the kind-chip hint on a dispatch, and the per-item review's read and pick on an item.
Until then each behaves as it does now.
`operator_later` is replaced by Snooze as a message naming the card.

## Since 28 Nov 2026: one Operator for each project

`docs/concepts/operator.md`, *One for each project*, says what changed.
**Two operations and some fields, and no event kind.**
The protocol ID moves with `crates/ipc/src/operator_settings.rs`, the operation files and the route table, and there is no number to take.
Pocket reads none of these types, so `cargo xtask verify-wire --write` writes nothing for them.
Nothing here is Electron's.

| Where | Side | Carries |
| --- | --- | --- |
| `list_operator_settings` (`GET /operator/settings`) | Bridge reads | `OperatorSettingsList` `{ projects, styles }`. Each project is `{ project, project_name, settings, set, session_id? }`: `settings` is every value resolved, `set` is what he set, each absent where he has not, and `session_id` is the live session, absent while none runs. `styles` are the output styles he may pick, each `{ name, description? }` with its file's own words where it has them, `armada:Operator` first |
| `set_operator_settings` (`POST /operator/settings`) | Bridge writes | `SetOperatorSettings` `{ project, set, unset }`. `set` holds the values to keep, and `unset` names the values to return to their default: `on`, `model`, `effort`, `role`, `words`, `prompt`, `output_style`, `decisions`, `name`, `accent`, `acts`, `tools`, or one of `tools.start_jobs`, `tools.merge`, `tools.end_jobs`, `tools.file_issues`. Answers `OperatorSettingsList` |
| `TalkToOperator` (`talk_to_operator`) | Bridge writes, Fleet reads | Gains `project`, the project of the screen he is on, and `looking_at`, the Job he has open. Each is absent when he has none. Fleet routes by them with the card `about` names, then by his words |
| `OperatorPage` (`operator_page`) | Bridge reads | Gains `operators`, each project's Operator as `list_operator_settings` answers it. `session_id` is the first session that runs |
| `OperatorTurnItem`, `OperatorFeedEntry` and `ToReviewItem` | Fleet writes, Bridge reads | Gain `operator`, an `OperatorTag` `{ project, name, accent? }`: the project's Operator the item belongs to, with the name and accent it is set to. Absent where no project's Operator holds it, and on a feed line kept before each project had one |

**`OperatorSettings` is every value resolved.**
`on`, `model`, `effort`, `role` (`copilot`, `dispatcher` or `reviewer`), `words`, `prompt`, `output_style`, `tools` `{ start_jobs, merge, end_jobs, file_issues }`, `decisions` (`cards` or `chat`), `name`, `accent?` and `acts?`.
A tool he did not set follows the role, and `accent` is absent until he sets one.

**Set and unset are refused whole, before anything is kept.**
An unknown project is 404 `fleet.no_such_project`.
An `accent` that is not `#rrggbb`, or a name in `unset` that is none of those above, is 422 `fleet.operator_setting_invalid`.
An empty `model`, `effort`, `words`, `prompt`, `output_style` or `name` is the same as unsetting it.

**What a change does to a live session.**
A change to `model`, `effort`, `role`, `words`, `prompt` or `output_style` hands the project's session on to a fresh copy at once.
A change to `decisions` tells the live session in one line, and everything else applies on its next call.

**Where the talk goes.**
`talk_to_operator` with `project`, `looking_at` or neither is routed by Fleet as `docs/concepts/operator.md`, *His words in All*, says.
Where none is sure the words are kept and the answer is the asking card, a row of Your turn with no `card`.
`raise_card` is refused with 422 `fleet.operator_chat_mode` for a project whose Operator asks its decisions in the talk.
A call that names another project's repository, Job or pull request is refused with `fleet.operator_other_project`, and he is asked on a card of Fleet's own.

## Skew

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
