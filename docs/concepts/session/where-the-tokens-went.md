# Where the tokens went

**Part of:** [Job](../session.md).

**Fleet keeps the usage of every turn, as well as the latest figures.** `usage` on the record stays the last `context_tokens`, `context_window` and `cost_micros`.
Beside it, `session_turn_usage` holds a row for each answered message: its id, when, the model, input, cache-creation, cache-read and output tokens, the names of the tools the turn called, whether a subagent answered it, and, where its first call was a shell command, a `bash_use` label: `read_search`, `scripts`, `git`, `build_test`, `loops_waits`, `forge` or `other`, from the first real command once any leading `cd x &&` is gone.
A message heard twice is one row: the larger output wins the figures and the calls are joined, because a harness writes the same message as it streams, a line for each block.
`get_session_usage` returns a job's turns for a window of time, oldest first; adding the rows by model, tool or label is the reader's.
Names and counts only: no tool input or message text, and a label is not the command.

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
The summary, the hours, the builds and Spent on weigh this way. The usage rules keep the weights they were set under.

## What the burn was spent on

**Spent on splits every turn's burn into nine parts, and Spent by into three.** Fleet reads each transcript in order and keeps an estimate of what the thread's context holds, a part at a time: four bytes of text are a token, and an image is 1,600.
The estimate and the calls still waiting on a result are kept with the file's offset in `transcript_offsets.carried`, so a read picks up where the last one stopped.

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

**A Job is told when its usage crosses a rule.** After each reading of a live Job's transcripts, Fleet checks the Job's recent turns against four rules, each an on-off and a limit in settings.json under `limits`.
It reads only the rows just ingested for that Job: its last hour and its latest few main-thread turns, never the table.
The Job is told once, in the words below, and Fleet does nothing else: it does not end, fork or move the Job, and it does not change its model.
The one exception is the Context rule with *Large context* set to carry on automatically, which hands the Job on as the owner's yes would.
A subagent's turn counts toward the hour's burn and its share and toward nothing else.

| Rule | Crossed when | The Job is told |
|---|---|---|
| Context | The last turn's context (input, cache-creation and cache-read) is over 300k tokens | `Context is 312k tokens. To carry on in a new job, ask the owner with the options "Carry on in a new Job", "Stay here", "Stay, don't ask again this Job" and "Remind me at a higher limit", and write your brief (where the work is, what is open, what you hold) in the description of "Carry on in a new Job". Start nothing yourself.` |
| Subagents | Subagents burned over 5 cost units in the last 60 minutes, over 50% of that hour's burn, and at least one subagent turn ran on an opus model. Subagents all on sonnet or haiku are never told | `Subagents burned 7.1 units in your last hour, 71% of it, on opus. Pass model sonnet on reading and searching agents.` |
| Cache hit rate | Cache-read over input, cache-creation and cache-read is under 70% across the last 10 turns | `Cache hit rate is 54% over your last 10 turns. Something in the prompt or tools changed each turn; check hooks and MCP servers.` |
| Opus on reading | The last 20 turns all ran on an opus model, each began with a read or a search (`Read`, or a shell command labelled `read_search`), and none edited | `Your last 20 turns on opus only read and searched. Subagents for reading can run on sonnet.` |

**The Context rule answers to the owner.** His answer on the Job's handoff card, and the setting *Large context* (`plan.contextHandoff`), decide whether the Job is told, handed on with no card, or left alone: [A handed-off Job](a-handed-off-job.md).

**Once, and again only after it cleared.** A Job over a limit is told the first time and nothing more while it stays over.
It is told again only after a reading found it under the limit and 30 minutes have passed since the last telling.
Each rule keeps its own place.
A Fleet that restarts does not tell a Job over a limit again until it has been seen under it, because the last telling is read back from the trip rows.

**Each telling is a row in `usage_rule_trips`:** the Job's id, the rule (`context`, `subagents`, `cache_hit` or `opus_reading`), when (the stamp of the newest turn), the figure that crossed (tokens, a percent or a count, by rule) and `burn_per_hour_before`, the burn in cost units per hour of the Job's active time over the 60 minutes up to that turn.
A telling that was not taken, because the Job had ended, writes no row and is tried again at the Job's next turn.

**What followed is kept on the same row.** Once a turn of the Job lands 60 minutes or more after the trip, in the same pass that read it, Fleet reads the Job's turns in that hour and fills `after_turns`, how many there were, and `burn_per_hour_after`.
That is the hour's cost units over the Job's **active time**: the sum of the gaps between consecutive turns in the hour, each gap counted as at most 5 minutes, so a Job that stopped for forty minutes does not read as a win.
Only the after is left empty for want of turns: a Job that made fewer than 5 turns in the hour has `after_turns` and no `burn_per_hour_after`, a trip with no reading.
A Job that never makes another turn is never measured.
`burn_per_hour_before` is counted the same way, so the two compare; it is stored whatever the number of turns in that hour, since the trip fired on the newest, and is 0 where there is one.

**`get_usage_rules` lists them.** For each of the four rules it returns whether the rule is on, its limit, and the trips in a window of time, newest first.
`docs/practices/protocol.md`, *The usage rules*.
