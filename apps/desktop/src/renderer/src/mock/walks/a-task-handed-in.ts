// A task whose Drone has submitted, with its Checks not yet answered, beside one
// still being worked (the owner, 5 Oct 2026).

import { card, tab, text, walk } from "../walk";

const aTaskHandedIn = walk("job/handedInATask", [
  { press: tab("Plan"), say: "T1 is submitted, T2 is still worked" },
  { look: text("Submitted · awaiting checks"), say: "The submitted task says so, and wears a mark of its own" },
  { press: card("Group 1"), say: "Its group's boundary" },
]);

export { aTaskHandedIn as "a-task-handed-in" };
