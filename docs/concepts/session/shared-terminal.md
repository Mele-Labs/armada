# The shared terminal

**Part of:** [Job](../session.md).

A Job has one shell that the person and the Job's agent both type into.
**In the Cockpit it is a Terminal tab**, in a pane like any other.
`+` on a pane's tab row offers Terminal, and the tab opens the shell at the pane's size, or shows the one already open.
`⌥⇧0` brings the Job's Terminal tab forward where it has one, and opens one where it has none, as a tab the Job opens itself does: in the pane that already holds a Terminal, else in a new pane on the right.
The shell stays open until it is ended, the Job ends, or the shell exits.
Closing the last Terminal tab puts the shell away and it keeps running; a Terminal tab opened again shows everything it printed.
A shell the agent opens draws a Terminal tab in the Job on screen, or in the window it already has.
Elsewhere, in a Job with no panes and in a call card, the shell is docked under the thread, where a block's run docks, and Terminal in the Job's Flight control opens it, lit while it shows.
There the tile pressed again puts the pane away with the shell still running, except in its window, where it brings the window to the front.
**End shell** stops the shell, and once it has ended the pane says *Shell ended* and offers New shell, which opens another in the same tab.
A block's run docks in the Job view, and with a Terminal tab open the run and the shell are two panes, the run in the Job and the shell in its tab.
Where a Job has no panes and both are open, the two are tabs in one dock, *Terminal* beside the run's command, and each keeps its own screen.
While the shell has focus, Esc goes to the shell, so a program that reads it still works.

**Move to window gives the shell a window of its own**, one per Job, titled *Terminal* with the Job's title, from the Terminal tab's menu or the pane's head.
The shell shows in one place at a time, so the tab says where it is and offers Move to the pane, and only the window sizes the shell.
Closing the window puts the shell back in its tab and it keeps running, with everything it printed.
Move to the pane in the tab, or Dock on the window's head, brings the shell back and closes the window.
Esc goes to the shell here too, and the window's own close is the way out.
The window has End shell, and New shell once the shell has ended.
A window opened after the shell started shows what it printed so far, since Bridge keeps the last 256 KiB a shell printed until the Job's next shell opens.
Terminal windows close when Bridge quits and are not restored, and every Job's terminal window shares one size.

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

**Bridge hears when it opens or ends.** `terminal.opened` follows the agent's first command into a shell the person had not opened, and Bridge draws the Terminal tab, or shows the pane, then.
`terminal.exited` follows its end, whoever caused it.

**Running a block is unchanged**, and stays the person's own act: one pane, one run at a time, in [Running a block](running-a-block.md).
The shell is a second pane, never a replacement.
Typing in the shell moves nothing on the Job and writes no Evidence.

The fields and the routes are in [Protocol 23.93](../../practices/protocol/protocol-23-93-a-jobs-shared-terminal.md).
