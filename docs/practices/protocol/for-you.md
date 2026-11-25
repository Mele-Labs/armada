# One Job's For you: `get_for_you`, its four acts and `session.for_you`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026, `armada-ehhu.22`. The native For you tab lists only the open Job's own items: what waits on him, the live Jobs it started, and its own pull requests still open.
**Additive**: five operations, five routes, one event kind and the types they carry. No field on an existing type changes.
Bridge's half is `classic-final:packages/protocol/src/for-you.ts`, written by hand, field for field from `crates/ipc/src/for_you.rs`.

| Where | Side | Carries |
| --- | --- | --- |
| `get_for_you` (`GET /sessions/for_you?session_id=`) | Fleet writes, Bridge and agents read | `ForYou` `{ job, waiting[], building[], landing[] }` |
| `ForYouItem` | Fleet writes | `{ id, kind, title, line?, placed, since, added_by, opens, approves }` |
| `ForYouBuild` | Fleet writes | `{ job, title, standing?, line? }`: a live Job it started, `line` the step it is on. The client draws its state from its own Jobs-list row for `job` |
| `ForYouLanding` | Fleet writes | `{ pr, title }`: one of its own pull requests still open |
| `withdraw_for_you` (`POST /sessions/for_you/withdraw`) | Bridge or the agent writes | `WithdrawForYou` `{ id, session_id? }`, answered with the `ForYou` after it |
| `order_for_you` (`POST /sessions/for_you/order`) | Bridge or the agent writes | `OrderForYou` `{ ids[], session_id? }`, answered with the `ForYou` after it |
| `line_for_you` (`POST /sessions/for_you/line`) | Bridge or the agent writes | `LineForYou` `{ id, line, session_id? }`, answered with the `ForYou` after it |
| `add_for_you` (`POST /sessions/for_you/add`) | The agent writes | `AddForYou` `{ title, kind, opens, line?, session_id? }`, answered with `ForYouAdded` `{ id }` |
| `session.for_you` | Fleet writes, Bridge reads | `ForYouChanged` `{ session_id }`: read `get_for_you` for that Job again |
| Refusals | Fleet | 422 `fleet.for_you_unknown` naming each id that is not a row, `fleet.for_you_blank`, `fleet.session_waiting_unnamed`; 409 `fleet.session_closed` for `add_for_you` on an ended Job |

**Read, not filled in.**
`waiting` is the Job's `waiting_for` on every read, less its `done:` item.
`building` is read off the Job's `started_to` links, and `landing` off its own `pr` attachments.
The store keeps only what is said on top, in `for_you_marks` (a row's place, line and withdrawal) and `for_you_added` (a row for what Fleet cannot see).
A row clears when its item leaves `waiting_for`, so his approval, his answer, a withdrawn walk or sketch and an ended Job clear it without a sweep of its own.

**One identity per row.**
A Fleet item keeps its own id (`walk:<url>`, `sketch:<title>`, `document:<path>`, `draft:<row>`, `plan:<version>`, `ask:<call>`).
An item the agent stated with `waiting_for` is `waiting:<id>`, so it cannot meet one of Fleet's.
A row `add_for_you` added is `added:<n>`.
Two rows whose `opens` are equal are one row, the first listed, so an added row for a walk Fleet already lists is that walk.

**`opens`** is tagged by `kind`: `artifact` with the `JobArtifact.id` for a sketch, page or draft, `walk` with the address, `plan`, and `waiting` with the item's id for every other item.
**`kind`** is `walk`, `document`, `plan`, `draft`, `sketch`, `question`, `run` or `approve`.
An agent item asking for approval is `approve`, one whose act is `run` is `run`, and any other item is `question`.
**`approves`** is the item's own, and true on the plan.

**Order.** Placed rows come first in the order `order_for_you` set, which replaces the one before. The rest follow oldest first.
**Withdraw** keeps the id withdrawn, so the item stays off the tab while it stands in `waiting_for`. It takes the row off For you only: the item stays in Needs you, and nothing is sent to the Job.

**The event stream.** `session.for_you` is published after each act, and after a `session.changed` whose waiting items, own pull requests or started Jobs read differently from the last.
A change to a started Job's title, state, standing or step publishes it for the Job that started it.
Fleet holds the last print of each Job in memory, so the first change after Fleet starts tells once more than it must.

**Skew.** The routes and the event kind are new, so the protocol ID moves.
A Bridge that does not know `session.for_you` drops it as an unknown event.
