# What can a person do about one failed task?

**Decided 2026-09-29.**

On `arc/group-failed`, T6 failed its group's Checks and the task panel offered nothing but a message box. The owner: *"When a task in a group fails. I need to be able to take action on it"* — pilot its Drone, message it, restart the task, or change the task so the retry is clearer. Fleet serves only the message, and that reaches the Job's one Drone rather than a Drone of the task's own.

**Chosen:** the failed task's panel offers all four. Message works today. Pilot names #250, Restart this task names #1656, and Edit this task names #1657, each raising `Not implemented` through `packages/protocol/src/pending.ts` until its route ships.

**Cost he took:** three buttons that do not work yet, and two new issues to keep in step with the registry.

**Where it landed:** `plan/task-dock`, with #1656 and #1657.
