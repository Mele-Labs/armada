# Does Workflow's step panel dim, like the other job-detail panels?

**Decided 2026-09-30.**

The same day every job-detail panel was made to dim (`2026-09-30-every-panel-dims.md`), Workflow's step panel was still the exception from 25 Sep: no scrim, so another step could be pressed on the canvas while one was open, and the canvas slid to keep the open step beside the panel.

**Chosen:** it dims like the rest. The step panel is the same floating `Sheet` Record, Drones and Plan use.

**Cost he took, stated in the question:** while a step is open the canvas is dimmed, so reading another step means closing this one first, and the slide that kept the open step in view goes, because it no longer helps.

**Where it landed:** `workflow/board-pass`. Supersedes the no-scrim half of the 25 Sep Workflow layer decision recorded in `tab-workflow.tsx`.
