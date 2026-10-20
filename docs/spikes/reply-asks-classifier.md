# Spike: Does a reply ask the person for something?

**Answer: yes, from rules alone. On replies the rules had never seen, precision was 0.96 and
recall 0.68, or 0.87 once the calls that depend on Armada's walk loop are set aside.** After the
rules were tuned on all 457 labelled replies, they read 0.97 and 0.89. A wrong call costs one
dismiss, so the rules lean toward calling; the misses are listed by kind below.

Measured on 17 Oct 2026. The code is `crates/fleet/src/session_host/asks.rs`; what it feeds is
`docs/concepts/session.md`, *What a Job is doing*.

**19 Oct 2026: a local language model was built and measured beside the rules, and nothing reached
0.90 precision on 300 fresh replies.** The rules read 0.72 and 0.26, the model alone 0.67 and 0.07,
and either one calling 0.69 and 0.33. The rules stay in force; see *The local model* below.

**20 Oct 2026: the model was fine-tuned, and still nothing reached 0.90 precision on 300 more fresh
replies.** The fine-tuned model alone read 0.77 precision and 0.58 recall, against the rules' 0.72
and 0.26, and either one calling read 0.73 and 0.67. The rules stay in force and the model stays
unfetched; see *The fine-tuned model* below.

## What was labelled

The last assistant message before the next human prompt, from the owner's local Claude Code
transcripts. Subagent lines and tool results were skipped, as were turns that ended on a tool.
One labeller, the agent that built the rules, marked each `asks` (the person must answer, decide
or do something) or `tells` (it only reports). A polite "let me know if" closer with nothing
pending is `tells`.

The text and labels stay outside the repository. Machine output (retro JSON, verdicts) was
dropped from every count, since no person reads it.

| Set | Replies | Drawn | Used for |
|---|---|---|---|
| A, dev | 189 | 200 random, 100 with a cue, split 200 and 100 | Writing the rules |
| A, test | 91 | The same draw | First held-out run |
| B | 98 | 60 random, 40 with a cue | Second held-out run |
| C | 79 | 45 random, 35 with a cue | Third held-out run |

"A cue" is a `?` or a phrase like "want me", "which", "approve" in the last 500 characters. In the
random draws 30% of replies ask (86 of 287).

## Held-out runs

Each row is the first run of the rules as they stood on a set nobody had tuned them against.
"Strict" leaves out the replies marked hard: 81 of the 184 asks, where what is owed is a walk, an
approval or a command rather than a question.

| Set | Rules | Precision | Recall | Strict recall |
|---|---|---|---|---|
| A, test | First pass, tuned on A dev | 0.909 | 0.606 | 0.636 |
| B | Second pass | 0.943 | 0.673 | 0.759 |
| C | Third pass | 0.963 | 0.684 | 0.867 |

## Final rules, all 457 replies

These were tuned on every set, so they are not held out.

| | Asks | Tells |
|---|---|---|
| Called | 164 | 6 |
| Not called | 20 | 267 |

Precision 0.965, recall 0.891. Strict: precision 0.990, recall 0.942. Random draws only:
precision 0.951, recall 0.907.

## The misses

| Kind | Count | What the reply said |
|---|---|---|
| A walk or an approval, as what comes next | 5 | "It lands only after you approve the walk" |
| Steps for him with no cue word | 4 | A numbered list that starts "Open Settings and set" |
| Ask inside a table cell or findings list | 4 | "needs your merge" in a status table |
| An offer to reframe | 4 | "If the decision is another one, say so and I will drop this" |
| Ask more than two paragraphs from the end | 3 | A question, then a status section |

The six wrong calls are four optional tips ("Press ⌘R if the tab is stale"), one relayed
instruction and one "say which" addressed to another agent.

## The rules

The close is the last two paragraphs that are words (not code, a table row or a link), widened to
the lines that introduce a list. In it, the last sentence that matches wins.

| Signal | Example |
|---|---|
| Ends in `?`, unless wholly quoted | "Want me to land it?" |
| A phrase in `ASKING` | "should I", "which one", "your call", "once you approve" |
| An instruction at the start of a clause | "Press Allow once", "Next: merge #12" |
| A fenced `bash` block after a line | The line that hands it over |
| "I can X, or Y" | "I can trace it now, or file it" |

Three things stop a match: a sentence that says nothing is owed ("nothing is waiting"), an offer
left open ("if you want"), and a status said mid-sentence ("is still waiting for you").

## The local model, 19 Oct 2026

The owner decided that Fleet reads agents' messages with a small local model. The model is a
sentence embedder run in pure Rust on the CPU, shared by every reader. The asks reader is a
logistic regression over its vectors. Code: `crates/adapters/src/language_model.rs` (fetch and
load), `crates/fleet/src/language_model.rs` (loaded once, in the background after start),
`crates/fleet/src/session_host/asks_model.rs` (the reader). On 20 Oct the fine-tuned model below
replaced this head and its weights.

