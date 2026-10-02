// A task with a Drone of its own, opened from Plan: what that Drone is doing
// now, its brief, its live log, its last edit and a stop for it alone. Mock-fed:
// Fleet runs one Drone per Job until slices 1 and 5 of `docs/spikes/022`.
// `plan-task-live.test.tsx` holds the claims.

import { button, card, role, tab, text, walk } from "../walk";

export const whatATasksOwnDroneIsDoing = walk("arc/executing-sequential", [
  { press: tab("Plan"), say: "The plan, as its groups and tasks" },
  { press: card("Draw what is running, in four lists"), say: "T5 is the task being worked" },
  { look: text("14 turns so far"), say: "What its Drone is doing now: turns, and no cost until it stops" },
  { look: button("stop this task"), say: "A stop for this task alone, beside its other acts. Mocked: it ends the Job's Drone" },
  { look: text("Keep the four lists in this order"), say: "What its Drone was told" },
  { look: role("group", "Drone on T5"), say: "Its live log: the Drone's own tail, with the reply box at its foot" },
  { look: role("list", "Last edit"), say: "The last file its Drone wrote" },
]);
