// Two tasks of one group running at once, each by a Drone of its own: spike
// 022, slice 5 (protocol 23.10). One Drone is stopped or told alone (#1666),
// Pulse names each process's Drone and counts both (#1651), and Plan says what
// running at once leaves unseen. Fixtures match the 23.10 wire.

import { button, dialog, inside, role, row, tab, text, walk } from "../walk";

const t6 = dialog("Drone on T6");
const GROUPS = role("list", "Groups, in the order they run");

const dronesAtOnce = walk("arc/executing-at-once", [
  { press: tab("Drones"), say: "Group three's two tasks, both running" },
  { look: row("Drone on T5"), say: "T5, on the Job's kept Drone" },
  { look: row("Drone on T6"), say: "T6, on a Drone of its own beside it" },
  { press: button("Drone on T6"), say: "Open T6's Drone" },
  { look: inside(t6, button("Kill drone")), say: "Its stop ends this Drone alone; T5 goes on" },
  { look: inside(t6, role("textbox")), say: "A message goes to this Drone, not to the Job's" },
  { press: inside(t6, button("Close")), say: "Back to the list" },
  { press: tab("Pulse"), say: "What the Job holds on this machine" },
  { look: text("Drones running"), say: "Both Drones counted" },
  { look: text("implement · T6"), say: "Each process names the Drone it belongs to" },
  { press: tab("Plan"), say: "The plan" },
  { press: tab("List"), say: "As a list" },
  {
    hover: inside(GROUPS, role("img", /at the same time/)),
    say: "Group three runs at once, and says what that cannot see",
  },
]);

// Named as the link spells it, `?walk=drones-at-once`.
export { dronesAtOnce as "drones-at-once" };
