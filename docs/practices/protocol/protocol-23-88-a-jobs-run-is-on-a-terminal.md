# Protocol 23.88: a Job's run is on a terminal

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Running a block*.
**Two things change that exist, two operations are new.** Both operations are `agent_access: No`: running a block and typing into it are the person's own acts.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session_run` | `StartSessionRun` gains optional `cols` and `rows` | The command runs on a pseudo-terminal of that size (80 by 24 where absent) with `TERM=xterm-256color`, as the job leader with the terminal as its controlling terminal. 422 for a `cols` or `rows` of 0 |
| `observe_session_run` | `SessionRunMessage` loses `lines` and gains `output` (`text`) | `text` is the terminal's bytes as UTF-8 with the escapes intact, sent as they arrive. A character a read ends inside waits for the next message, so none is split. A late viewer's first `output` is the held tail, the last 256 KiB, replacing the last 2000 lines |
| `write_session_run` (`POST /sessions/run/write`) | `WriteSessionRun`: `run_id`, `data`. Answers 200 `SessionRunUnderway` | `data` is written to the terminal as it is. Ctrl-C is the byte 3 and reaches the command as SIGINT. 422 on a run that has ended or Fleet does not hold, the shape a stop has |
| `resize_session_run` (`POST /sessions/run/resize`) | `ResizeSessionRun`: `run_id`, `cols`, `rows`. Answers 200 `SessionRunUnderway` | 422 as `write_session_run`'s, and for a size of 0 |

**The Job is told when the run ends**, as Fleet tells a Job anything: the command, `Exit <code>.` or `Stopped.`, and the last 40 lines with escapes and carriage-return overwrites resolved.
A `run` item in the Job's `waiting_for` whose target is the command, trimmed, is taken off the list in the same moment when the exit is 0. Bridge no longer answers it.
A Job that has ended is not told.
*Protocol 23.89 keeps a hosted thread's telling as a `ran` row.* A slash command sent to a terminal Job is no run, and Bridge still settles its item.

**Skew.** A Fleet before 23.88 sends `lines`, which a newer Bridge ignores, and answers 404 to the two routes.
A Bridge before it ignores `output`.
The IDs differ, so the two are refused regardless.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.
There is no number to take: `armada need --status` showed nothing standing on these paths.
