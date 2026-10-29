# A Job that came up without its tools

**Part of:** [Job](../session.md).

**A hosted Job's agent can start without the `armada` mod, without Fleet's door, or without task tools, and the CLI says so on the first line of every turn.**
Fleet reads that `init` line and keeps three facts from it: whether the mod is among the loaded plugins, whether the door is among the MCP servers and `connected`, and whether the CLI serves `TaskCreate` or the older `TodoWrite`.

**What is missing is on the Job's record, and absent when nothing is.**
`tools_missing` names the CLI's version and the pieces: `mod`, `door`, `task_tools`.
The mod counts as missing whenever the line lists no `armada` plugin, whether or not the launch passed `--plugin-dir`.
Each `init` replaces the last, and the mark stays after the process is let go, so it says how the last one came up.

**A missing door, or a missing mod where the launch passed `--plugin-dir`, is mended by one restart.**
Fleet ends the process and starts a replacement that resumes the conversation, as it does for a process that was gone before it took its lines, and sends it the lines the old one had not finished.
A Job with no conversation yet starts fresh under the same id.
Missing task tools are marked and never mended: a restart gives the same CLI.

**The restart is once for the process Fleet started it for.**
The replacement is remembered by its generation, so if it comes up without the piece too, the mark stays and nothing loops.
A process started later for any other reason is mended again.

**The restart cuts into the turn.**
The old process had said `init` and not yet answered, so the turn's message is sent again to the replacement and the turn starts once, as far as the thread and the timeline can tell.
The replacement resumes the conversation and pays for it again, and the subagents and background work of earlier turns go with the old process, as they do in any restart (*What a restart does to a session*).
A line that finds Fleet with nothing unfinished to send, or the Job waiting to be moved into a slot, is marked and not restarted.

**Restart the agent is the person's own press, and has no once-only guard.**
The tools band and the mark's menu offer it for the `mod` piece, and it calls `mend_session_tools`.
Fleet fills its copy of the mod first, then lets the process go and starts a replacement that resumes the conversation, sending it the lines the old one had not finished.
A Job with none has no turn to carry the restart, so Fleet tells the replacement `Fleet restarted your process with the armada mod. Nothing is asked of you.`: news, which leaves the agent's `all_done` standing, kept from any mute or rule since it answers the press.
A Job waiting to be moved into a slot, or paused, is left as it is, because the move or the resume starts the process that loads the copy.
The answer comes once the restart is under way, and the mark clears when the new process's own `init` line says nothing is missing.
A copy that cannot be filled refuses the press with the script's words and restarts nothing; a terminal Job is refused too, since Fleet does not host its process.

**Bridge draws the mark as the `unplug` glyph**, on the Job's row in the Jobs list and in its header, with a tooltip that is the label: the CLI's version, then a sentence for each piece missing, such as `Claude Code 2.1.312. No armada mod. No task tools.`
The row's inspect card carries the same sentence as its Tools line.
The row's column is kept for every row while one in the list has the mark, and an ended Job draws none.

**The task list reaches the working band through the mod.**
Current CLIs serve `TaskCreate` and `TaskUpdate`, which Fleet's gate sees without their results, so the band reads the whole list the mod reports as `tasks`; a `TodoWrite` the gate sees still sets it on an older CLI.

**The reading and the mark are kept in memory only.** A Fleet that restarts has neither until the next `init`.
