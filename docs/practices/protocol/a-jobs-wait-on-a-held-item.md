# A Job's wait on a held item: `SessionRecord.claim_waits`, `claims_unsent`

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Since 14 Nov 2026. A Job that takes a tracker item another Job holds, on this Mac or another, or one a person is assigned in beads, now waits on it, and the wait is drawn on the Job.
**Additive only**: two optional fields out on `SessionRecord`, the two types they carry and one notice word; no operation, route or event kind.
The protocol ID moves with `crates/ipc/src` and `packages/protocol/src`, and there is no number to take.
The Rust half is `ClaimWait` in `crates/ipc/src/sessions.rs`; Fleet's is `crates/fleet/src/job_claims.rs`.
Bridge's half is `packages/protocol/src/sessions.ts` and `notices.ts`, written by hand, field for field.
Electron Bridge reads neither field; native Bridge draws the wait line.

| Where | Side | Carries |
| --- | --- | --- |
| `SessionRecord.claim_waits` | Fleet answers | `ClaimWait[]`, each item the Job waits on. Absent where it waits on none |
| `SessionRecord.claims_unsent` | Fleet answers | `true` where the Job holds claims its project's store has not taken, because the store did not answer. Absent where all are sent |
| `ClaimWait` | Fleet writes | `item`, the tracker's name for it; `holder`, the holding Job's forge login or the beads assignee; `machine` and `title`, the holding Job's Mac and title, absent for a person; `since`, when the wait began; `ends` |
| `ClaimWaitEnds` | Fleet writes | `claim_ends` for another Job's claim, `unassigned` for a person in beads |
| `Notice` `claims` | Fleet writes | The kind of each word Fleet tells a Job about its claims: a wait begun or ended, or a file another Job writes too |

**How long it has waited is `since`**, and the client counts from it.
**What ends it is `ends`**, without words, so the client says it in its own.
Either way Fleet wakes the Job with a word when the wait ends; there is no act on the line.

**Skew.** The IDs differ, so a Bridge and a Fleet either side of this change refuse one another and the lifeboat serves.
**The event stream is neither better nor worse.** `session.changed` already carries the record whole, and a pass publishes it only where a wait or `claims_unsent` moved.
