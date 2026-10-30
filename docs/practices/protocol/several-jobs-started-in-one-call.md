# Several Jobs started in one call: `jobs` on `start_another_session`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 30 Oct 2026. A lead Job that splits a request into several Jobs starts them with one call and gets every id back in order.
**One field in and one answer type out**: no operation, route, event kind or error code.
The protocol ID moves with `crates/ipc/src`, the operation file and `packages/protocol/src`, and there is no number to take.
Bridge's half is `StartAnotherSessions`, `SessionsHeld` and `HeldOrRefused` in `packages/protocol/src/sessions.ts`, written by hand. Bridge sends neither: the call is an agent's.

| Where | Side | Carries |
| --- | --- | --- |
| `StartAnotherSessions` (the body of `POST /sessions/start_another`) | Agent sends | The one-Job fields of `StartAnotherSession`, each now optional, and `jobs`, a list of `StartAnotherSession`. A body gives one or the other: one Job's fields beside `jobs` is refused 400, and so is a one-Job body missing `manifest_id`, `title` or `message`, naming the field and the body's whole shape |
| `SessionsHeld` | Fleet answers, to a body with `jobs` | `jobs`: one `HeldOrRefused` per Job in the order given |
| `HeldOrRefused` | Fleet answers | `held` (a `SessionHeld`) where the Job started, or `refused` (the `WireError` its own call would have answered). Exactly one is present |

**A body without `jobs` is answered as before**, with `SessionHeld` and the same refusals.
**Each Job in `jobs` is started by `start_another_session` itself**, one after another, through `Sessions::start_other_sessions`, a provided method on the daemon trait: the same placing of the caller, the same linking (`started_by` and `started_to`) and the same refusals.
**A refused Job leaves the others started**, so the call answers 200 whatever its Jobs answered, with one exception.
**A call that places no hosted Job is refused whole**, with the 422 `fleet.agent_start_unplaced` a one-Job call gets.
Placement is the caller's, so the route reads it off the first Job's refusal and tries no other.
The code is declared once, as `ipc::AGENT_START_UNPLACED`, for the route to match on.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** Each Job publishes what one call's Job publishes, so a batch of N is N calls' events arriving together, as N calls in a row would send them.
The unbounded sink and the missing resync message are untouched.
