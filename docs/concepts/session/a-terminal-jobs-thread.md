# A terminal job's thread

**Part of:** [Job](../session.md).

Open a job a person runs in a terminal and Bridge draws its conversation and a message box.
`get_session` reads the transcript file the agent CLI keeps for it (`adapters::terminal_thread`, found by job id under the person's home) and answers rows in the shape a hosted thread has: what the person typed, what the agent said, and one line per tool call.
Fleet then watches the file for as long as the job is live and publishes each new line as `session.row`, so the thread follows the terminal.
A row's id is the transcript line's own, so a line read twice replaces itself.

A subagent has a thread of its own.
`get_session_subagent` reads `<project>/<session id>/subagents/agent-<subagent id>.jsonl`, the file the CLI keeps beside the job's, and answers it in the same rows with whether the subagent `finished` (a turn ended with nothing more to run, a hand-back, or the CLI's interrupt line where its parent's `TaskStop` or the person cut it off) and its `report`, the last thing it said.
**It also says the `model` its last answer was written under and `context_tokens`, the input side of that answer's usage (input, cache write and cache read), against `context_window`**, which Bridge draws in the sheet's head as the Job's own figure is drawn; the type of agent is not read here, it is the `type` the spawning call's attachment already holds.
The transcript names the model and never its window, so the window is the parent Job's reported one where it runs on the same model id, else the usual 200k until the tokens outgrow it and 1M after, a guess made in one place (`window_for`).
It reads that one path and no other, for hosted and terminal jobs alike.
Bridge reads it again while the subagent runs.
**The engine's stop settles it**: the mod's `SubagentStop` hook makes the row `spent` and stamps when, even on a row already spent, because a transcript can end on a line that reads as still working (13 Oct 2026).
Where no stop came, a `standing` row is shown `spent` once its transcript shows a turn that ended, and the first `get_session_subagent` that sees it finished settles it.
**A parent can send a settled subagent more work**, which is no new spawn and no hook, so Fleet looks every two seconds (`subagents_followed`) at the transcripts of the settled subagents of live jobs, a `stat` each until one grows: one whose last line is work stamped after the row settled is `standing` again, and the timeline enters `Subagent resumed`.
Work stamped before the stop opens nothing (14 Oct 2026).

> **Rule.** The thread is drawn and never read as a claim. Nothing Fleet knows about a terminal job, its title or its state, comes from its rows.

A slash command the person ran is one `command` row, as typed, and the summary the CLI writes where it compacted a long conversation is one `compaction` row, so neither reads as the person's words.
What is not drawn: the agent's reasoning, a tool's answer, a command's output, a subagent's own turns, the CLI's bookkeeping (a caveat, a reminder, a task notification) and a message from another job.
No wrapper tag the CLI writes for itself reaches a row.

**A message reaches a terminal by its own mod.** Fleet cannot push into a terminal.
`send_session_message` to a terminal job holds the text, and the `armada` mod in that job holds an ask for `take_held_messages` open (`wait_ms`, up to 25 seconds, one at a time), which Fleet answers the moment something is held, and submits each text as the person's own prompt, which starts a turn whether the job is idle or busy (spike 27).
The ask is also how Fleet knows the job is listening: a send to one with no ask open that has not asked within ten seconds is refused as `fleet.terminal_session_unreachable`, so the person is told at once.
A file or picture is saved by Fleet as for a hosted job and sent as `Attached file: <path>` in the text. A file over 32 KB, a picture aside, is sent with its size and a line to read it in parts, so a pasted response is not opened whole.
The mod reports what the terminal runs on (`tuned`: its model, effort, permission mode and the commands it lists) and Bridge draws those in the composer.
A model or effort chosen in Bridge is held as a command, and the mod runs it as `/model <x>` or `/effort <x>`; the engine refuses a slash command submitted as text, so it goes through the mods API's command call (spike 27).
**The permission mode is shown and not set**: nothing in the mods API switches a live job's, and it is read from the settings-hook inputs at each turn, so it is stale between a change in the terminal and the next turn.
End job works on a terminal job too: *Ending a terminal job*, below.
The sent text shows in the thread when the transcript has it, which is when the turn starts.

## Ending a terminal job

**End job ends a terminal job everywhere**: the agent's process in the
person's terminal ends, the slot it stood in is parked, and the row ends as a
hosted close does, `end_reason` `closed`. Fleet cannot end a process it does not
host, so `close_session` on a terminal job holds an `end` for its mod, in
the answer to the ask it already holds open (`take_held_messages`). The mod runs
it as `/exit` through the mods API's command call, behind any message held with
it, and the engine runs a command only once the job is idle, so a turn
under way, or one a message just started, finishes first. Measured on 12 Oct
2026 against the harness at 2.1.296: `$.command.run({ command: "exit" })` from a
mod ended an interactive `claude` (exit status 0) and fired `session.end` with
`reason` `prompt_input_exit`.

| Moment | What happens |
|---|---|
| The press, mod listening | The end is held and the press is answered with the row, still live |
| The press, mod not listening | Refused as `fleet.terminal_session_unreachable`. Nothing is held, ended or parked |
| The press, mod older than 0.3.10 | Refused the same way: an older mod would drop the end unread. `/reload-plugins` in the terminal fixes it |
| The mod's next answer | The messages, then `end`. The mod runs `/exit` after them |
| The terminal ends | The mod reports `ended`; the row ends with `end_reason` `closed` |
| The slot | Parked once the process holding its lease has gone, as a hosted close parks (the branch it is on, what still runs in it ended first). The mod reports from inside the exit, so Fleet asks the pool again for up to ten seconds. A holder still running then (a person's own terminal, another job's lease) is left alone; a park the pool refuses for another reason is said on Fleet's log. The ledger's rows were given back when the terminal reported its end, so unlike a hosted close they do not wait on the park |

**A terminal that ends on its own is parked too**, the same way and with the
same wait for its process, and its row keeps the terminal's reason:

| Reason the mod reports | Slot |
|---|---|
| `prompt_input_exit`: `/exit`, ctrl+c, ctrl+d | Parked |
| `other`: the window closed (a hangup), another signal, a `-p` run done | Parked |
| `logout` | Parked, once the process has gone |
| `clear`, `resume` | Kept. The same process goes on under another job id, in the same directory |
| None: a crash or a kill no hook outlives | Kept. The job reads Quiet and its slot is held for a resume |

What Fleet remembers about a press is held in memory only, so a Fleet restarted
between the press and the end ends the row with the terminal's reason, and parks
by the table above.

**A terminal's question is an ask in its thread too.** The `armada` mod hooks `AskUserQuestion` and posts the question once with `ask_from_terminal`, which Fleet keeps in its store as the same `ask` row a hosted job writes, so Bridge draws the same card, and answers with the call's id.
The terminal's own prompt opens once Fleet has answered that post, or after a second and a half without an answer, and not at all where Fleet refused the question because a walk is open (*Showing a window*).
The mod then polls the same route with `wait`, each poll held about 25 seconds, so no one request has to outlast the question: a poll that fails or times out is asked again, and a Fleet that restarts finds the question where it was.
A person answering in Bridge has the answer kept until the mod's next poll collects it, and the mod returns it as the tool's result, which drops the terminal's prompt.
A person answering in the terminal first makes the mod send `settled`, which closes the card.
A card whose job ended, or whose mod has not polled for three minutes, is closed too, so Bridge never offers one that cannot be answered.
Fleet out of reach leaves the terminal's prompt alone.
