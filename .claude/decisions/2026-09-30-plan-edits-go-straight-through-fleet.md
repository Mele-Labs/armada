# Are edits to a plan made directly, or asked of the planning Drone?

**Decided 2026-09-30.**

Move up / Move down / Remove on a group sent a request to the planning Drone, which rewrote the plan or refused. #897 had ruled out reordering this milestone. Offered drag and drop as the same request, he refused the frame: *"Thats insane, edits to the plan should just be made directly through fleet."*

**Chosen:** a person's edits to a plan are made directly, through Fleet: add, drop, edit, reorder, and remove a group. None is routed through the planning Drone. **Propose a change** stays as the one ask to the Drone, for when a person wants it to rethink part of the plan rather than make an edit themselves.

- Reorder is drag and drop of groups and tasks, direct. Fleet has no route yet (#1685), so a drop raises `Not implemented` naming it.
- Remove a group drops each of its tasks through `drop_task`, with one reason.
- Edit is #1657.

**Cost he took:** it reverses #897's "no reordering this milestone", and a direct move can put a task before one it depends on. The planner's order is no longer the last word.

**Where it landed:** `plan/task-dock`, with #1685.
