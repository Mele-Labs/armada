# Usage a turn at a time

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 17 Oct 2026. What each of a Job's turns cost, so usage can be read by job, model and tool over a window.
**Additive only**: one fact, one operation and the types they carry; no event kind, no error and no field on `SessionRecord`.
Bridge's half is `packages/protocol/src/turn-usage.ts` and `sessions.ts`, written by hand.
The migrations `20261017T1800Z-session_turn_usage.kept` and `20261017T2000Z-session_turn_usage.subagent_bash_use` are additive.
**Names and counts only**: a tool is its name, never its input.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionFact` `turn_measured` | The mod writes, and Fleet itself from the transcripts | `turn`: `TurnUsage` `{ message_id, at, model, input_tokens, cache_creation_tokens, cache_read_tokens, output_tokens, tools?, subagent?, bash_use? }`; `subagent` is left out when false and `bash_use` when the first call was not a shell command |
| `get_session_usage` (`GET /sessions/usage?session_id=&since=&until=`) | Bridge reads | `SessionTurnUsage`: `turns[]` from `since` (inclusive) to `until` (exclusive), oldest first. Either bound is optional RFC3339. 422 `fleet.session_unnamed` for a job Fleet does not know |

**A turn is one row per job and `message_id`.** A harness writes a message more than once as it streams, the output count growing, so a repeat is merged into the row: the larger output wins the figures and the calls are joined.
A repeat publishes no `session.changed`: nothing on a record shows usage.

**Fleet is the source.** It reads the agent CLI's transcripts itself (`docs/concepts/session.md`, *Where the tokens went*) and keeps the turns without the fact; `turn_measured` stays for a reporter that has no transcript.
The mod's `session.measure` is throttled and carries only the context figure, so it cannot be the source.

**Skew.** A Fleet that predates this refuses the fact and the mod goes on; a Bridge that predates it never asks.
