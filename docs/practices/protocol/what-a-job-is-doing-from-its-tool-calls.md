# What a Job is doing, from its tool calls

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Oct 2026. `docs/concepts/session.md`, *Working on*.
**Additive only**: one optional field on `SessionRecord` and three types.
No operation, event kind or error is new.
Bridge's half is `packages/protocol/src/session-activity.ts` and `sessions.ts`, written by hand.
There is no number to take: the protocol ID moves with `crates/ipc/src/session_activity.rs`.

| Where | Carries | Absent |
| --- | --- | --- |
| `SessionRecord.activity` (`session.changed`, `get_session`, `list_sessions`) | `SessionActivity`: `now?`, `todos[]` | Before a hosted Job's first tool call, on a Job a terminal runs, and once it ends |
| `SessionActivity.now` | One short fact for the latest call Fleet can name: `Editing gate.rs`, `Reading gate.rs`, `Running cargo check`, `Searching`, `Fetching`, `Running a subagent` | A call Fleet cannot name leaves it as it was; the field is left out until one is |
| `SessionActivity.todos` | `SessionTodo`: `text`, `state` (`done`, `doing` or `todo`) | Left out while empty |
| `SessionActivity.account` (since 22 Oct 2026) | `SessionAccount`: `for`, `done[]`, `now?`, `next?`, `group?`, `at` | Until a small model has written one for the person's last message; cleared when a new message from him starts a turn |
| `SessionActivity.live` (since 22 Oct 2026) | `SessionLive`: `url`, `title` | Until the Job shows a page or serves one after the person's last message |

**`for` is the id of the person's message the account answers**, and `group` the id of the approved plan group it is on, `SessionPlanGroup.id`.
`done`, `now` and `next` are the model's words and carry no file name, path, code identifier or tool name; a line that does is dropped before it is kept.
The account is rewritten at most every few seconds and reads the turn so far, and it stays after the turn ends.
`live` is the newest of a page the Job showed with `show_window` and the address of a server it started with `start_server`.

**`state` is a plain enum Bridge matches on**, as `WaitingMode` is, because no registry declares the set.
**The gate that detects it is `gate_session_call`**, and its route is unchanged: Fleet notes the call before it decides it.

**Publishing is coalesced.** A change to `now` or `todos` publishes `session.changed` at once when none went out for the Job's activity within two seconds, and otherwise once, when the two seconds are up.
A call that changes neither publishes nothing.
A turn starting clears `now` on the row that announces it and keeps `todos`.
No call time is carried: a silent turn is the `stuck` state.
A Fleet restart starts the field empty.

**Skew.** A Bridge that predates this ignores an `activity` it does not read.
A Fleet that predates it sends none, and Bridge treats that as no activity.
The same change moves `HostedSessions::gate_session_call` to `self: Arc<Self>` in `crates/api`, which is Rust only and not on the wire.
