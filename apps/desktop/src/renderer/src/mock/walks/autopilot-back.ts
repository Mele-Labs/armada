// Back at the window with Autopilot running. After ten minutes with no input the first press asks whether to keep it on and does
// not act on what was under it. Either answer opens what Autopilot answered meanwhile, in a HUD the switch's lamp opens: Agree,
// Change with words for the Job, and the Job itself with Back coming to that row. Over `autopilot-back`, whose Fleet holds the
// answers (`slices/autopilot-decisions.ts`).

import { button, deckRow, dialog, inside, region, role, walk } from "../walk";

const hardware = region("Autopilot switch");
const asking = dialog("Autopilot is running");
const hud = dialog("What Autopilot answered");
const answer = (job: string) => inside(hud, role("group", job));

const autopilotBack = walk("autopilot-back", [
  { later: hardware, say: "Autopilot is running, and ten minutes pass with nobody at the window" },
  { press: deckRow("Friday lunch"), say: "The first press asks. It does not open the Job under it" },
  { press: inside(asking, button("Keep it on")), say: "Keep it on, and Autopilot runs on. Turn it off would do the same here" },
  { look: hud, say: "Everything it answered that he has not reviewed: the Job, what was asked, what it chose and why, and when. The keys are in the list" },
  { key: "Enter", on: answer("Friday lunch"), say: "Enter agrees with the row under the cursor, and it leaves the list" },
  { key: "t", on: answer("Reconnect fix"), say: "t opens a box on the row to change the answer" },
  { type: "feat\n", into: inside(hud, role("textbox", "Change")), say: "Enter sends the Job his words, and the row leaves" },
  { key: "o", on: answer("Release notes"), say: "o opens the Job the answer was given in" },
  { key: "Escape", on: region("Job: Release notes"), say: "Esc is Back, and it comes to the row he left" },
  { look: answer("Release notes"), say: "Same row, same place. A landed pull request has no Job, so it offers only Agree" },
  { key: "Escape", on: hud, say: "Esc puts the list away" },
  { look: inside(hardware, button("What Autopilot answered")), say: "The lamp on the switch's plate stays lit while an answer waits. Pressing it opens the list again" },
]);

export { autopilotBack as "autopilot-back" };
