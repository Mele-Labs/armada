# Running a block

**Part of:** [Job](../session.md).

A fenced shell block in a Job's thread shows Run when the block is hovered or focused.
Pressing it opens a terminal docked in the Job's view, between the thread and the message box, and runs the command in the Job's working directory, its slot if it has one.
A Run key on a Needs-you item opens the same pane, and so does a message that is one line starting with `!` (`! rbenv install 4.0.6`) sent from the Job's box: it is run, not sent to the agent, and when it ends Fleet tells the Job as for any run.
A bare `!` or a message of several lines is sent as written.
There is one pane and one run at a time: Run again stops the old run first and starts afresh.
The pane's header carries the command and Stop while it runs; after, a mark with the exit code on its tooltip (*Exit N*, or *Stopped*), Run again and Close.
Close puts the pane away, and the block's mark brings the same run back.
Keys typed go to the program while it runs, and its size follows the pane's.

**It is the person's own act, and the command runs on a terminal.** It is the job leader of a pseudo-terminal with `TERM=xterm-256color`, so a program sees a tty, keys written with `write_session_run` reach it (Ctrl-C as an interrupt) and `resize_session_run` changes the size it sees.
Output is the terminal's bytes with the escapes intact, held as the last 256 KiB.
The pane is xterm.js (`@xterm/xterm`), drawing those bytes as they arrive; its keys go out as `write_session_run`, its fitted size as `resize_session_run`, and the run starts at the size the pane first fits (`cols` and `rows` on `start_session_run`, `POST /sessions/run/start`).
Bridge reads what it prints on `observe_session_run`, a socket of its own, until `exited`.
A Job that has ended, or a run Fleet does not know, is refused in the same words as any other act on a Job.

**When the run ends, Fleet tells the Job** as it tells it anything: the command, `Exit <code>.` or `Stopped.`, and the last 40 lines as a person would have read them, escapes and carriage-return overwrites resolved.
**A hosted Job's thread keeps that as one `ran` row** (command, exit code or stopped, the plain tail), and the agent is sent the same words once, with no message from Fleet beside the row.
A terminal Job cannot be written to, so its mod is handed the words as a message (`told_by_fleet`, held for it).
**A run that exits 0 also takes the waiting item whose `run` act names that command off the Job's list**, from the thread or from the Needs you band, in the same moment, and the message says so; Bridge answers nothing.
A Job that has ended by then is not told.

The screen is not kept past the run. Closing the window, or the Job, ends the reading; a reopened thread shows the block without it, and a hosted thread keeps the run's `ran` row with its tail.

**A block with no language is a shell block when it reads as one** (`readsAsShell`): every line opens like a command, after an optional `$ `, and none reads as code, JSON, prose or a diagram.
A doubt is no Run.
In a **terminal** Job, inline code that is exactly a command the terminal lists (`` `/update-armada` ``, `` `/model opus` ``) is pressable instead: it is run in the terminal as typed with `run_session_command` (`POST /sessions/command`), no output comes back, and a check says Sent.
A Fleet-hosted Job has no terminal, so the code stays plain.
A waiting item whose `run` target is such a command takes the same path: Run in the Needs-you list sends it to the terminal instead of a shell, and a Sent settles the item.
