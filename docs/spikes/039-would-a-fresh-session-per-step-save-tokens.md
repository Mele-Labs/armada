# Spike 39 — Would a fresh session at each workflow step save tokens?

**Answer: a little, and most of it is what the 250k compaction window already saves.**
At today's 400k window, a fresh session at every step boundary saves 7% to 15% of what multi-step Jobs' main threads spend, which is 2.0% to 4.4% of all spend over the 14 days.
With a 250k window, fresh sessions on top save −4.4% to 5.0% of those Jobs' spend, which is −1.1% to 1.2% of all spend.

**A fresh session is a compaction at a step boundary, with the Job's brief in place of the summary.**
The two reset a thread to about the same size, 67k against 70k, so whichever comes first takes the saving.
Starting fresh only where the step carries 150k or more is the version that holds up beside the window: 13.5% of those Jobs' spend at 400k, and 5.8%, or 1.5% of all spend, on top of 250k.

**More than half the boundaries end a step in which the owner wrote to the Job.**
138 of 256 boundaries end such a step, 291 messages in all.
A brief carries only what its writer chose and the last few rows of the thread.

Measured 4 Nov 2026 on this Mac, from Claude Code transcripts of this repository from 21 Oct to 4 Nov and a backup of Fleet's store, both read only.
The question is the owner's: whether starting each step of a hosted Job in a new session, carried by a written brief, saves tokens without losing the thread.
Today a Job keeps one conversation for its whole workflow, started with `--session-id` and resumed with `--resume` in `crates/adapters/src/hosted_session.rs`.

## How it was run

`039-run.py` reads the transcripts and the store and prints aggregates only; `039-output.txt` is its output.
It weighs a request as the token usage measurement did: Opus and Fable 5, Sonnet 3, Haiku 1; cache read 0.1, cache write 2 on a main thread and 1.25 on a subagent, output 5.
A unit is a million tokens at weight 1.

A request is counted once by its message id, in the first thread that holds it, so forks and copies of a transcript across slots are not counted twice.
The token usage measurement's 137,766 requests counted every project folder and each copy of a request, so it is not this count.

### Where a step starts

**The step boundaries come from Fleet's store, `session_rail.visits`, and from Fleet's step messages for Jobs that have none.**
Fleet writes a `step<TAB>time` line each time a Job's step changes, in `crates/fleet/src/rail_following.rs`, since `crates/store/migrations/20261027T1500Z-session_rail.visits.sql`.
For Jobs before that, the boundary is Fleet's message `Fleet: this Job follows the … workflow, and you are on its "…" step.` in the transcript.

| Source | Jobs | Boundaries | Covers |
|---|---|---|---|
| Rail visits in the store | 68 | 234 | 28 Oct to 4 Nov |
| Step messages in the transcript | 94 | 22 | 21 Oct to 4 Nov |

**The step messages miss most boundaries.** On the 67 Jobs that hold both, the visits find 234 boundaries and the messages 48, since a step with no instructions sends none.
So the first week is undercounted, and the saving is a lower bound there.
A boundary is the first request at or after a move to a different step, the Job's first step is its start, and two moves before one request are one boundary.

### The simulation

**Each multi-step Job's main thread is run again from its own context growth, under four policies.**
At a boundary the old session writes the brief, and the new one starts from 56k of start-up, the brief, the re-read allowance and the step's first message, written to the cache once.
From there it grows as the real thread grew; a request that found its cache cold rewrites the simulated context, not the real one.

| Input | Value | From |
|---|---|---|
| Start-up | 56k | A main thread's first request, measured |
| Brief | 0.8k, and 3k as a variant | The five handed-off Jobs' starting text, 2,909 characters on average |
| Re-read | 5k, 10k, and 20k pessimistic | Post-compaction growth, 32k against 27k over 20 steps |
| 400k window | The real compactions, at a median 366k | The transcripts |
| 250k window | Compact past 250k to 70k, with 6k of summary | The token usage simulation |

Run with no boundaries, the simulation lands 1.4% above what those Jobs actually spent: 956 against 943.

## What it measured

