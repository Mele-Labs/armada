# Spike: Does the person's reply approve what the Job put to him?

**Answer: not yet. A head is trained and measured, and there are too few approvals to trust it or to put it in force.**
The training part holds 4 replies that approve, and the fresh part holds 1.
The target was at least 25.
On the fresh part the head ranked the one approval first of 36 replies and scored it 0.059, so at the threshold chosen out of fold (0.50) it calls nothing: precision is undefined (no call), recall 0 of 1.

Measured on 31 Oct 2026.
The trainer is `tools/lm-train` (`approves-cv`, `approves-final`, `approves-fresh`).
The reading is `TextReading::ReplyApproves`, head `approves`, in `crates/adapters/src/language_model.rs`.

## Why it was asked

A Job put a design page and a plan to the owner.
He replied in chat "Run the plan and merge it. I'm AFK", and Fleet kept the page asking.
Fleet would read his chat replies and settle a page as approved above a threshold.
A false approval settles a page he never approved, and a miss leaves the page asking as it does today, so the threshold leans toward precision.

## What was labelled

The first thing the person typed after an assistant turn that put something to him: a `show_window`, a `record_plan`, a `show_sketch`, a `show_file` of a `.armada/documents/*.html` page, or a write of one.
The source is the owner's local Claude Code transcripts (1,001 of 18,144 files carry a put).
The text and labels stay outside the repository, under `.armada/training/reply-approves/` of the slot that ran this, which is ignored.

| Step | Rows |
|---|---|
| First typed message after a put | 3,399 |
| After removing what is not his typing, and duplicates | 327 |
| After removing annotation notes and 13 notices | 179 |

What was removed as not his typing: tool results, task notifications, Fleet's notices (merged, auto-merge, conflicts, slot behind, linked Job, context), and Bridge's button messages (`Approved: <walk address>`, `Approved A, <take>`, `Approved sketch:`, `Sent back`, `The owner approved plan`, `Re:` answers).
**Annotation notes were removed too**: a note left on one element of a walk, sketch or document (`On <Component> ← …`, `Pointed at:`, `1. Sketch "…"`, `1. Document "…"`).
They are his typing, but they ask for a change to a named element and are never a chat reply, so they would only have padded the `not` side with easy cases.
None of the element notes whose first line sounded approving (love, good, great, looks) was an approval.
Thirteen more were not his typing: a shell echo, a task result, Fleet's task and cache notices, a peer's message and Bridge's command echoes.

One labeller, the agent that built this reader, marked each reply `approves` (he is fine going on from what was put to him as it stands) or `not` (a question, a change asked first, a take-back, a complaint, or something unrelated), with a one-phrase reason.
The fresh part was labelled in a second pass and was not scored until the threshold was fixed.

| Part | Replies | Approves | Not | Dates |
|---|---|---|---|---|
| Training | 143 | 4 | 139 | 8 Oct to 30 Oct |
| Fresh (newest 20% by timestamp) | 36 | 1 | 35 | 30 Oct to 31 Oct |

By what was put: training 110 walks, 22 plans, 11 sketches; fresh 27 walks, 5 plans, 3 sketches and 1 document write.
Median reply 94 characters; 3 of 143 are longer than 500, which is about where 128 word pieces end, so the 128 pieces stay as set.

The four approvals in training:

| Reply | Reason |
|---|---|
| "Lets make this happen. I love it so far. We can keep iterating after it lands in the app" | Goes on, with more to do after |
| "Looks good" | To the summary and walk |
| "Yes" | To a recorded plan |
| "C" | Picks the option the agent suggested |

The one in fresh is the motivating reply, "Run the plan and merge it. I'm AFK".

**Why so few.**
Most approvals reach Fleet as a Bridge button (`Approved: …`), which is a message Fleet itself writes, so it is not a reading of his words.
What he types after a walk or a plan is mostly a change, a question or a complaint: 139 of 143.

## The head

A linear head and a sigmoid on the first token's vector, trained on the frozen committed encoder, so the adapter and every earlier head stay as they were.
Input: the reply from its start, 128 pieces (`Keep::Start`).
Trained by AdamW at 5e-3 for 30 epochs, a batch of 16, the positive class weighted 10.
Three settings were tried on the out-of-fold figures (lr 1e-3 for 10 epochs, then the two below); with 4 positives that choice is itself noise.

