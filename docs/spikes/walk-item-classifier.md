# Spike: Does an agent's waiting item ask the person to walk, or repeat what Fleet lists?

**Answer: walks, from rules; the pending plan, from the local language model.** On the held-out half,
the rules caught 18 of 18 walk items and called none of the other 43. The model reading an item
against what Fleet already lists catches no walk the rules miss and refused a real question in
cross-validation, so it reads plans only. Both are in force.
The set is small and comes from one machine, so a miss on a fresh shape is likely; each is one phrase to add.

Rules measured on 19 Oct 2026, the model on 20 Oct. The code is `crates/fleet/src/session_host/walk_asks.rs`
and `plan_asks.rs`; what they feed is
`docs/concepts/session/waiting-on-the-person.md`.

## Why it was asked

On 19 Oct three Jobs put a walk on the owner's Needs you list in their own words: one with no
window behind it, and one that stayed after he approved the window.
Fleet already lists each window a Job shows as `walk:<url>` and takes it off at his Approve, and it
already dropped an agent item with a `walk` act on a shown address.
None of the three items had an act, so nothing caught them.

## What was labelled

Every `waiting_for` item agents sent in the owner's local Claude Code transcripts for this
repository: 121 distinct pairs of id and text, after two empty texts were dropped.
One labeller, the agent that wrote the rules, marked an item `walk` where it asks him to walk,
try or approve a page or change shown or to be shown, and `other` otherwise.
31 are walks.

The items stay outside the repository.
They were shuffled with a fixed seed and halved before any rule was written.

| Set | Items | Walks | Used for |
|---|---|---|---|
| Dev | 60 | 13 | Writing the rules |
| Held out | 61 | 18 | The one measured run |

## Held-out run

| | Walk | Other |
|---|---|---|
| Called | 18 | 0 |
| Not called | 0 | 43 |

The rules were not changed after it.

## The rules

An item is a walk when any of these holds. Text is lowercased first.

| Signal | Walks it catches | Walks only it catches |
|---|---|---|
| The act is `walk` | 1 | 0 |
| The text's first word is `walk` | 28 | 1 |
| The id's first word is `walk` | 25 | 1 |
| `leave a note` or `leave notes` | 23 | 1 |
| `in the window`, `walk window` or `?walk=` | 12 | 0 |

Agents mostly send no act: 29 of the 31 walks had none, one sent `walk`, and one sent its address
as the act.
The word `walk` anywhere would add four wrong calls: a branch named for a walk, a merge of a walk's
fix, a context question naming "walk then PR", and a plan about this rule.

## The model against what Fleet lists, 20 Oct 2026

The owner asked for a real model to read agents' text, and Fleet has one (`reply-asks-classifier.md`).
The question here is narrower than "is this a walk": does an item restate an ask Fleet already holds
for the same Session, that is a window shown and not yet approved, or the plan recorded and not yet
approved. An item about a walk that is not open, or one that only mentions a walk, is not a repeat.

### Data

Every `waiting_for` item agents sent that carried the context of the open walks and the pending plan
at that moment: 400 items from the owner's local transcripts, labelled by the agent that built the
reader, outside the repository. Seven `unsure` were dropped.

| Label | Items | Before 19 Oct | From 19 Oct |
|---|---|---|---|
| `walk`: repeats an open walk | 20 | 20 | 0 |
| `plan`: repeats the pending plan | 7 | 6 | 1 |
| `other` | 366 | 340 | 26 |

Thresholds were chosen by stratified five-fold cross-validation on the 366 items before 19 Oct 00:00 UTC,
then fixed on all of them and scored once on the 27 from then on. **The scored half has one positive**,
a plan item, and no walk. Its figures confirm nothing; the cross-validation carries the evidence.

### The readers

| Reader | Calls an item a repeat when |
|---|---|
| Rules | `asks_to_walk` holds. They know walks only, whether or not one is open |
| Model | Its cosine to an open walk is at least `t_walk`, or to a pending plan at least `t_plan` |
| Either | Either of the above |

