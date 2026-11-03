// Reported bugs on Autopilot's setup: a box that exists only for the armada repository, off until it is checked, and the report it
// sets aside once Autopilot is started with it. Over `autopilot-reports`.

import { button, inside, region, role, walk } from "../walk";

const hardware = region("Autopilot switch");
const setup = region("Autopilot setup");
const box = (name: string) => inside(setup, role("checkbox", name));

const autopilotReports = walk("autopilot-reports", [
  {
    press: inside(hardware, button("Lift guard")),
    say: "The setup opens over the Map",
  },
  {
    look: region("On autopilot"),
    say: "With no repository checked, Reported bugs is not offered. Only armada has reported bugs",
  },
  {
    press: box("armada"),
    say: "Check armada",
  },
  {
    look: box("Reported bugs"),
    say: "Reported bugs appears, unchecked",
  },
  {
    press: box("Reported bugs"),
    say: "Check it to let Autopilot pick up the bugs agents reported",
  },
  {
    press: inside(setup, button("Start Autopilot")),
    say: "Start begins it with the box as set",
  },
  {
    look: inside(setup, region(/^Set aside/)),
    say: "A report Autopilot was unsure of waits here with its reason",
  },
]);

export { autopilotReports as "autopilot-reports" };
