# Protocol 23.1: a Drone per task, and the task a change moved

**Part of:** [Protocol: the Fleet/Bridge seam](../protocol.md).

Spike 022, the wire lock for the new Job, slice 1b (#1762, carrying #1752).

**Four optional fields on three bodies, all additive.** `JobDrone` gains `task`, the plan task a
Drone was put on, absent on a Drone that worked its whole step. `JobPlanChanged` gains `task` and
`state`, the task a change moved and where it now stands, both absent on a whole recording. Spike
022's *why a transition rides `job.plan_changed`* is the reason the event carries them rather than
a new kind. `StepDetail` gains `drone_per_task`, absent at false, because the spike has Bridge
read the step key where it derived the step that works the tasks; the spike's wire row for 1b
names the first three and not this one.

**Fleet is a third author of `job.plan_changed`.** On a step declaring `drone_per_task`, Fleet
marks each task itself, `actor` `fleet`: `working` when its Drone is spawned, `handed_in` at that
Drone's `submit_evidence`, and `done` once the step's Checks pass. `update_task` still refuses
`handed_in`, and a task's Drone is not offered it at all. the retired Plan concept, *A Drone per
task*.

**On the unmeasured risk above: worse, by a counted amount.** A task adds three
`job.plan_changed` and a `drone.spawned` and `drone.exited` pair, against one pair per step
before. Measured on a dev Fleet on 2 Oct 2026, with a scripted agent handing in two seconds after
each spawn: five events per task, and 43 in the minute that held a five-task Job's whole run,
against a `BACKLOG` of 256. A real task takes minutes, so the rate per Job-minute is five over
how long a task takes. #1759 has the line; `[broadcast-capacity]` stays open.

**Store V92** keeps which Drone was put on which task, and what each handed in, so `JobDrone.task`
and the step's one submission survive a Fleet restarting mid-step.
