// A task on the Plan's graph opens its panel: the brief, its Drone, and a
// reply to that Drone in the same place. `job-detail-plan.test.tsx` holds the claims.

import { card, role, tab, text, walk } from "../walk";

export const aTaskOnThePlan = walk("arc/executing-sequential", [
  { press: tab("Plan"), say: "The plan, as its groups and tasks" },
  { press: card("Draw what is running, in four lists"), say: "T5 is the task being worked" },
  { look: text("Keep the four lists in this order"), say: "Its panel opens on the brief" },
  { look: role("group", "Drone on T5"), say: "The Drone on it, working" },
  { type: "Draw the empty list too.", into: role("textbox", "Message the drone"), say: "A reply goes to the Drone from here" },
]);
