# Where the tokens went

**Part of:** [Job](../session.md).

**Fleet keeps the usage of every turn, as well as the latest figures.** `usage` on the record stays the last `context_tokens`, `context_window` and `cost_micros`.
Beside it, `session_turn_usage` holds a row for each answered message: its id, when, the model, input, cache-creation, cache-read and output tokens, the names of the tools the turn called, whether a subagent answered it, and, where its first call was a shell command, a `bash_use` label: `read_search`, `scripts`, `git`, `build_test`, `loops_waits`, `forge` or `other`, from the first real command once any leading `cd x &&` is gone.
A message heard twice is one row: the larger output wins the figures and the calls are joined, because a harness writes the same message as it streams, a line for each block.
`get_session_usage` returns a job's turns for a window of time, oldest first; adding the rows by model, tool or label is the reader's.
Names and counts only: no message text, and no tool input but a Read's file path, which is kept so the same file read twice can be found (*Waste*). A label is not the command.

**Nothing has to report a turn.** Fleet reads the agent CLI's transcripts itself, `<project>/<session>.jsonl` and the subagents' under `<project>/<session>/subagents/`, in `crates/fleet/src/turn_ingest.rs`.
The files of every live job are read on the notice loop from the offset the last read reached, and a while after start the files written to in the last seven days are read once, one at a time with a pause between, so a week of jobs does not load the machine.
The offset of each file is kept in `transcript_offsets` with the turns it gave, so a restart reads only what was appended.
A job found under several project folders is counted once, by its largest file.
A transcript of a job Fleet does not know gets rows too.
`docs/practices/protocol.md`, *Usage a turn at a time*, has the wire.

**The Usage page reads sums, not rows.** `get_usage_summary` adds the kept turns over a window in SQL and returns burn in cost units: the total split by model, token kind, first tool, shell use, day and weekday-by-hour in the caller's local time, the thirty largest Jobs the same way with the rest rolled into one, and the equal period before for comparison.
`docs/practices/protocol.md`, *The usage summary*.

**Burn weighs the model and how long a cache write is held.** A turn's tokens are weighed input 1, a cache write held 5 minutes 1.25, a cache write held an hour 2, cache read 0.1 and output 5, over a million, and the sum is multiplied by the model's weight: Haiku 1, Sonnet 3, Opus and Fable 5, and 1 for a model of any other name.
`cache_creation_1h_tokens` keeps the hour's writes, which are inside `cache_creation_tokens`, from the transcript's `cache_creation.ephemeral_1h_input_tokens`.
A turn heard only from the mod, or kept before Fleet read its file again, counts every write at 5 minutes.
The summary, the hours, the builds, Spent on and the usage rules weigh this way.

## What the burn was spent on

**Spent on splits every turn's burn into nine parts, and Spent by into three.** Fleet reads each transcript in order and keeps an estimate of what the thread's context holds, a part at a time: four bytes of text are a token, and an image is 1,600.
The estimate and the calls still waiting on a result are kept with the file's offset in `transcript_offsets.carried`, so a read picks up where the last one stopped.
An offset kept before there was anything to carry has `carried` empty, and that file is read again from the top once, so its older turns gain their shares.

| Part | Holds |
|---|---|
| `start_up` | What the first request after the start or a compaction held beyond the text the transcript shows, the instructions and reminders written before that request, and a compaction's summary |
| `reads` | Results of `Read`, `Grep` and `Glob`, and of shell commands labelled `read_search` |
| `own_words` | The agent's own text, thinking and calls |
| `other_results` | The result of every other call |
| `messages` | Messages to the agent, and the skill bodies it loaded |
| `announced` | Connector instructions, tool lists and reminders written after the first request |
| `cold_cache` | The cache writes of a turn that found the cache cold |
| `output` | Output |
| `background` | A compaction's own request |

**A turn is charged for what it carried.** Its cache read is shared among the parts as they stood at the request before it, and its input and cache writes among what was added since.
A turn more than 5 minutes after the one before it, whose cache write is larger than its cache read, found the cache cold, and its writes are `cold_cache`.
A compaction empties the estimate, and the next request starts it again from `start_up`.
A turn's shares are kept on its row in `spent_on`.
A turn with none, heard only from the mod or kept before this, has its input shared as the rest of its Job's turns in the window are, or else as the window's.

**A compaction's own request is background.** The transcript does not hold it, so Fleet keeps it in `session_background_usage`: the `preTokens` its boundary names, read from the cache on the thread's model, and the summary written after it, as output.
It counts in the summary as a turn that called nothing, and in both splits as `background`. Titles are written outside the transcript and are not counted.

**Each split adds up to the burn.** Spent by is main threads, subagents and background, and its subagents are the same units as the page's Burn in subagents.
`get_usage_spent` returns the 24 hours, 7 days and 14 days up to now, each whole and for its thirty largest Jobs.
`docs/practices/protocol.md`, *What the burn was spent on*.

