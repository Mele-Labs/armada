// Every Drone a Job has had, off `list_job_drones` (21.3), and one opened to
// its own rows alone (21.4) — a killed one, a finished one with its cost, and
// the running one with turns so far, thinking. Each state is a mark, named on
// hover (the owner, 2 Oct 2026).

import { button, dialog, inside, row, tab, text, walk } from "../walk";

const dronesTabEveryDrone = walk("drones/every-drone-had", [
  { press: tab("Drones"), say: "Every Drone this Job has had, running or not" },
  { look: row("Killed"), say: "Root cause's first Drone, ended by hand" },
  { look: row("9 turns · $0.30"), say: "A finished Drone, with what it cost" },
  { look: row("6 turns · $0.10"), say: "The running one: turns and cost as of its last finished invocation" },
  { press: button("This Job's Drone"), say: "Open the running one" },
  {
    look: inside(dialog("This Job's Drone"), text("Re-exporting the selectors")),
    say: "Its own transcript, and no other Drone's",
  },
  { look: inside(dialog("This Job's Drone"), text("~980 tokens")), say: "A finished thinking run, with what it thought" },
  { look: inside(dialog("This Job's Drone"), text("~1,140 tokens")), say: "Thinking now: the mark pulses, the estimate rises" },
]);

// Named as the link spells it, `?walk=drones-tab-every-drone`.
export { dronesTabEveryDrone as "drones-tab-every-drone" };
