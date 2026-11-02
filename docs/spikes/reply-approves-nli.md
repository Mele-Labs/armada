# Spike: Can a free local zero-shot NLI model read whether his reply approves?

**Answer: no, not as a replacement for the Claude call.**
The best of three models and three wordings ranks the two real approvals first and second of 179 replies, and only when the threshold is picked after looking at the scores.
Every threshold fixed in advance either calls nothing or calls many replies that do not approve, and the three short approvals ("Yes", "C", "Looks good") score near zero or are buried under non-approvals.
Five approvals in 179 replies cannot show a recall, so this is not evidence that any model would catch them.

Measured on 1 Nov 2026.
The runner is `tools/nli-spike`, outside the workspace like `tools/lm-train`.
Nothing in `crates/` changed and nothing is in force.

## Why it was asked

Fleet reads his reply to a page with one cheap Claude call, `question()` in `crates/fleet/src/session_host/page_replies.rs`.
A trained head on Fleet's own encoder was tried first and failed for lack of positives: see [`reply-approves-reader.md`](reply-approves-reader.md).
A zero-shot NLI model needs no positives to train, so the question is whether it clears the same bar: precision 0.90 or more with at least one approval caught.

## What was run

Same 179 labelled replies as the reader spike, 5 of them approvals.
Nothing is trained, so all 179 are scored, and the fresh part (36 replies, 1 approval) is reported beside them.
The premise is the reply, cut to its first 128 word pieces; the hypothesis is one of three wordings.

| | Hypothesis, fixed before any score was read |
|---|---|
| H1 | The person approves the plan and tells the agent to go ahead. |
| H2 | The person says to proceed with what was proposed. |
| H3 | The person is satisfied with this and wants it to continue as it is. |

The score is P(entailment).
For the three-label model the entailment-against-contradiction score (`entailment / (entailment + contradiction)`) is also shown, since that is how a zero-shot pipeline is often read.

| Model (pinned revision) | Architecture, tokenizer | File | Per reply on CPU |
|---|---|---|---|
| `MoritzLaurer/xtremedistil-l6-h256-zeroshot-v1.1-all-33` (`c07f66d9cbf781191bee66edfe8ad7856f045781`) | BERT, 6 layers, hidden 256; WordPiece with the same `vocab.txt` as the committed encoder, read by the repo's own `wordpiece.rs` | 25.5 MB, half precision | 8 to 13 ms median |
| `MoritzLaurer/deberta-v3-base-zeroshot-v2.0` (`8e7e5af5983a0ddb1a5b45a38b129ab69e2258e8`) | DeBERTa-v3 base, two labels; SentencePiece through the `tokenizers` crate | 369 MB, half precision | 205 ms median, 360 ms at the 95th percentile |
| `cross-encoder/nli-deberta-v3-small` (`fa2804872c3b4bd748f38c0185cc85775361e735`) | DeBERTa-v3 small, three labels (contradiction, entailment, neutral); same tokenizer | 568 MB, full precision | 119 ms median, 228 ms at the 95th percentile |

Timings are one pair at a time, 4 threads, `taskpolicy -c utility` on a Mac doing other work, so they are noisy: the first model measured 8 and 13 ms on two runs.
Label order was read from each `config.json` (`entailment`, `not_entailment` for the first two; `contradiction`, `entailment`, `neutral` for the third) and the plumbing was checked on plain pairs before any reply was scored.
"A man is playing a guitar on stage" entails "A person is playing an instrument" at 0.94 to 1.00 on all three, and "Nobody is playing music" at 0.00.

**Not run: `cross-encoder/nli-MiniLM2-L6-H768`.**
It is a RoBERTa model with a byte-level BPE vocabulary, not a BERT with WordPiece, so the repo's tokenizer cannot read it.
**DeBERTa runs in candle 0.9 with no C library**: `candle-transformers` has the model and `tokenizers` builds with `fancy-regex` instead of `onig`.
That tokenizer crate is what `wordpiece.rs` was written to avoid, so adopting either DeBERTa model would bring it into the workspace.

Weights and vocabularies are fetched by hand into `.armada/models/<name>@<revision>/` (ignored by git), and the runner refuses any file whose SHA-256 is not the pinned one, as `language_model.rs` does.

## Results

AUC is over the replies named; "rank" is the position of the motivating reply among 179, 1 being highest.

| Model, score | H | AUC all (179) | AUC fresh (1 approval) | Motivating reply: score, rank |
|---|---|---|---|---|
| xtremedistil, P(entailment) | H1 | 0.589 | 0.943 | 0.054, 7 |
| | H2 | 0.332 | 1.000 | 0.863, 1 |
| | H3 | 0.708 | 0.657 | 0.006, 57 |
| deberta-v3-base-zeroshot, P(entailment) | H1 | 0.797 | 0.829 | 0.001, 33 |
| | H2 | 0.833 | 1.000 | 0.993, 2 |
| | H3 | 0.823 | 0.571 | 0.0002, 83 |
| nli-deberta-v3-small, P(entailment) | H1 | 0.856 | 0.743 | 0.0003, 53 |
| | H2 | 0.741 | 1.000 | 0.980, 1 |
| | H3 | 0.853 | 0.714 | 0.0007, 52 |
| nli-deberta-v3-small, entailment against contradiction | H1 | 0.895 | 0.543 | 0.0003, 84 |
| | H2 | 0.963 | 1.000 | 0.993, 2 |
| | H3 | 0.907 | 0.486 | 0.039, 69 |

