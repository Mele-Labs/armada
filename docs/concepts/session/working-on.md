# Working on

**Part of:** [Job](../session.md).

**A hosted Job shows what it is doing, and Fleet detects it.** `SessionRecord.activity` (`SessionActivity`: `now?`, `todos[]`) is built from the tool calls Fleet's PreToolUse gate sees, which is every tool call of a Job Fleet hosts: the hook's matcher is `*`.
Until 14 Oct 2026 the matcher named only the five tools a lease is taken on, so a read, a search, a subagent or a `TodoWrite` never reached Fleet and `now` and `todos` stayed empty for a Job that was not writing.
A call to any other tool is noted and let through before the pool or the Manifest is read.
No tool asks the agent to say it, because Fleet does not rely on an agent calling a tool to know its state (the owner, 14 Oct 2026).
The one thing the agent does say is its own to-do list, and Fleet reads that off the call too.

| Field | Where it comes from | What it is |
| --- | --- | --- |
| `now` | Detected, from the call's tool and input | One short fact for the latest call Fleet can name. A call it cannot name leaves it as it was |
| `todos` | Reported, as the agent's own `TodoWrite` list | Each `TodoWrite` replaces the whole list. `text` is the item's `content`, and `completed`, `in_progress` and `pending` are `done`, `doing` and `todo`. An empty list clears it. A `TodoWrite` never changes `now` |

`now` reads in one of these forms, with no counts and no narration:

| Tool | `now` |
| --- | --- |
| `Edit`, `Write`, `MultiEdit`, `NotebookEdit` | `Editing gate.rs`, the file's name without its directory |
| `Read` | `Reading gate.rs` |
| `Bash` | `Running cargo check`: the command's name, and its subcommand where it is a tool that has them (`git`, `cargo`, `pnpm`, `armada` and the like) and the next word is a plain one. A leading `cd x &&`, an environment assignment or a wrapper such as `time` is skipped, and a line with nothing in it that runs a command leaves `now` alone |
| `Grep`, `Glob` | `Searching` |
| `WebFetch`, `WebSearch` | `Fetching` |
| `Agent`, `Task` | `Running a subagent` |

**It is kept with the rest of a Job's runtime state, in memory.** It is not in the store, a Fleet restart starts it empty, and it is dropped when the Job ends.
The gate notes a call only for a Job that is hosted and open, after the check that already passes an unknown or ended one, and does so whatever the gate then decides about the call.

**A call does not publish a row.** The row is large and goes out whole with every `session.changed`, and a busy Job makes many calls.
A change to `now` or `todos` publishes at once when the Job's row has not gone out for what it is doing within the last two seconds.
Otherwise one publish follows when those two seconds are up, carrying the latest, so the last change is never lost and a burst costs two rows.
A call that changes neither publishes nothing, so a long run of identical calls costs no rows.
The gate takes a lock for a moment and awaits nothing, and the publish is a task of its own, so the answer to the call is not held for it.

**A turn starting clears `now` and keeps `todos`.** The line of the last turn is not the new turn's, and the agent's list is still true across turns.
It is the same moment `done_since` stands down, and a turn Fleet's own notice begins counts: a new turn is a new turn.
The row that announces the turn carries the clear.
A turn ending clears nothing.

**Bridge draws nothing for a Job whose turn is not running.** Whether a line or a band is drawn is Bridge's rule, read off `hosted.turn` and these fields.

**Silence is not here.** A running turn nothing has been heard from is the `stuck` state, from the activity of the process Fleet hears, whatever it calls.
`activity` says what a Job is doing, and carries no time to read a silence from.

**Only a hosted Job is covered.** A Job run in a terminal makes its tool calls in a process Fleet does not host.
Its mod reports a few tools through `report_session` and does not call the gate, so such a row carries no `activity`.
