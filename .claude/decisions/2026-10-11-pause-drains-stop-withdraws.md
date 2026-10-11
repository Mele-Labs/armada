# What do Pause and Stop do to Autopilot?

**Decided 2026-10-11.**

Autopilot is mid-run: one Job working, one at review, one just picked and not
yet started.

**Chosen:** Pause stops new picks and lets started Jobs finish, and Resume
carries on. Stop does the same, also withdraws what was picked but not started,
and ends Autopilot.

**Cost he took:** neither one stops spending on a Job already running. That is
the Job's own Pause, as it is today.

**Where it landed:** not built.
