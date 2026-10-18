# Protocol 23.30: a Scout's Finding picked up

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 5 Oct 2026: a stranded slot's Finding gets a third act beside Scrap and Stash, which
proposes a Job that continues the work from the slot's branch. `../concepts/fleet.md`,
*Rescuing a stranded slot*.

**Additive only.** One variant and one optional field; a 23.29 Bridge never sends the variant and
never sends the field.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `pick_up` | `RescueAct` | Stash, then a proposal. The answer is `SlotRescued`, as a stash's | A 23.29 peer sends `start`, `stop`, `scrap` or `stash` |
| `continue_from` | `ProposeJob` | A branch the Job's worktree is cut from in place of the base. Kept as the proposal's landing `from_ref`, so the approval shows it and refuses it unless the repository holds it | Cut from the base, as every proposal before it |

**The seam is the landing the approval already has.** `from_ref` was set only at the approval; a
proposal can now arrive with it set, and the approval's own body still replaces it. A blank
`continue_from` is none.

**Pick up uses the proposer, not a workflow of its own.** Fleet assigns no workflow by default
(the `proposing` module (deleted since)), so the request, the branch and the Finding's items, goes to the
proposer as a dispatched request does, and the head of its plan is cut from the branch. A plan's
other Jobs wait on the head and are cut from the base.

**Two refusals, both 409s:** `fleet.rescue_unread` (no Finding of the slot has items or words to
carry) and `fleet.rescue_nothing_left` (its verdict is `scraps`). A stash's refusals apply too.

**No migration, no store change and no event.** The landing is the existing `job_landing` row.
