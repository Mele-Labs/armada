// What a send leaves over the title bar's field: a slim strip with the Autopilot's repository, and Open.
// ↵ or a press opens it; typing again or Esc lets it go. A Session opens itself and leaves none. Over `quick-dispatch`, sent from the Dispatch sheet.

import { button, inside, region, role, tab, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const strip = role("group", "Last dispatch");
const cockpit = region("Jobs", { exact: true });

const dispatchedStripOpensIt = walk("quick-dispatch", [
  { key: "n", on: cockpit, say: "N opens the Dispatch sheet" },
  { type: "Work through the open Bridge bugs until 7am", into: request, say: "Words for Autopilot" },
  { press: inside(frame, tab("Autopilot")), say: "Autopilot chosen in the band" },
  { press: inside(frame, button("Read it", { exact: true })), say: "Read it" },
  { press: inside(frame, button("Start Autopilot", { exact: true })), say: "Start Autopilot" },
  { look: inside(strip, text("armada")), say: "A strip over the field holds the repository" },
  { key: "Enter", on: strip, say: "Press ↵" },
  { look: region("Autopilot"), say: "Autopilot is open on armada" },
]);

export { dispatchedStripOpensIt as "dispatched-strip-opens-it" };