**The motivating reply, "Run the plan and merge it. I'm AFK", is caught by H2 and by nothing else.**
Under H1 and H3 it scores near zero on every model, and under H2 it scores 0.86 to 0.99.
Under H2 the fresh AUC is 1.000 for every model, which means the one fresh approval ranks first among 36, and says nothing more about how well it generalises.

### The bar, at thresholds

H2 is the wording that reaches the bar at all.
The best result of any run is the nli-deberta-v3-small H2 entailment-against-contradiction score, shown here with the zero-shot base model's H2 beside it.

| Threshold | nli-small H2, called / right | zeroshot-base H2, called / right |
|---|---|---|
| 0.50 | 29 / 5 | 7 / 2 |
| 0.70 | 15 / 3 | 6 / 2 |
| 0.90 | 11 / 3 | 3 / 2 |
| 0.95 | 8 / 3 | 3 / 2 |
| 0.99 | 2 / 2 | 2 / 2 |

At 0.50 the three-label model calls 29 replies and 5 are approvals, a precision of 0.17 at a recall of 5 of 5.
Precision of 0.90 or more with an approval caught exists only at 0.993 (nli-small) and 0.993 (zeroshot-base): two replies called, both approvals.
**Those two thresholds were read off these same 179 replies after scoring them.**
No part was held out and nothing was chosen out of fold: each threshold is the score of the second approval, the highest value that still catches two.
The reader spike chose its threshold before scoring the fresh part; this one could not.

### Selection

The table above shows every row that was run; none was left out.
H2 and the entailment-against-contradiction reading are the best-looking ones, and they were picked by looking at the scores, not in advance.
Most rows have an AUC under 0.9, and the xtremedistil H2 row ranks the motivating reply first while ranking the four training approvals below chance (train AUC 0.169).
A result that high in one wording and that low in another is a sign of how little 5 positives fixes.

## The misses

The replies scored highest without approving, under H2, are the ones a threshold has to sit above.

| Score | Reply | Why it is not an approval |
|---|---|---|
| 0.966 to 0.982 | "I'm AFK so I want you to do a first pass of the video. Make it realistic like a demo but also flashy with cool armada themed cuts/transitions." | A new task, not an answer to a page |
| 0.986 | "> A smaller card that sits over the map without moving the camera. A pinned drone that stops writing hands the card to whoever is writing. If I am hovering in the panel, it would be cool if we weren't restricted to the panel area..." | A quoted proposal and a change |
| 0.974 | "1. What happens when there is also a plan? 2. I think we can make this look/feel more like the battlefield/drone/cockpit theme..." | A question and a change |
| 0.971 | "When did I approve the walk?" | A question |
| 0.884 | "For the stuff that fleet doesn't send. lets do this in phases. 1. Deliver the updated panel with what fleet sends. Land that 2. Build fleet to support the other stuff. Land that..." | A plan of his own, to do after the change |

The first reply shares "I'm AFK" with the motivating one and scores almost as high, so no threshold separates them.
Every reply above is about something other than the page, and a model reading one reply alone has no way to tell.

The approvals that were missed:

| Reply | H2 score (zeroshot-base, nli-small against contradiction) | Rank |
|---|---|---|
| "Yes" | 0.011, 0.958 | 41, 7 |
| "C" | 0.006, 0.661 | 50, 21 |
| "Looks good" | 0.003, 0.691 | 66, 16 |

Each depends on what was put to him: "Yes" answers a question, "C" picks an option, "Looks good" refers to the page.
The Claude call is given the page title and the person's next messages in order, and it is told what approving means.
Here the premise is the reply alone, so an NLI model has nothing to entail "go ahead" from.
Giving the model the title or the last assistant turn as part of the premise was not tried.

## What it costs

| | Claude call | xtremedistil | DeBERTa base or small |
|---|---|---|---|
| Per reply | One cheap model call | About 10 ms | 120 to 205 ms |
| Download | None | 26 MB | 369 to 568 MB |
| Code in the workspace | None | Candle BERT plus a pooler and classifier, with the repo's `wordpiece.rs` | Candle DeBERTa and the `tokenizers` crate |
| Reads the page and earlier turns | Yes | Not as run | Not as run |

The cost of a wrong approval is a page settled that he never approved, and the cost of a miss is a page that stays asking as it does today.
A local model that misses three of five and sets off on a new task as an approval is worse than the call, for a saving that is one cheap call per reply.

## Not in force

Fleet keeps reading his reply with `models.dispute`.
This changes nothing in `crates/`.

## What would change the answer

- **More approvals.** Five cannot establish recall or a threshold; the reader spike lists the ways to get more.
- **The page in the premise.** Putting the title, or the assistant's last turn, in front of the reply may rescue "Yes" and "C"; it is the one change that could move the short approvals.
- **A threshold chosen out of fold**, once there are enough positives to hold some out.

## What this does not cover

- **Fine-tuning an NLI model** on the labelled replies; that is a trained head again and has the same shortage of positives.
- **Other hypothesis wordings.** Three were tried and all three are shown.
- **One labeller** (an agent) and one owner on one machine, as in the reader spike.
- **Speed on the machine Fleet runs on.** The timings are from a loaded Mac with 4 threads.

## Reproducing

```bash
cd tools/nli-spike && cargo build --release
RAYON_NUM_THREADS=4 taskpolicy -c utility target/release/nli-spike bert <folder> <out>
```

The first argument is `bert` (xtremedistil), `deberta-zeroshot` or `deberta-nli-small`; `<folder>` holds the pinned files named in `src/main.rs`; `<out>` receives one score file per wording.
