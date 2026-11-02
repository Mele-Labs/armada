// Autopilot's setup and its guarded switch, on the whole ship: quick boxes for the repositories, what goes on autopilot and who takes each kind of decision, then two acts
// to turn it on.

import { button, inside, region, role, walk } from "../walk";

const setup = region("Autopilot setup");
const hardware = region("Autopilot switch");
const guard = (name: string) => inside(hardware, button(name));
const box = (name: string) => inside(setup, role("checkbox", name));

const autopilotArm = walk("cockpit-ship-flightdeck", [
  { look: hardware, say: "Autopilot's switch on the console, with its guard down" },
  { press: guard("Lift guard"), say: "The guard flips up, the amber lamp blinks, and the setup comes up over the Map" },
  { press: box("armada"), say: "Check each repository it applies to" },
  { look: region("On autopilot"), say: "What goes on autopilot: its backlog, the Jobs already running, or both" },
  { look: region("Instruction"), say: "Several lines of instruction" },
  { look: region("Limits"), say: "Jobs at a time, the time to stop at, the most to spend and the most Jobs to start" },
  { look: region("Decisions"), say: "Checked, Autopilot decides. Unchecked, it blocks until you are back" },
  { press: box("Fix a red main"), say: "One press moves a decision across" },
  { look: inside(region("Permissions"), role("checkbox", "Destructive")), say: "Reads and Writes are yours to hand over. Destructive stays locked" },
  { press: inside(setup, button("Start Autopilot")), say: "One press starts it, and the console's switch flips with it" },
  { press: inside(setup, button("Stop Autopilot")), say: "One press stops it. Jobs it started carry on" },
  { key: "Enter", on: box("armada"), say: "By keys: a opens the setup, arrows move, Space checks, Enter goes to Start, Enter starts" },
]);

export { autopilotArm as "autopilot-arm" };
