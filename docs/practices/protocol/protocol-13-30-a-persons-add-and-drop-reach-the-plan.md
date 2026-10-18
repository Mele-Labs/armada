# Protocol 13.30: a person's add and drop reach the plan

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

`#897`. `add_task` and `drop_task` are new routes, so the minor moves —
`13.27`, `13.28` and `13.29` having each reached `main` first for unrelated
changes. A person adds a task (`title`, `detail`, `after`) or drops one with
a reason (`task`, `reason`), each kept under `store::PlanHand::Person` so the
record shows who made the change. Both answer with `ipc::WorkPlan`, the plan
the change leaves.

**Delivery follows `redirect_drone`'s rule.** With a working Drone mid-step, a
turn is injected naming the task and, for a drop, the reason — its own
`Occasion::Plan`, so the log shows it was not a redirect. At a step boundary,
or with no job, nothing is sent, because the next brief's THE PLAN is
built from the record at every spawn and carries the change already. Neither
route ever respawns a Drone to deliver itself.

Refused by name: no plan recorded (`fleet.no_plan`), a task or a place to add
after the plan does not hold (`fleet.no_such_task`), a task already `done` or
already `dropped` (`fleet.task_already_settled`, a 409 — a person's drop does
not repeat a decision already made), and a blank title or reason, on
`redirect_drone`'s reuse of `fleet.unacceptable_proposal` for a value that
cannot work.
