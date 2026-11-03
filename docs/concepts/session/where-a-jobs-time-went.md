# Where a Job's time went

**Part of:** [Job](../session.md).

**Every second between a Job's start and its end lands in one bucket.** There are nine, and a Job's nine figures sum to its length.
Fleet splits a Job once it has ended and keeps the answer (`crates/fleet/src/job_time.rs` is the split, `crates/fleet/src/job_timing.rs` gathers its input and keeps it).
`docs/practices/protocol.md`, *Where a Job's time went*, has the wire.

## The buckets

**Where two buckets claim a second, the first in this table takes it.** Work wins over waiting, and a wait the Job is not in wins over nothing.
A claim by work that reaches back past a restart of the agent's process starts at the restart, because the gap is not work.

| Bucket | Counts | Read from |
|---|---|---|
| Checks | A Check queued or ran | Check runs of the Job; a run with no end counts its `took_ms` |
| Subagents | A subagent stood on the Job | The Job's subagent attachments, from standing until not |
| Tools | Tool calls of the main thread | A turn's first call to its last result |
| Model | Waiting on the model's answer | The prompt or last tool result a turn answered, to the turn's end |
| Slot | Waiting for a slot, or to be moved into it | Slot waits, `line` and `move` |
| You | Waiting on the owner | Below |
| Pull request | CI and the merge queue, with nothing else running | The Job's pull request attachments, while standing |
| Stopped | The agent was not running, or Fleet was down | Process stops and ends, and Fleet's outages |
| Idle | Nothing running and nothing asked | Whatever no other bucket reaches |

**A wait on the owner opens at one event and closes at the first close after it.** Where no close follows, it runs to the Job's end.

| | Events |
|---|---|
| Opens | A question or wait on the timeline, a window shown, a question asked in a window, a draft proposed, a plan recorded |
| Closes | An answer or resolution on the timeline, a window ended, the owner's next message, a plan approved or set aside |

A call to `AskUserQuestion` returns when the owner answers, so its span from first call to last result is You and not Tools.
A Job that ends with its start card still held is You throughout ([A held Job](the-start-card.md)).

## What is kept

**A row for each ended Job, in `job_times`, never pruned.** It holds the repository, the workflow, when the Job began and ended, the nine figures in milliseconds, three flags, when it was kept and the fold that made it.
Beside it `job_time_builds` holds the split cut at each build, and `job_time_pull_requests` each pull request's stint and wait.
The row and its parts are written in one transaction.

**The timeline the questions and answers are read from keeps 30 days, and the row outlasts it.**

**Filled as Jobs end, and backfilled once for those before.** Every 30 seconds Fleet takes the most recently ended Job with no row and splits it, with a pause between Jobs.
It reads the Job's transcripts first, and from the top again where its turns lack the stamps the split needs.
A Job reopened and ended again has a row for the earlier end, so it is split again and the row replaced.

**Fleet splits old rows again when the fold changes.** A row names the reading that made it (`FOLD` in `crates/fleet/src/job_time.rs`), and a row of an earlier reading, or of none, is the next Job the pass takes.
A change to what the fold gives for a Job must raise `FOLD`.
Left alone, the old rows keep the old rule, and the usual, the builds and the retro's timing row would compare Jobs split by two different rules.

## The flags

**A flag marks a row whose input was thin.** A flagged row stays in every sum and in every usual.

| Flag | Set when | What reads wrong |
|---|---|---|
| `tool_times_missing` | A turn called tools and kept no end for them, its Job's last turn excepted | Tools reads short |
| `model_starts_guessed` | A turn kept no stamp for where its model wait began | Model holds a guess |
| `timeline_cut` | The Job began before the timeline's oldest entry | You may read short |

**A guessed model wait runs up to five minutes back from the turn's end.** It starts no earlier than the end of the turn before, loses to every other bucket, and what is left stays as Model.

## You has no cap

**You stays as measured.** A start card left for days, a walk left open overnight and a window never ended each read as You for as long as they stood.

**The per-event figures do not use You**, so they are not skewed by it.
The usual and the far-over test do use it, because they compare buckets, and a Job that waited on the owner reads far over in You.

## Builds

**A build is a run of Fleet's code, from the restart that wrote its commit until the next.** A restart onto the commit already running is not another build, and a commit run again after another is a second build.
The Usage page reads figures per build and per hour.

**The figures are per event and not a median Job.** A build or an hour holds few Jobs and many turns, calls and runs, so a median Job per build or hour would be the middle of a handful and one slow Job would move it.

| Figure | Is |
|---|---|
| Model per turn | Median wait on the model, per turn of the main thread |
| Tools per call | Median run of a turn's tools, over turns that called any; not a question to the owner |
| Checks per run | Median time a Check run queued or ran |
| Pull request wait | Median wait in CI and the merge queue, per pull request that waited |
| Burn per turn | Cost units over the build, divided by its turns, subagents' included |

A turn is a build's when it was kept between the build's start and the next one's, and a Check run or pull request wait by when it ended.
The hour chart has the first four for each hour.

**A change shows at 5% or more against the build before.** A build with under 30 turns shows none, and neither does the build after it.

**Time is cut at a build's start.** A Job's split is cut at each build that began inside it, so a minute belongs to the build running when it passed and not to the build the Job began on.
A build that began as the Job did, or as it ended, takes none of it.
The stretch before the first build known belongs to no build.

**A build from before builds were kept has "first told" as its start.** Fleet's first rows are the builds it had told a Job of, stamped with the time of the first telling, and a build told to no Job has none.
`source` (`main` or `preview`) and the main commit under a preview may be unknown, and Fleet asks git for them where the repositories it serves hold the commit.

**What a preview holds is `Armada/preview-held`.** `scripts/preview` writes the preview commit, the main commit it stands on and a line for each branch merged into it, and Fleet keeps that with the build (`docs/practices/running-locally.md`).

## Far over

**The usual for a workflow is the median of each bucket over every kept Job of it**, whatever window is read and the Job itself included.
Each bucket is taken alone, so the usual is not one Job and its nine figures need not sum to a Job's length.
A Job with no workflow has no usual.

**A bucket is far over when it is at least twice its usual and at least ten minutes beyond it.** Both must hold: 13 minutes against 4 is not far over, and nor is 59 against 30.

**Only a Job with a bucket far over gets the retro's timing row** (`time:1`, [Retro](../retro.md)).
It holds the Job's whole split, the usual and the buckets far over, and the retro writes one item for the bucket that cost the most over its usual, says why from the other rows and says where the fix lands.
The Retros page starts a Job on that item with one press.

## Where to read it

| Where | Shows |
|---|---|
| Usage page, Time view | The slowest Jobs with a bar each, the four figures by hour, the window's split and the Builds panel |
| Usage page, Tokens view | The same builds, narrowing the Jobs and tiles to the one picked |
| A Job's System tab | The Job's split, each bucket far over beside its usual |
| `armada job get_job_time` | One Job's split and the usual of its workflow |
| `armada job get_usage_time` | Jobs that ended in a window, the slowest and the usual of each workflow |

`v` switches the Usage page between Tokens and Time, and `c` moves to the Builds panel.
A Job still running has no split.
