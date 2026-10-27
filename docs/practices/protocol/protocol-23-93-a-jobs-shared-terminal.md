# Protocol 23.93: a Job's shared terminal

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *The shared terminal*.
**Additive only**: new operations, events and types, with the run operations of 23.88 reused unchanged.
There is no number to bump: the protocol ID moves with the files below.

| Where | Carries | Notes |
| --- | --- | --- |
| `open_terminal` (`POST /sessions/terminal/open`) | `OpenTerminal`: `session_id`, optional `cols` and `rows`. Answers 200 `SessionRunUnderway` | The Job's one shell, or the one already open. 80 by 24 where the size is absent, used only where the shell opens now. 422 for a Job that has ended or Fleet has no row for, and for a size of 0. `agent_access: No` |
| `start_terminal_run` (`POST /sessions/terminal/run`) | `StartTerminalRun`: `command`, `dir`, optional `cols` and `rows`. Answers 200 `SessionRunUnderway` | A command run in an existing directory with no Job behind it, such as a fix a pull request panel names. Nobody is told how it ended and no `terminal.*` event follows. 422 for a blank command, a `dir` that is not a directory, and a size of 0. `agent_access: No` |
| `type_in_terminal` (`POST /sessions/terminal/type`) | `TypeInTerminal`: `text`, `enter`, `wait_seconds`, `session_id`. Answers 200 `TerminalTyped`: `run_id`, `printed`, `still_printing` | `enter` absent is `true`. `wait_seconds` absent is 10, held to 1 to 60. Opens the shell where none is open. `printed` is the last 200 lines and 32 KiB at most, escapes and carriage-return overwrites resolved. `agent_access: Yes` |
| `read_terminal` (`GET /sessions/terminal/read?lines=&session_id=`) | Answers 200 `TerminalScreen`: `run_id`, `screen` | `lines` absent is 60, held to 1 to 500. 422 where no shell is open, one that has exited included. `agent_access: Yes` |
| `terminal.opened`, `terminal.exited` on `/events` | `JobTerminal`: `session_id`, `run_id` | `opened` once when the shell starts, not when an open is handed one already open. `exited` once the process has gone and `observe_session_run` has sent `exited`. `agent_access: Bridge only` |

**The Job is the one the connection places when Fleet hosts it**, else `session_id`.
A call that places none is 422, as is a Job `open_terminal` refuses.

**The shell is a run**, so `write_session_run`, `resize_session_run`, `observe_session_run` and `stop_session_run` take its `run_id` as they take any run's.
Its end sends the Job no message, and no `ran` row is made.
A shell that has exited is replaced by the next open under a new `run_id`.

**Skew.** A Fleet before 23.93 has no route and answers 404, and publishes neither event.
A Bridge before it never calls the routes and ignores the two event kinds.
Bridge's half is `packages/protocol/src/sessions.ts` and `packages/protocol/src/events.ts`, written by hand like the rest.
There is no number to take: `armada need --status` showed nothing standing on these paths.
