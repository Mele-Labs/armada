# Protocol 23.76: a shell block run from a Job's thread

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`. **Additive only**: three operations, all `agent_access: No`, and their types. No type that exists changes.

| Where | Carries | Notes |
| --- | --- | --- |
| `start_session_run` (`POST /sessions/run/start`) | `StartSessionRun`: `session_id`, `command`. Answers 202 `SessionRunUnderway`: `run_id` | Runs `$SHELL -lc <command>` (`/bin/zsh` where `SHELL` is unset or empty) in its own process group, in the worktree of the slot the Job holds, else the Job's directory. Output and errors are one stream. 422 `fleet.session_run_refused` for a blank command, an unknown Job, an ended one, or a directory the command cannot start in |
| `stop_session_run` (`POST /sessions/run/stop`) | `StopSessionRun`: `run_id`. Answers 200 `SessionRunUnderway` | SIGTERM to the group, SIGKILL after 3 seconds. 422 on a run not in flight |
| `observe_session_run` (`GET /sessions/run/:run_id/observe`, WebSocket) | `SessionRunMessage`, tagged by `kind`: `opened` (`protocol_id`, `run_id`), `lines` (`lines[]`), `missed` (`dropped`), `exited` (`code`: integer, or `null` where the run was stopped or signalled) | `exited` is sent once, last, then the socket closes. 422 on a run Fleet does not hold |

**Nothing is written down.** A run's output is held in memory while it lives and for 90 seconds after it exits, so an observer that opens late replays it and then `exited`.
After that the run is unknown.
*Protocol 23.88 moved the run onto a terminal and replaced `lines` with `output`.*

**The event stream.** None of this rides `/events`.
Each observer has a bounded broadcast of 256 messages and is told `missed` with how many it lost, so a slow viewer does not slow the run and does not believe a gap is whole; on `missed` it should reopen the socket, which replays the held tail.
This is the bounded-with-a-count shape the open WebSocket risk asks for, one subject over.

**Skew.** A Fleet before 23.76 has neither route and answers 404, so a Bridge that offers the block has nothing to call.
A Bridge before it never asks.
Bridge's half is `packages/protocol/src/sessions.ts`, written by hand like the rest.
