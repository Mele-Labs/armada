# How does a person get back after a panel sends them to another one?

**Decided 2026-09-29.**

In job detail, pressing something in one panel can land you in another destination's panel, for example a task's Drone card opening that Drone in Drones. There was no way back. Two approaches were built and compared: the Drone opening in a second panel to the left of the current one, and a way back.

**Chosen:** the way back. *"Yeah this is much better. lets go with this."* Each jump between destinations remembers the tab and the panel it left. The landed panel's head carries **Back to T6** (the thing you were reading), with the tab in its tooltip, and `⌘[`, the back half of the registered `history` act, does the same. It goes back one step at a time along a chain. Pressing a tab or leaving the Job clears the trail. It is one `back` slot on `Sheet`, and the stack lives in `packages/screens/src/trail.ts`.

**Cost he took:** the left-hand panel approach is dropped. Workflow is not yet a place Back appears, because its step panel belongs to the Workflow session.

**Changed 30 Sep:** Close goes back. On a panel reached by a jump, Close and `Esc` do what Back does while the trail holds a place; once it is empty, Close just closes. Back stays, because it names where you go.

**Where it landed:** `plan/task-dock`.
