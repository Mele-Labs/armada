// One dispatch panel over one draft, grown down from the title bar's own field once something is chosen
// to dispatch. A Job, a Session and an Autopilot are the panel's kinds, switched in its band or with Tab.
// The repository is a chip read from the words, and asked inline when the send has none, whatever its
// kind. Autopilot reads its settings in the panel and starts from it.

import { button, dialog, inside, place, region, role, tab, text, walk } from "../walk";

const frame = role("region", "Dispatch", { exact: true });
const request = role("textbox", "Request");
const strip = role("group", "Last dispatch");
const cockpit = region("Jobs and Sessions");
const chip = (name: string) => inside(frame, button(name));
const act = (name: string) => inside(frame, button(name, { exact: true }));
const kind = (name: string) => inside(frame, tab(name));
const choices = inside(frame, role("listbox", "Repository"));
// The Session behind it has selects of its own, so the palette is named.
const bar = inside(dialog("Command palette"), role("combobox"));
const panel = dialog("Dispatch");
const inPanel = (target: ReturnType<typeof role>) => inside(panel, target);

const dispatchPanelFromTheTitleBar = walk("dispatch-panel-from-the-title-bar", [
  // (a) A Job, from the title bar.
  { key: "n", on: cockpit, say: "N opens the title bar's field, Dispatch leading" },
  { type: "Show the Drone's branch on each Bridge cockpit tile", into: bar, say: "The first words, naming Bridge" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { look: inPanel(request), say: "The panel grows down from the title bar's field, holding the words" },
  { look: chip("Repository, finding"), say: "The chip's ring turns while the words are read" },
  { look: chip("Repository armada"), say: "Read as the armada repository" },
  { hover: chip("Repository armada"), say: "The tooltip says where it came from" },
  { look: kind("Job"), say: "A Job, with Session and Autopilot beside it" },
  { press: act("Dispatch"), say: "Dispatch sends the request to armada" },
  { look: strip, say: "The panel has gone and a strip over the field holds the Job's title" },

  // (b) A vague Job: Dispatch asks inline.
  { key: "n", on: strip, say: "N starts the next draft" },
  { type: "Clean up the old retry logic", into: bar, say: "A request that names no repository" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { look: chip("Repository, not found"), say: "The chip stays an empty dashed outline" },
  { hover: chip("Repository, not found"), say: "The tooltip names it" },
  { press: act("Dispatch"), say: "Dispatch builds nothing without a repository" },
  { look: choices, say: "The panel asks in place: the frame and band take the call's hue and the set-up repositories list, armada first" },
  { press: inside(choices, role("option", /storefront/)), say: "Picking one sends the request" },
  { look: strip, say: "A strip holds the Job's title where the panel was" },

  // (c) A Session on All: it starts in a repository, so one naming none asks, as a Job does.
  { key: "n", on: strip, say: "N starts the next draft" },
  { type: "Check the migration order", into: bar, say: "Words that name no repository" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { key: "Tab", on: inPanel(request), say: "Press Tab" },
  { look: kind("Session"), say: "A Session is chosen" },
  { look: chip("Repository, not found"), say: "The chip is an empty outline" },
  { press: act("Start"), say: "Start asks where the Session starts" },
  { look: choices, say: "The panel asks in place, listing the set-up repositories" },
  { press: inside(choices, role("option", /storefront/)), say: "Picking one starts the Session there" },
  { look: role("textbox", "Message"), say: "The Session started and opened, holding what was typed" },

  // (d) The title bar: search first, dispatch from the same words.
  { key: "Meta+k", on: role("textbox", "Message"), say: "Press ⌘K" },
  { type: "Pin the Bridge store", into: bar, say: "Words that also find a Job" },
  { look: role("option", /Pin the Bridge store clock/), say: "Search results show, with the dispatch row after them" },
  { key: "Tab", on: bar, say: "Press Tab: kinds step through Session, Autopilot, Job" },
  { key: "Tab", on: bar, say: "Press Tab again" },
  { look: role("option", /^Dispatch “Pin the Bridge store” as a Job/), say: "The row dispatches the words as a Job" },
  { key: "Meta+Enter", on: bar, say: "Press ⌘↵" },
  { look: inPanel(request), say: "The panel grows down from the title bar's field, holding the words" },

  // (e) Autopilot: read in the panel, edited, started.
  { type: "Work through the open Bridge bugs until 7am", into: inPanel(request), say: "The words rewritten as a sentence for Autopilot" },
  { press: inPanel(tab("Autopilot")), say: "Autopilot chosen in the band" },
  { look: inPanel(button("Repository armada")), say: "Armada, read from the words as before" },
  { press: inPanel(button("Read it", { exact: true })), say: "Read it" },
  { look: inPanel(role("textbox", "Instruction")), say: "Fleet's reading is in the panel: the instruction, then At once, Until, Spend and Jobs, each editable" },
  { type: "3", into: inPanel(role("textbox", "At once")), say: "At once changed to 3" },
  { press: inPanel(button("Start Autopilot", { exact: true })), say: "Start Autopilot" },
  { press: place("Cockpit"), say: "The panel has gone. Back on the Cockpit" },
  { press: inside(region("Autopilot"), button("Repository", { exact: true })), say: "The Autopilot card asks for a repository while none is chosen" },
  { press: role("menuitem", "armada", { exact: true }), say: "Armada" },
  { look: inside(region("Autopilot"), text("Running")), say: "Autopilot is running on armada" },
]);

export { dispatchPanelFromTheTitleBar as "dispatch-panel-from-the-title-bar" };
