# Protocol 23.84: a Job's running turn stopped

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`docs/concepts/session.md`, *Stopping a turn*. **Additive only**: one operation and one type, `StopSession`, which carries a `session_id`.

| Where | Carries | Notes |
| --- | --- | --- |
| `stop_session` (`POST /sessions/stop`) | `StopSession`: `session_id` | Answers the `SessionRecord` with `hosted.turn` `idle`. The process is ended as after a quiet spell and the next message resumes it; a question it was held on is withdrawn; the slot is kept as it stands, nothing parked or committed. An ended job answers its row. 422 `fleet.session_command_refused` for a terminal job, whose process Fleet does not host |

**Skew.** A Fleet before 23.84 has no route and answers 404, so a Bridge that draws Stop shows the refusal. A Bridge before it never calls it. Bridge's half is `packages/protocol/src/hosted-sessions.ts`, written by hand like the rest; the part that offers the act is `stopSession` on the preload.
