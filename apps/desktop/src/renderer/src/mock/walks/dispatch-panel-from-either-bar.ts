// One dispatch panel over one draft, from either bar. The Cockpit's bar grows into it where it stands,
// and the title bar's one bar grows it down from its own field once something is chosen to dispatch.
// A Job, a Session and an Autopilot are the panel's kinds, switched in its band or with Tab. The
// repository is a chip read from the words, asked inline when a Job or an Autopilot has none, and
// optional for a Session. Autopilot reads its settings in the panel and starts from it. Over
// `dispatch-panel-from-either-bar`.

import { button, dialog, inside, region, role, tab, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const chip = (name: string) => inside(frame, button(name));
const act = (name: string) => inside(frame, button(name, { exact: true }));
const kind = (name: string) => inside(frame, tab(name));
const choices = inside(frame, role("listbox", "Repository"));
// The Session behind it has selects of its own, so the palette is named.
const bar = inside(dialog("Command palette"), role("combobox"));
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);

const dispatchPanelFromEitherBar = walk("dispatch-panel-from-either-bar", [
  // (a) The Cockpit's bar, a Job.
  { look: request, say: "The Cockpit's dispatch bar, no repository chosen" },
  { type: "Show the Drone's branch on each Bridge cockpit tile", into: request, say: "The first words, naming Bridge" },
  { look: chip("Repository, finding"), say: "The bar has grown into the panel in place, the words and the cursor kept. The chip's ring turns while the words are read" },
  { look: chip("Repository armada"), say: "Read as the armada repository" },
  { hover: chip("Repository armada"), say: "The tooltip says where it came from" },
  { look: kind("Job"), say: "A Job, with Session and Autopilot beside it" },
  { press: act("Dispatch"), say: "Dispatch sends the request to armada" },
  { look: request, say: "The panel is the bar again, the request empty" },

  // (b) A Session: the repository is optional.
  { type: "Check the migration order", into: request, say: "Words that name no repository" },
  { key: "Tab", on: request, say: "Press Tab" },
  { look: kind("Session"), say: "A Session is chosen" },
  { look: chip("Repository, not found"), say: "The chip is an empty outline, and nothing asks for it" },
  { press: act("Start"), say: "Start opens the Session without a repository" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },

  // (c) The title bar: search first, dispatch from the same words.
  { key: "Meta+k", on: role("textbox", "Message"), say: "Press ⌘K" },
  { type: "Pin the Bridge store", into: bar, say: "Words that also find a Job" },
  { look: role("option", /Pin the Bridge store clock/), say: "Search results show, with the dispatch row after them" },
  { key: "Tab", on: bar, say: "Press Tab: kinds step through Session, Autopilot, Job" },
  { key: "Tab", on: bar, say: "Press Tab again" },
  { look: role("option", /^Dispatch “Pin the Bridge store” as a Job/), say: "The row dispatches the words as a Job" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { look: inPanel(request), say: "The panel grows down from the title bar's field, holding the words" },

  // (d) Autopilot: read in the panel, edited, started.
  { type: "Work through the open Bridge bugs until 7am", into: inPanel(request), say: "The words rewritten as a sentence for Autopilot" },
  { press: inPanel(tab("Autopilot")), say: "Autopilot chosen in the band" },
  { look: inPanel(button("Repository armada")), say: "Armada, read from the words as before" },
  { press: inPanel(button("Read it", { exact: true })), say: "Read it" },
  { look: inPanel(role("textbox", "Instruction")), say: "Fleet's reading is in the panel: the instruction, then At once, Until, Spend and Jobs, each editable" },
  { type: "3", into: inPanel(role("textbox", "At once")), say: "At once changed to 3" },
  { press: inPanel(button("Start Autopilot", { exact: true })), say: "Start Autopilot" },
  { press: inside(role("navigation", "Work"), button("Cockpit")), say: "The panel has gone. Back on the Cockpit" },
  { look: region("Autopilot"), say: "Autopilot is running on armada" },

  // (e) A vague Job: Dispatch asks inline.
  { type: "Clean up the old retry logic", into: request, say: "A request that names no repository, in the Cockpit's bar" },
  { look: chip("Repository, not found"), say: "The chip stays an empty dashed outline" },
  { hover: chip("Repository, not found"), say: "The tooltip names it" },
  { press: act("Dispatch"), say: "Dispatch builds nothing without a repository" },
  { look: choices, say: "The panel asks in place: the frame and band take the call's hue and the set-up repositories list, armada first" },
  { press: inside(choices, role("option", /storefront/)), say: "Picking one sends the request" },
  { look: request, say: "The panel is the bar again, the request empty" },
]);

export { dispatchPanelFromEitherBar as "dispatch-panel-from-either-bar" };
