# The Operator's page, kept by Fleet: `operator_page`, the card acts and `operator.page_changed`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. The Operator is one agent session per Mac, and its page is one card in front with the cards queued behind it and a feed beside them.
Fleet keeps the cards, the feed and each project's level, and serves them as one page.
Bridge draws it.
`docs/concepts/operator.md` says what the page is for.
The rules the session runs under are `crates/adapters/src/operator_rules.md`.
Electron shows no Operator and sends none of this.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/operator_page.rs`; Bridge's half is `packages/protocol/src/operator-page.ts`, written by hand, field for field.

## What the wire carries

| Where | Side | Carries |
| --- | --- | --- |
| `operator_page` (`GET /operator/page`) | Bridge reads | `OperatorPage`. Gains `feed`, `session_id` and each project's `operator_acts`. `turn` is the card in front, then the Waiting cards |
| `raise_card` (`POST /operator/cards`) | The Operator's session writes | `RaiseCard` `{ kind, title, read, options, sessions?, repository?, talk_of? }`. Answers `CardRaised` `{ card }` |
| `revise_card` (`POST /operator/cards/revise`) | The Operator's session writes | `ReviseCard` `{ card, read?, options? }`. Answers `CardRevised` `{ card }` |
| `settle_card` (`POST /operator/cards/settle`) | The Operator's session writes | `SettleCard` `{ card, said }`. Answers `OperatorCardSettled` `{ card }` |
| `list_operator_acts` (`GET /operator/acts`) | The Operator's session reads | `OperatorActsList` `{ projects: [{ project, name, acts? }] }` |
| `set_operator_acts` (`POST /operator/acts`) | The Operator's session and Bridge write | `SetOperatorActs` `{ project, acts? }`, `acts` one of `act_then_tell`, `ask_before_starting`, `only_when_told`, absent to unset. Answers `OperatorActsList` |
| `close_session` (`POST /sessions/close`) | The Operator's session writes | Ends a Job, as Bridge's Close does |
| `operator.page_changed` | Fleet writes, Bridge reads | Nothing. Bridge reads `operator_page` again |

**The card acts are the Operator's alone.**
Fleet refuses them from any other caller with `fleet.not_the_operator`, refuses a card it cannot take with `fleet.card_refused`, an unknown one with `fleet.card_unknown`, and a press on a card while the session is not running with `fleet.operator_session_not_running`.

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
`revise_card` and `settle_card` name it, and so do his press, Later and the composer.
`revise_card` changes the read or the options of a card that stands, and the same card stays in front.
`settle_card` closes it with `said`, what he chose or what was done, and it leaves the page.

## The page

**`turn` is the card in front, then the Waiting cards.**
The first element is the card in front, and the rest are queued behind it, with a card he sent back by Discuss later left out until its time.
The head draws the count of the rest as Waiting and opens their list.
A press on one brings it to the front.
A card has what it was raised with, its `id`, when it was raised, and `later_until` while it is sent back.
It carries no count of other cards.

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

**His press, Later and each composer send reach the session as messages that name the card.**
Fleet writes the message and puts it in the Operator's session, and the Operator acts on it.
There is no answer that does the pick for him, no Always and no rule kept: a press is a message, and the Operator does what the option said and settles the card.

| He does | The session is told |
| --- | --- |
| Presses an option, or keys 1 to 4 | The card and the option |
| Presses Discuss (`/`) and sends | The card and his words. The card folds to one line and the talk runs below it |
| Presses Discuss later (`L`) | The card and when it comes back. The card is hidden until then |

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

`operator.page_changed` is sent when a card is raised, revised or settled, when a card goes Later or comes back, when the feed gains an entry, when a project's value changes, and when a repository's main turns red or green.
It is not sent when the slot count moves, since Bridge reads `slots` with the page.
It carries nothing beyond the stream's cursor: Bridge reads `operator_page` again.
A client that missed one reads the page again on reconnecting.

## What goes from the wire

These leave with the old machinery, and a later change removes them: `operator_answer` and its Yes, Always here, Always and No, `operator_undo_ruling`, `operator_turn_order`, the Then offers, `follows` and `ruled`, the kind-chip hint on a dispatch, and the per-item review's read and pick on an item.
Until then each behaves as it does now.
`operator_later` is replaced by Discuss later as a message naming the card.

## Skew

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
