# Spike: Which sentences of a reply's close ask, and which does the agent keep for itself?

**Answer: the follow-up head reads, and the asks head ranks but barely calls. Held out and read once, `sentence_follows_up` found 102 of 191 follow-ups at 0.86 precision; `sentence_asks` found 28 of 137 asks at 0.90 precision, and 50 at 0.85. Both are built and measured, and neither is in force: the owner picks.**

Measured on 21 Oct 2026.
The code is `crates/fleet/src/session_host/extracting.rs`, with the heads in `crates/adapters/src/language_model/tuned.safetensors` beside the two of `docs/spikes/reply-asks-classifier.md`.
Nothing here is in force (`extracting::IN_FORCE`); the owner picks.

## Why

The asks reader reads a reply's whole close and raises one Needs you item with one sentence of it.
A reply asks several things at once, so the second and third asks reach no one.
The item is skipped entirely when the agent has any item of its own open (`waiting.rs`, `called`), so an ask beside a listed item is lost too.
And the owner was shown a section heading, "Bigger than it looked?", as his question, because a sentence ending in `?` is an ask and headings were kept.
So each sentence of the close is read alone by two new heads: whether it asks him, and whether it is a follow-up the agent keeps for itself.
Rules then read the act from an ask's words, and the heading rule drops what introduces the next paragraph.

## What was labelled

The closing sentences of 660 replies, as `asks::closing_sentences` splits them: 360 turn ends since 18 Oct 2026 that trained the asks head, and the 300 fresh replies it was scored on.
One labeller, an agent that had seen no reader, marked each sentence `question` (he must answer), `act` (he must do something: approve, walk, run, restart), `follow_up` (the agent says what it will do next) or `none`.
The replies were split in half by reply, seed 7, before anything was trained; the dev half chose everything, and the held half was scored once at the end.
The text and labels stay outside the repository.

| Label | Sentences | Dev half | Held half |
|---|---|---|---|
| `question` | 30 | 15 | 15 |
| `act` | 232 | 110 | 122 |
| `follow_up` | 408 | 217 | 191 |
| `none` | 2,427 | 1,222 | 1,205 |
| **All** | **3,097** | **1,564** | **1,533** |

30 questions are too few for a head of their own, so one head, `sentence_asks`, reads `question` and `act` together and the rules read the act's kind.
The other, `sentence_follows_up`, reads `follow_up`.

## Method

Each sentence is read alone, 64 pieces from its start, through the committed adapter's frozen layers: the LoRA adapters of the `asks` head stay as they are, so `asks` and `refused` keep their exact weights, and only a linear layer and sigmoid per reading were trained on the start mark's vector, the way `refused` was trained on the frozen `asks` adapter.
The vectors were cached once and the heads trained with AdamW at 1e-3 and weight decay 0.01, a batch of 16, 8 epochs (20 in the first run, below), positives weighted 3.
Five stratified folds by reply on the dev half gave each sentence a score from a head that had not trained on it; the threshold is the most recall at which those scores kept precision at 0.90 or above.
Then one head per reading was trained on all of dev, and the held half scored once, at dev's thresholds, after everything was fixed.
The trainer is `tools/lm-train`, mode `sentences-train`; it includes the committed encoder, `asks.rs` and `extracting.rs` by path and ran one process at a time under `taskpolicy -c utility`.
Trainer and `Local::read` agree to six decimals on every probe, and the `asks` and `refused` heads read the same before and after.

## Two runs

The first run trained 20 epochs, and the held half was scored once at its thresholds.
On the dev folds the `sentence_asks` operating point at precision 0.90 wandered with the epoch (recall 0.26 at epoch 3, 0.22 at epoch 8, 0.04 at epoch 20) while AUC crept from 0.935 to 0.954: past a few epochs the head's top scores thin out, and a 0.01 step of threshold swings the call.
So a second run trained 8 epochs, where the dev folds' AUC had levelled and the 0.90 point held, and the held half was scored once more.
Both runs are given; the second is the one committed.
The dev-fold rows of the second run are the first eight of the first, since the seeds are the same.

## `sentence_asks`

125 positives in the dev half, 137 in the held half.

| Run, set | AUC | Threshold | Precision | Recall | Right | Wrong | Missed |
|---|---|---|---|---|---|---|---|
| 20 epochs, dev folds, at 0.90 | 0.954 | 0.96 | 1.000 | 0.040 | 5 | 0 | 120 |
| 20 epochs, dev folds, at 0.80 | | 0.86 | 0.846 | 0.352 | 44 | 8 | 81 |
| 20 epochs, held, at dev's 0.96 | 0.965 | 0.96 | 1.000 | 0.073 | 10 | 0 | 127 |
| 20 epochs, held, at dev's 0.86 | | 0.86 | 0.825 | 0.343 | 47 | 10 | 90 |
| **8 epochs, dev folds, at 0.90** | 0.950 | **0.85** | 0.964 | 0.216 | 27 | 1 | 98 |
| 8 epochs, dev folds, at 0.80 | | 0.79 | 0.800 | 0.384 | 48 | 12 | 77 |
| **8 epochs, held, at dev's 0.85** | 0.961 | 0.85 | 0.903 | 0.204 | 28 | 3 | 109 |
| 8 epochs, held, at dev's 0.79 | | 0.79 | 0.847 | 0.365 | 50 | 9 | 87 |

