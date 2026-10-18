# Protocol 14.5: when each plan task was being worked

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1185`. `PlanTask.working_windows`, additive and left out where empty: each
stretch a task was marked `working`, as `WorkingWindow { entered, left? }`,
oldest first. Fleet folds it from the plan's history — a move into `working`
opens one, any move out (`open`, `done`, `dropped`) closes it, and a new
recording starts every task with none. `left` is absent while the task is
still working. Bridge places a turn in the task whose window holds its
instant, so the Working area can group a step's activity by task — and where no
window holds an Edit, in the task whose `scope` names the file (`#1498`), which
is the only thing that places a turn without one. **A claim, like the state it
comes from**: a Drone that never calls `update_task` sends no windows, so
nothing but a declared path can place its work at all.