**A Job is told when its usage crosses a rule.** After each reading of a live Job's transcripts, Fleet checks the Job's last turn against one rule, an on-off and a limit in settings.json under `limits`.
It reads only the rows just ingested for that Job: its last hour and its latest main-thread turn, never the table.
The Job is told once, in the words below, and Fleet does nothing else: it does not end, fork or move the Job, and it does not change its model.
The one exception is the Context rule with *Large context* set to carry on automatically, which hands the Job on as the owner's yes would.
A subagent's turn counts toward the hour's burn and toward nothing else.

| Rule | Crossed when | The Job is told |
|---|---|---|
| Context | The last turn's context (input, cache-creation and cache-read) is over 300k tokens | `Context is 312k tokens, past the 300k limit. Write only a brief of where the work is, what is open and what you hold, and send it with handoff_brief. The owner answers on a card. Ask him nothing and start nothing yourself. Once it is sent, end your turn.` |

**The Context rule answers to the owner.** His answer on the Job's handoff card, and the setting *Large context* (`plan.contextHandoff`), decide whether the Job is told, handed on with no card, or left alone: [A handed-off Job](a-handed-off-job.md).

**Once for the Job, at a quiet moment.** A Job over a limit is told the first time and never again by size, however its figure moves after.
A quiet moment is a turn that has ended with nothing waiting on the person: no question open or lapsed, no Needs you item, walk, plan or draft awaiting him (the same quiet the stop nudge needs, [What a Job is doing](what-a-job-is-doing.md)).
Over the limit while he has something to decide, the telling is held and sent at the next quiet moment, never mid-turn and never over a decision.
A Fleet that restarts does not tell a Job again, because the last telling is read back from the trip rows.
The owner's own *Remind at* figure is his answer and still tells the Job again past the figure he set.

**Each telling is a row in `usage_rule_trips`:** the Job's id, the rule (`context`), when (the stamp of the newest turn), the figure that crossed (tokens) and `burn_per_hour_before`, the burn in cost units per hour of the Job's active time over the 60 minutes up to that turn.
A telling that was not taken, because the Job had ended, writes no row and is tried again at the Job's next turn.

**What followed is kept on the same row.** Once a turn of the Job lands 60 minutes or more after the trip, in the same pass that read it, Fleet reads the Job's turns in that hour and fills `after_turns`, how many there were, and `burn_per_hour_after`.
That is the hour's cost units over the Job's **active time**: the sum of the gaps between consecutive turns in the hour, each gap counted as at most 5 minutes, so a Job that stopped for forty minutes does not read as a win.
Only the after is left empty for want of turns: a Job that made fewer than 5 turns in the hour has `after_turns` and no `burn_per_hour_after`, a trip with no reading.
A Job that never makes another turn is never measured.
`burn_per_hour_before` is counted the same way, so the two compare; it is stored whatever the number of turns in that hour, since the trip fired on the newest, and is 0 where there is one.

**`get_usage_rules` lists them.** For the rule it returns whether the rule is on, its limit, and the trips in a window of time, newest first.
`docs/practices/protocol.md`, *The usage rules*.

## Waste

**Fleet names the burn it can already measure as avoidable, with what it cost, the Jobs it hit and a fix.** `get_usage_waste` answers for the last day, week or fortnight, and the Usage page leads with it: how many units of the window's whole went to waste, then a row for each kind, each opening to its Jobs.
Every figure is in the units above, so `total_units` is the summary's for the same span and `waste_units` is the kinds added up.
`title` and `fix` are Fleet's own sentences, so the page and an agent read the same words.
`docs/practices/protocol.md`, *The waste in the burn*.

| Kind | Waste is | Counted as |
|---|---|---|
| `cold_cache` | The cache writes of a turn that found the cache cold, as Spent on's `cold_cache` | The turn's input and cache writes, weighed, times the share it kept as `cold_cache`. One occurrence for each cold turn |
| `fresh_starts` | What the first main-thread request after a compaction held as `start_up` | The turn's cache read times its `start_up` read share, plus its input and writes times its `start_up` write share. One occurrence for each such start |
| `repeat_reads` | A Read of a file the same thread already read, with no compaction between | The repeat result's tokens (its bytes over 4), as a share of the input and writes of the request that carried it, times that request's weighed input and writes, out of what the request has left. One occurrence for each repeat read |
| `big_results` | The part of a tool's result past 20 KB | The same, for the bytes past 20 KB of each such result that is not a repeat read. One occurrence for each such result |
| `main_context` | The part of a main-thread request past 300k tokens of context | What is left of the request's burn, times (context less 300k) over context. One occurrence for each such request |
| `subagent_context` | The same, for a subagent's request | The same |

**A request's burn is handed out once, in that order.** The cold cache takes its part first, a fresh start takes its start-up part of what is left, repeat reads and then big results take theirs of what is left after those, and the context kinds take their fraction of what is still left.
Both result kinds sit after the two kinds that are read off the request's own shares and before the context kinds, so a result that makes a request long is waste as a result and the context kind keeps only what it did not take.
So a request is in as many kinds as it earned, and no unit is in two: the kinds add up to at most the burn of the requests they read.
The 300k is one constant, `LONG_CONTEXT` in `crates/store/src/usage_summary.rs`, the figure the Context rule tells a Job at.
A request at exactly 300k is under it.

