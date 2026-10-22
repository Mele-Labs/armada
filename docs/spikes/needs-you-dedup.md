# Spike: Are two items on a Job's Needs you list one ask, by the local language model's cosine?

**Answer: the base vectors fold a few repeats with no wrong merge at a high threshold, and miss most.**
At the threshold the development half chose, 0.845, the held-out half folded 6 of 44 pairs that were one ask and merged 1 of 106 that were not.
Below it the two kinds mix, so the fold is built, measured and shipped off (`dedup::IN_FORCE`); the owner picks whether 6 in 44 is worth a read on every list.

Measured on 21 Oct 2026.
The code is `crates/fleet/src/session_host/dedup.rs`; what it folds is `docs/concepts/session/waiting-on-the-person.md`.

## Why it was asked

The owner's Needs you list for one Job showed the same ask more than once.
An agent lists an item with `waiting_for`, its reply's close asks the same thing, and Fleet raises `replied:<turn ended>` for it; or an agent item restates a walk or the plan Fleet already lists in its own words.
Fleet drops an agent item that names the same act and target as its own, and the walk rules and the plan reader (`walk-item-classifier.md`) catch two shapes of restatement.
The rest is worded a dozen ways, so the question was whether the model's vectors tell one ask from two.

## What was labelled

300 pairs of items from the owner's local Fleet, each pair from one Job at one moment, outside the repository.
Each side is an agent item (`agent`), the sentence of a reply that asks (`reply`) or Fleet's own item text (`fleet`).
One labeller marked each pair `same` where both sides ask the person for one thing, and `different` otherwise.
89 are `same`.
The pairs were halved before any threshold was read.

| Set | Pairs | Same | Used for |
|---|---|---|---|
| Dev | 150 | 45 | Choosing the threshold |
| Held out | 150 | 44 | The one measured run |

| Bucket | Dev pairs | Dev same | Held pairs | Held same |
|---|---|---|---|---|
| agent / agent | 27 | 1 | 33 | 0 |
| agent / fleet | 30 | 2 | 30 | 3 |
| agent / reply | 61 | 32 | 59 | 35 |
| reply / fleet | 32 | 10 | 28 | 6 |

## The reader

Each side's text is embedded with the base `bge-small-en-v1.5` vectors, the same `embed` Fleet's `TextModel` gives, and the pair's score is the dot product of the two unit vectors.
No id is folded into the text, since the pairs carry none; `plan_asks` reads `<id in words>: <text>`.
`tools/lm-train pairs` writes the scores.

## The threshold rule

The threshold is the lowest cosine at which no `different` pair of the development half scores at or above it: no two different asks are merged.
The closest different pair on dev scored 0.8445, so the threshold is **0.845**.
Dev recall at it is under 0.3, so the rule the brief named for that case was also read: the threshold at precision 0.95 on dev is 0.860, which catches the same 11 pairs, since nothing lies between 0.8445 and 0.8598.
Nothing was tuned on the held-out half.

## Dev half at 0.845

| Bucket | Pairs | Same | Caught | Different merged | Precision | Recall | AUC |
|---|---|---|---|---|---|---|---|
| all | 150 | 45 | 11 | 0 | 1.000 | 0.244 | 0.676 |
| agent / agent | 27 | 1 | 0 | 0 | — | 0.000 | 0.769 |
| agent / fleet | 30 | 2 | 1 | 0 | 1.000 | 0.500 | 0.839 |
| agent / reply | 61 | 32 | 9 | 0 | 1.000 | 0.281 | 0.791 |
| reply / fleet | 32 | 10 | 1 | 0 | 1.000 | 0.100 | 0.418 |

## Held-out half at 0.845, scored once

| Bucket | Pairs | Same | Caught | Different merged | Precision | Recall | AUC |
|---|---|---|---|---|---|---|---|
| all | 150 | 44 | 6 | 1 | 0.857 | 0.136 | 0.641 |
| agent / agent | 33 | 0 | 0 | 1 | 0.000 | — | — |
| agent / fleet | 30 | 3 | 1 | 0 | 1.000 | 0.333 | 0.716 |
| agent / reply | 59 | 35 | 4 | 0 | 1.000 | 0.114 | 0.735 |
| reply / fleet | 28 | 6 | 1 | 0 | 1.000 | 0.167 | 0.417 |

At 0.860 the held-out half caught 4 of 44 and merged none.
The one wrong merge is two agent items of one Job that ask two things in near the same words.

## What the buckets say

The reply / fleet AUC is under 0.5 on both halves: a reply's sentence and Fleet's own text ("Walk <name>", "Plan to approve") share no words, and the vectors measure topic, so a restatement of Fleet's item scores no higher than a different ask about the same page.
The agent / reply bucket carries most of the `same` pairs and most of what is caught: an agent that lists an item and then asks the same thing in prose uses near the same words.
The plan reader's own threshold, 0.712 against the fixed anchor "Plan to approve", is well under 0.845, so folding it into this reader would lose the plans it catches; `plan_asks` stays its own reader.

## What is in force

`dedup::IN_FORCE` is `true` and `FOLDS_AT` is 0.845.
It went in force on 22 Oct 2026 by the owner's pick.
`waiting_from` folds the list on every read once the model is loaded: Fleet's own item stands over the agent's over the reply's sentence, an item with an act over one without, then the earlier, and an item not embedded yet is kept as is until the batch is in and the Session is published again.
`plan_asks` is unchanged.

## What it costs

`lm-train pairs` embedding two texts in one batch, release build, with a `cargo check` of the workspace running beside it: median 119 ms a pair, mean 238 ms, slowest 2.8 s.
In Fleet each text is embedded once and kept by its hash, so a list is embedded on its first read and on a new item only, on a blocking thread off the read.

## What it does not cover

- **A restatement in other words**, which is most of them: the base vectors put 34 of 45 development `same` pairs under the threshold, among the different pairs.
- **A reply against Fleet's own text**, where the vectors read worse than chance.
- **A pair head**: a small model trained on these pairs was not built in this pass; the labelled set is 300 pairs from one machine, and a head would be measured here before it reads anything.
- **Labels from one labeller**, as in `reply-asks-classifier.md`.
- **The agent's item beside its own reply's sentence**: today an agent item stops the `replied:` item being raised at all (`called` in `waiting.rs`), so that pair, 120 of the 300, does not reach the list together; the figures stand for when it does.
