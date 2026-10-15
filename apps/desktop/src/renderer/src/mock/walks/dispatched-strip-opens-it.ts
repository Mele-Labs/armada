// What a send leaves over the title bar's field: a slim strip with the kind's mark, the Job's title or the
// Autopilot's repository, and Open. ↵ or a press opens it; typing again or Esc lets it go. A Session
// opens itself and leaves none. Over `dispatch-panel-from-the-title-bar`.

import { button, dialog, inside, place, region, role, tab, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const strip = role("group", "Last dispatch");
const cockpit = region("Jobs and Sessions");
const bar = inside(dialog("Command palette"), role("combobox"));
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);
const title = "Show the Drone's branch on each Bridge cockpit tile";

const dispatchedStripOpensIt = walk("dispatch-panel-from-the-title-bar", [
  // (a) A Job.
  { key: "n", on: cockpit, say: "N opens the title bar's field" },
  { type: title, into: bar, say: "A request naming Bridge" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { look: inside(frame, button("Repository armada")), say: "Read as the armada repository" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: inside(strip, text(title)), say: "A strip over the field holds the Job's title" },
  { hover: inside(strip, button("Dismiss")), say: "The tooltip names the dismiss" },
  { key: "Enter", on: strip, say: "Press ↵" },
  { look: role("heading", /Show the Drone's branch/), say: "The Job is open" },
  { press: place("Cockpit"), say: "Back on the Cockpit" },
  { look: cockpit, say: "The strip is gone with the Job opened" },

  // (b) The strip lets go on Esc.
  { key: "n", on: cockpit, say: "N opens the title bar's field" },
  { type: "Rename the Bridge tile legend", into: bar, say: "A Job sent" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: strip, say: "The strip holds the latest send" },
  { key: "Escape", on: strip, say: "Press Esc" },
  { look: cockpit, say: "The strip has gone" },

  // (c) An Autopilot.
  { key: "Meta+k", on: cockpit, say: "Press ⌘K" },
  { type: "Work through the open Bridge bugs until 7am", into: bar, say: "Words for Autopilot" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { press: inPanel(tab("Autopilot")), say: "Autopilot chosen in the band" },
  { press: inPanel(button("Read it", { exact: true })), say: "Read it" },
  { press: inPanel(button("Start Autopilot", { exact: true })), say: "Start Autopilot" },
  { look: inside(strip, text("armada")), say: "A strip over the title bar's field holds the repository" },
  { key: "Enter", on: strip, say: "Press ↵" },
  { look: region("Autopilot"), say: "Autopilot is open on armada" },
]);

export { dispatchedStripOpensIt as "dispatched-strip-opens-it" };
