# Spike: Does a Job's reply turn a Fleet tell down?

**Answer: not yet, on the tells that matter. The `refused` head ranks a refusal of the behind-main notice
above a take seven times in eight, and ranks a refusal of the tells in scope below most of the takes. Out of fold it caught none of the five refusals of those tells, so the reader is built and
not in force.**

Measured on 20 Oct 2026. The code is `crates/fleet/src/session_host/tell_refusal.rs`; the head and
how it was trained are in `docs/spikes/reply-asks-classifier.md`, *The fine-tuned model*.

## What the owner decided

When a Job turns down one of these Fleet tells in prose, it calls him: the Job's row shows under Needs
you with the sentence where it said no, and he decides whether it stands.

| In scope | Sent by |
|---|---|
| Stranded slot | `stranding.rs`, the tell to a stranded slot's last holder |
| Context | `usage_rules.rs` and `usage_watch.rs`, the Context rule's tell |
| Subagent burn | the same, the Subagents rule |
| Slot check | Fleet sends none now. The 6 replies in the data were to a message no current code sends, so nothing records it |

The "N commits behind main" notices are out: a Job refusing one is almost always right, and he did not
pick them.

## Data

215 Fleet tells and the reply that followed each, from the owner's local Claude Code transcripts
between 14 and 20 Oct 2026, labelled by an agent that had seen no reader: `took`, `refused`,
`deferred`, `ignored` or `unsure`, with the sentence that refuses where there is one. Two `unsure`
were dropped, leaving 213. The head was trained on all 213, so the figures below are out of fold.

| Tell | Replies | Refused |
|---|---|---|
| Behind main | 163 | 23 |
| Context | 20 | 5 |
| Subagent burn | 19 | 0 |
| Slot check | 6 | 0 |
| Stranded slot | 5 | 0 |
| **In scope** | **50** | **5** |

The data holds no reply text in the repository, and the tests use paraphrases.

## Method

Stratified five-fold, seed 7: for each fold the `refused` head was trained on the other four, with the
frozen `asks` adapter under it, and scored the fold. It is the trainer that produced the committed
head, run one process at a time under `taskpolicy -c utility`. The head was trained for 30 epochs at 1e-2 on cached vectors. The committed head's own epochs and rate
were not written down; this setting reproduces the spike's out-of-fold AUC on all 213 (0.82), and a
first try at 5 epochs and 1e-3 did not (0.63, the head barely trained), so it is not a looser reading.

Each reply was scored two ways. *As trained* is the whole of what the Job said after the tell, joined.
*As Fleet hands it* is the last 1,500 characters of the last thing it said, which is all Fleet keeps
(`standing::tail`). The head reads the first 256 tokens of either.

## Scores

| Out of fold | AUC as trained | AUC as handed |
|---|---|---|
| All 213 | 0.82 | 0.81 |
| The 50 in scope | 0.52 | 0.43 |

| Threshold | All kinds, caught and wrong | In scope, caught and wrong |
|---|---|---|
| 0.50 | 12, 15 | 0, 1 |
| 0.80 | 7, 7 | 0, 1 |
| 0.90 | 6, 3 | 0, 1 |
| **0.95** | **2, 0** | **0, 0** |

Counts are of replies as Fleet hands them. As trained, the in-scope wrong calls are 3, 3, 2 and 1.

The five refusals in scope are all of the Context tell. As Fleet hands them they score between 0.002
and 0.15, and rank 14, 20, 21, 44 and 45 of 50. The highest score among the 45 takes in scope is 0.93.
The head learned what a refusal of the behind-main notice sounds like, which is 23 of its 28
positives, and the five it needs here are not that. Stranded slot, subagent burn and slot check
have none at all.

**The sentence.** Two of the five have a refusing sentence recorded; the other three were labelled
without one. For both, the sentence is not among the close's last six, the ones scored, so the one
picked was another about something else. A refusal that sits ahead of the close shows the wrong line.

## Threshold and what is in force

`REFUSED_AT` is 0.95: the lowest threshold at which the head, reading replies as Fleet hands them, made
no wrong call on any kind. It catches 2 of 28 refusals and none in scope.

The ship rule was: in force only if, out of fold on the in-scope tells, precision is at least 0.75 with
at least one refusal caught. None was caught, so **`tell_refusal::IN_FORCE` is false**. With it false:

| | |
|---|---|
| Recorded | Fleet keeps each in-scope tell it sends (`session_tells`), so the record is there when the reader is |
| Read | Nothing. No reply is scored, no item is raised, and the reader does not ask Fleet to load the model |
| Built | The item (`refused:<tell sent at>`, drawn as a `replied` item), the first-reply-only rule, and the sentence picking, all tested against a fake model |

## What would move it

The five refusals are too few to train on or to measure with. A precision of 0.75 means little from
fewer than ten calls, and the head needs positives of these tells to learn what they sound like.
About 25 more refusals of the three in-scope tells, 30 in all, would let the head be trained on them
and scored out of fold with ten or more caught. At the rate in the data (5 in 50 in six days, all of
one tell), that is on the order of 250 more replies, and the stranded-slot, subagent-burn and
slot-check tells would need to produce some. The sentence needs the whole reply scored rather than its
close, which is a change to what Fleet keeps (`session_turn_end` holds the last 1,500 characters).

A reader that read only these tells' replies is not tried here: 50 replies are too few to hold out.
