// What a send leaves over the title bar's field: a slim strip with the kind's mark, the Job's title or the
// Autopilot's repository, and Open. ↵ or a press opens it; typing again or Esc lets it go. A Session
// opens itself and leaves none. Over `quick-dispatch`, sent from the Dispatch sheet.

import { button, inside, place, region, role, tab, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = inside(frame, role("textbox", "Request"));
const strip = role("group", "Last dispatch");
const cockpit = region("Jobs and Sessions");
const repository = (name: string) => inside(frame, role("radio", `Repository ${name}`));
const title = "Show the Drone's branch on each Bridge cockpit tile";

const dispatchedStripOpensIt = walk("quick-dispatch", [
  // (a) A Job.
  { key: "n", on: cockpit, say: "N opens the Dispatch sheet" },
  { type: title, into: request, say: "A request naming Bridge" },
  { look: repository("armada"), say: "Read as the armada repository" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: inside(strip, text(title)), say: "A strip over the field holds the Job's title" },
  { hover: inside(strip, button("Dismiss")), say: "The tooltip names the dismiss" },
  { key: "Enter", on: strip, say: "Press ↵" },
  { look: role("heading", /Show the Drone's branch/), say: "The Job is open" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
  { look: cockpit, say: "The strip is gone with the Job opened" },

  // (b) The strip lets go on Esc.
  { key: "n", on: cockpit, say: "N opens the Dispatch sheet" },
  { type: "Rename the Bridge tile legend", into: request, say: "A Job sent" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: strip, say: "The strip holds the latest send" },
  { key: "Escape", on: strip, say: "Press Esc" },
  { look: cockpit, say: "The strip has gone" },

  // (c) An Autopilot.
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
