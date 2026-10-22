# Spike: Can the owner act on an item an agent puts on his Needs you list?

**Answer: the rules catch it, and the head adds nothing. Held out and read once, the rules called 27 of 36 unclear items at 0.73 precision (10 clear items wrongly called). The `item_clear` head alone found 3 of 36 at 0.75, and beside the rules it added 4 wrong calls and no right one. Everything is built and off (`clarity::IN_FORCE`); the owner picks.**

Measured on 22 Oct 2026.
The code is `crates/fleet/src/session_host/clarity.rs`, with the head in `crates/adapters/src/language_model/tuned.safetensors` beside the four of `docs/spikes/reply-sentence-readers.md`.

## Why it was asked

The owner was shown a reply's section heading, "Bigger than it looked?", as his question.
The sentence readers drop headings from a reply's close, but the rules' own sentence (`asks::asking_line`) is picked from lines the heading rule never sees, and an agent's own `waiting_for` item is not read at all.
Fleet already refuses an agent item that asks him to walk (`walk_asks.rs`) or repeats the plan (`plan_asks.rs`).
This asks the general question: is the item one he can act on as written?

## What was labelled

569 items from the owner's local Fleet, from 118 Sessions, outside the repository.
Each was marked `clear` or `unclear` with a one-line reason, and the items were split by Session before any rule or threshold was written.
An item is clear when it holds a verb aimed at him and the thing it names, reads alone, and is not a status report.

| Set | Items | Clear | Unclear | Used for |
|---|---|---|---|---|
| Dev | 285 | 257 | 28 | Writing the rules, training the head, choosing the thresholds |
| Held out | 284 | 248 | 36 | The one measured run |

The 28 unclear dev items by reason: 15 status ("#2645 in the merge queue", "waiting on a merge"), 8 a noun, a topic or a statement with no ask, 3 several asks folded into one, 2 pointing at choices stated elsewhere.
The held half's reasons were not broken down, and its items were not read.

## The rules

An item is **unclear** if it points elsewhere ("see above", "the options below", "three choices" with none listed), else if no clause holds a verb aimed at him with the thing it names; it reads as a **status** when that clause-less item has a word of one ("waiting", "queued", "merging", "pass"), and as **no ask** otherwise.
A fenced command handed over is the verb and the thing.
A clause leads with a verb from a list of 111 (seven of them programs, as in a command line handed over), or, at the start of the line, with a question word; after a colon a question word needs the `?`.
A line ending in `?` that offers "X, or Y" is a choice.
A verb takes its thing from its own clause, the clause after its colon, or the clauses before it.
"Bigger than it looked?" ends in a question mark and leads with an adjective, so it is `no ask`.

The first cut called 30 of the 257 clear dev items unclear.
They were questions with no `?`, choices, a command line handed over ("git worktree prune"), "Walk: the sidebar", and verbs missing from the list.
The second cut is the one scored: dev precision 0.96, recall 0.93.
That is a figure for rules written against those items, so it is the top of the range.

## The head

`item_clear` reads the item's text, 64 pieces from its start, through the committed adapter's frozen layers, the way the sentence heads do: only a linear layer and sigmoid were trained, on the start mark's vector.
The adapter layers and the four earlier heads are the same bits, which `tuned.rs` hashes.
Eight epochs of AdamW at 1e-3, an unclear item weighted 3, the score for "unclear" taken as one minus the head's probability.
Five folds by Session on the dev half gave each item a score from a head that had not seen it; then one head was trained on all of dev and the held half was scored once.
`tools/lm-train clarity`.

| Reader | Set | Threshold | Precision | Recall | Right | Wrong | Missed |
|---|---|---|---|---|---|---|---|
| Rules | dev | | 0.963 | 0.929 | 26 | 1 | 2 |
| Head, folds (AUC 0.804) | dev | 0.72 | 1.000 | 0.071 | 2 | 0 | 26 |
| Head, folds | dev | 0.64 | 0.875 | 0.250 | 7 | 1 | 21 |
| Rules or head, folds | dev | 0.64 | 0.929 | 0.929 | 26 | 2 | 2 |
| **Rules** | **held** | | **0.730** | **0.750** | 27 | 10 | 9 |
| **Head** (AUC 0.850) | **held** | **0.72** | **0.750** | **0.083** | 3 | 1 | 33 |
| **Rules or head** | **held** | **0.64** | **0.659** | **0.750** | 27 | 14 | 9 |

## The threshold

The dev point is the most recall at precision 0.90 or above over the five folds: **0.72** for the head alone, **0.64** for the head beside the rules.
`clarity::UNCLEAR_AT` is 0.64, since extracting reads the two together.
Neither was moved after the held half was read.

## What it says

The head ranks (held AUC 0.85) and cannot call: with 28 unclear items to learn from, the ones it is sure of are three.
Beside the rules it finds nothing they miss on dev or held, and on held it called 4 more clear items unclear, so the rules alone are the better reader as measured.
The rules fall from 0.96 to 0.73 precision between the half they were written on and the half they were not, and a clear item wrongly refused costs the agent a reworded list: 10 of 248 held clear items (4.0%), 14 with the head (5.6%).
The held half's 10 wrong and 9 missed items were not read; doing that is the next step before putting the rules in force, and changes the held figure into a second dev figure.

## What is in force

Nothing.
`clarity::IN_FORCE` is false, so `waiting_for` keeps the list as the agent sent it and a reply's asks are extracted as `reply-sentence-readers.md` says.
Switched on:

- `waiting_for` refuses an item the rules call unclear, naming the item and why in the voice of the walk refusal. It reads the rules only: the head cannot say why, and it did not catch what the rules missed.
- `extracting` words an ask that does not read as clear as the clearest of the agent's own sentences near it, and leaves it out where none is clear. The candidates are the ask's sentence and the rest of the paragraph or list item it sits in, or, for a heading, the paragraph it heads. The rules decide first and the head's probability breaks a tie. A clear ask keeps its own words. "Bigger than it looked?" followed by a paragraph ending "Split it into two files or leave it as one." is raised as that last sentence. The asks are numbered among themselves. The head reads a sentence of a close here, not an item, and that was not measured. The choice among candidates is tested with made-up replies, not measured on real ones.

## What it costs

| | |
|---|---|
| Rules | 8.6 microseconds per item, over the 569 labelled, no model |
| Head | one item as a one-item list: mean 68 ms, median 43, p95 153, max 1,912, under a load average of 45, so an upper figure |
| Memory | 163 MB resident, of a process that holds only the model and adapter |
| Adapter | 206,396 bytes for five heads, 1,700 more than the four |
| Training | seconds; the 569 items cache once through the frozen layers |

`language_model::wanted` still counts only the readers that read with the model; the rules in `waiting_for` need none.

## What this does not cover

The labels are one person's reading of what is clear, and the 15 status items of the dev half are the bulk of what the rules catch; a held half with fewer of them would read lower.
"Call cards: pick what a Sleep card offers, and when Sleep ends" holds two asks and the rules call it clear; nothing here reads for folded asks.
An item with an act is read by its words and a command only; an `approve_pr` act with a number is not taken as the verb.
A heading whose paragraph says only what happened, with no sentence that asks, is left out; the owner then sees nothing of it.
