// A Drone held on a command it was not given, asking on its own card in the
// task's panel — not only on Overview. `held-drone-asks.test.tsx` holds the claims.

import { button, card, inside, role, tab, text, walk } from "../walk";

export const aDroneHeldOnACommand = walk("held/command", [
  { press: tab("Plan"), say: "The plan, as its groups and tasks" },
  { look: inside(card("Draw what is running, in four lists"), text("Needs you")), say: "On the graph, T5's card says a command waits for you" },
  { look: role("group", "Needs you"), say: "And a card beside it asks, with the command and the answers" },
  { press: card("Draw what is running, in four lists"), say: "Open it: T5 is the task being worked" },
  { look: role("group", "Drone on T5"), say: "Its Drone's card, with the log and a box to reply" },
  { look: inside(role("group", "Drone on T5"), text("pnpm add -D reselect@5.1.1")), say: "The command it reached for, asked right here" },
  { look: inside(role("group", "Drone on T5"), button("Send this answer")), say: "The same answers Overview offers" },
]);
