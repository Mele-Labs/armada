# Which kind of floating panel does job detail keep?

**Decided 2026-09-30.**

Two kinds had landed on the one `Sheet` component, and the code called both "Helm's dock". Record's and Drones' `floating` covers the work area under the title row, held off every edge, and dims everything under it. Plan's `docked` task panel floated beside the content and left it live, so pressing another task swapped the panel.

**Chosen:** everything dims. Plan's task panel becomes the same floating sheet Record and Drones use, and `docked` goes.

**Cost he took, stated in the question:** it undoes the Helm-like panel his note of 28 Sep asked for (`zkz7`), and pressing another task behind the panel no longer swaps it. The panel keeps what was built for it since then: the close in its corner, its fixed head over a scrolling body, the resize handle (note `xvpc`), the file diff beside it, and Back.

**Where it landed:** `plan/task-dock`.

Supersedes the docked half of `2026-09-28` Plan's panel note, `zkz7`.
