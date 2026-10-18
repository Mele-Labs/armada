# A forked job

**Part of:** [Job](../session.md).

A job that is **ended or dead** offers Fork, in place of its message box. `start_session` takes `fork { session_id }` (23.69) and starts a new job hosted by Bridge as a copy of that conversation. Spike 28 measured the agent side.

| | |
|---|---|
| Dead | The ledger says `ended`, or it is a terminal job whose mod has not asked for ten seconds (`terminal.listening` is absent). A hosted job is dead only once it is closed |
| Refused | A live job, 409 `fleet.session_fork_live`. An unknown id, 422 `fleet.no_such_session`. Nothing is written |
| The fork | Its own id, ledger and slot. It takes the old title unless one is given, starts in the repository's main checkout, and leases a slot at its first write like any other job. It holds none of the old job's slot, branch or pull requests |
| First process | Resumes the old id as a fork under the new one, so the conversation is copied and the new id is Fleet's. Once it has run, the job resumes by its own id |
| Ledger | A `forked_from` row on the fork and a `forked_to` row on the old job, both `spent` |
| Told | **Every process of a fork has a note in its system prompt** (`--append-system-prompt`): forked from <title> (<id>) at <time>, that job is live or has ended, and nothing it started is the fork's. Its subagents, background tasks and shells, slots, branches, windows, pull requests and peer jobs stay its own, and a task-notification about its background work is not acted on, relaunched or reported. A fork launch cannot be built without the note (`HostedLaunch::forking` takes it) |

> **Rule.** The whole of a job is forked only once it is ended or dead. A live one is forked at a message, below.
> Why: a copy of the whole of a conversation still being written is two jobs racing to the same next message, and a job that can still be spoken to needs no copy.

## A fork at a message

**Words selected inside one message offer Fork from here**, on a live job as well as a dead one. `fork { session_id, at }` (23.79) copies the conversation up to and including the message whose thread row id is `at`, and nothing after it. The old job is not touched: it keeps its process, its slot and its next turn, and gains only the `forked_to` row. The fork is hosted by Bridge as any fork is.

| | |
|---|---|
| Refused | An `at` no message of the thread has, or one the agent's conversation does not hold, 422 `fleet.fork_point_unwritten`. Nothing is written, and the line is looked for before the fork is made |
| Ledger | The `forked_from` row's detail carries `at`, the row Bridge cuts the folded old rows at, and `cut`, the transcript line Fleet gives the agent. The detail is the ledger's, so a Fleet that restarts before the fork's first process still has the cut |
| First process | As above, with `--resume-session-at <cut>`. Only that process is told it; the fork's own file holds the cut after |
| The line | A terminal job's row ids are its transcript lines' `uuid`s (`<uuid>.<n>` for a later row of one line), so the cut is the row id without its suffix. **A hosted job's rows carry no `uuid`**: Fleet minted their ids and the `Said` events the stream is read into drop the line's. Fleet lays the thread's messages over the transcript's from the end, matching the agent's by its words and a person's by being inside the line Fleet wrote (it adds a mention line and attachment paths), and the cut is where `at` lands. A message with no words, or none that lands, is refused: it is not guessed |

The agent side is spike 30. A cut at a message the agent is part way through a tool call on is repaired by the CLI.

**Why a note, and not a cleaner copy.** The CLI copies the conversation, and the conversation says the old job launched subagents, shells and a slot; the CLI then replays a notification for each of them that had not finished ("didn't finish before the previous job ended", "failed", "stopped") into the fork. Stripping the notifications from a copy of the transcript would leave the launches that caused them in the words the fork reads, and the copy would be a file Fleet writes into the CLI's own folder in a format Fleet does not own. The fork is told instead, in the one place it cannot read past. Found 13 Oct 2026: a fork of a live job relaunched the old job's builder into the old job's slot, and showed the owner the old job's windows as its own.

**Bridge** keeps every job that ended in the last seven days (by when it was last seen) beside the live ones, under an Ended heading (a terminal job that has not ended but whose mod is not asking is Quiet, its own heading above it, and still offers Fork), and finds an older one by search, and draws a dead job without a message box and with Fork in its head. It owns nothing: its slots, branches and pull requests are not offered to a chip, and it is not offered to `@`. A press on Fork asks Fleet, and Bridge opens the new job once it holds it. The ledger draws a Forks section only where a job has a row for it.

**A terminal that comes back is live again.** The mod asking again flips `listening`, which Fleet publishes as `session.changed` while a window has the thread open. Fork on a job whose terminal woke between the press and Fleet is refused as live. After a Fleet restart every terminal job reads dead until its mod's next ask, about two seconds.