The model embeds the item as `<id in words>: <text>` with the base `bge-small-en-v1.5` vectors (`embed`),
and compares with Fleet's own wording of what it lists: `Walk <title or the url's walk=name>` for each
open walk, and `Plan to approve`, the text of Fleet's plan item, for the plan. Three other forms of
the plan's document (its group titles joined, each separately, or after `Plan to approve`) did no
better or worse on 12 items with a plan pending. The titles pull in questions that share the plan's
vocabulary, such as a question about a full pool beside a plan that adds a slot, so the anchor is the
fixed text alone. Variation names added nothing to the walk's document.

A threshold is the lowest cosine at which no `other` item the rules do not already refuse is called a
repeat in the training folds. For the plan the 340 development `other` items are all scored against
the anchor as if a plan were pending, since only 5 of them had one.

### Scores

Cross-validation over the 366 development items (20 walk, 6 plan, 340 other). A wrong refusal is an
`other` item refused; "beyond the rules" is one the rules do not refuse.

| Reader | Walks caught | Plans caught | `other` refused | Beyond the rules |
|---|---|---|---|---|
| Rules | 20 of 20 | 0 of 6 | 10 of 340 | 0 |
| Model | 11 of 20 | 5 of 6 | 2 of 340 | 2 |
| Either | 20 of 20 | 5 of 6 | 12 of 340 | 2 |
| **Either, the model on plans only** | 20 of 20 | 5 of 6 | 10 of 340 | 0 |

Scored once on the items from 19 Oct (0 walk, 1 plan, 26 other):

| Reader | Plans caught | `other` refused | Beyond the rules |
|---|---|---|---|
| Rules | 0 of 1 | 2 of 26 | 0 |
| Model | 1 of 1 | 0 of 26 | 0 |
| Either | 1 of 1 | 2 of 26 | 0 |
| Either, the model on plans only | 1 of 1 | 2 of 26 | 0 |

| Threshold | Per fold | On all development items |
|---|---|---|
| `t_walk` | 0.790 to 0.845 | 0.845 |
| `t_plan` | 0.691 to 0.712 | **0.712** |

### What is in force

**Either, with the model reading plans only** (`plan_asks::IN_FORCE`, `REPEATS_AT` 0.712).

- The walk side of the model catches no walk the rules miss: every walk in the set has a cue word. Its
  best threshold still called "Clash mark icon" a repeat of a walk titled "Write-clash mark". Topic
  is what the vectors measure, and a question about a page reads as close as a request to walk it.
  It would have refused 2 of 340 real items in cross-validation for no gain, so the rules stand
  alone for walks.
- The plan side separates by the ask rather than the topic. In cross-validation it caught 5 of 6 and
  refused none of 340, and on the one scored item it caught the plan the rules cannot. The closest
  `other` item, "Read the revised red-main claim spec, then approve", scored 0.711 against a threshold
  of 0.712, so one fresh near-duplicate of that shape would be refused. With one scored positive, that
  is the weak point of this choice.

The refusal is the walk item's refusal path: `waiting_for` refuses the list, naming the item and the
plan version, and tells the agent Fleet lists it as "Plan to approve" until he approves or sends
changes. It stands in for nothing while the model is not loaded, and embeds nothing while no plan is
pending. The thought router already puts the model in force (`language_model::wanted`), so this reader
adds no fetch or load of its own; with every reader on rules, nothing would be fetched.

### What it costs

One batch per `waiting_for` call that has a plan pending: Fleet's plan item first, then every item of
the list, on a blocking thread. With the model loaded, release build, load average about 5:

| Items in the list | Median | Slowest of 10 |
|---|---|---|
| 1 | 27 ms | 30 ms |
| 3 | 54 ms | 94 ms |
| 6 | 79 ms | 83 ms |
| 10 | 114 ms | 120 ms |

### The rules refuse real items too

The 366 `other` items include 12 the rules refuse, 10 before 19 Oct and 2 after. In 6 of them the id
alone starts with `walk`, in 3 a phrase such as "walk window" appears in a question or a restart
request, and 3 carry a `walk` act on an item that is not a walk. The held-out run above found none in
43 `other` items; over 366 the rate is 3%. Nothing here changes them: a model that refuses
less than the rules would need the same label set to be called better, and the label is one agent's.

## What it does not cover

- **A walk worded with no cue**: "Look at the reopened panel" or "Try the new tab" with an id that
  does not start with `walk`. Two items of that shape were labelled `other`, since they ask him to
  try a build in his own Bridge rather than a page.
- **A plan repeated in words unlike "approve"**: the model catches what reads like the plan's ask.
- **Labels from one labeller**, as in `reply-asks-classifier.md`.
- **Plan item groups and tasks**: the data holds group titles only, so they are not in the reader.
