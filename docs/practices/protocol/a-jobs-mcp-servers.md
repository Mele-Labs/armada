# A Job's MCP servers: `get_session_mcp`, `act_on_session_mcp`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 21 Oct 2026. What a hosted Job's process has connected, and a person's Reconnect, Turn on, Turn off and Sign in on one server.
**Additive only**: two operations, two routes and the types they carry; no event kind, no field on an existing type, nothing kept.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/session-mcp.ts`, written by hand, field for field, and `readSessionMcp` and `actOnSessionMcp` on the BridgeApi.

| Where | Side | Carries |
| --- | --- | --- |
| `get_session_mcp` (`GET /sessions/mcp?session_id=`, `Bridge only`) | Bridge reads | `SessionMcp` `{ servers }`. `SessionMcpServer`: `name`, `state`, `error?`, `scope?`, `tools` |
| `act_on_session_mcp` (`POST /sessions/mcp/act`, `Bridge only`) | Bridge sends, Fleet answers 200 | `ActOnSessionMcp` `{ session_id, server, act }`; answers `SessionMcpActed` `{ servers, sign_in? }`, the servers read again after the act |
| `McpServerState` | | `connected`, `failed`, `needs_sign_in`, `connecting`, `off`. A word the harness says outside these reads as `failed` with the word in `error` |
| `McpServerAct` | | `reconnect`, `turn_on`, `turn_off`, `sign_in`. Anything else is the transport's 400 |
| `McpSignIn` | | `url`, `callback_expected`. Present only on a `sign_in` that needs the person |

**Asked of the Job's process each time, never kept.** The harness pushes nothing when a server's state changes, so no event says to read again: Bridge reads on opening the Loadout and takes the list each act answers.
**A control line is not a turn**: nothing is counted, queued or sent again, and a Job with no process running is refused rather than started.
**An act waits for the server to settle**: ninety seconds for `reconnect` and `turn_on`, thirty for `sign_in`, ten for `turn_off` and for a read.
Bridge's main waits `MCP_ACT_MS`, 110 seconds, so Fleet's refusal arrives first.
**Bridge opens `sign_in.url` with `openLink`**, which takes `http:` and `https:` only.
With `callback_expected` the process reconnects the server itself once the person is done; without it, Bridge asks `reconnect` after.

| Refusal | Status | What a person reads |
| --- | --- | --- |
| `fleet.session_mcp_unreachable` | 422 | The Job runs in a terminal, or has no process running and its next message starts one |
| `fleet.session_mcp_refused` | 422 | The process refused, in its own words, ended before it answered, or did not answer in time |
| `fleet.session_closed` | 409 | The Job was closed, and takes nothing more |
| `fleet.no_such_session` | 422 | No Job Fleet hosts has that id |

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream.** Untouched: both are request and answer, so the unmeasured sink risk is neither better nor worse.
