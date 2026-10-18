# Protocol 23.18: walk notes, kept on the Job, and a server started for review

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

What a person points at while walking a Prototype's served mock in a Bridge window, kept by Fleet
on the Job so it outlives the throwaway worktree, and handed to the next Drone when the Job is sent
back.

**Additive only.** Two new routes, four new DTOs and three optional fields; a 23.17 Bridge sends no
`with_walk_notes` and reads no `walk_notes`, and sees exactly what it did.

| Change | Where | Carries | Absent |
|---|---|---|---|
| `capture_walk_note` | `POST /jobs/:job_id/walk_notes` | `CaptureWalkNote { said, capture, frame? }` in, `WalkNotes { notes }` out | — |
| `remove_walk_note` | `POST /jobs/:job_id/walk_notes/remove` | `RemoveWalkNote { id }` in, `WalkNotes { notes }` out | — |
| `walk_notes` | `JobDetail` | `WalkNote { id, said, at, element, selector, location, served?, frame?, sent }` per note, oldest first, sent ones included | A Job with no walk notes |
| `for_review` | `ServerState` | Fleet started it because a step with `evidence.walked` stopped for a person, on the Manifest's `walk` server; Bridge opens it when that Job is opened | Started by anything else |
| `with_walk_notes` | `ChangesRequested` | Append every unsent walk note to the note delivered, and mark them sent | False |

**The frame is a path under the machine directory**, `<machine>/walks/<job_id>/<note_id>.png`,
never the worktree. **A blank `note` is taken with `with_walk_notes`** where at least one unsent
note exists, because the notes are then what the Drone is told; otherwise blank is refused as it
was. The notes are marked sent only once the send lands, so a refused one leaves them unsent.

**Store V103**, `job_walk_notes`: one row per note, keyed to the Job so `forget_job` takes it. A
trigger refuses any update but `sent_at` moving once from null.
