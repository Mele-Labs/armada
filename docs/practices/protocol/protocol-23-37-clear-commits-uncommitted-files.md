# Protocol 23.37: Clear commits uncommitted files

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

The owner, 5 Oct 2026: freeing a worktree loses nothing once its work is committed on a branch.
`../../.claude/decisions/2026-10-05-a-branch-commit-is-enough-to-free-a-slot.md`; `../concepts/fleet.md`,
*Worktree slots*.

**Additive only.** One optional field; a Bridge before 23.37 reads past it and says nothing about
what was committed.

| Change | Where | Carries | Absent |
| --- | --- | --- | --- |
| `saved` | `WorktreeReclaimed` | `{ "commit", "files" }`: the WIP commit a Clear made on `branch.branch`, and the files it took | No uncommitted files, and every answer before 23.37 |
| `release` | `SlotAct`, in `ChangeSlotPool` | Commit a session-held slot's uncommitted files to its branch and free it | An older Fleet refuses the unknown act |
| `holder` | `ChangeSlotPool` | For `release`: the holder the person was shown, `claude (pid 44698)`. Required by it | Every other act |
| `stranded` | `WorktreeSlot` | Also on a session-held slot with uncommitted files, as `uncommitted` alone, with no commits | Every slot a job holds clean, and every read before 23.37 |
| `released` | `SlotPoolChanged` | `{ "branch", "saved"? }`, `saved` as above | Every act but `release` |
| `fleet.slot_holder_changed`, `fleet.slot_not_parkable` | 409 | The slot has another holder now, or git would not commit to a branch | |
| `fleet.slot_holder_unnamed` | 422 | A release naming no holder | |
| `fleet.slot_session_working` | 409 | A release of a bay a hosted Job holds, while its turn runs. `holder` is the Job's id, as `HostedSession` shows it. Not a new field or act | |

A release hands a slot an agent job holds back by the same park, and only for the holder named, so a slot re-leased since is refused. The job's process is not touched.

A bay a hosted Job holds is released the same way with the Job's id as `holder`, and is refused with `fleet.slot_session_working` while its turn runs, since the park ends every process in the slot.

`reclaim_worktree` no longer refuses a slot for uncommitted files. It commits them to the Job's
branch, never pushes, and frees the slot; a Job's own worktree outside the pool is committed the same
way before it is removed.

**Bridge's half**, mirrored by hand: `ReclaimedSaved` and `WorktreeReclaimed.saved` in `reclaimed.ts`; `SlotAct`'s `release`, `ChangeSlotPool.holder`, `SlotPoolChanged.released` and `SlotReleased` in `holding.ts`.
