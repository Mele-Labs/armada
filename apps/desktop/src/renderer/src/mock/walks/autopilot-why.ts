// Autopilot brought down by a cap: the switch is off, the setup says which cap, and Start begins it again. Fleet refuses a start
// over a paused Autopilot, so Start stops it first, in the same press. Over `autopilot-why`.

import { button, inside, region, role, text, walk } from "../walk";

const hardware = region("Autopilot switch");
const setup = region("Autopilot setup");

const autopilotWhy = walk("autopilot-why", [
  { look: inside(hardware, role("switch", "Autopilot")), say: "A cap brought Autopilot down while he was away: the switch is off" },
  { press: inside(hardware, button("Lift guard")), say: "The setup opens with that repository already checked" },
  { look: inside(setup, text("Job cap reached")), say: "The line says which cap stopped it. A repository that worked without trouble has no line" },
  { press: inside(setup, button("Start Autopilot")), say: "Start begins it again with the limits set here. The paused one is stopped first, in the same press" },
  { look: inside(setup, button("Stop Autopilot")), say: "Start has become Stop: it is running, and the line is gone" },
]);

export { autopilotWhy as "autopilot-why" };
