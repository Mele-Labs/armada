# Where do Drop and Add task live, now that Overview's plan region is gone?

**Decided 2026-09-30.**

#1671 took Overview's plan region away, and with it the only place a person could drop a task or add one. Nothing in the app offered either.

**Chosen:** **Drop this task** goes in Plan's task panel, beside Pilot, Restart and Edit, and asks for its reason in place. **Add task** goes in each group's head on the plan list. Both come back on this branch, with the four drop checks #1671 deleted brought back.

**Cost he took:** more scope before `plan/task-dock` lands, so the Workflow session waits a little longer.

**Where it landed:** `plan/task-dock`.
