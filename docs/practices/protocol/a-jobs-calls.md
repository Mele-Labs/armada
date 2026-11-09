# What calls the person: `get_calls`, `calls.changed` and `open_call`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 9 Nov 2026. Fleet reads which things call the person, which of them he has not opened, and what began to wait on him since, and the native Bridge and Pocket read it instead of working it out.
Electron Bridge works it out in TypeScript, in the renderer for the calls and the Dock count and in the main process for the banner and the sound, and keeps doing so for now.
**Additive only**: two operations, one event and the types they carry; no field on an existing type.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `crates/ipc/src/standing_calls.rs`; the reading is `crates/fleet/src/standing_calls.rs`, which hears every `session.changed` and `merge_line_view.changed`.
Bridge's half is `packages/protocol/src/standing-calls.ts`, written by hand, field for field.
The files are not `calls.rs` and `calls.ts`, since `crates/ipc/src/call.rs` and `packages/protocol/src/calls.ts` are the permission calls', and `Call` and `CallKind` are taken in both.

| Where | Side | Carries |
| --- | --- | --- |
| `get_calls` (`GET /calls`, `agent_access = "No"`) | Bridge reads | `StandingCalls { calls }`, oldest first |
| `calls.changed` | Fleet writes, Bridge reads | `CallsChanged { calls, arrived }`: the calls whole, and the news since the last publish |
| `open_call` (`POST /calls/open`, `agent_access = "No"`) | Bridge writes | `OpenStandingCall { key }`, answered with `StandingCalls` |
| `StandingCall` | | `{ key, owner, kind, session_id?, title, text, since, tone, opened }` |
| `CallArrival` | | `{ session_id, title, text, tone, kind }`, `kind` being `needs` or `stopped` |

## What moved, and from where

Each reading is a port of Electron's, check for check, and the order of the checks is the rule.
A change to one side is a change to the other until Electron reads this too.

| What | Electron | Fleet |
| --- | --- | --- |
| A Job's items that call him | `callsFromWaiting`, `touchedAt`, `waitingOf` and `BLOCKING` in `cockpit/waiting.ts` | `Keeper::job_calls` |
| A Job's row where it needs him | the needs call in `shipOf` (`cockpit/deck/ship.ts`) | `Keeper::job_calls`, and `marking::needs_him` |
| Main red, a held checkout, and a pull request of his | `lineItems` in `Dashboard.tsx`, on the `command-central` tab | `Keeper::line_calls` |
| Who is already on a failure | `ownerOfCall`, `isOnIt` and `failedAt` in `cockpit/owner.ts` | `Keeper::pull_owner` and `Keeper::on_it` |
| One unopened entry for each owner, the Dock count | `incomingOf`, `callOwnerOf` and `needingOf` in `cockpit/deck/store.ts` | `opened` on each call, and `StandingCalls::unopened` |
| What is news | `SessionAttention` in `main/session-attention.ts` | `Keeper::hear` |

