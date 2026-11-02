// Back at the window with Autopilot running. After ten minutes with no input the first press asks whether to keep it on and does
// not act on what was under it. Keep it on closes it. What Autopilot answered meanwhile is calls to review: Agree, a correction to
// the Job, and Agree alone where there is no Job to correct. After ten more minutes the question comes back, and Turn it off puts
// the switch down. Over `autopilot-back`, whose Fleet holds the answers (`slices/autopilot-decisions.ts`).

import { button, deckRow, dialog, inside, region, role, text, walk } from "../walk";

const list = region("Jobs", { exact: true });
const hardware = region("Autopilot switch");
const asking = dialog("Autopilot is running");
const lunch = region(/^Autopilot answered: Order the lunch/);
const branch = region(/^Autopilot answered: Name the branch/);
const landed = region(/^Autopilot answered: Pull request 1894/);

const autopilotBack = walk("autopilot-back", [
  { look: inside(hardware, role("switch", "Autopilot")), say: "Autopilot is running: the switch stands up under its guard" },
  { later: hardware, say: "Ten minutes pass with nobody at the window" },
  { press: deckRow("Order the lunch"), say: "The first press asks. It does not open the Job under it" },
  { look: asking, say: "Keep it on, or turn it off. Enter keeps it on, and so does Esc" },
  { press: inside(asking, button("Keep it on")), say: "Keep it on closes it, and Autopilot runs on" },
  { key: "w", on: list, say: "w brings the next call forward: an answer Autopilot gave" },
  { look: lunch, say: "What was asked, and under it what it chose and why" },
  { look: inside(lunch, text("Large")), say: "What it chose" },
  { press: button(/Agree/), say: "Agree keeps the answer, and the card leaves" },
  { key: "w", on: list, say: "The next answer" },
  { look: branch, say: "An answer given in a Job" },
  { press: button(/Change/), say: "Change goes to the Job's own box" },
  { type: "feat\n", into: inside(branch, role("textbox", "Message")), say: "Enter sends the correction to the Job, and the card leaves" },
  { key: "w", on: list, say: "The last answer" },
  { look: landed, say: "A pull request it landed has no Job to correct, so Agree is the only act" },
  { press: button(/Agree/), say: "Agree" },
  { later: hardware, say: "Ten more minutes pass" },
  { press: deckRow("Name the branch"), say: "The question comes back on the first press" },
  { press: inside(asking, button("Turn it off")), say: "Turn it off stops Autopilot. The Jobs it started carry on" },
  { look: inside(hardware, role("switch", "Autopilot")), say: "The switch goes down" },
]);

export { autopilotBack as "autopilot-back" };
