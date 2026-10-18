# A Job's window: origins and a sign-in

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 15 Oct 2026. `docs/concepts/session.md`, *Showing a window*, and `docs/practices/capture-window.md`, *Origins a Job names*.
**Additive only**: optional fields on `ShowWindow` and on the `window` row, and an operation with its DTOs.
No event kind or error code is new.
Bridge's half is `packages/protocol/src/sessions.ts` and `hosted-sessions.ts`, written by hand.
There is no number to take: the protocol ID moves with the wire files, and `armada need --status` showed nothing standing on these paths.

| Where | Side | Change |
| --- | --- | --- |
| `ShowWindow.origins?` (`show_window`) | The agent writes, Fleet checks | A list of `http` or `https` origins, a host and an optional port with no path. A call with another entry is 422, nothing written. Left out when empty |
| `ShowWindow.sign_in?` (`show_window`) | The agent writes, Fleet holds | A list of addresses, each on the shown origin or a named one (422 otherwise). **A secret**: the type's `Debug` prints a count, and the field is in no row, record, event, timeline entry or receipt. Left out when empty |
| `SessionRow::Window.origins?` (`session.row`) | Fleet writes, Bridge reads | The origins, kept when the walk ends as `variations` are. The ledger keeps them too, as the window artifact's `detail.origins`, a JSON string |
| `take_window_sign_in` (`POST /sessions/window/sign_in`) | Bridge asks, Fleet answers | `TakeWindowSignIn` (`session_id`, `url`) answers `WindowSignIn` (`sign_in`). Once: the addresses leave Fleet's memory with the answer, and it forgets them after two minutes. `agent_access` `No` |

**Why a call and not the row.** The row is stored, replayed to every viewer that reads a thread, and folded into the state Bridge's renderer holds, so a login on it would be kept and shown far from the window that needs it.
The call hands it to main once.

**Skew.** The ID moves, so a Bridge and a Fleet that differ are refused and the lifeboat serves.
A Bridge that predates the row field opens the window with no origins, as it has no word for them.
The migration is none: the ledger's `detail` holds strings and `origins` is one more.
