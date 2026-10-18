// Dispatch in one press: the Cockpit's lower deck holds a Dispatch tile between Flight control and Autopilot, and
// it and `n` open the same centred sheet. The repositories are a row of chips with their keys, the
// one the words name lit as they are typed; with none chosen the act gives the row the keys. Over `quick-dispatch`.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const deckPanel = role("region", "Dispatch controls");
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);
const repository = (name: string) => inside(frame, role("radio", `Repository ${name}`));
const row = inside(frame, role("radiogroup", "Repository"));

const quickDispatchDeckButton = walk("quick-dispatch", [
  // (a) A Session, from the Cockpit's tile.
  { look: deckPanel, say: "A Dispatch panel stands between Flight control and Autopilot" },
  { hover: inside(deckPanel, button("Dispatch")), say: "The tile carries N" },
  { press: inside(deckPanel, button("Dispatch")), say: "Pressing it opens the sheet" },
  { look: row, say: "Every set-up repository is a chip, each with its key" },
  { type: "Add a docs-site page for the retry settings", into: inPanel(request), say: "A request naming docs-site" },
  { look: inside(frame, role("img", "Repository docs-site")), say: "docs-site is read from the words and its chip fills" },
  { key: "Tab", on: inPanel(request), say: "Press Tab" },
  { look: inside(frame, tab("Job")), say: "Job and Autopilot sit on the band, and Tab steps through them" },
  { key: "Tab", on: inPanel(request), say: "Press Tab again: it comes round to Job" },
  { key: "Alt+2", on: inPanel(request), say: "Press ⌥2" },
  { look: repository("armada"), say: "The second chip is chosen" },
  { key: "Enter", on: inPanel(request), say: "A Job starts on ↵" },
  { look: role("textbox", "Message"), say: "The Job started and opened, holding what was typed" },

  // (b) Nothing chosen: the act gives the row the keys.
  { key: "n", on: region("Jobs", { exact: true }), say: "N opens the sheet again" },
  { type: "Check the migration order", into: inPanel(request), say: "Words that name no repository" },
  { press: inside(frame, button("Start", { exact: true })), say: "Start with none chosen" },
  { look: row, say: "The frame takes the call's hue and the row has the keys" },
  { key: "ArrowRight", on: repository("armada"), say: "→ moves along the row" },
  { press: repository("billing"), say: "↵ on a chip picks it and sends" },
  { look: role("textbox", "Message"), say: "The Job started and opened, holding what was typed" },
]);

export { quickDispatchDeckButton as "quick-dispatch-deck-button" };
