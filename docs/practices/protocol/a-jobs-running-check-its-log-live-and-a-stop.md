# A Job's running Check: its log live, and a stop

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 16 Oct 2026. `docs/concepts/session.md`. **Additive only**: one value on `SessionCheckAct` (`chunk`), one optional field on `SessionCheckAnswer` (`stop`), `SessionCheckStop`, `SessionCheckOutputMessage` and its opening, and three operations, `observe_session_check` (`GET /sessions/checks/:run/observe`, a WebSocket), `stop_session_check` and `force_stop_session_check` (`POST /sessions/checks/:run/stop` and `/force_stop`), all `agent_access` `No`. Fleet and Bridge both carry them; Bridge's half is `packages/protocol/src/sessions.ts`, written by hand. There is no number to take: the protocol ID moves with the wire files, and `armada need` has nothing to order.

**Fleet never reaches the Check's process.** `armada check` runs in the Job's own process, so it tells Fleet what it printed in `chunk` calls (a quarter-second apart) and Fleet answers each with the stop a person asked for. `armada check` then ends its own process group: `SIGTERM` and `SIGKILL` after a grace period, or `SIGKILL` at once, and tells `end` as for any run. Fleet holds a running run's log in memory, up to 8 MiB, and serves it over `api::Follow`, so one reader draws a Job's Check, a merge line's and a Job's.

**Skew.** The IDs differ, so a Bridge and a Fleet that differ are refused regardless. **The event stream is untouched**, so the back-pressure risk below is neither better nor worse: the log has its own socket and reads from a cursor, so nothing on it is dropped. The lifeboat is untouched.