**A fresh start is not every request with a `start_up` share.** Each request reads the start-up again from the cache, so the share alone is on every turn.
A start is the first main-thread request after a compaction, which the Job's own first request never is, and a subagent's request is never one: it starts its own thread and does not restart the Job's.
A compaction's own request is background, the fix and not the waste, and is in no kind.
A request with no shares kept, heard only from the mod or kept before shares existed, has no cold part and no start-up part, and counts for the context kinds alone.

**A kind with nothing is left out, and the kinds are largest first.** Each kind lists the thirty Jobs it hit most, largest first, with their units and occurrences.
The two context kinds also carry the most context one of a Job's counted requests held, as `peak_context`.
A kind is a variant in `crates/store/src/usage_waste.rs` and the words Fleet gives it in `crates/fleet/src/usage_waste.rs`.

**A result is paid for by the request that first carried it.** A tool's result enters the thread's context in the first request of that thread after the result is written, and that request's input or cache writes hold it.
Its tokens are its bytes over 4, as everywhere here, and the request's `written` figure (its weighed input and writes) times the result's share of the request's input and writes is the waste.
A request's results are summed first and the share is held to one, so estimate error cannot take more than the request wrote.
A repeat read comes first: its whole result is the waste.
A result over 20 KB that is not a repeat read gives the bytes past 20 KB, so a repeat that is also big is counted once, as a repeat read.
The 20 KB is one constant, `BIG_RESULT_BYTES` in `crates/store/src/usage_waste.rs`, 20 x 1,024 bytes. A result of exactly that size is under it.

**A repeat read is the same file Read again while the earlier read is still in the thread's context.** The same path, in the same thread, with the earlier result already returned, and no compaction between.
A thread is the session's own, or one subagent's transcript, which is its own context: a file the main thread read and a subagent then read is not a repeat.
A compaction empties the context, so each read after one starts again.
The path is compared as written, so `/a/f.rs` and `/a/../a/f.rs` are two files, and a Read of another range of a file is the same file: Fleet keeps no offset or limit, so a file read in two slices counts as read again.
An Edit or Write between the two reads does not reset it.

**`causes` says which file or which tool.** Each Job of `repeat_reads` and `big_results` carries its three largest causes, `{ name, units }`, largest first: file paths for the first and tool names for the second, including Fleet's own, such as `mcp__armada-fleet__waiting_for`.
A cause's units are the kind's units for the Job split by the results' tokens, so a Job's causes add up to its units before the cut to three.
No other kind has causes.

**What is kept, to find these.** `session_tool_calls` holds a row for each tool call: the session, the call's id, its message, when, the thread (empty for the session's own, a subagent's transcript name for a subagent's), the compactions the thread had had, the tool's name, a Read's file path (null for any other tool), the result's size in bytes and the id of the request that carried it.
No result text and no other input is kept.
The rows come from the same transcript read as the turns: the size is the result's content measured as it is decoded, as the estimate above is, and the carrier is the first message of the thread after the result.
A transcript read before this was kept is read again from the top once, as one read before searches were counted was, so a week of old turns gains its rows.
A Job that ran before that, with its transcript gone, has none, and has no repeat reads or big results.

## A Job compacted while it waits

**A trial, shipped off.** The setting *Compact a Job waiting on you* (`tokens.compactWhileWaiting`) switches it on, and is read at each sweep of quiet Jobs, about every 30 seconds.
A Job that waits past its prompt cache's hour starts again from its whole context, written to the cache at 1.25. Compacted first, it starts from a summary.

**What triggers it.** Fleet sends `/compact` to a hosted Job, as Claude Code's own command, when all of these hold:

- No turn is running, and the Job is neither paused nor held on its start card.
- Its waiting list has an item.
- Its last main-thread turn in `session_turn_usage` was 50 minutes ago or more, so the sweep and the resume land inside the cache's hour.
- That turn's context (input, cache-creation and cache-read) is 100,000 tokens or more.
- No compaction is kept after that turn: once a wait.

The thread shows `/compact` as a message from Fleet. The turn it starts leaves the Job's `all_done` standing.
`crates/fleet/src/session_host/compacting.rs`.

**What is kept.** Each compaction is a row in `waiting_compactions`: the Job, when Fleet sent it (`at`), and `context_before`, the context of the last main-thread turn before it.
Once the Job's first main-thread turn after it is read, from a transcript or a turn measured, the row takes that turn's `next_at`, `next_input`, `next_cache_write` and `next_cache_read`.
Until then they are empty.
A paused or ended Job is sent nothing and writes no row.

**Cost against saved.** The compaction's own request reads `context_before` from the cache, at 0.1, and writes the summary as output, at 5.
What it saves is read off the next request: started cold it would have written `context_before` at 1.25, and it wrote `next_cache_write` at 1.25 and read `next_cache_read` at 0.1.
A `next_at` inside the hour after the turn before the compaction would have read the cache warm, and for it the compaction is cost only.
A row with no `next_at` is a Job that has not made a turn since.
