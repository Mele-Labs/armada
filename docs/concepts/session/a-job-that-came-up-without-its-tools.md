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

**The reading and the mark are kept in memory only.** A Fleet that restarts has neither until the next `init`.
