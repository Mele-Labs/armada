# Protocol 23.17: a person reshapes the slot pool

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 4 Oct 2026: from Cleanup's bay grid, add a slot, remove one, or close one for a while.
`../concepts/fleet.md`, *Worktree slots*, is the concept.

**One field and one command, both additive.** `WorktreeSlot` gains `closed`. `change_slot_pool`
is `POST /worktrees/slots?manifest_id=` with `ChangeSlotPool` (`act`, and `slot` for all but
`add`), answered by `SlotPoolChanged`.

| Field | On | Absent |
| --- | --- | --- |
| `closed` | `WorktreeSlot` | A Fleet before 23.17, which closes nothing: read as open |

**Each refusal has its own code**, so Bridge says on the slot why it cannot go:
`fleet.slot_held`, `fleet.slot_stranded`, `fleet.slot_busy`, `fleet.slot_not_a_checkout`,
`fleet.slot_dirty` and `fleet.slot_last` are 409s, and `fleet.no_such_slot` a 422.

**No store change.** The pool's shape is written beside its slots at `.armada/slots/pool`, which
the CLI reads too.
