// What a send leaves where the bar was: a slim strip with the kind's mark, the Job's title or the
// Autopilot's repository, and Open. ↵ or a press opens it; typing again or Esc lets it go. A Session
// opens itself and leaves none. Over `dispatch-panel-from-either-bar`.

import { button, dialog, inside, region, role, tab, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const strip = role("group", "Last dispatch");
const bar = inside(dialog("Command palette"), role("combobox"));
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);
const title = "Show the Drone's branch on each Bridge cockpit tile";

const dispatchedStripOpensIt = walk("dispatch-panel-from-either-bar", [
  // (a) The Cockpit's bar, a Job.
  { type: title, into: request, say: "The Cockpit's dispatch bar, a request naming Bridge" },
  { look: inside(frame, button("Repository armada")), say: "Read as the armada repository" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: inside(strip, text(title)), say: "The bar is a strip holding the Job's title" },
  { hover: inside(strip, button("Dismiss")), say: "The tooltip names the dismiss" },
  { key: "Enter", on: request, say: "Press ↵" },
  { look: role("heading", /Show the Drone's branch/), say: "The Job is open" },
  { press: inside(role("navigation", "Work"), button("Cockpit")), say: "Back on the Cockpit" },
  { look: request, say: "The bar is a bar again" },

  // (b) The strip lets go on typing.
  { type: "Pin the Bridge store clock", into: request, say: "A Job sent" },
  { press: inside(frame, button("Dispatch", { exact: true })), say: "Dispatch" },
  { look: strip, say: "The strip holds the latest send" },
  { key: "Escape", on: request, say: "Press Esc" },
  { look: request, say: "The strip has gone" },

  // (c) The title bar: an Autopilot.
  { key: "Meta+k", on: request, say: "Press ⌘K" },
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
