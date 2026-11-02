// Autopilot's setup and its guarded switch, on the whole ship: quick boxes for the repositories, what goes on autopilot and who takes each kind of decision, then two acts
// to turn it on. `&take=lip`, `bay` or `setup` chooses where the switch stands. A mock: the boxes are not on the wire yet.

import { button, inside, region, role, walk } from "../walk";

const setup = region("Autopilot setup");
const hardware = region("Autopilot switch");
const guard = (name: string) => inside(hardware, button(name));
const flip = inside(hardware, role("switch", "Autopilot"));
const box = (name: string) => inside(setup, role("checkbox", name));

const autopilotArm = walk("cockpit-ship-flightdeck", [
  { look: hardware, say: "Autopilot's switch, with its guard down" },
  { press: guard("Lift guard"), say: "First act: the guard flips up, the amber lamp blinks, and the setup opens" },
  { press: box("armada"), say: "Check each repository it applies to" },
  { look: region("On autopilot"), say: "What goes on autopilot: its backlog, the Jobs already running, or both" },
  { look: region("Decisions"), say: "Checked, the co-commander decides. Unchecked, it blocks until you are back" },
  { press: box("Walk of a visible change"), say: "One press moves a decision across" },
  { look: inside(region("Decisions"), role("checkbox", "Anything destructive or irreversible")), say: "Destructive or irreversible stays locked, as you set for Sleep" },
  { press: flip, say: "Second act: the red switch. The green lamp lights and Autopilot runs" },
  { press: flip, say: "Flip it down and the guard falls shut. Jobs it started carry on" },
  { key: "Enter", on: box("armada"), say: "By keys: a opens the setup, arrows move, Space checks, Enter goes to the guard, Space lifts it, Space flips the switch" },
]);

export { autopilotArm as "autopilot-arm" };