| | |
|---|---|
| Model | `BAAI/bge-small-en-v1.5` at revision `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, 384 dimensions, 133 MB |
| Where it is kept | `~/Library/Application Support/Armada/models/`, fetched once and checked against pinned SHA-256s |
| Head input | The mean of the vectors of the close's last three sentences (the close the rules read) |
| Head | L2-regularised logistic regression, λ = 0.1 on standardised vectors, trained on all 457 replies |
| Threshold | 0.91 for the model alone, 0.85 beside the rules |
| Sentence shown | The rules' sentence where they call it, otherwise the closing sentence the head scores highest |

The trainer ran outside the repository against the committed encoder and close-splitting code, and
only the 385 weights were committed.

### Choosing the model

Each figure is out of set: a head trained on two of A, B and C reads the third. AUC is over all 457.
Times are per reply, up to twelve closing sentences, on an Apple M5 Pro while other agents ran
(load average about 5).

| Model | Head input | AUC | Mean ms |
|---|---|---|---|
| `all-MiniLM-L6-v2`, mean pooling | Whole close | 0.65 | — |
| `all-MiniLM-L6-v2`, mean pooling | Last three sentences | 0.71 | — |
| `bge-small-en-v1.5`, first-token pooling | Whole close | 0.62 | — |
| `bge-small-en-v1.5`, first-token pooling | Last three sentences | **0.74** | 108 |

The vectors say what a sentence is about far more than whether it asks: "Want me to land it?" and
"Should I merge the pull request?" sit at a cosine of 0.19 in MiniLM. bge read the act a little
better, at twice MiniLM's layers. The thresholds are the most recall at which the out-of-set heads
kept precision at 0.90:

| Reader, out of set | Precision | Recall |
|---|---|---|
| Model alone, 0.91 | 0.902 | 0.201 |
| Either one calling, 0.85 | 0.904 | 0.918 |

The rules' own row is not out of set: they were tuned on all three sets.

### The fresh set

300 replies, every turn end since 18 Oct where the agent did not ask through a tool in that turn,
labelled by an agent that had seen neither reader. 81 ask and 219 tell. 80 of the 81 asks are hard:
what is owed is a walk, an approval or a restart rather than a question. That is the mix now that
plain questions go through tools. Strict recall would rest on one reply, so it is not given. Each
reader was scored once, after training was final, and nothing was tuned on this set.

| Reader | Precision | Recall | Called right | Called wrong | Missed |
|---|---|---|---|---|---|
| Rules | 0.724 | 0.259 | 21 | 8 | 60 |
| Model alone | 0.667 | 0.074 | 6 | 3 | 75 |
| Either one calling | 0.692 | 0.333 | 27 | 12 | 54 |

**The rules stay in force** (`asks_model::SHIPPED`). The ship rule was more recall at precision
0.90 or above, and no reader reached 0.90. Either one calling finds six more asks for four more
wrong calls; that trade is the owner's. With the rules in force Fleet fetches and loads nothing
(`language_model::wanted`), and putting another reader in force is that one constant.

### What it costs

| | |
|---|---|
| First start | 133 MB fetched in the background; 2.5 s here, including the load |
| Later starts | 15 to 30 ms to load from disk, with the file in the page cache |
| Per reply | Mean 108 ms, median 91, p95 229, max 404, over the 457 labelled replies |
| Memory | 200 to 290 MB resident, of a process that holds only the model |
| Build | `candle-core`, `candle-nn` and `candle-transformers` 0.9 with what they bring: 62 crates, all Rust, 16 s of CPU cold (42 s wall at six jobs on a busy machine) |

A reply is read once, on a blocking thread. The derive that first sees it answers by the rules, and
the Session is published again when the read is in.

## The fine-tuned model, 20 Oct 2026

The owner chose to fine-tune after the head above. The base model is the same pinned
`bge-small-en-v1.5`, and its 133 MB stay where they are fetched. The repository holds only what was
trained on top of it, in `crates/adapters/src/language_model/tuned.safetensors`.

| | |
|---|---|
| Adapter | LoRA, rank 8 and alpha 16, on the query and value projections of the top 4 of 12 layers |
| Heads | One linear layer and a sigmoid per reading, on the first token's vector: `asks`, and `refused` |
| Size | 201,272 bytes, adapter and both heads |
| Encoder | Fleet's own BERT forward pass in `language_model/bert.rs`, so the adapter can sit inside it. It gives the same base vectors as `candle-transformers` to five decimals, and that crate is gone from the build |
| Asks input | The close the rules read, as one text: its last 128 tokens |
| Sentence shown | The rules' sentence where they call it, otherwise the one of the close's last six sentences the head scores highest |
| Threshold | 0.84, alone or beside the rules |

**Trained in Rust with candle, not Python.** This machine's Python has neither numpy nor torch.
Training through the same encoder that Fleet runs also means there is no export step to drift
from. The trainer lives outside the repository and includes the committed encoder and
close-splitting code by path. It ran one process at a time under `taskpolicy -c utility`. For both
heads, the trainer's probabilities and the committed reader's agree to six decimals.

### Training

The frozen lower eight layers were run once per reply and cached. Only the top four layers, their
adapters and the heads were trained, using AdamW at 1e-3 with weight decay 0.01, a batch of 16, five
epochs, and a weighted cross-entropy.

| Reading | Replies | Positive | Label |
|---|---|---|---|
| `asks` | The 457 above, weight 1, and 360 turn ends since 18 Oct, weight 2.5 | 184 and 88 | The reply asks the person |
| `refused` | 213 agent replies to a Fleet notice, two marked unsure dropped | 28 | The agent turned the notice down; took, deferred and ignored are negative |

The later replies carry more weight because they are today's mix: plain questions now go through
tools, so what is left to read is mostly a walk, an approval or a restart. Every number below is
from stratified five-fold cross-validation, with each reply read by a model that had not trained on
it.

| `asks`, out of fold | AUC | Precision | Recall |
|---|---|---|---|
| All 817 replies | 0.833 | | |
| The 360 since 18 Oct, at 0.5 | 0.898 | 0.682 | 0.682 |
| The 360 since 18 Oct, at 0.84 | | 0.900 | 0.511 |
| The rules, on the same 360 | | 0.821 | 0.261 |

The rules are already below 0.90 on those replies, so there is no room to run the model looser
beside them than alone. One threshold serves both.

**The `refused` head is trained and measured only.** A reader of it is built and not in force
(`docs/spikes/tell-refusal-reader.md`, 20 Oct), which scores it on the tells it was wanted for. Three ways of training it scored the same within the noise of 28 positives:

| `refused`, out of fold | AUC | Precision, recall at 0.5 | Recall at precision 0.90 |
|---|---|---|---|
| Head on the base model | 0.84 | 0.43, 0.43 | 0.04 |
| Head on the `asks` adapter | 0.82 | 0.46, 0.43 | 0.04 to 0.07 |
| Its own adapter | 0.81 | 0.41, 0.50 | 0.04 |

**One adapter with two heads was committed**, the simplest of the three. As a read, it is weak:
it ranks a refusal above a take about four times in five, but it cannot call one with confidence.

### The fresh set

The new set has 300 turn ends, drawn after the training replies and labelled by an agent that had
seen no reader: 81 ask and 219 tell. Each reader was scored once, after training was final, and
nothing was tuned on this set. Strict recall would rest on one reply, so it is not given.

| Reader | Precision | Recall | Called right | Called wrong | Missed |
|---|---|---|---|---|---|
| Rules | 0.724 | 0.259 | 21 | 8 | 60 |
| Fine-tuned model alone | 0.770 | 0.580 | 47 | 14 | 34 |
| Either one calling | 0.730 | 0.667 | 54 | 20 | 27 |

In 29 of these replies, nothing was waiting on the person when the reply was written: there were
no listed items, no walk shown and no plan recorded. Six of those 29 ask, and an ask that is missed
there reaches no one.

| Reader, nothing listed before | Precision | Recall | Called right | Called wrong | Missed |
|---|---|---|---|---|---|
| Rules | 0.667 | 0.333 | 2 | 1 | 4 |
| Fine-tuned model alone | 0.750 | 0.500 | 3 | 1 | 3 |
| Either one calling | 0.833 | 0.833 | 5 | 1 | 1 |

**The rules stay in force.** No reader reached 0.90, and with the rules in force Fleet neither
fetches nor loads the model. The model alone more than doubles the rules' recall at higher
precision. Either one calling finds 33 more asks than the rules, for 12 more wrong calls. Whether
that is worth putting in force is the owner's call, and doing so means changing `asks_model::SHIPPED`.

### What it costs

| | |
|---|---|
| Per reply | The close and six sentences in one batch: mean 200 ms, median 180, p95 365, max 562 |
| Memory | 252 to 264 MB resident, of a process that holds only the model and adapter |
| Load | The base as before, plus the 201 KB adapter compiled into the binary |
| Training | 1 to 2 min to cache the lower layers, then 3 to 10 min per `asks` fold and about 5 min for the final run |

## What this does not cover

A terminal Job's mod used to report only the first line of its last message, so the rules saw too
little of one to call it (#2534). It now sends the close of the last message, the same last 1,500
characters from a line start that Fleet keeps for a hosted Job, and the rules read it as they read a
hosted one. A mod older than 0.3.39 still sends the first line, and its asks are missed as before.