The head ranks well (AUC 0.95 to 0.97) and calls poorly at 0.90 precision: the asks it is sure of are the plain ones, and most of the 262 positives are an approval or a walk said in passing ("Tests are still held until you approve the flight controls walk").
`extracting::ASKS_AT` is 0.85, the 8-epoch dev folds' point at 0.90.

## `sentence_follows_up`

217 positives in the dev half, 191 in the held half.

| Run, set | AUC | Threshold | Precision | Recall | Right | Wrong | Missed |
|---|---|---|---|---|---|---|---|
| 20 epochs, dev folds, at 0.90 | 0.901 | 0.85 | 0.910 | 0.465 | 101 | 10 | 116 |
| 20 epochs, dev folds, at 0.80 | | 0.61 | 0.800 | 0.627 | 136 | 34 | 81 |
| 20 epochs, held, at dev's 0.85 | 0.943 | 0.85 | 0.860 | 0.613 | 117 | 19 | 74 |
| 20 epochs, held, at dev's 0.61 | | 0.61 | 0.737 | 0.791 | 151 | 54 | 40 |
| **8 epochs, dev folds, at 0.90** | 0.897 | **0.81** | 0.906 | 0.442 | 96 | 10 | 121 |
| 8 epochs, dev folds, at 0.80 | | 0.58 | 0.802 | 0.636 | 138 | 34 | 79 |
| **8 epochs, held, at dev's 0.81** | 0.936 | 0.81 | 0.864 | 0.534 | 102 | 16 | 89 |
| 8 epochs, held, at dev's 0.58 | | 0.58 | 0.751 | 0.759 | 145 | 48 | 46 |

The follow-up head is the better of the two: a sentence that says what the agent will do next has a shape ("I'll", "once it merges", "when both report") the vectors carry.
`extracting::FOLLOWS_UP_AT` is 0.81, the 8-epoch dev folds' point at 0.90.

## The rules around the heads

The heading rule drops three shapes from the close before any sentence is read: a `#` line, a line that is wholly bold, and a line under six words ending in `:` or `?` that stands alone before the next paragraph.
A line a fenced `bash` block follows is kept whatever its shape, since it hands the command over.
Over the 660 replies it dropped 28 of 3,097 closing sentences, every one labelled `none`.
A first cut of the rule read any line starting with `#` as a heading and dropped 88, eleven of them an act or a follow-up that opened with a pull request number such as "#2568 is still held"; a heading is `#`s and a space.

An ask's act is read from its words: `run` with the fenced command when a `bash` block follows the line, `approve_pr` with `#N` when the sentence names `#N` with approve, merge or land, otherwise `answer`.
An `answer` gets options from two shapes: "X, or Y?" yields the alternatives, each trimmed of the question's lead ("Should the registry be read once at start, or on every Studio open?" gives "Once at start" and "On every Studio open"), and a question that closes a list yields the list's lines.
Of the 262 `question` and `act` sentences kept, the rule gives options to none: no labelled ask has the "X, or Y?" shape, and none closes a list of short lines with a word of choosing.
A first cut of the list shape took any list before any ask and gave 23 asks the findings above them as choices, so the shape now needs a choosing word ("which", "pick", "choose", "prefer", "one of", "option") and two to six lines of eight words or fewer.
The rule earns its place on the paraphrases in `crates/fleet/src/tests/extracting.rs`, not on this data.

The rules' own sentence (`asks::asking_line`) is always the first ask where the rules call it, so nothing the asks reader catches by rules today is lost; the asks head's own pick of sentence, where only it called the reply, is not carried over.

## What is in force

Nothing.
`extracting::IN_FORCE` is false, so a reply raises one item as before, the `called` rule still skips it beside a listed item, and `language_model::wanted` does not count the readers.
With it on, every ask is an item `replied:<ended>:<n>` with its act and options, the `called` rule goes, and the follow-ups ride on `Waited::later` for the record.

## What it costs

| | |
|---|---|
| Per reply | The close's sentences, mean 4.7, through both heads as two batches: mean 338 ms, median 245, p95 879, max 2,603, over the 660 labelled replies |
| Conditions | Two rayon threads under `taskpolicy -c utility` while a Fleet test build ran beside it (load average 12 to 18); Fleet reads with every core, so this is an upper figure |
| Memory | 248 MB resident, of a process that holds only the model and adapter |
| Adapter | 204,696 bytes for four heads, 3,424 more than the two |
| Training | 125 to 145 s to cache the 3,097 sentences once, then about 0.2 s per fold and a few seconds for the rest: the heads are 385 weights each |

A read is one more pair of forward passes beside the asks reader's, on the same blocking thread and with the same `Reads` cache by reply text, so a reply is read once however many times its Session is derived.

## What this does not cover

The 30 `question` sentences are read by the same head as the 232 `act` ones, and nothing says which of the two a sentence is beyond the act rules, so a question with no act words is an `answer` with no options.
The asks head's own sentence, where only it called the reply, is not among the extracted asks.
A heading between a list and the question that closes it ends the list, so the question gets no options.
