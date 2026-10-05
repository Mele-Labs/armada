// Answering a held Drone from its own card: the ask clears and the Drone is
// running again. `held-drone-asks.test.tsx` holds the claims.

import { button, card, inside, role, tab, text, walk } from "../walk";

const DRONE = role("group", "Drone on T5");

export const allowingAHeldCommandFromTheDrone = walk("arc/executing-held", [
  { press: tab("Plan"), say: "The plan, as its groups and tasks" },
  { look: inside(card("Draw what is running, in four lists"), text("Command to allow")), say: "The graph's T5 card says a command waits for you" },
  { press: card("Draw what is running, in four lists"), say: "T5's Drone is waiting on you, on its card" },
  { press: inside(DRONE, role("radio", "Allow for this job")), say: "Allow it for this Job" },
  { press: inside(DRONE, button("Send this answer")), say: "Sent from the card, to the same call Overview sends" },
  { look: role("list", "Drones on this task"), say: "The ask is gone, and the Drone is running again" },
]);
