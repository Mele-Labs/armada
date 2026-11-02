// Autopilot's setup and its guarded switch, on the whole ship: quick boxes for the repositories, what goes on autopilot and who takes each kind of decision, then two acts
// to turn it on. `&take=lip`, `bay` or `setup` chooses where the switch stands. A mock: the boxes are not on the wire yet.

import { button, inside, region, role, walk } from "../walk";

const setup = region("Autopilot setup");
const hardware = region("Autopilot switch");
const inHud = region("Setup switch");
const guard = (name: string) => inside(hardware, button(name));
const flip = (where: typeof hardware) => inside(where, role("switch", "Autopilot"));
const box = (name: string) => inside(setup, role("checkbox", name));

// Where the console holds only a key, that key brings the setup up and the guard is met there.
const onConsole = new URLSearchParams(window.location.search).get("take") !== "setup";
const opening = onConsole
  ? [
      { look: hardware, say: "Autopilot's switch on the console, with its guard down" },
      { press: guard("Lift guard"), say: "First act: the guard flips up, the amber lamp blinks, and the setup comes up over the Map" },
      { look: inHud, say: "The setup has a switch of its own, and its guard rose with the console's" },
    ]
  : [
      { press: button("Autopilot"), say: "The console's key brings the setup up over the Map" },
      { look: inHud, say: "The switch stands on the setup, with its guard down" },
      { press: inside(inHud, button("Lift guard")), say: "First act: the guard flips up and the amber lamp blinks" },
    ];

const autopilotArm = walk("cockpit-ship-flightdeck", [
  ...opening,
  { press: box("armada"), say: "Check each repository it applies to" },
  { look: region("On autopilot"), say: "What goes on autopilot: its backlog, the Jobs already running, or both" },
  { look: region("Instruction"), say: "Several lines of instruction" },
  { look: region("Limits"), say: "Jobs at a time, the time to stop at, the most to spend and the most Jobs to start" },
  { look: region("Decisions"), say: "Checked, the co-commander decides. Unchecked, it blocks until you are back" },
  { press: box("Fix a red main"), say: "One press moves a decision across" },
  { look: inside(region("Permissions"), role("checkbox", "Destructive")), say: "Reads and Writes are yours to hand over. Destructive stays locked, as you set for Sleep" },
  { press: flip(inHud), say: "Second act: the red switch, here in the setup. The green lamp lights and Autopilot runs" },
  onConsole ? { press: flip(hardware), say: "The console's switch moved with it, and either one turns it off. The guard falls shut. Jobs it started carry on" } : { press: flip(inHud), say: "Flip it down and the guard falls shut. Jobs it started carry on" },
  { key: "Enter", on: box("armada"), say: "By keys: a opens the setup, arrows move, Space checks, Enter goes to the guard, Space lifts it, Space flips the switch" },
]);

export { autopilotArm as "autopilot-arm" };
