# The shared terminal

**Part of:** [Job](../session.md).

A Job has several terminals, each a shell that the person and the Job's agent both type into.
**In the Cockpit each is a Terminal tab**, in a pane like any other.
`+` on a pane's tab row offers Terminal, and it always opens a new shell, in a tab of its own at the pane's size.
A terminal ends when it is ended, when its tab closes, when the Job ends, or when the shell exits.
A terminal an agent opens arrives as a tab behind the one in front, and takes no focus.
`⌥⇧0` brings a Terminal tab forward where the Job has one, and opens one where it has none, as a tab the Job opens itself does: in the pane that already holds a Terminal, else in a new pane on the right.
**End shell** stops the shell, and once it has ended the pane says *Shell ended* and offers New shell, which opens another in the same tab.
While a terminal has focus, Esc goes to the shell, so a program that reads it still works.

**Each terminal has an ID and a name.**
The ID is `t1`, `t2` and so on, numbered by Fleet in the order the Job opens them, and never used twice in a Job.
The name is *Terminal N* until the person renames it, and a blank name puts the default back.
The ID is what the agent is given and what a tab holds, so it names the same terminal after its shell has ended.

| A tab | Keeps its shell |
|---|---|
| Moves to another pane | The same shell, with everything it printed |
| Is reloaded | The same shell, drawn again by the ID the tab holds |
| Meets a Fleet that restarted | A new shell under the same ID, since a restart ends every shell |

**The tab's menu** has three acts for a terminal, ahead of the acts every tab has.

| Act | Does |
|---|---|
| Send to agent | Writes a terminal chip into the Job's message box and sends nothing. The Thread tab comes forward, opening where the Job has none, and the box takes focus |
| Copy terminal ID | Copies `t2`, or the terminal's own |
| Rename | Makes the tab's name a field, and Fleet keeps the name so the agent sees it |

**Closing a tab ends that terminal.**
A terminal with a program running asks first, in a dialog with End terminal, and an idle one closes straight away.
A tab closed by its ×, a middle click or its menu does not come to the front first.

**Move to window gives a terminal a window of its own**, one for each terminal, titled with the terminal's name and then the Job's title, from the tab's menu or the pane's head.
The shell shows in one place at a time, so the tab says where it is and offers Move to the pane, and only the window sizes the shell.
Closing the window puts the shell back in its tab and it keeps running, with everything it printed.
Move to the pane in the tab, or Dock on the window's head, brings the shell back and closes the window.
Esc goes to the shell here too, and the window's own close is the way out.
The window has End shell, and New shell once the shell has ended.
A window opened after the shell started shows what it printed so far, since Bridge keeps the last 256 KiB a terminal printed until the next shell under its ID replaces it.
Terminal windows close when Bridge quits and are not restored, and every terminal window shares one size.

**A Job without panes keeps one terminal, the Job's first.**
In a Job with no panes and in a call card, that terminal is docked under the thread, where a block's run docks, and Terminal in the Job's Flight control opens it, lit while it shows.
The tile opens a new shell where none is running, and pressed again it puts the pane away with the shell still running, except in its window, where it brings the window to the front.
A block's run docks in the Job view, and with a Terminal tab open the run and the shell are two panes, the run in the Job and the shell in its tab.
Where a Job has no panes and both are open, the two are tabs in one dock, *Terminal* beside the run's command, and each keeps its own screen.
A shell the agent opens draws in the Job on screen, or in the window it already has.

**Each is an interactive `$SHELL -l` on a pseudo-terminal**, in the Job's slot worktree, or its directory where it holds no slot.
`/bin/zsh` stands where `SHELL` is unset or empty, and `TERM` is `xterm-256color`.
Opening a terminal by its ID is handed the shell already open under it, or a new shell under the same ID where it has ended.
Ending the Job stops every one.
A stop sends SIGHUP, a closing terminal's signal, which a shell leaves on where it ignores SIGTERM, and SIGKILL after 3 seconds.

**Each is a run to the code that carries it.** Its keys, size, output and stop are `write_session_run`, `resize_session_run`, `observe_session_run` and `stop_session_run` on its `run_id`.
Output is held in memory as a run's is, the last 256 KiB, and nothing is written down.
Its end tells the Job nothing, because no command was run to be told about.
Fleet keeps a row for each terminal for as long as the Job lives, with its name and the shell its agent last saw, so an ID is never given twice and a name outlives the shell.

**The Job's agent uses its terminals with three tools**, all open to the agent.
`list_terminals` names the open terminals, lowest number first, each with its name and whether a program other than the shell has the foreground.
`type_in_terminal` types text, with Enter after it unless told otherwise, and `read_terminal` answers the last lines of the screen as plain text, 60 where unset and at most 500.
Both take `terminal`, and both answer which terminal they used.

| The call | Means |
|---|---|
| `terminal` named | That terminal. Refused where it is not open, naming `list_terminals` |
| `new` | A new terminal of the agent's own, opened first, then typed into. Refused with `terminal` |
| Neither, typing | The agent's last used terminal, where it is open. Where it has ended, a new one opens and nothing is typed, and the refusal names both. Where the agent never used one, a new one opens and the text is typed into it |
| Neither, reading | The agent's last used terminal where it is open, else the lowest open |

**Typing never falls to the person's terminal.**
An agent that has used none opens its own, so the shell a person is working in is only typed into when the agent is given its ID or names it.
The person hands an ID over with Send to agent, and the agent passes it as `terminal`.
`type_in_terminal` answers what printed after the text once the shell has been quiet for 1.5 seconds, or after `wait_seconds` (10 where unset, 1 to 60), and says whether it was still printing.
Ctrl-C is the one character U+0003 with Enter off.
The agent uses a terminal for what the person should see or join, a dev server for one; its own shell tool stays for ordinary work.

**A watch may name a terminal.** `in_terminal` on `start_watch` takes `terminal`, and unnamed means the agent's last used terminal when the watch starts, else the lowest open.
The watch keeps that terminal for its life.

**Bridge hears when a terminal opens or ends.** `terminal.opened` follows the agent's first command into a terminal the person had not opened, and Bridge draws its Terminal tab behind, or shows the pane, then.
`terminal.exited` follows an end, whoever caused it.
Both carry the terminal's ID, so a Job with three shells publishes up to three of each.

**Running a block is unchanged**, and stays the person's own act: one pane, one run at a time, in [Running a block](running-a-block.md).
A terminal is a pane beside it, never a replacement.
Typing in a terminal moves nothing on the Job and writes no Evidence.

The fields and the routes are in [A Job's several terminals](../../practices/protocol/a-jobs-several-terminals.md), over [Protocol 23.93](../../practices/protocol/protocol-23-93-a-jobs-shared-terminal.md), which holds the run routes.
