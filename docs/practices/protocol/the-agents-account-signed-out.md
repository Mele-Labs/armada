# The agents' account signed out: `agent_sign_in.changed`, `get_agent_sign_in`, `start_agent_sign_in`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

**Additive only**: two operations, one event kind, one type.
The protocol ID moves with `crates/ipc/src`, the operation files, the route table and `packages/protocol/src`, and there is no number to take.
Bridge's half is `packages/protocol/src/agent-sign-in.ts` and `events.ts`, written by hand.
The names are the vendor's nowhere: the gate refuses a vendor literal outside `adapters`.

| Where | Carries | Notes |
| --- | --- | --- |
| `get_agent_sign_in` (`GET /agent/sign_in`) | `AgentSignIn`: `signed_out_since?`, `signing_in` | Absent `signed_out_since` is signed in. `signing_in` is left out when false |
| `agent_sign_in.changed` (event) | `AgentSignIn`, whole | Published on every move. Served and published, so a Bridge opened later asks |
| `start_agent_sign_in` (`POST /agent/sign_in`) | no body; answers `AgentSignIn` | Starts Fleet's one sign-in run. A press while one runs starts nothing and answers the state |

**One state a Fleet, in memory only.** Every Job shares the account, so the state names no Job.
`signed_out_since` is the instant the first Job was heard signed out, and a repeat failure keeps it.
**It clears two ways**: the sign-in run exiting 0, or the model answering in any Job while the Fleet is signed out.
Either wakes every Job the expiry stopped with `Continue`, under the `sign_in_wake` notice; the Job that answered is not woken.
A sign-in that exits otherwise leaves the Fleet signed out with `signing_in` false.

**A Job in a terminal reports it through the mod** as the `signed_out` fact, with no fields, which marks the Job as a hosted one's harness line does.
Its next `turn_completed` is the account working again, as a hosted Job's answer is.

All three are `Bridge only`: the sign-in opens the person's browser, and only his press starts it.

**The event stream.** One more kind, sent when the account signs out or in and when a sign-in starts or ends, which is rare.
A Bridge that missed one reads `get_agent_sign_in` on the resync that follows a `Missed`.