**`touchedAt` is read against `viewed_at`.**
The last instant he was in a Job is `SessionRecord.viewed_at`, or now while the Job is on screen ([a Job's row mark](a-jobs-row-mark.md)).
Fleet counts a message he sends and a card he answers as a visit, so Electron's own last-message half of `touchedAt` is in `viewed_at`.
An item older than that is no call, except a `document`, which waits on his OK however long he has been in the Job.

**A call is opened** when the Job is on screen or he has been in it since the call began, unless the Job asks him: its mark reads `waiting`, `stopped_asks` or `unstarted`, or it is held on its start card.
A Job that asks him stays unopened once visited, as `needingOf` has it.
A main, checkout or pull request call is opened by `open_call`, which Fleet keeps in memory.
Electron's `visited` and `held` are window memory and are lost on a reload; Fleet's are lost on a restart, which matches.
A Job call is opened by `view_session`, so `open_call` on a `session:` key is a no-op success, and so is a key that names no call standing.

**A call's owner** is `callOwnerOf`'s string: `session:<id>` for a Job's calls and for a failure whose Job holds the pull request and is not on it, else the key.
A badge counts each owner with an unopened call once.
`StandingCalls::unopened` does it for a Rust client, newest first.

**What is news** is an item newly entering a live, unpaused Job's `waiting_for`, or the start card as id `start` with the text `Not started`, once.
What stands when Fleet starts only seeds.
An item posted again under its id is the same item, and nothing is told for a paused Job.
What a paused Job held stays read, so Resume does not tell it again; an item first posted while it is paused is not on the wire, since Fleet lists nothing for a paused Job, and is told on Resume.
Its tone is `blocked` where the source is one `BLOCKED` names, else `waiting`.
Merge-line calls never arrive as news, and nothing is told for a merge or a finish.

## Every call the list raises

The Dock count is the unopened calls of `useItems("command-central")` and the needs calls `shipOf` adds to them, so each kind is here.
A call that has been answered or dismissed for good is a window's own memory and stays the client's.

| Kind | `key` | `title` and `text` | Raised when |
| --- | --- | --- | --- |
| `job`, an item | `session:<id>:<item>` | the Job's title, and the item's text | a live, unpaused Job lists it, and he has not been in the Job since it began, or it is a document; none while a turn works unless the item holds the turn |
| `job`, the Job's row | `session:<id>:needs:<last turn>` | the Job's title, and what its mark says | no item of the Job calls, and the Cockpit files the Job under needs him: its mark asks (`waiting`, `done`, `unstarted`, `stopped_asks`, `stuck`, `failing`, an owed walk), or a pull request of its own failed while it sits idle; or it is held on its start card |
| `main` | `main:<root>` | `main is red`, and the failed Check and test | main reads red with nothing newer running, and the Job that broke it is not on it |
| `checkout` | `checkout:<root>` | `<n> commits behind <base>`, and why it is held | a served checkout is held back |
| `pull`, failing | `pull:<number>` | `#<number> <branch>`, and `checks failed` or `unmergeable` | a pull request of his failed or cannot merge, and its Job is not on it |
| `pull`, by hand | `pull:<number>` | `armada.yml edit #<number>`, and `Needs merging by hand` | the forge would not auto-merge an `armada.yml` edit, and its Job is not on it |

**A Job that needs him stays a call after he has seen its item**: the item leaves the list, and the row's own call takes its place.
It is opened once he has been in the Job since its last turn ended, unless the Job asks him.
**A Job Fleet gave up restarting** is news too, with `kind` `stopped`, the tone `waiting` and the words `stoppedSaid` in `packages/screens/src/session-state.ts` gives its reason, and a call as any Job that needs him is.

## Where Fleet's reading differs from Electron's

- **A stopped Job after Fleet was down** says `Stopped: Fleet was down` and no range of times, since the clock a range is spelt in is the client's.
- **The needs call's words** for a Job that is `limited`, runs a command, starts `after` another Job, or was woken, say the state plainly, since the clock, the task name and the condition are the client's: they reach the call only through a failing pull request on an idle Job.
- **A Job's own pull request** is read from its `pr` attachments, as the row mark reads it, where Electron prefers the merge line's listing of it.
- **A Custom tab's Mod Job that asks** does not mark the Jobs that hold the tab. Mods come after the switch to the native app.
- **A failure's `since`** is the instant main first read red or the pull request's `updated_at`, which `failedAt` reads, and the epoch where the forge gave none. Electron's own items carry no instant.
- **A failure's `tone`** is `waiting`, as it has no source that holds a turn.
- **Whether a Job was told of a failure** is read from its thread: a message of his, or another Job's or Fleet's, after the failure arose.
  Electron reads the same rows.
- **A pull request's key** is `pull:<number>` with no repository, as Dashboard has it, so two repositories listing one number share a key.
- **What he opened stays opened** while Fleet runs, even once the failure clears and returns, as Electron's does until a reload.
- **Order** within one instant is the key's bytes, where Electron's is `localeCompare` on the item's id.

## What stays the client's

- The two-second window that groups arrivals into one banner: Fleet publishes each arrival, and the client collects them.
- Which sound plays, and the playing of it.
- The banner, its text over several arrivals, and where pressing it lands.
- The Dock badge write, and the count's shape.
- A call he dismissed for good, which is a window's own memory.
- Whether the window is on screen and he at the machine: Fleet counts the places that sent `view_session` `on`.

## The event

`calls.changed` is published when the computed calls or the news differ from the last publish, off every Job's row and every merge line, and not on every ledger tick.
`arrived` is empty, and absent on the wire, where only openedness or the list moved.
A resync does not carry it: a client that reconnects reads `get_calls`, and what it missed is not told.

## Skew and the event stream

The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
It adds one event kind, which an older Bridge does not read.
