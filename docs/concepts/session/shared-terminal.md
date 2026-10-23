# The shared terminal

**Part of:** [Job](../session.md).

A Job has one shell that the person and the Job's agent both type into.
Terminal in the Job's Flight control opens it ([Cockpit](../cockpit.md), *The Job's flight controls*), docked where a block's run docks, and it stays open until it is ended, the Job ends, or the shell exits.
The tile is lit while the shell shows, and pressed again it puts the pane away with the shell still running.
Beside a page in the walk panel, where no Flight control is drawn, Terminal is in the Job's header.
Hide puts the pane away and the shell keeps running; End shell stops it.
Once it has ended the pane says *Shell ended* and offers New shell, which opens another, and Close.
With a block's run open as well, the two are tabs in one dock, *Terminal* beside the run's command, and each keeps its own screen.

**Where it opens is the setting `terminal.openJobTerminalIn`**: `docked`, as above and as shipped, or `side panel`.
In the side panel it opens where a pull request or an issue does, headed *Terminal* with the Job's title, and the thread keeps its whole height.
Close puts the panel away and the shell keeps running; Terminal brings it back with everything it printed.
While the shell has focus, Esc goes to the shell, so a program that reads it still works, and Close is the way out.
A change of the setting takes effect at the next Terminal, and a block's run stays docked either way.

**It is an interactive `$SHELL -l` on a pseudo-terminal**, in the Job's slot worktree, or its directory where it holds no slot.
`/bin/zsh` stands where `SHELL` is unset or empty, and `TERM` is `xterm-256color`.
Opening it again, the person's or the agent's, is handed the shell already open.
Ending the Job stops it.
A stop sends SIGHUP, a closing terminal's signal, which a shell leaves on where it ignores SIGTERM, and SIGKILL after 3 seconds.

**It is a run to the code that carries it.** Its keys, size, output and stop are `write_session_run`, `resize_session_run`, `observe_session_run` and `stop_session_run` on its `run_id`.
Output is held in memory as a run's is, the last 256 KiB, and nothing is written down.
Its end tells the Job nothing, because no command was run to be told about.

**The Job's agent types into it with two tools**, both open to the agent.
`type_in_terminal` types text, with Enter after it unless told otherwise, and opens the shell in the agent's worktree where none is open.
It answers what printed after the text once the shell has been quiet for 1.5 seconds, or after `wait_seconds` (10 where unset, 1 to 60), and says whether it was still printing.
`read_terminal` answers the last lines of the screen as plain text, 60 where unset and at most 500.
Ctrl-C is the one character U+0003 with Enter off.
The agent uses the shell for what the person should see or join, a dev server for one; its own shell tool stays for ordinary work.

**Bridge hears when it opens or ends.** `terminal.opened` follows the agent's first command into a shell the person had not opened, and Bridge shows the pane, or opens the panel, then.
`terminal.exited` follows its end, whoever caused it.

**Running a block is unchanged**, and stays the person's own act: one pane, one run at a time, in [Running a block](running-a-block.md).
The shell is a second pane beside it, never a replacement.
Typing in the shell moves nothing on the Job and writes no Evidence.

The fields and the routes are in [Protocol 23.93](../../practices/protocol/protocol-23-93-a-jobs-shared-terminal.md).
