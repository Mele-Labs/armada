# Spike: Does an agent's waiting item ask the person to walk?

**Answer: yes, from rules alone. On the held-out half, the rules caught 18 of 18 walk items and
called none of the other 43.** On the dev half they read 13 of 13 and none of 47.
The set is small and comes from one machine, so a miss on a fresh shape is likely; each is one phrase to add.

Measured on 19 Oct 2026. The code is `crates/fleet/src/session_host/walk_asks.rs`; what it feeds is
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

## What it does not cover

- **A walk worded with no cue**: "Look at the reopened panel" or "Try the new tab" with an id that
  does not start with `walk`. Two items of that shape were labelled `other`, since they ask him to
  try a build in his own Bridge rather than a page.
- **Labels from one labeller**, as in `reply-asks-classifier.md`.

## Why no model

Nothing in Armada calls a model to label text; the one model call is the Judge.
A call inside `waiting_for` would add a round trip to every list an agent sends, or let an item
show before it was judged.
`reply-asks-classifier.md` chose rules over a model for the same kind of call, and its reasons hold.
A miss costs what the bug cost before: one item beside Fleet's own walk.
