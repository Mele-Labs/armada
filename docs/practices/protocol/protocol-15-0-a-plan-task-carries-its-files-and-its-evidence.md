# Protocol 15.0: a plan task carries its files and its evidence

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#1421`. `PlanTask` gains `scope`, `expects` and `shown`, and its `detail` is renamed `note`.
`AddTask` moves the same way. **The rename is what makes this a major**, and it is the whole of
what is not additive: an older Bridge reads `note` as absent and draws a task with no note at all.

A planning step already recorded which files each task touches — it wrote them into `detail` as
prose, and the step after it re-derived them by searching. `scope` is that list as a list, so the
next Drone starts from it. `expects` is what the planner says should prove the task and `shown` is
what the work says did; they are kept apart rather than reconciled, because the two disagreeing is
the fact worth seeing.

**`shown` is written by a later `update_task`, not with the recording.** It rides on the change, so
a task reopened and finished again keeps what its first pass showed.
