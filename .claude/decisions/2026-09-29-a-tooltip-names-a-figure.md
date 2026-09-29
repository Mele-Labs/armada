# Where does a short explanation go when it doesn't need a guide?

**Decided 2026-09-29.**

Pulse's Processes head showed `8.2% · 384.0 MiB`, and its Worktrees head `1.2 GiB`, with no labels, because the owner had just refused explanatory text on screen. Hovering them, he asked for tooltips: *"CPU Usage"* and *"Memory Usage"*, and one saying the size is this job's total on disk. Then as a rule: *"In general we should have tooltips as labels wherever it makes sense to explain something a bit more that doesn't need a guide."*

**Chosen:** a bare figure or an icon-only control gets its name in a tooltip. The screen stays facts; the tooltip is the label a person asks for by hovering. A `?` guide is still where something needs a whole explanation.

**Cost he took:** anything only in a tooltip is invisible until hovered, and unreachable on touch.

**Where it landed:** `.claude/skills/how-the-owner-decides/`, and the Pulse rebuild on `pulse/from-its-board`.