| Setting | Out-of-fold AUC |
|---|---|
| lr 1e-3, 10 epochs, weight 5 | 0.872 |
| lr 5e-3, 30 epochs, weight 10 (used) | 0.946 |
| lr 1e-2, 30 epochs, weight 30 | 0.939 |

## Out of fold, training part

Stratified four-fold cross-validation, so each fold held out one approval.
AUC 0.946.

| Threshold | Replies called | Right | Precision | Recall |
|---|---|---|---|---|
| 0.20 | 6 | 3 | 0.50 | 0.75 |
| 0.30 | 3 | 1 | 0.33 | 0.25 |
| 0.40 | 1 | 1 | 1.00 | 0.25 |
| 0.50 | 1 | 1 | 1.00 | 0.25 |
| 0.80 | 1 | 1 | 1.00 | 0.25 |

Precision 1.00 rests on one reply ("Yes", 0.81).
Out of fold, "Looks good" read 0.24 and "C" 0.22, and the two highest `not` replies were "I see the sketch. Very cool" (0.38) and "Still there" (0.35).
The head has learned that an approval is short and warm, which is not the same as telling the Job to go on.

## The threshold

**0.50**, chosen from the out-of-fold table before the fresh part was scored.
It sits above the highest `not` reply out of fold (0.38) and below the one right call (0.81).
It favours precision as asked, and it is the middle of a flat stretch that rests on a single reply, so no other value is better supported.

## The fresh set, scored once at 0.50

| | |
|---|---|
| Replies | 36, of which 1 approves |
| AUC | 1.000 (the approval ranks first) |
| At 0.50 | 0 called: precision undefined, recall 0 of 1 |
| The approval's score | 0.059 ("Run the plan and merge it. I'm AFK") |
| Next highest | 0.039 ("When I get back I expect to see a few walks…") |

The head puts the motivating reply on top and is not near confident enough to call it.
Nothing was tuned after this reading.
Scoring the one approval at 0.059 against the next reply at 0.039 is not evidence that the ranking would hold on more.

## What it costs

The head adds 384 weights and a bias, and the file grows from 206,396 to 208,096 bytes.
The reader runs the same encoder as the other heads.
Training took a minute per run here, since the encoder layers are cached once and only the head trains.

## Checks that the rest did not move

- All 26 tensors of the committed file (16 adapter and 10 head tensors) are bit-identical in the new file; only `head.approves.weight` and `head.approves.bias` are new.
- The existing digest test in `crates/adapters/src/tests/tuned.rs` still passes unchanged for the adapter and the four earlier heads.
- The trainer's probabilities and the committed reader's agree to the seventh decimal on every training approval and a sample of the rest.
- The other five heads read "Both fixes are pushed. Want me to land them?" as before, to six decimals.

## Not in force

Do not settle a page on this head.
Fleet reads his reply to a page with a `models.dispute` call instead (`crates/fleet/src/session_host/page_replies.rs`), the owner's choice of 1 Nov 2026.
Nothing reached the bar (precision 0.90 or more with at least one approval caught) on the fresh part.
More approvals are needed before it can be measured; here 5 of the 179 typed replies approve.
The ways to get them are not built: reading later messages in the same window and not only the first, and keeping Fleet's own record of the reply and what the owner then did, going forward.

## What this does not cover

- **A reply to something other than the last put.**
  The first message after a put is taken as its reply, with up to 132 turns in between; 14 of the 179 come more than ten turns after the put, and a few answer a later question.
  They are labelled by what they say.
- **Button approvals.** `Approved: …`, `Sent back` and `The owner approved plan` are not read; they are already a decision.
- **Annotation notes** on an element of a page.
- **Mixed replies.** "A is good, but add a fallback" is labelled `not`, since it asks a change first; "approved, also add X later" would be `approves`.
  Those lines are thin in the data.
- **Other people's replies, other machines and other repositories.** Every example is one owner on one machine.
- **One labeller** (an agent) and no second reading of the borderline replies ("Yes" and "C" in training).