Over the 14 days, 34,118 requests weighed 3,362 units: main threads 1,664 and subagents 1,698.
Hosted Jobs of this repository were 162 threads and 1,662 of it.
80 of them crossed at least one step boundary, 256 boundaries in all, and spent 943, 28% of all spend.

| | Requests a Job | Requests a step | Context carried across a boundary |
|---|---|---|---|
| Median | 107 | 15 | 107k |
| Mean | | 27 | 124k |

63 boundaries carried 150k or more; 28 of 336 steps lasted under three requests.

### The saving

Each row is the share of the 80 Jobs' main-thread spend saved against the same window with no boundaries, and the same saving as a share of all spend.

| Window | Policy | Re-read | Their spend | All spend |
|---|---|---|---|---|
| 400k | Fresh at every boundary | 5k | 15.4% | 4.4% |
| 400k | Fresh at every boundary | 10k | 12.6% | 3.6% |
| 400k | Fresh at every boundary | 20k | 7.0% | 2.0% |
| 400k | Fresh at every boundary, 3k brief | 10k | 10.2% | 2.9% |
| 400k | Fresh only past 150k | 10k | 13.5% | 3.8% |
| 250k | Fresh at every boundary | 5k | 5.0% | 1.2% |
| 250k | Fresh at every boundary | 10k | 1.9% | 0.5% |
| 250k | Fresh at every boundary | 20k | −4.4% | −1.1% |
| 250k | Fresh at every boundary, 3k brief | 10k | −0.9% | −0.2% |
| 250k | Fresh only past 150k | 10k | 5.8% | 1.5% |

**The 250k window alone saves 12.1% of these Jobs' spend, 3.4% of all; with fresh steps on top, 13.8% and 3.9%.**
A brief is written as output, at five times a write, so a 3k brief costs more than the 0.8k a handed-off Job starts from.
A boundary on a small context costs a whole new start-up write for little carried context saved.

## The cost side

**Five Jobs were handed off with "Carry on in a new Job", between 26 Oct and 4 Nov.**
Each started from 0.8k of handoff text, the brief its old Job wrote of 0.5k to 1.8k characters inside it.
Their first 30 requests set against two comparable stretches, medians:

| Stretch | Count | Starts at | Grows | Costs | Read and run calls |
|---|---|---|---|---|---|
| A handed-off Job's first 30 requests | 5 | 51k | 47k | 1.96 | 25 |
| Another hosted Job's first 30 requests | 136 | 47k | 64k | 2.20 | 46 |
| 30 requests after a step boundary, same session | 197 | 100k | 39k | 2.32 | 38 |

**A handed-off Job grew about 8k more than a continuing step does, inside the 5k to 10k re-read the simulation allows.**
It made fewer read and run calls than either, so it did not re-explore the way a Job starting from the owner's task does.
Five Jobs is too few to rest the re-read figure on.

## What a boundary drops

**A brief carries what its writer chose and Fleet's handoff text, which ends with the last rows of the thread.**
Corrections the owner gave earlier in the step are not in either unless the writer carries them.
Three of the 291 messages, each in a step that a boundary then ended:

- "We already have a job card that should be reused here instead of the weird blue monospace one being used here."
- "Fleet should never notify the job agent twice about the same comment unless its been edited"
- "Instead of a dialog this should just show in the new tab that opens with the run button."

Also lost at a boundary, from what a session holds and Fleet does not:

- Skills the step loaded, which the next session loads again inside its re-read.
- The session's own task list, which Fleet sets again from the recorded plan.
- Results of subagents the step ran, unless the brief or a document carries them.

## What was not measured

- **Whether work is worse after a fresh start.** The simulation counts tokens, not quality or the owner's extra time.
- **The re-read, from a real per-step restart.** It comes from compactions and five handoffs, not from Jobs that restarted at a step.
- **Subagents.** They already start fresh, and are unchanged in every row.
- **A compaction's own summary request**, which does not appear in the transcript; the 250k rows estimate it.
- **Prices.** The weights are assumed, not read from a price list or the plan meter.
- **The first week's boundaries**, undercounted by the step messages.
- **Requests in flight**: Jobs were running while the script read, so a re-run gives slightly different counts.
