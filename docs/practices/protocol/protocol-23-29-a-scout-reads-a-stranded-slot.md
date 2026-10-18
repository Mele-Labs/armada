# Protocol 23.29: a Scout reads a stranded slot

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: when a worktree slot is stranded, send an agent to look at the work, then
choose to scrap it or stash it. `../concepts/scout.md`, *Starting from a stranded slot*, and
`../concepts/fleet.md`, *Rescuing a stranded slot*.

**Additive only.** One command, six DTOs and two optional fields on `WorktreeSlot`; a 23.18 Bridge
reads neither field and sees exactly what it did.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `rescue_slot` | `POST /worktrees/slots/rescue?manifest_id=` | `RescueSlot { act, slot }` in, `SlotRescued { manifest_id, slot, branch?, branch_kept, committed? }` out. `act` is `start`, `stop`, `scrap` or `stash` | — |
| `stranded` | `WorktreeSlot` | `SlotStranded { uncommitted, commits, unpushed }`: what a Scrap would lose. Each commit is `{ sha, subject, home }`, `home` being `only_here` (on no remote branch and not on the local base), `on_remote` or `on_main` | A slot that is not stranded |
| `rescue` | `WorktreeSlot` | `SlotFinding { state, commit, uncommitted, cut, read, searched, verdict?, items?, summary?, why?, cost_micros? }`, `state` being `reading`, `answered`, `stopped` or `failed`. `verdict` is `unfinished` or `scraps`; `items` is what is left to do under the first and one line of leftovers under the second. `summary` is the Scout's own words, kept only where its answer was not that shape | No Scout has read it, or it has moved off the commit read |

**A Job's slot the Job could not give back is on the wire as kept.** `SlotHolding::Job` gains
`job_status` (where the Job ended), `kept` (why its release was refused, so its work is still in
the slot) and `completed`, all optional or defaulted. A kept slot carries `stranded` too, and
`rescue_slot` acts on it as on a stranded one; each act ends the Job's claim, so the slot is free
after.

**A bay a hosted Session holds is on the wire as that Session.** A Session's lease is written as a Job of
its own id, so `WorktreeSlot.held` was `job` with a Session id and Bridge drew the id. Fleet now serves it as
`SlotHolding::HostedSession { session_id, session_title? }`, and gives a process holder
(`SlotHolding::Session`) the optional `session_id` and `session_title` of the hosted Session whose agent started
it, found by walking down from each Session's agent (or its `session-keep`) to the holder's pid. Both are
additive or a new state; a Bridge that matches `held.state` without a default would miss the new one, which is
the major-bump rule above. A pid that no hosted Session started is served as before.

**Each refusal has its own code**, all 409s: `fleet.slot_not_stranded`, `fleet.slot_busy`,
`fleet.rescue_reading`, `fleet.rescue_not_running`, `fleet.rescue_on_no_branch`,
`fleet.rescue_on_the_base` and `fleet.rescue_no_remote`. `fleet.no_such_slot` is a 422.

**Bridge re-reads `GET /worktrees` while a Finding is `reading`.** No event carries it.

**Store V104 and V105**, `slot_rescues` (V105 adds `verdict` and `items`): one row per slot, keyed by Manifest and slot number, replaced as
the Scout reads and deleted by a Scrap or a Stash. A row left `reading` by a restart is set to
`failed` when Fleet starts.
